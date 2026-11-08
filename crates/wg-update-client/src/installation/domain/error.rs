use crate::catalog::CatalogError;

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
    /// Saved work belongs to a different resolved release, even if its selection matches.
    #[error(
        "the pinned release differs from the requested release in {differences}; continue with the pinned release or use a different output directory"
    )]
    PinnedReleaseMismatch {
        /// Release fields whose values differ from the saved work.
        differences: String,
    },
    /// The product saved with the installation is invalid.
    #[error(
        "the saved installation product is invalid: {0}; restore a valid .wg-launcher/state.json record"
    )]
    InvalidInstalledProduct(#[source] CatalogError),
    /// The recorded region contradicts the saved product.
    #[error(
        "the saved installation region '{recorded}' differs from its product region '{product}'; restore a consistent .wg-launcher/state.json record"
    )]
    InstalledRegionMismatch { recorded: String, product: String },
    /// Failure from the underlying catalog operation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

pub(crate) type Result<T> = std::result::Result<T, InstallationError>;
