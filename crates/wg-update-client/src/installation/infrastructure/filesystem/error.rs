use crate::installation::InstallationError;
use std::path::PathBuf;

/// Failures owned by installation filesystem storage.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Filesystem or other I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Product configuration, pinned manifest, or installed-build JSON is invalid.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A transport or archive path was unsafe for local extraction.
    #[error("unsafe asset path returned by WGUS: {0}")]
    UnsafeAssetPath(String),
    /// A required cached volume was missing or had the wrong size.
    #[error(
        "cached asset is missing or incomplete: {path:?} (expected {expected} bytes, got {actual})"
    )]
    IncompleteCache {
        /// Affected local file path.
        path: PathBuf,
        /// Advertised or required byte size.
        expected: u64,
        /// Observed byte size; zero can indicate a missing cache file.
        actual: u64,
    },
    /// Another process holds the installation mutation lock.
    #[error("another installation is already running for {0:?}")]
    InstallationLocked(PathBuf),
    /// Failure from the underlying installation operation.
    #[error(transparent)]
    Installation(#[from] InstallationError),
}

pub(crate) type Result<T> = std::result::Result<T, StorageError>;
