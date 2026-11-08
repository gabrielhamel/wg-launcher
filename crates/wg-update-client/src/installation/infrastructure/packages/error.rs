use crate::catalog::ReleaseError;
use crate::installation::StorageError;
use std::path::PathBuf;

/// Failures owned by archive and delta application.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// Filesystem or other I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Transport archive decoding or entry extraction failed.
    #[error("package archive extraction failed: {0}")]
    ArchiveExtraction(#[from] sevenz_rust::Error),
    /// A delta could not be decoded or applied.
    #[error("failed to apply {format} delta {path:?}: {detail}")]
    DeltaDecode {
        /// Delta format identifier.
        format: &'static str,
        /// Affected local file path.
        path: PathBuf,
        /// Decoder or validation failure description.
        detail: String,
    },
    /// A delta package entry did not describe a valid target path.
    #[error("invalid delta path in package: {0}")]
    InvalidDeltaPath(String),
    /// The game file required as a delta source was missing.
    #[error("delta target does not exist: {0:?}")]
    MissingDeltaTarget(PathBuf),
    /// A delta decoder unexpectedly produced an empty output file.
    #[error("delta decoder produced no output: {0:?}")]
    EmptyDeltaOutput(PathBuf),
    /// Failure from the underlying release operation.
    #[error(transparent)]
    Release(#[from] ReleaseError),
    /// Failure from the underlying storage operation.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub(crate) type Result<T> = std::result::Result<T, PackageError>;
