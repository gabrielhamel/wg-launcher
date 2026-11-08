//! Overlapping downloads and ordered patch application under one installation lock.

use super::finalization::finalize_installation;
use super::patch::{PatchProgress, apply_and_checkpoint_patch};
use super::prefetch::{DOWNLOAD_PREFETCH_BYTES, PatchDownloads, merge_download_report};
use super::{
    ExtractionEvent, ExtractionOptions, InstallationEvent, InstallationPipeline, InstallationReport,
};
use crate::installation::infrastructure::filesystem::{
    InstallationLayout, InstallationLock, records, storage::prepare_installation,
};
use crate::installation::{AssetDownloader, DownloadOptions, PinnedManifest};
use crate::{Error, Result};
use std::{path::Path, sync::Arc};

impl InstallationPipeline {
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
        let storage = InstallationLayout::for_root(&install_root);
        let _lock = InstallationLock::acquire(&storage)?;
        let cache = storage.cache.clone();
        let mut manifest = super::resume_manifest(manifest, &storage)?;
        let patches = manifest.planned_patches()?;
        records::save_manifest(&manifest, &storage)?;
        let observer: Arc<dyn Fn(InstallationEvent) + Send + Sync> = Arc::new(observer);
        let extraction_observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = {
            let observer = Arc::clone(&observer);
            Arc::new(move |event| observer(InstallationEvent::Extraction(event)))
        };

        let prepare_storage = storage.clone();
        tokio::task::spawn_blocking(move || prepare_installation(&prepare_storage))
            .await
            .map_err(crate::installation::PipelineError::WorkerFailed)??;

        let total = patches.len();
        let mut report = InstallationReport::default();
        let mut downloads = PatchDownloads::start(
            downloader,
            patches,
            cache.clone(),
            download_options,
            Arc::clone(&observer),
            DOWNLOAD_PREFETCH_BYTES,
        );

        let result = async {
            for index in 0..total {
                let downloaded = downloads.next(index).await?;
                merge_download_report(&mut report.download, downloaded.report);
                let patch = downloaded.patch;

                let patch_observer = Arc::clone(&extraction_observer);
                let mut checkpoint_manifest = manifest.clone();
                let checkpoint_storage = storage.clone();
                let extraction = tokio::task::spawn_blocking(move || {
                    let completion = apply_and_checkpoint_patch(
                        &mut checkpoint_manifest,
                        &patch,
                        &checkpoint_storage,
                        PatchProgress { index, total },
                        extraction_options,
                        &patch_observer,
                    )?;
                    Ok::<_, Error>((completion, checkpoint_manifest))
                });
                let (completion, checkpoint_manifest) = extraction
                    .await
                    .map_err(crate::installation::PipelineError::WorkerFailed)??;
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

        let finalize_manifest = manifest.clone();
        let finalize_storage = storage.clone();
        let finalize_observer = Arc::clone(&extraction_observer);
        report.extraction = tokio::task::spawn_blocking(move || {
            finalize_installation(
                &finalize_manifest,
                &finalize_storage,
                extraction_options,
                report.extraction.applied_deltas,
                finalize_observer,
            )
        })
        .await
        .map_err(crate::installation::PipelineError::WorkerFailed)??;
        Ok(report)
    }
}
