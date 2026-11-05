use crate::CatalogError;

/// Failures owned by installation consistency.
#[derive(Debug, thiserror::Error)]
pub enum InstallationError {
    /// The pinned manifest did not match the requested selection.
    #[error(
        "the cached build is {cached} but the requested build is {requested}; use matching options or a different output directory"
    )]
    CachedSelectionMismatch {
        /// Human-readable selection recorded in the pinned manifest.
        cached: String,
        /// Requested identifier or build selection.
        requested: String,
    },
    /// Failure from the underlying catalog operation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

pub(crate) type Result<T> = std::result::Result<T, InstallationError>;
