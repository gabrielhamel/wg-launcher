use crate::installation::StorageError;

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
    pub(crate) fn new(
        kind: ErrorKind,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            source: Box::new(source),
        }
    }

    /// Returns the actionable failure category, including failures nested in retries.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        StorageError::from(error).into()
    }
}

/// Result of a high-level launcher workflow.
pub type Result<T> = std::result::Result<T, Error>;
