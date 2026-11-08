use crate::catalog::{CatalogError, ReleaseError, WgusError};
use crate::installation::{
    DownloadError, ExtractError, InstallationError, InstalledBuildError, PackageError,
    PipelineError, StorageError, TransferError,
};

/// Actionable category of a workflow failure, independent of internal implementation layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The requested product, component, client type, or language is invalid.
    InvalidSelection,
    /// No installed build identity exists at the requested path.
    InstallationNotFound,
    /// No pinned download exists for local extraction.
    DownloadNotFound,
    /// Another process is mutating this installation.
    InstallationLocked,
    /// Cached packages belong to a different selection or product.
    IncompatibleCache,
    /// Required cached packages are missing or incomplete.
    IncompleteCache,
    /// A network request or resumed transfer failed.
    Network,
    /// The publisher returned invalid metadata or an unusable patch chain.
    InvalidResponse,
    /// Package contents, sizes, or delta data are invalid or cannot be applied.
    InvalidPackage,
    /// A package or asset path is unsafe.
    UnsafePath,
    /// A filesystem operation failed.
    Io,
    /// Stored installation data is unreadable as JSON or has an inconsistent identity.
    InvalidData,
    /// A background worker failed unexpectedly.
    Internal,
}

/// A workflow failure with a stable category and a diagnostic source chain.
///
/// Use [`Self::kind`] for recovery decisions and [`std::error::Error::source`]
/// for technical diagnostics. Internal protocol, storage, and decoder errors
/// are not part of the supported public API.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct Error {
    kind: ErrorKind,
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl Error {
    /// Returns the actionable failure category, including failures nested in retries.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

fn release_kind(error: &ReleaseError) -> ErrorKind {
    match error {
        ReleaseError::UnknownClientType { .. } | ReleaseError::UnsupportedLanguage { .. } => {
            ErrorKind::InvalidSelection
        }
        _ => ErrorKind::InvalidResponse,
    }
}

fn installation_kind(error: &InstallationError) -> ErrorKind {
    match error {
        InstallationError::CachedSelectionMismatch { .. }
        | InstallationError::PinnedReleaseMismatch { .. } => ErrorKind::IncompatibleCache,
        InstallationError::Catalog(_) => ErrorKind::InvalidSelection,
        InstallationError::InvalidInstalledProduct(_)
        | InstallationError::InstalledRegionMismatch { .. } => ErrorKind::InvalidData,
    }
}

fn storage_kind(error: &StorageError) -> ErrorKind {
    match error {
        StorageError::Io(_) => ErrorKind::Io,
        StorageError::Json(_) => ErrorKind::InvalidData,
        StorageError::UnsafeAssetPath(_) => ErrorKind::UnsafePath,
        StorageError::IncompleteCache { .. } => ErrorKind::IncompleteCache,
        StorageError::InstallationLocked(_) => ErrorKind::InstallationLocked,
        StorageError::Installation(error) => installation_kind(error),
    }
}

fn transfer_kind(error: &TransferError) -> ErrorKind {
    match error {
        TransferError::Storage(error) => storage_kind(error),
        TransferError::Io(_) => ErrorKind::Io,
        TransferError::SizeMismatch { .. } => ErrorKind::InvalidPackage,
        TransferError::DownloadRetries { source, .. } => transfer_kind(source),
        TransferError::Http(_) | TransferError::ResumeRejected { .. } => ErrorKind::Network,
    }
}

fn package_kind(error: &PackageError) -> ErrorKind {
    match error {
        PackageError::Io(_) => ErrorKind::Io,
        PackageError::Storage(error) => storage_kind(error),
        PackageError::Release(error) => release_kind(error),
        PackageError::InvalidDeltaPath(_) => ErrorKind::UnsafePath,
        _ => ErrorKind::InvalidPackage,
    }
}

macro_rules! workflow_errors {
    ($($error:ty => $classify:expr),* $(,)?) => {$(
        #[doc(hidden)]
        impl From<$error> for Error {
            fn from(error: $error) -> Self {
                Self { kind: ($classify)(&error), source: Box::new(error) }
            }
        }
    )*};
}

workflow_errors!(
    CatalogError => |_| ErrorKind::InvalidSelection,
    DownloadError => |_| ErrorKind::InvalidSelection,
    ReleaseError => release_kind,
    InstallationError => installation_kind,
    WgusError => wgus_kind,
    TransferError => transfer_kind,
    PackageError => package_kind,
    StorageError => storage_kind,
    ExtractError => |_| ErrorKind::DownloadNotFound,
    InstalledBuildError => |_| ErrorKind::InstallationNotFound,
    PipelineError => |_| ErrorKind::Internal,
);

fn wgus_kind(error: &WgusError) -> ErrorKind {
    match error {
        WgusError::Http(_) => ErrorKind::Network,
        WgusError::Catalog(_) => ErrorKind::InvalidSelection,
        WgusError::Release(error) => release_kind(error),
        _ => ErrorKind::InvalidResponse,
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        StorageError::from(error).into()
    }
}

/// Result of a high-level launcher workflow.
pub type Result<T> = std::result::Result<T, Error>;
