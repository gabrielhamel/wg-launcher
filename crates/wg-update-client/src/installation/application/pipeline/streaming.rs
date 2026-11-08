//! Overlapping downloads and ordered patch application under one installation lock.

use super::patch::PatchProgress;
use super::prefetch::{DOWNLOAD_PREFETCH_BYTES, PatchDownloads, merge_download_report};
use super::{
    ExtractionEvent, ExtractionOptions, InstallationEvent, InstallationPipeline, InstallationReport,
};
use crate::installation::{DownloadOptions, VolumeDownloader};
use crate::{Error, Result};
use std::sync::Arc;

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
    /// extraction task may continue while retaining the installation lock.
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, invalid patch chains, failed downloads,
    /// filesystem, archive, delta, or background-task failures. On extraction failure,
    /// pending downloads are cancelled before returning the error.
    pub async fn download_and_extract_with_observer<F>(
        self,
        downloader: VolumeDownloader,
        download_options: DownloadOptions,
        extraction_options: ExtractionOptions,
        observer: F,
    ) -> Result<InstallationReport>
    where
        F: Fn(InstallationEvent) + Send + Sync + 'static,
    {
        let (mut session, patches) = self.into_execution();
        let cache = session.layout.cache.clone();
        let _lock = session.retain_lock();
        let observer: Arc<dyn Fn(InstallationEvent) + Send + Sync> = Arc::new(observer);
        let extraction_observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = {
            let observer = Arc::clone(&observer);
            Arc::new(move |event| observer(InstallationEvent::Extraction(event)))
        };

        session = tokio::task::spawn_blocking(move || {
            session.prepare()?;
            Ok::<_, Error>(session)
        })
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

        let finalization_observer = Arc::clone(&extraction_observer);
        let pending_downloads = &mut downloads;
        let result = async move {
            for index in 0..total {
                let downloaded = pending_downloads.ready_for_application(index).await?;
                merge_download_report(&mut report.download, downloaded.report);
                let patch = downloaded.patch;

                let patch_observer = Arc::clone(&extraction_observer);
                let extraction = tokio::task::spawn_blocking(move || {
                    let completion = session.apply_patch(
                        &patch,
                        PatchProgress { index, total },
                        extraction_options,
                        &patch_observer,
                    )?;
                    Ok::<_, Error>((completion, session))
                });
                let (completion, next_session) = extraction
                    .await
                    .map_err(crate::installation::PipelineError::WorkerFailed)??;
                session = next_session;
                report.released_package_bytes += completion.released_package_bytes;
                if completion.released_package_bytes > 0 {
                    observer(InstallationEvent::ReleasedPackages);
                }
            }
            Ok::<_, Error>((session, report))
        }
        .await;
        // Wait for cancellation before releasing the installation lock. The
        // JoinSet also aborts the producer if this entire future is dropped.
        downloads.cancel_and_wait().await;
        let (session, mut report) = result?;

        report.extraction = tokio::task::spawn_blocking(move || {
            session.finalize(extraction_options, finalization_observer)
        })
        .await
        .map_err(crate::installation::PipelineError::WorkerFailed)??;
        Ok(report)
    }
}
