use serde::{Deserialize, Serialize};
use url::Url;

/// One full-install or delta step for a component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    /// Component identifier to which this patch belongs.
    pub part: String,
    /// Normalized source of this patch, retaining the existing JSON representation.
    #[serde(default, rename = "version_from")]
    pub source: PatchSource,
    /// Target component version after this patch.
    pub version_to: String,
    /// Transport volumes that together form this patch archive.
    pub files: Vec<PatchFile>,
    /// Optional publisher torrent hash; not used for download verification.
    pub torrent_hash: Option<String>,
    /// Published torrent URLs; the downloader uses HTTP assets instead.
    pub torrent_urls: Vec<Url>,
}

impl Patch {
    /// Whether this patch supplies a complete component instead of changing an existing one.
    pub(crate) fn is_full_install(&self) -> bool {
        self.source == PatchSource::Full
    }
}

/// A transport archive volume listed in a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Whether a patch supplies a component in full or changes an existing version.
/// WGUS omission is normalized before construction. Unspecified preserves the
/// previous handling of nullable sources in saved manifests, without inferring a chain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Option<String>", into = "Option<String>")]
pub(crate) enum PatchSource {
    Full,
    Version(String),
    #[default]
    Unspecified,
}

impl PatchSource {
    pub(crate) fn version(&self) -> Option<&str> {
        match self {
            Self::Full => Some("0"),
            Self::Version(version) => Some(version),
            Self::Unspecified => None,
        }
    }
}

impl From<String> for PatchSource {
    fn from(version: String) -> Self {
        if version == "0" {
            Self::Full
        } else {
            Self::Version(version)
        }
    }
}

impl From<Option<String>> for PatchSource {
    fn from(version: Option<String>) -> Self {
        version.map(Self::from).unwrap_or_default()
    }
}

impl From<PatchSource> for Option<String> {
    fn from(source: PatchSource) -> Self {
        source.version().map(str::to_owned)
    }
}
