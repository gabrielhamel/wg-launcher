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
    /// Selected client component identifiers.
    #[serde(rename = "parts")]
    pub component_ids: Vec<String>,
    /// Target version of every selected component, including unchanged components.
    pub component_versions: ComponentVersions,
    /// Published patch chain, including full-install and delta patches.
    pub patches: Vec<Patch>,
    /// Published HTTP seed endpoints; asset URLs use the first seed.
    pub web_seeds: Vec<WebSeed>,
    /// Resolved transport volumes across the published patches.
    #[serde(rename = "assets")]
    pub volumes: Vec<TransportVolume>,
}

impl ResolvedRelease {
    /// Returns the sum of all release asset sizes in compressed bytes.
    ///
    /// This is a transport total, not required disk space or remaining network traffic;
    /// it does not subtract cached files or filter the installation chain.
    pub fn published_volume_bytes(&self) -> u64 {
        self.volumes.iter().map(|asset| asset.size).sum()
    }
}

impl ResolvedRelease {
    /// Validate selected component targets in publisher order, leaving unused patches alone.
    pub(crate) fn components(
        &self,
    ) -> impl Iterator<Item = Result<super::ComponentRelease<'_>, super::ReleaseError>> {
        self.component_ids.iter().map(move |component_id| {
            super::ComponentRelease::new(
                component_id,
                self.component_versions.get(component_id),
                &self.patches,
            )
        })
    }
}
