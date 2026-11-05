use crate::{
    CatalogError, DownloadError, ExtractError, InstallationError, PackageError, PipelineError,
    ReleaseError, StatusError, StorageError, TransferError, WgusError,
};

/// Failure returned by a high-level launcher workflow.
///
/// Match the owning operation, then its concrete error. Display messages and
/// source chains preserve the underlying diagnostic.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failure owned by catalog behavior.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// Failure owned by release behavior.
    #[error(transparent)]
    Release(#[from] ReleaseError),
    /// Failure owned by installation behavior.
    #[error(transparent)]
    Installation(#[from] InstallationError),
    /// Failure owned by wgus behavior.
    #[error(transparent)]
    Wgus(#[from] WgusError),
    /// Failure owned by transfer behavior.
    #[error(transparent)]
    Transfer(#[from] TransferError),
    /// Failure owned by package behavior.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// Failure owned by storage behavior.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Failure owned by download behavior.
    #[error(transparent)]
    Download(#[from] DownloadError),
    /// Failure owned by extract behavior.
    #[error(transparent)]
    Extract(#[from] ExtractError),
    /// Failure owned by status behavior.
    #[error(transparent)]
    Status(#[from] StatusError),
    /// Failure owned by pipeline behavior.
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
}

// Filesystem operations at workflow boundaries use the storage error contract.
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error.into())
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}

// Preserve convenient conversion of protocol failures at consumer boundaries.
impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Self::Wgus(error.into())
    }
}
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Wgus(error.into())
    }
}
impl From<quick_xml::DeError> for Error {
    fn from(error: quick_xml::DeError) -> Self {
        Self::Wgus(error.into())
    }
}

/// Result of a high-level launcher workflow.
pub type Result<T> = std::result::Result<T, Error>;
