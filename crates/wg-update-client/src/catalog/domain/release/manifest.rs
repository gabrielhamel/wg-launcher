use crate::{Asset, Patch, WebSeed};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Resolved build identity, patch chains, and transport assets.
///
/// High-level downloads pin this Serde-serializable value in
/// [`DOWNLOAD_MANIFEST_FILE`](crate::DOWNLOAD_MANIFEST_FILE) for resumable, local extraction.
/// Fields describe the payload rather than the OS running this library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchManifest {
    /// Publisher catalog region.
    pub region: String,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: crate::Product,
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
    /// Component versions required before applying this chain; empty for a fresh install.
    /// Successful patches advance these checkpoints in the pinned manifest.
    pub base_versions: BTreeMap<String, String>,
    /// Target version of every selected component, including unchanged components.
    pub component_versions: BTreeMap<String, String>,
    /// Published patch chain, including full-install and delta patches.
    pub patches: Vec<Patch>,
    /// Published HTTP seed endpoints; asset URLs use the first seed.
    pub web_seeds: Vec<WebSeed>,
    /// Resolved transport volumes across the published patches.
    pub assets: Vec<Asset>,
}

impl PatchManifest {
    /// Returns the sum of all manifest asset sizes in compressed bytes.
    ///
    /// This is a transport total, not required disk space or remaining network traffic;
    /// it does not subtract cached files or filter the installation chain.
    pub fn download_size(&self) -> u64 {
        self.assets.iter().map(|asset| asset.size).sum()
    }

    /// Returns a relative directory name identifying this build selection.
    ///
    /// The name includes game, region, channel, version, payload platform, architecture,
    /// client type, and language. Missing architecture becomes `unknown-arch`.
    /// Values are lowercased and sanitized. Updates keep the initial directory name.
    /// No directory is created.
    pub fn default_output_directory(&self) -> PathBuf {
        let architecture = self.architecture.as_deref().unwrap_or("unknown-arch");
        PathBuf::from(format!(
            "{}-{}-{}-{}-{}-{}-{}-{}",
            slug(&self.product.game()),
            slug(&self.region.to_string()),
            slug(&self.channel),
            slug(&self.version),
            slug(&self.platform),
            slug(architecture),
            slug(&self.client_type),
            slug(&self.language),
        ))
    }
}

fn slug(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous_separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '.' {
            output.push(character);
            previous_separator = false;
        } else if !previous_separator && !output.is_empty() {
            output.push('-');
            previous_separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    output
}
