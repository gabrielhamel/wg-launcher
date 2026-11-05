use std::path::PathBuf;

/// Failures owned by package transfer.
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    /// Invalid local path or other installation storage failure.
    #[error(transparent)]
    Storage(#[from] crate::installation::StorageError),
    /// HTTP client creation, request, status, or response-stream failure.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Filesystem or other I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A completed transfer did not match the advertised byte size.
    #[error("downloaded file has the wrong size: {path:?} (expected {expected}, got {actual})")]
    SizeMismatch {
        /// Affected local file path.
        path: PathBuf,
        /// Advertised or required byte size.
        expected: u64,
        /// Observed byte size; zero can indicate a missing cache file.
        actual: u64,
    },
    /// A resumed request received an unacceptable HTTP response.
    #[error("invalid response while resuming {path:?}: HTTP {status}")]
    ResumeRejected {
        /// Affected local file path.
        path: PathBuf,
        /// HTTP status code returned by the server.
        status: u16,
    },
    /// A volume failed after exhausting its retry budget.
    #[error("download failed after {attempts} attempts: {url}: {source}")]
    DownloadRetries {
        /// Number of attempts performed.
        attempts: usize,
        /// Download URL that failed.
        url: url::Url,
        /// Final error from the last transfer attempt.
        #[source]
        source: Box<TransferError>,
    },
}

pub(crate) type Result<T> = std::result::Result<T, TransferError>;
