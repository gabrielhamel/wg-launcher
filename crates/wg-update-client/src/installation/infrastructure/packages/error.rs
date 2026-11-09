use crate::ErrorKind;
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
    /// The selected patch has no volumes to decode as an archive.
    #[error("cannot decode patch for {component}: no archive volumes were provided")]
    MissingArchiveVolumes {
        /// Component whose patch has no archive volumes.
        component: String,
    },
    /// Failure from the underlying storage operation.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub(crate) type Result<T> = std::result::Result<T, PackageError>;

impl PackageError {
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            PackageError::Io(_) => ErrorKind::Io,
            PackageError::Storage(error) => error.kind(),
            PackageError::InvalidDeltaPath(_) => ErrorKind::UnsafePath,
            PackageError::MissingArchiveVolumes { .. }
            | PackageError::ArchiveExtraction(_)
            | PackageError::DeltaDecode { .. }
            | PackageError::MissingDeltaTarget(_)
            | PackageError::EmptyDeltaOutput(_) => ErrorKind::InvalidPackage,
        }
    }
}

impl From<PackageError> for crate::Error {
    fn from(error: PackageError) -> Self {
        Self::new(error.kind(), error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn missing_archive_volumes_are_invalid_packages_with_a_package_diagnostic() {
        let error = Error::from(PackageError::MissingArchiveVolumes {
            component: "client".into(),
        });
        assert_eq!(error.kind(), ErrorKind::InvalidPackage);
        assert_eq!(
            error.to_string(),
            "cannot decode patch for client: no archive volumes were provided"
        );
    }
}
