use super::InstallationEvent;
use crate::Result;
use crate::catalog::{Patch, PlannedPatch};
use crate::installation::{AssetDownloader, DownloadOptions, DownloadReport};
use std::{path::PathBuf, sync::Arc};

// Stop starting patches once this much data is queued ahead of the current
// extraction. One patch may cross the threshold: it must be downloaded in full
// before it can be extracted, even when it is larger than the buffer.
pub(super) const DOWNLOAD_PREFETCH_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub(super) struct PatchDownloads {
    tasks: tokio::task::JoinSet<()>,
    reports: tokio::sync::mpsc::UnboundedReceiver<Result<DownloadedPatch>>,
    current: tokio::sync::watch::Sender<usize>,
}

impl PatchDownloads {
    pub(super) fn start(
        downloader: AssetDownloader,
        patches: Vec<PlannedPatch>,
        cache: PathBuf,
        options: DownloadOptions,
        observer: Arc<dyn Fn(InstallationEvent) + Send + Sync>,
        prefetch_bytes: u64,
    ) -> Self {
        let (reports_tx, reports) = tokio::sync::mpsc::unbounded_channel();
        let (current, mut current_rx) = tokio::sync::watch::channel(0_usize);
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move {
            let sizes = patches
                .iter()
                .map(|planned| {
                    planned
                        .packages
                        .iter()
                        .map(|asset| u128::from(asset.size))
                        .sum::<u128>()
                })
                .collect::<Vec<_>>();
            for (index, planned) in patches.into_iter().enumerate() {
                loop {
                    let current = *current_rx.borrow_and_update();
                    if can_prefetch_patch(&sizes, current, index, prefetch_bytes) {
                        break;
                    }
                    if current_rx.changed().await.is_err() {
                        return;
                    }
                }
                let progress = Arc::clone(&observer);
                let result = downloader
                    .download_with_observer(
                        planned.packages,
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
                                patch: planned.patch,
                                report,
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
            current,
        }
    }

    pub(super) async fn next(&mut self, index: usize) -> Result<DownloadedPatch> {
        self.current.send_replace(index);
        match self.reports.recv().await {
            Some(result) => result,
            None => {
                let error = match self.tasks.join_next().await {
                    Some(Err(error)) => crate::installation::PipelineError::WorkerFailed(error),
                    _ => crate::installation::PipelineError::DownloadsStopped,
                };
                Err(error.into())
            }
        }
    }

    pub(super) async fn stop(&mut self) {
        self.tasks.shutdown().await;
    }
}

pub(super) fn merge_download_report(total: &mut DownloadReport, patch: DownloadReport) {
    total.downloaded_files += patch.downloaded_files;
    total.skipped_files += patch.skipped_files;
    total.downloaded_bytes += patch.downloaded_bytes;
}

/// A patch becomes ready for application together with its transfer result.
pub(super) struct DownloadedPatch {
    pub(super) patch: Patch,
    pub(super) report: DownloadReport,
}

/// The current patch is always allowed. Later patches may start while the packages
/// already queued ahead of extraction are below the budget. The new patch's own
/// size is excluded so a single large patch can cross the budget and still finish.
fn can_prefetch_patch(sizes: &[u128], current: usize, next: usize, budget: u64) -> bool {
    let queued_bytes = sizes[current.saturating_add(1).min(next)..next]
        .iter()
        .sum::<u128>();
    next <= current || queued_bytes < u128::from(budget)
}
