use super::InstallationEvent;
use crate::{Asset, AssetDownloader, DownloadOptions, DownloadReport, Error, Result};
use std::{path::PathBuf, sync::Arc};

// Stop starting patches once this much data is queued ahead of the current
// extraction. One patch may cross the threshold: it must be downloaded in full
// before it can be extracted, even when it is larger than the buffer.
pub(super) const DOWNLOAD_PREFETCH_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub(super) struct PatchDownloads {
    tasks: tokio::task::JoinSet<()>,
    reports: tokio::sync::mpsc::UnboundedReceiver<Result<DownloadReport>>,
    current: tokio::sync::watch::Sender<usize>,
}

impl PatchDownloads {
    pub(super) fn start(
        downloader: AssetDownloader,
        patches: Vec<Vec<Asset>>,
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
                .map(|assets| {
                    assets
                        .iter()
                        .map(|asset| u128::from(asset.size))
                        .sum::<u128>()
                })
                .collect::<Vec<_>>();
            for (index, assets) in patches.into_iter().enumerate() {
                loop {
                    let current = *current_rx.borrow_and_update();
                    let queued_bytes = sizes[current.saturating_add(1).min(index)..index]
                        .iter()
                        .sum::<u128>();
                    if index <= current || queued_bytes < u128::from(prefetch_bytes) {
                        break;
                    }
                    if current_rx.changed().await.is_err() {
                        return;
                    }
                }
                let progress = Arc::clone(&observer);
                let result = downloader
                    .download_with_observer(assets, cache.clone(), options.clone(), move |event| {
                        progress(InstallationEvent::Download(event));
                    })
                    .await;
                let failed = result.is_err();
                if reports_tx.send(result).is_err() || failed {
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

    pub(super) async fn next(&mut self, index: usize) -> Result<DownloadReport> {
        self.current.send_replace(index);
        match self.reports.recv().await {
            Some(result) => result,
            None => {
                let detail = match self.tasks.join_next().await {
                    Some(Err(error)) => error.to_string(),
                    _ => "Package downloads stopped before all packages were ready".to_owned(),
                };
                Err(Error::ExtractionTask(detail))
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
