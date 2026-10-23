use crate::{Asset, Error, Result};
use futures_util::{StreamExt, stream};
use reqwest::{Client, StatusCode, header};
use serde::Serialize;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
};

/// HTTP volume transfer policy. Defaults to three concurrent files and five attempts.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Maximum simultaneous file transfers; zero is treated as one.
    pub concurrency: usize,
    /// Maximum attempts per file, including the initial request; zero becomes one.
    /// Retries use a quadratic delay of 500 milliseconds times the attempt squared.
    pub attempts: usize,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            concurrency: 3,
            attempts: 5,
        }
    }
}

/// Progress for an individual transport volume.
///
/// Names identify assets across concurrent transfers. Retried attempts may restart
/// the event sequence. Byte counts describe the current attempt and include resumed
/// prefixes. Size-based cache hits do not verify file contents.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DownloadEvent {
    /// A network response is ready and a transfer attempt is starting.
    Started {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
        /// Existing byte prefix retained for this attempt; zero when starting over.
        resumed_at: u64,
    },
    /// Additional response bytes were written for the current attempt.
    Progress {
        /// Relative transport volume name.
        name: String,
        /// Bytes present so far, including the resumed prefix; not a per-event increment.
        downloaded: u64,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
    /// The volume has the expected size and was promoted to its final filename.
    Completed {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
    /// A final file already had the expected size; no transfer was needed.
    Skipped {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
}

/// Totals for successful downloads and cache hits.
///
/// No report is returned if any selected file ultimately fails.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DownloadReport {
    /// Volumes completed or promoted from complete partial files.
    /// A promoted partial file counts here even when no network request was needed.
    pub downloaded_files: usize,
    /// Volumes whose final files already had the expected size.
    pub skipped_files: usize,
    /// New bytes retained by successful file attempts, excluding resumed prefixes
    /// and skipped volumes. This is not total network traffic across failed attempts.
    pub downloaded_bytes: u64,
}

/// Reusable HTTP downloader for transport volumes.
///
/// Uses expected file sizes to reuse or validate files, without hashing their
/// contents. Does not resolve manifests, pin state, extract packages, or acquire
/// an installation lock. Prefer [`download`](crate::download) for the package workflow.
#[derive(Clone)]
pub struct AssetDownloader {
    http: Client,
}

impl AssetDownloader {
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
            .user_agent(concat!("wg-launcher/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http })
    }

    /// Uses a caller-configured HTTP client.
    ///
    /// The caller controls timeouts, TLS, proxies, and user agent; default constructor
    /// settings are not added to this client.
    pub fn with_http_client(http: Client) -> Self {
        Self { http }
    }

    /// Downloads volumes without progress notifications.
    ///
    /// Uses the same retry, resume, and cache behavior as
    /// [`Self::download_with_observer`]. Does not print terminal output.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::download_with_observer`].
    pub async fn download(
        &self,
        assets: Vec<Asset>,
        output: impl AsRef<Path>,
        options: DownloadOptions,
    ) -> Result<DownloadReport> {
        self.download_with_observer(assets, output, options, |_| {})
            .await
    }

    /// Transfers `assets` beneath `output`, retaining resumable partial files.
    ///
    /// Asset names must be safe relative paths. Matching final file sizes are reused;
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
        assets: Vec<Asset>,
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
            if outcome.skipped {
                report.skipped_files += 1;
            } else {
                report.downloaded_files += 1;
                report.downloaded_bytes += outcome.bytes;
            }
        }
        Ok(report)
    }

    async fn download_with_retries(
        &self,
        asset: Asset,
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

    async fn download_once(
        &self,
        asset: &Asset,
        output: &Path,
        observer: Arc<dyn Fn(DownloadEvent) + Send + Sync>,
    ) -> Result<FileOutcome> {
        let relative = safe_relative_path(&asset.name)?;
        let target = output.join(relative);
        if let Ok(metadata) = fs::metadata(&target).await {
            if metadata.len() == asset.size {
                observer(DownloadEvent::Skipped {
                    name: asset.name.clone(),
                    size: asset.size,
                });
                return Ok(FileOutcome {
                    skipped: true,
                    bytes: 0,
                });
            }
            fs::remove_file(&target).await?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        let partial = target.with_extension(format!(
            "{}.part",
            target
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
        ));
        let mut resumed_at = fs::metadata(&partial)
            .await
            .map(|meta| meta.len())
            .unwrap_or(0);
        if resumed_at > asset.size {
            fs::remove_file(&partial).await?;
            resumed_at = 0;
        }
        if resumed_at == asset.size {
            fs::rename(&partial, &target).await?;
            observer(DownloadEvent::Completed {
                name: asset.name.clone(),
                size: asset.size,
            });
            return Ok(FileOutcome {
                skipped: false,
                bytes: 0,
            });
        }

        let mut request = self.http.get(asset.url.clone());
        if resumed_at > 0 {
            request = request.header(header::RANGE, format!("bytes={resumed_at}-"));
        }
        let response = request.send().await?.error_for_status()?;
        let append = resumed_at > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
        if resumed_at > 0 && !append && response.status() != StatusCode::OK {
            return Err(Error::ResumeRejected {
                path: partial,
                status: response.status().as_u16(),
            });
        }
        if !append {
            resumed_at = 0;
        }

        observer(DownloadEvent::Started {
            name: asset.name.clone(),
            size: asset.size,
            resumed_at,
        });
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&partial)
            .await?;
        let mut writer = BufWriter::new(file);
        let mut downloaded = resumed_at;
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk?;
            writer.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            observer(DownloadEvent::Progress {
                name: asset.name.clone(),
                downloaded,
                size: asset.size,
            });
        }
        writer.flush().await?;
        drop(writer);

        let actual = fs::metadata(&partial).await?.len();
        if actual != asset.size {
            return Err(Error::SizeMismatch {
                path: partial,
                expected: asset.size,
                actual,
            });
        }
        fs::rename(&partial, &target).await?;
        observer(DownloadEvent::Completed {
            name: asset.name.clone(),
            size: asset.size,
        });
        Ok(FileOutcome {
            skipped: false,
            bytes: asset.size - resumed_at,
        })
    }
}

struct FileOutcome {
    skipped: bool,
    bytes: u64,
}

fn safe_relative_path(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::UnsafeAssetPath(name.to_owned()));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_that_can_escape_the_output_directory() {
        assert!(safe_relative_path("../secret").is_err());
        assert!(safe_relative_path("/absolute/file").is_err());
        assert!(safe_relative_path("volume/file.wgpkg").is_ok());
    }
}
