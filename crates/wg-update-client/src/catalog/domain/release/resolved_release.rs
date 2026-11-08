use super::{Patch, TransportVolume, WebSeed};
use crate::catalog::ComponentVersions;
use serde::{Deserialize, Serialize};

/// Resolved build identity, published patch chains, and transport assets.
///
/// Fields describe the payload rather than the OS running this library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRelease {
    #[serde(flatten)]
    pub(crate) identity: crate::catalog::BuildIdentity,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: super::super::Product,
    /// Resolved WGUS-compatible hostname after protocol redirects.
    #[serde(rename = "host")]
    pub resolved_hostname: String,
    /// Normalized languages advertised for this build.
    pub available_languages: Vec<String>,
    /// Publisher identifiers advertised in metadata.
    pub publisher_ids: Vec<String>,
    /// Published HTTP seed endpoints; asset URLs use the first seed.
    pub web_seeds: Vec<WebSeed>,
    /// Related component and package data is replaced only as one unit.
    #[serde(flatten)]
    pub(crate) contents: ReleaseContents,
}

impl ResolvedRelease {
    /// Returns the sum of all release asset sizes in compressed bytes.
    ///
    /// This is a transport total, not required disk space or remaining network traffic;
    /// it does not subtract cached files or filter the installation chain.
    pub fn published_volume_bytes(&self) -> u64 {
        self.contents.volumes.iter().map(|asset| asset.size).sum()
    }

    /// Validate selected component targets in publisher order, leaving unused patches alone.
    pub(crate) fn components(
        &self,
    ) -> impl Iterator<Item = Result<super::ComponentRelease<'_>, super::ReleaseError>> {
        self.contents.component_ids.iter().map(move |component_id| {
            super::ComponentRelease::new(
                component_id,
                self.contents.component_versions.get(component_id),
                &self.contents.patches,
            )
        })
    }
}

/// Immutable component selections, targets, patch declarations, and transport sources.
/// Validation remains scoped to the chain actually selected for installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReleaseContents {
    /// Selected client component identifiers.
    #[serde(rename = "parts")]
    component_ids: Vec<String>,
    /// Target version of every selected component, including unchanged components.
    component_versions: ComponentVersions,
    /// Published patch chain, including full-install and delta patches.
    patches: Vec<Patch>,
    /// Resolved transport volumes across the published patches.
    #[serde(rename = "assets")]
    volumes: Vec<TransportVolume>,
}

impl ReleaseContents {
    pub(crate) fn new(
        component_ids: Vec<String>,
        component_versions: ComponentVersions,
        patches: Vec<Patch>,
        volumes: Vec<TransportVolume>,
    ) -> Self {
        Self {
            component_ids,
            component_versions,
            patches,
            volumes,
        }
    }

    pub(crate) fn component_ids(&self) -> &[String] {
        &self.component_ids
    }
    pub(crate) fn component_versions(&self) -> &ComponentVersions {
        &self.component_versions
    }
    pub(crate) fn patches(&self) -> &[Patch] {
        &self.patches
    }
    pub(crate) fn volumes(&self) -> &[TransportVolume] {
        &self.volumes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn release_contents_keep_the_flat_saved_contract_and_required_targets() {
        let saved = json!({
            "game_id": "EXAMPLE", "region": "eu", "channel": "production",
            "version": "1", "client_type": "sd", "language": "EN",
            "platform": "windows", "architecture": "x64", "metadata_version": "1",
            "product": {
                "game": "example", "name": "Example", "region": "eu",
                "host": "https://example.test/", "game_id": "EXAMPLE",
                "channel": "production"
            },
            "host": "example.test", "available_languages": ["EN"],
            "publisher_ids": [], "web_seeds": [], "parts": ["client"],
            "component_versions": {"client": "1"}, "patches": [], "assets": []
        });
        let release: ResolvedRelease = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(serde_json::to_value(&release).unwrap(), saved);
        // Reading a saved release must not eagerly plan an unavailable chain.
        assert!(super::super::required_volumes(&release, &ComponentVersions::default()).is_err());
        let mut missing_targets = saved;
        missing_targets
            .as_object_mut()
            .unwrap()
            .remove("component_versions");
        assert!(serde_json::from_value::<ResolvedRelease>(missing_targets).is_err());
    }
}
