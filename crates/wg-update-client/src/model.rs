use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};
use url::Url;

/// Publisher metadata describing the available client variants and languages.
#[derive(Debug, Clone, Serialize)]
pub struct GameMetadata {
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
    /// Publisher application identifier.
    pub game_id: String,
    /// Human-readable game name.
    pub name: String,
    /// Publisher-suggested filesystem name.
    pub filesystem_name: String,
    /// Publisher environment label.
    pub environment: String,
    /// Normalized publisher default language.
    pub default_language: String,
    /// Normalized languages advertised by the publisher.
    pub supported_languages: Vec<String>,
    /// Publisher default client variant identifier; high-level defaults still select `sd`.
    pub default_client_type: String,
    /// Client variants and their component definitions.
    pub client_types: Vec<ClientType>,
    /// Executable metadata for the payload; the launcher does not run these files.
    pub executables: Vec<Executable>,
    /// Publisher identifiers advertised in metadata.
    pub publisher_ids: Vec<String>,
}

impl GameMetadata {
    /// Finds a client variant by its exact, case-sensitive identifier.
    ///
    /// Returns `None` when the publisher did not advertise that identifier.
    pub fn client_type(&self, id: &str) -> Option<&ClientType> {
        self.client_types.iter().find(|client| client.id == id)
    }
}

/// A publisher client variant and the components it requires.
#[derive(Debug, Clone, Serialize)]
pub struct ClientType {
    /// Exact publisher client variant identifier.
    pub id: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Components required by this variant.
    pub parts: Vec<ClientPart>,
}

/// A component referenced by a client variant.
///
/// The flags describe publisher metadata; they do not enable local integrity scans.
#[derive(Debug, Clone, Serialize)]
pub struct ClientPart {
    /// Publisher component identifier used in patch requests.
    pub id: String,
    /// Optional publisher application type.
    pub app_type: Option<String>,
    /// Whether the publisher marks this component as language-specific.
    pub language_specific: bool,
    /// Whether the publisher marks this component for integrity checking.
    pub integrity_checked: bool,
}

/// Publisher information about a game executable; no launch behavior is provided.
#[derive(Debug, Clone, Serialize)]
pub struct Executable {
    /// Executable path as advertised by the publisher.
    pub path: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Optional publisher emulation hint.
    pub emulation: Option<String>,
}

/// Resolved build identity, patch chains, and transport assets.
///
/// High-level downloads pin this Serde-serializable value in
/// [`DOWNLOAD_MANIFEST_FILE`](crate::DOWNLOAD_MANIFEST_FILE) for resumable, local extraction.
/// Fields describe the payload rather than the OS running this library.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Published patch chain, including full-install and delta patches.
    pub patches: Vec<Patch>,
    /// Published HTTP seed endpoints; asset URLs use the first seed.
    pub web_seeds: Vec<WebSeed>,
    /// Resolved transport volumes across the published patches.
    pub assets: Vec<Asset>,
}

impl PatchManifest {
    /// Clones assets whose component matches any exact identifier in `parts`.
    ///
    /// An empty slice selects every asset. Unknown identifiers contribute no matches.
    /// This filters manifest order; use [`installation_assets`](crate::installation_assets)
    /// when you need the ordered installation chain.
    pub fn assets_for_parts(&self, parts: &[String]) -> Vec<Asset> {
        if parts.is_empty() {
            return self.assets.clone();
        }
        let selected: BTreeSet<&str> = parts.iter().map(String::as_str).collect();
        self.assets
            .iter()
            .filter(|asset| selected.contains(asset.part.as_str()))
            .cloned()
            .collect()
    }

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

/// One full-install or delta step for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Component identifier to which this patch belongs.
    pub part: String,
    /// Source component version; `Some("0")` denotes a full-install patch.
    /// Service resolution infers omitted values from earlier patches for the same
    /// component, starting at `0`; manually constructed manifests may still use `None`.
    pub version_from: Option<String>,
    /// Target component version after this patch.
    pub version_to: String,
    /// Transport volumes that together form this patch archive.
    pub files: Vec<PatchFile>,
    /// Optional publisher torrent hash; not used for download verification.
    pub torrent_hash: Option<String>,
    /// Published torrent URLs; the downloader uses HTTP assets instead.
    pub torrent_urls: Vec<Url>,
}

/// A transport archive volume listed in a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchFile {
    /// Relative transport volume path, not a game-owned inner `.pkg` asset.
    pub name: String,
    /// Expected compressed volume size in bytes.
    pub size: u64,
    /// Publisher-reported unpacked size in bytes, when present.
    pub unpacked_size: Option<u64>,
    /// Publisher-reported delta size in bytes, when present.
    pub diffs_size: Option<u64>,
}

/// An HTTP source advertised for package downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSeed {
    /// Base URL used to resolve relative transport paths.
    pub url: Url,
    /// Optional publisher seed label.
    pub name: Option<String>,
    /// Optional publisher concurrency hint; not automatically applied to download options.
    pub threads: Option<usize>,
}

/// One downloadable transport volume with a resolved HTTP URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Component identifier to which this patch belongs.
    pub part: String,
    /// Source component version; `Some("0")` denotes a full-install patch.
    /// Service resolution infers omitted values from earlier patches for the same
    /// component, starting at `0`; manually constructed manifests may still use `None`.
    pub version_from: Option<String>,
    /// Target component version after this patch.
    pub version_to: String,
    /// Relative transport volume path, not a game-owned inner `.pkg` asset.
    pub name: String,
    /// Resolved download URL for this volume.
    pub url: Url,
    /// Expected compressed volume size in bytes.
    pub size: u64,
    /// Publisher-reported unpacked size in bytes, when present.
    pub unpacked_size: Option<u64>,
    /// Publisher-reported delta size in bytes, when present.
    pub diffs_size: Option<u64>,
}
