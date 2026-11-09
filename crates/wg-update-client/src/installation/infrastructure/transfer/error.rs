use crate::ErrorKind;
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

impl TransferError {
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            TransferError::Storage(error) => error.kind(),
            TransferError::Io(_) => ErrorKind::Io,
            TransferError::SizeMismatch { .. } => ErrorKind::InvalidPackage,
            TransferError::DownloadRetries { source, .. } => source.kind(),
            TransferError::Http(_) | TransferError::ResumeRejected { .. } => ErrorKind::Network,
        }
    }
}

impl From<TransferError> for crate::Error {
    fn from(error: TransferError) -> Self {
        Self::new(error.kind(), error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installation::StorageError;
    use crate::{Error, ErrorKind};

    #[test]
    fn retry_wrappers_preserve_the_category_and_diagnostic_cause() {
        for (cause, expected) in [
            (
                TransferError::Storage(StorageError::UnsafeAssetPath("../volume".into())),
                ErrorKind::UnsafePath,
            ),
            (
                TransferError::Storage(StorageError::IncompleteCache {
                    path: "volume".into(),
                    expected: 10,
                    actual: 3,
                }),
                ErrorKind::IncompleteCache,
            ),
            (
                TransferError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "cache is read-only",
                )),
                ErrorKind::Io,
            ),
        ] {
            let diagnostic = cause.to_string();
            let mut original: &dyn std::error::Error = &cause;
            while let Some(next) = original.source() {
                original = next;
            }
            let original_diagnostic = original.to_string();
            let mut wrapped = cause;
            for _ in 0..2 {
                wrapped = TransferError::DownloadRetries {
                    attempts: 3,
                    url: "http://localhost/volume".parse().unwrap(),
                    source: Box::new(wrapped),
                };
            }
            let error = Error::from(wrapped);
            assert_eq!(error.kind(), expected);
            assert!(error.to_string().contains(&diagnostic));
            let mut source = std::error::Error::source(&error).unwrap();
            while let Some(next) = source.source() {
                source = next;
            }
            assert_eq!(source.to_string(), original_diagnostic);
        }
    }
}
