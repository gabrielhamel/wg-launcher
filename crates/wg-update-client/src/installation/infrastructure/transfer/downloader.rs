//! Concurrent volume downloads, retries, and transfer outcome aggregation.

use super::error::{Result, TransferError as Error};
use super::{DownloadEvent, DownloadOptions, DownloadReport};
use crate::catalog::TransportVolume;
use futures_util::{StreamExt, stream};
use reqwest::Client;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// Reusable HTTP downloader for transport volumes.
///
/// Uses expected file sizes to reuse or validate files, without hashing their
/// contents. Does not resolve manifests, pin state, extract packages, or acquire
/// an installation lock. Prefer [`download`](crate::download) for the package workflow.
#[derive(Clone)]
pub struct VolumeDownloader {
    pub(super) http: Client,
}

impl VolumeDownloader {
    /// Creates a downloader with a 15-second connection timeout, 60-second read
    /// timeout, and the crate version in its user agent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if the HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http })
    }

    /// Transfers `assets` beneath `output`, retaining resumable partial files.
    ///
    /// TransportVolume names must be safe relative paths. Matching final file sizes are reused;
    /// incomplete final files are removed. Partial files use a `.part` suffix and HTTP
    /// range requests; a server returning a full response causes a restart.
    /// No installation lock is acquired; callers must coordinate destination access.
    ///
    /// Observers can run concurrently across files. Retries may emit repeated start
    /// events, and byte progress includes the resumed prefix for that attempt. The
    /// method waits for all selected transfers before reporting any failure.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DownloadRetries`] with the final per-file error when attempts
    /// are exhausted, including unsafe paths, I/O, HTTP, and size mismatch failures.
    /// Successfully completed files and partial files are retained after failure.
    pub async fn download_with_observer<F>(
        &self,
        assets: Vec<TransportVolume>,
        output: impl AsRef<Path>,
        options: DownloadOptions,
        observer: F,
    ) -> Result<DownloadReport>
    where
        F: Fn(DownloadEvent) + Send + Sync + 'static,
    {
        let output = output.as_ref().to_path_buf();
        let observer: Arc<dyn Fn(DownloadEvent) + Send + Sync> = Arc::new(observer);
        let concurrency = options.concurrency.max(1);
        let attempts = options.attempts.max(1);
        let downloader = self.clone();

        let results = stream::iter(assets.into_iter().map(|asset| {
            let output = output.clone();
            let observer = Arc::clone(&observer);
            let downloader = downloader.clone();
            async move {
                downloader
                    .download_with_retries(asset, output, attempts, observer)
                    .await
            }
        }))
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

        let mut report = DownloadReport::default();
        for result in results {
            let outcome = result?;
            match outcome {
                FileOutcome::Reused => report.skipped_files += 1,
                FileOutcome::Downloaded { new_bytes } => {
                    report.downloaded_files += 1;
                    report.downloaded_bytes += new_bytes;
                }
            }
        }
        Ok(report)
    }

    async fn download_with_retries(
        &self,
        asset: TransportVolume,
        output: PathBuf,
        attempts: usize,
        observer: Arc<dyn Fn(DownloadEvent) + Send + Sync>,
    ) -> Result<FileOutcome> {
        let mut last_error = None;
        for attempt in 1..=attempts {
            match self
                .download_once(&asset, &output, Arc::clone(&observer))
                .await
            {
                Ok(outcome) => return Ok(outcome),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < attempts {
                        tokio::time::sleep(Duration::from_millis(
                            500 * attempt as u64 * attempt as u64,
                        ))
                        .await;
                    }
                }
            }
        }
        Err(Error::DownloadRetries {
            attempts,
            url: asset.url,
            source: Box::new(last_error.expect("at least one download attempt")),
        })
    }
}

/// A successful volume attempt, distinguishing reuse from bytes newly retained.
pub(super) enum FileOutcome {
    Reused,
    Downloaded { new_bytes: u64 },
}
