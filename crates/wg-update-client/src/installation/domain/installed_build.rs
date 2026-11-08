use crate::catalog::ComponentVersions;
use serde::{Deserialize, Serialize};

/// Build identity serialized in `.wg-launcher/state.json`.
///
/// Created from a manifest only after extraction succeeds. Records the current
/// installed build, while the installation directory may retain its original version
/// name. Component versions select subsequent update chains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledBuild {
    #[serde(flatten)]
    pub(crate) identity: crate::catalog::BuildIdentity,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: crate::catalog::Product,
    /// Successfully installed version of each selected component.
    pub component_versions: ComponentVersions,
}

impl InstalledBuild {
    /// Check the saved identity before using it for discovery or a pending update.
    pub(crate) fn validate_consistency(&self) -> super::error::Result<()> {
        self.product
            .validate_identity()
            .map_err(super::InstallationError::InvalidInstalledProduct)?;
        if !self
            .product
            .region
            .eq_ignore_ascii_case(&self.identity.region)
        {
            return Err(super::InstallationError::InstalledRegionMismatch {
                recorded: self.identity.region.clone(),
                product: self.product.region.clone(),
            });
        }
        Ok(())
    }

    pub(crate) fn selection(&self) -> super::selection::ResolvedSelection<'_> {
        super::selection::ResolvedSelection {
            product: &self.product,
            client_type: &self.identity.client_type,
            language: &self.identity.language,
        }
    }

    pub(crate) fn differs_from(&self, target: &crate::catalog::ResolvedRelease) -> bool {
        self.identity.version != target.identity.version
            || &self.component_versions != target.contents.component_versions()
    }
}
