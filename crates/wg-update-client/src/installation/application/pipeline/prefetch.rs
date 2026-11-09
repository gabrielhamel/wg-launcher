use super::prefetch_budget::PrefetchBudget;
use super::{InstallationEvent, patch::PatchProgress};
use crate::Result;
use crate::catalog::PlannedPatch;
use crate::installation::infrastructure::filesystem::cache::LocalPatch;
use crate::installation::{DownloadOptions, DownloadReport, VolumeDownloader};
use std::{path::PathBuf, sync::Arc};

// Stop starting patches once this much data is queued ahead of the current
// extraction. One patch may cross the threshold: it must be downloaded in full
// before it can be extracted, even when it is larger than the buffer.
pub(super) const DOWNLOAD_PREFETCH_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub(super) struct PatchDownloads {
    tasks: tokio::task::JoinSet<()>,
    reports: tokio::sync::mpsc::UnboundedReceiver<Result<DownloadedPatch>>,
    applying_patch: tokio::sync::watch::Sender<usize>,
    next_index: usize,
    total: usize,
}

impl PatchDownloads {
    pub(super) fn start(
        downloader: VolumeDownloader,
        patches: Vec<PlannedPatch>,
        cache: PathBuf,
        options: DownloadOptions,
        observer: Arc<dyn Fn(InstallationEvent) + Send + Sync>,
        prefetch_bytes: u64,
    ) -> Self {
        let total = patches.len();
        let (reports_tx, reports) = tokio::sync::mpsc::unbounded_channel();
        let (applying_patch, mut applying_patch_rx) = tokio::sync::watch::channel(0_usize);
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move {
            let budget = PrefetchBudget::new(&patches, prefetch_bytes);
            for (index, planned) in patches.into_iter().enumerate() {
                loop {
                    let applying_patch = *applying_patch_rx.borrow_and_update();
                    if budget.allows(applying_patch, index) {
                        break;
                    }
                    if applying_patch_rx.changed().await.is_err() {
                        return;
                    }
                }
                let progress = Arc::clone(&observer);
                let local = match LocalPatch::from_plan(&planned, &cache) {
                    Ok(local) => local,
                    Err(error) => {
                        let _ = reports_tx.send(Err(error.into()));
                        return;
                    }
                };
                let result = downloader
                    .download_with_observer(
                        planned.into_volumes(),
                        cache.clone(),
                        options.clone(),
                        move |event| {
                            progress(InstallationEvent::Download(event));
                        },
                    )
                    .await;
                let failed = result.is_err();
                if reports_tx
                    .send(
                        result
                            .map(|report| DownloadedPatch {
                                patch: local,
                                report,
                                progress: PatchProgress { index, total },
                            })
                            .map_err(Into::into),
                    )
                    .is_err()
                    || failed
                {
                    return;
                }
            }
        });
        Self {
            tasks,
            reports,
            applying_patch,
            next_index: 0,
            total,
        }
    }

    /// Advance through the planned patches, announcing each position before waiting.
    /// No caller-supplied index can move the prefetch budget ahead of this sequence.
    pub(super) async fn next_ready_patch(&mut self) -> Result<Option<DownloadedPatch>> {
        if self.next_index == self.total {
            return Ok(None);
        }
        self.applying_patch.send_replace(self.next_index);
        match self.reports.recv().await {
            Some(result) => {
                let downloaded = result?;
                self.next_index += 1;
                Ok(Some(downloaded))
            }
            None => {
                let error = match self.tasks.join_next().await {
                    Some(Err(error)) => crate::installation::PipelineError::WorkerFailed(error),
                    _ => crate::installation::PipelineError::DownloadsStopped,
                };
                Err(error.into())
            }
        }
    }

    /// Cancel the download producer and wait for it to stop before releasing the lock.
    pub(super) async fn cancel_and_wait(&mut self) {
        self.tasks.shutdown().await;
    }
}

pub(super) fn merge_download_report(total: &mut DownloadReport, patch: DownloadReport) {
    total.completed_volumes += patch.completed_volumes;
    total.reused_volumes += patch.reused_volumes;
    total.new_bytes_retained += patch.new_bytes_retained;
}

/// A patch becomes ready for application together with its transfer result.
pub(super) struct DownloadedPatch {
    pub(super) patch: LocalPatch,
    pub(super) report: DownloadReport,
    pub(super) progress: PatchProgress,
}
