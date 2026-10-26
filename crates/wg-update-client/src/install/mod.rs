//! Ordered installation, archive extraction, and local installation storage.

mod archive;
mod chain;
mod delta;
mod extraction;
mod paths;
mod prefetch;
mod storage;
mod types;

pub use chain::installation_assets;
pub(crate) use storage::installation_storage;
pub use storage::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, INSTALL_MARKER_FILE, STAGING_DIRECTORY,
};
pub use types::{
    ExtractionEvent, ExtractionOptions, ExtractionReport, InstallationEvent, InstallationReport,
    InstalledBuild,
};

use crate::{AssetDownloader, DownloadOptions, Error, PatchManifest, Result};
use chain::{assets_for_patches, ordered_patches};
use extraction::{InstallationPaths, extract_patch_sync, extract_sync, finalize_installation};
use prefetch::{DOWNLOAD_PREFETCH_BYTES, PatchDownloads, merge_download_report};
use std::{path::Path, sync::Arc};
use storage::{InstallationLock, prepare_installation, remove_patch_volumes};

/// In-process transport archive extraction and delta application.
///
/// Acquires the per-installation lock and runs blocking archive/filesystem work
/// on Tokio's blocking pool. Game-owned inner `.pkg` files remain packed. These
/// lower-level methods accept a resolved manifest; they do not discover or pin it.
/// An error can leave partially changed game files; no whole-install rollback is provided.
#[derive(Debug, Clone, Copy, Default)]
pub struct PackageExtractor;

impl PackageExtractor {
    /// Creates a stateless package extractor.
    pub const fn new() -> Self {
        Self
    }

    /// Extracts cached packages without progress notifications and with default cleanup.
    ///
    /// See [`Self::extract_with_observer`] for the installation lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::extract_with_observer`].
    pub async fn extract(
        &self,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
    ) -> Result<ExtractionReport> {
        self.extract_with_observer(manifest, install_root, |_| {})
            .await
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
        manifest: PatchManifest,
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

    /// Extracts cached packages with a cache-retention policy and no progress notifications.
    ///
    /// See [`Self::extract_with_options_and_observer`] for the installation lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::extract_with_options_and_observer`].
    pub async fn extract_with_options(
        &self,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
        options: ExtractionOptions,
    ) -> Result<ExtractionReport> {
        self.extract_with_options_and_observer(manifest, install_root, options, |_| {})
            .await
    }

    /// Extracts local packages with an explicit cache-retention policy.
    ///
    /// Acquires the installation lock, verifies required volume sizes, rebuilds staging
    /// work, applies ordered patches, and records the installed build. Does not load
    /// or save a pinned manifest. Callbacks run on the blocking extraction worker;
    /// see the [observer contract](crate#observers-and-async-execution).
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, incomplete cache, invalid patch chains or
    /// paths, filesystem failures, invalid archives, failed deltas, or worker failure.
    /// Cache verification uses sizes rather than content hashes.
    pub async fn extract_with_options_and_observer<F>(
        &self,
        manifest: PatchManifest,
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
        .map_err(|error| Error::ExtractionTask(error.to_string()))?
    }

    /// Streams downloads and extraction without progress notifications.
    ///
    /// See [`Self::download_and_extract_with_observer`] for the installation lifecycle.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::download_and_extract_with_observer`].
    pub async fn download_and_extract(
        &self,
        downloader: AssetDownloader,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
        download_options: DownloadOptions,
        extraction_options: ExtractionOptions,
    ) -> Result<InstallationReport> {
        self.download_and_extract_with_observer(
            downloader,
            manifest,
            install_root,
            download_options,
            extraction_options,
            |_| {},
        )
        .await
    }

    /// Streams package downloads and ordered extraction under the installation lock.
    ///
    /// Downloads later patches while extracting the current patch, with bounded
    /// prefetching. A complete patch may exceed the prefetch threshold. Cache cleanup
    /// controls both early volume release after successful patches and final removal.
    /// Writes the installed-build record on success, but does not pin the supplied
    /// manifest; high-level [`install`](crate::install) handles manifest persistence.
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
        manifest: PatchManifest,
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
        let patches = ordered_patches(&manifest)?
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let assets = assets_for_patches(&manifest, &patches)?;
        if assets.is_empty() {
            return Err(Error::MissingPatchChain(manifest.game_id.clone()));
        }
        let observer: Arc<dyn Fn(InstallationEvent) + Send + Sync> = Arc::new(observer);
        let extraction_observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = {
            let observer = Arc::clone(&observer);
            Arc::new(move |event| observer(InstallationEvent::Extraction(event)))
        };

        let game = install_root.clone();
        let work = storage.work.clone();
        let prepare_root = install_root.clone();
        let prepare_storage = storage.clone();
        tokio::task::spawn_blocking(move || prepare_installation(&prepare_root, &prepare_storage))
            .await
            .map_err(|error| Error::ExtractionTask(error.to_string()))??;

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

                let cache_for_patch = cache.clone();
                let game_for_patch = game.clone();
                let work_for_patch = work.clone();
                let patch_observer = Arc::clone(&extraction_observer);
                let extraction = tokio::task::spawn_blocking(move || {
                    let paths = InstallationPaths {
                        cache: &cache_for_patch,
                        game: &game_for_patch,
                        work: &work_for_patch,
                    };
                    let deltas = extract_patch_sync(&patch, paths, index, total, &patch_observer)?;
                    let released = if extraction_options.clean_download_cache {
                        remove_patch_volumes(&cache_for_patch, &patch)?
                    } else {
                        0
                    };
                    Ok::<_, Error>((deltas, released))
                });
                let (deltas, released) = extraction
                    .await
                    .map_err(|error| Error::ExtractionTask(error.to_string()))??;
                report.extraction.applied_deltas += deltas;
                report.released_package_bytes += released;
                if released > 0 {
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
        .map_err(|error| Error::ExtractionTask(error.to_string()))??;
        Ok(report)
    }
}
