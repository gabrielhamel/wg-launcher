use super::{Patch, TransportVolume, WebSeed};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub host: String,
    /// Normalized languages advertised for this build.
    pub available_languages: Vec<String>,
    /// Publisher identifiers advertised in metadata.
    pub publisher_ids: Vec<String>,
    /// Selected client component identifiers.
    pub parts: Vec<String>,
    /// Target version of every selected component, including unchanged components.
    pub component_versions: BTreeMap<String, String>,
    /// Published patch chain, including full-install and delta patches.
    pub patches: Vec<Patch>,
    /// Published HTTP seed endpoints; asset URLs use the first seed.
    pub web_seeds: Vec<WebSeed>,
    /// Resolved transport volumes across the published patches.
    pub assets: Vec<TransportVolume>,
}

impl ResolvedRelease {
    /// Describe incompatibility with saved work without changing release equality.
    pub(crate) fn differing_fields(&self, other: &Self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        macro_rules! compare {
            ($($field:ident => $label:literal),* $(,)?) => {$(
                if self.$field != other.$field { fields.push($label); }
            )*};
        }
        compare!(
            identity => "build identity", product => "product", host => "publisher endpoint",
            available_languages => "available languages", publisher_ids => "publisher identifiers",
            parts => "selected components", component_versions => "component targets",
            patches => "patch chains", web_seeds => "download endpoints", assets => "transport volumes",
        );
        fields
    }

    /// Returns the sum of all release asset sizes in compressed bytes.
    ///
    /// This is a transport total, not required disk space or remaining network traffic;
    /// it does not subtract cached files or filter the installation chain.
    pub fn published_volume_bytes(&self) -> u64 {
        self.assets.iter().map(|asset| asset.size).sum()
    }
}

/// A selected component's target and published steps, borrowed from a release.
/// This view groups related data without changing the saved release representation.
pub(super) struct ComponentRelease<'a> {
    pub(super) id: &'a str,
    pub(super) target: &'a str,
    pub(super) patches: Vec<&'a Patch>,
}

impl ResolvedRelease {
    /// Validate each selected component as it is visited in publisher order.
    pub(super) fn components(
        &self,
    ) -> impl Iterator<Item = super::error::Result<ComponentRelease<'_>>> {
        self.parts.iter().map(move |part| {
            let target = self.component_versions.get(part).ok_or_else(|| {
                super::ReleaseError::MissingComponentTarget { part: part.clone() }
            })?;
            Ok(ComponentRelease {
                id: part,
                target,
                patches: self
                    .patches
                    .iter()
                    .filter(|patch| &patch.part == part)
                    .collect(),
            })
        })
    }
}
