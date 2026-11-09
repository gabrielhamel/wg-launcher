use crate::ErrorKind;
use crate::catalog::CatalogError;

/// Failures owned by installation consistency.
#[derive(Debug, thiserror::Error)]
pub enum InstallationError {
    /// The pinned manifest did not match the requested selection.
    #[error("{0}")]
    CachedSelectionMismatch(Box<super::selection::SelectionMismatch>),
    /// Saved work belongs to a different resolved release, even if its selection matches.
    #[error(
        "the pinned release differs from the requested release in {differences}; continue with the pinned release or use a different output directory"
    )]
    PinnedReleaseMismatch {
        /// Release fields whose values differ from the saved work.
        differences: super::release_compatibility::ReleaseDifferences,
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
    /// Selected components have not all reached the pinned release targets.
    #[error(
        "component '{component}' is at version '{reached}', but requires '{target}'; resume installation before recording it as complete"
    )]
    IncompleteInstallation {
        component: String,
        reached: String,
        target: String,
    },
    /// A selected release component has invalid published data.
    #[error(transparent)]
    Release(#[from] crate::catalog::ReleaseError),
    /// Failure from the underlying catalog operation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

pub(crate) type Result<T> = std::result::Result<T, InstallationError>;

impl InstallationError {
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            InstallationError::CachedSelectionMismatch(_)
            | InstallationError::PinnedReleaseMismatch { .. } => ErrorKind::IncompatibleCache,
            InstallationError::Catalog(_) => ErrorKind::InvalidSelection,
            InstallationError::Release(error) => error.kind(),
            InstallationError::InvalidInstalledProduct(_)
            | InstallationError::InstalledRegionMismatch { .. }
            | InstallationError::IncompleteInstallation { .. } => ErrorKind::InvalidData,
        }
    }
}

impl From<InstallationError> for crate::Error {
    fn from(error: InstallationError) -> Self {
        Self::new(error.kind(), error)
    }
}
