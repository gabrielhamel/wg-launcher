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
    /// Selected components have not all reached the pinned release targets.
    #[error(
        "component '{component}' is at version '{reached}', but requires '{target}'; resume installation before recording it as complete"
    )]
    IncompleteInstallation {
        component: String,
        reached: String,
        target: String,
    },
    /// Failure from the underlying catalog operation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

pub(crate) type Result<T> = std::result::Result<T, InstallationError>;

impl std::fmt::Display for super::selection::SelectionDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{} ({}/{}: {}, {}, {})",
            self.region,
            self.client_type
                .as_deref()
                .unwrap_or("publisher default or smallest variant"),
            self.language.as_deref().unwrap_or("publisher default"),
            self.product.game(),
            self.product.name,
            self.product.host,
            self.product.game_id,
            self.product.channel
        )
    }
}

impl std::fmt::Display for super::selection::SelectionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use super::selection::SelectionField;
        write!(
            f,
            "the cached build is {} but the requested build is {} (different ",
            self.cached, self.requested
        )?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            f.write_str(match field {
                SelectionField::Product => "product",
                SelectionField::Region => "region",
                SelectionField::ClientType => "client type",
                SelectionField::Language => "language",
            })?;
        }
        f.write_str("); use matching options or a different output directory")
    }
}
