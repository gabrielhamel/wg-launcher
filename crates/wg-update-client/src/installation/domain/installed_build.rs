use super::PinnedManifest;
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
    pub component_versions: std::collections::BTreeMap<String, String>,
}

impl From<&PinnedManifest> for InstalledBuild {
    fn from(manifest: &PinnedManifest) -> Self {
        Self {
            identity: manifest.release.identity.clone(),
            product: manifest.release.product.clone(),
            component_versions: manifest.release.component_versions.clone(),
        }
    }
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

    pub(crate) fn compare_with(
        &self,
        manifest: &PinnedManifest,
    ) -> std::result::Result<BuildComparison, crate::catalog::ReleaseError> {
        let update_available = self.identity.version != manifest.release.identity.version
            || self.component_versions != manifest.release.component_versions;
        let download_bytes = if update_available {
            manifest
                .installation_assets()?
                .iter()
                .map(|asset| asset.size)
                .sum()
        } else {
            0
        };
        Ok(BuildComparison {
            update_available,
            download_bytes,
        })
    }
}

/// Whether the target differs and the package bytes needed to reach it.
pub(crate) struct BuildComparison {
    pub(crate) update_available: bool,
    pub(crate) download_bytes: u64,
}
