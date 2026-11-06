//! Shared streaming and local installation orchestration.

mod extraction;
mod patch;
mod prefetch;
mod types;
use crate::catalog::assets_for_patches;
use crate::installation::infrastructure::filesystem::{
    InstallationLock, installation_storage, records, retry, storage::prepare_installation,
};
use crate::installation::{AssetDownloader, DownloadOptions, PinnedManifest};
use crate::{Error, Result};
use extraction::{extract_sync, finalize_installation};
use patch::{PatchProgress, apply_and_checkpoint_patch};
use prefetch::{DOWNLOAD_PREFETCH_BYTES, PatchDownloads, merge_download_report};
use std::{path::Path, sync::Arc};
pub use types::{
    ExtractionEvent, ExtractionOptions, ExtractionReport, InstallationEvent, InstallationReport,
};

/// In-process transport archive extraction and delta application.
///
/// Acquires the per-installation lock and runs blocking archive/filesystem work
/// on Tokio's blocking pool. Game-owned inner `.pkg` files remain packed. These
/// lower-level methods accept and pin a resolved manifest without discovery.
/// An error can leave partially changed game files; no whole-install rollback is provided.
#[derive(Debug, Clone, Copy, Default)]
pub struct PackageExtractor;

impl PackageExtractor {
    /// Creates a stateless package extractor.
    pub const fn new() -> Self {
        Self
    }

    /// Extracts an already downloaded manifest with default cache cleanup.
    ///
    /// Equivalent to [`Self::extract_with_options_and_observer`] with [`ExtractionOptions::default`].
    /// Reads volumes from `install_root/.wg-launcher/download` without network access and writes
    /// the installed-build record on success.
    ///
    /// # Errors
    ///
    /// Returns the lock, cache, patch-chain, filesystem, archive, delta, or worker
    /// errors described by [`Self::extract_with_options_and_observer`].
    pub async fn extract_with_observer<F>(
        &self,
        manifest: PinnedManifest,
        install_root: impl AsRef<Path>,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        self.extract_with_options_and_observer(
            manifest,
            install_root,
            ExtractionOptions::default(),
            observer,
        )
        .await
    }

    /// Extracts local packages with an explicit cache-retention policy.
    ///
    /// Acquires the installation lock, verifies required volume sizes, applies the
    /// remaining patches, and records the installed build. Pins the supplied manifest
    /// and preserves completed checkpoints and original delta sources on retry.
    /// Callbacks run on the blocking extraction worker; see the
    /// [observer contract](crate#observers-and-async-execution).
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, incomplete cache, invalid patch chains or
    /// paths, filesystem failures, invalid archives, failed deltas, or worker failure.
    /// Cache verification uses sizes rather than content hashes.
    pub async fn extract_with_options_and_observer<F>(
        &self,
        manifest: PinnedManifest,
        install_root: impl AsRef<Path>,
        options: ExtractionOptions,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        let install_root = install_root.as_ref().to_path_buf();
        let storage = installation_storage(&install_root)?;
        let _lock = InstallationLock::acquire(&storage, &install_root)?;
        let observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = Arc::new(observer);
        tokio::task::spawn_blocking(move || {
            extract_sync(&manifest, &install_root, &storage, options, observer)
        })
        .await
        .map_err(|error| crate::installation::PipelineError::ExtractionTask(error.to_string()))?
    }

    /// Streams package downloads and ordered extraction under the installation lock.
    ///
    /// Downloads later patches while extracting the current patch, with bounded
    /// prefetching. A complete patch may exceed the prefetch threshold. Cache cleanup
    /// controls both early volume release after successful patches and final removal.
    /// Pins the manifest under the lock and checkpoints component progress before
    /// releasing packages. Writes the installed-build record after all patches succeed.
    ///
    /// Callbacks may run concurrently on download and blocking extraction workers.
    /// Dropping the future stops pending async download work, but a started blocking
    /// extraction task may continue; await completion before reusing its destination.
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, invalid patch chains, failed downloads,
    /// filesystem, archive, delta, or background-task failures. On extraction failure,
    /// pending downloads are cancelled before returning the error.
    pub async fn download_and_extract_with_observer<F>(
        &self,
        downloader: AssetDownloader,
        manifest: PinnedManifest,
        install_root: impl AsRef<Path>,
        download_options: DownloadOptions,
        extraction_options: ExtractionOptions,
        observer: F,
    ) -> Result<InstallationReport>
    where
        F: Fn(InstallationEvent) + Send + Sync + 'static,
    {
        let install_root = install_root.as_ref().to_path_buf();
        let storage = installation_storage(&install_root)?;
        let _lock = InstallationLock::acquire(&storage, &install_root)?;
        let cache = storage.cache.clone();
        let mut manifest = retry::resume_manifest(manifest, &storage)?;
        let patches = manifest
            .ordered_patches()?
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let assets = assets_for_patches(&manifest.release, &patches)?;
        records::save_manifest(&manifest, &storage)?;
        let observer: Arc<dyn Fn(InstallationEvent) + Send + Sync> = Arc::new(observer);
        let extraction_observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = {
            let observer = Arc::clone(&observer);
            Arc::new(move |event| observer(InstallationEvent::Extraction(event)))
        };

        let prepare_root = install_root.clone();
        let prepare_storage = storage.clone();
        tokio::task::spawn_blocking(move || prepare_installation(&prepare_root, &prepare_storage))
            .await
            .map_err(|error| {
                crate::installation::PipelineError::ExtractionTask(error.to_string())
            })??;

        let total = patches.len();
        let mut report = InstallationReport::default();
        let mut downloads = PatchDownloads::start(
            downloader,
            assets,
            cache.clone(),
            download_options,
            Arc::clone(&observer),
            DOWNLOAD_PREFETCH_BYTES,
        );

        let result = async {
            for (index, patch) in patches.into_iter().enumerate() {
                let downloaded = downloads.next(index).await?;
                merge_download_report(&mut report.download, downloaded);

                let game_for_patch = install_root.clone();
                let patch_observer = Arc::clone(&extraction_observer);
                let mut checkpoint_manifest = manifest.clone();
                let checkpoint_storage = storage.clone();
                let extraction = tokio::task::spawn_blocking(move || {
                    let completion = apply_and_checkpoint_patch(
                        &mut checkpoint_manifest,
                        &patch,
                        &checkpoint_storage,
                        &game_for_patch,
                        PatchProgress { index, total },
                        extraction_options,
                        &patch_observer,
                    )?;
                    Ok::<_, Error>((completion, checkpoint_manifest))
                });
                let (completion, checkpoint_manifest) = extraction.await.map_err(|error| {
                    crate::installation::PipelineError::ExtractionTask(error.to_string())
                })??;
                manifest = checkpoint_manifest;
                report.extraction.applied_deltas += completion.applied_deltas;
                report.released_package_bytes += completion.released_package_bytes;
                if completion.released_package_bytes > 0 {
                    observer(InstallationEvent::ReleasedPackages);
                }
            }
            Ok::<(), Error>(())
        }
        .await;
        // Wait for cancellation before releasing the installation lock. The
        // JoinSet also aborts the producer if this entire future is dropped.
        downloads.stop().await;
        result?;

        let finalize_root = install_root.clone();
        let finalize_manifest = manifest.clone();
        let finalize_storage = storage.clone();
        let finalize_observer = Arc::clone(&extraction_observer);
        report.extraction = tokio::task::spawn_blocking(move || {
            finalize_installation(
                &finalize_manifest,
                &finalize_root,
                &finalize_storage,
                extraction_options,
                report.extraction.applied_deltas,
                finalize_observer,
            )
        })
        .await
        .map_err(|error| crate::installation::PipelineError::ExtractionTask(error.to_string()))??;
        Ok(report)
    }
}

/// Failure of a background installation worker.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// A background extraction or download-coordination task failed.
    #[error("background extraction task failed: {0}")]
    ExtractionTask(String),
}
