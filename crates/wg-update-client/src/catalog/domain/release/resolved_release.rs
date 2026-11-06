use super::{Asset, Patch, WebSeed};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Resolved build identity, published patch chains, and transport assets.
///
/// Fields describe the payload rather than the OS running this library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRelease {
    /// Publisher catalog region.
    pub region: String,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: super::super::Product,
    /// Publisher release channel.
    pub channel: String,
    /// Resolved WGUS-compatible hostname after protocol redirects.
    pub host: String,
    /// Publisher application identifier.
    pub game_id: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Selected normalized language identifier.
    pub language: String,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
    /// Publisher game build version.
    pub version: String,
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
    pub assets: Vec<Asset>,
}

impl ResolvedRelease {
    /// Returns the sum of all release asset sizes in compressed bytes.
    ///
    /// This is a transport total, not required disk space or remaining network traffic;
    /// it does not subtract cached files or filter the installation chain.
    pub fn download_size(&self) -> u64 {
        self.assets.iter().map(|asset| asset.size).sum()
    }
}
