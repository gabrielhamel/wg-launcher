use super::PinnedManifest;
use serde::{Deserialize, Serialize};

/// Build identity serialized in `.wg-launcher/state.json`.
///
/// Created from a manifest only after extraction succeeds. Records the current
/// installed build, while the installation directory may retain its original version
/// name. Component versions select subsequent update chains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledBuild {
    /// Publisher application identifier.
    pub game_id: String,
    /// Publisher catalog region.
    pub region: String,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: crate::catalog::Product,
    /// Publisher release channel.
    pub channel: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Selected normalized language identifier.
    pub language: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Successfully installed version of each selected component.
    pub component_versions: std::collections::BTreeMap<String, String>,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
}

impl From<&PinnedManifest> for InstalledBuild {
    fn from(manifest: &PinnedManifest) -> Self {
        Self {
            game_id: manifest.release.game_id.clone(),
            region: manifest.release.region.clone(),
            product: manifest.release.product.clone(),
            channel: manifest.release.channel.clone(),
            version: manifest.release.version.clone(),
            client_type: manifest.release.client_type.clone(),
            language: manifest.release.language.clone(),
            platform: manifest.release.platform.clone(),
            architecture: manifest.release.architecture.clone(),
            metadata_version: manifest.release.metadata_version.clone(),
            component_versions: manifest.release.component_versions.clone(),
        }
    }
}

impl InstalledBuild {
    pub(crate) fn build_info(&self) -> crate::catalog::BuildInfo {
        crate::catalog::BuildInfo {
            game: self.product.game(),
            name: self.product.name.clone(),
            game_id: self.game_id.clone(),
            region: self.region.clone(),
            channel: self.channel.clone(),
            version: self.version.clone(),
            client_type: self.client_type.clone(),
            language: self.language.clone(),
            platform: self.platform.clone(),
            architecture: self.architecture.clone(),
        }
    }

    pub(crate) fn selection(&self) -> crate::catalog::BuildSelection {
        crate::catalog::BuildSelection {
            game: Some(self.product.game()),
            region: self.region.clone(),
            products: Some(vec![self.product.clone()]),
            client_type: Some(self.client_type.clone()),
            language: Some(self.language.clone()),
        }
    }

    pub(crate) fn differs_from(&self, manifest: &PinnedManifest) -> bool {
        self.version != manifest.release.version
            || self.component_versions != manifest.release.component_versions
    }
}
