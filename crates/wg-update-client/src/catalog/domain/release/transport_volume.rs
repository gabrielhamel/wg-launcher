use super::PatchSource;
use serde::{Deserialize, Serialize};
use url::Url;

/// An HTTP source advertised for package downloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSeed {
    /// Base URL used to resolve relative transport paths.
    pub url: Url,
    /// Optional publisher seed label.
    pub name: Option<String>,
    /// Optional publisher concurrency hint; not automatically applied to download options.
    pub threads: Option<usize>,
}

/// One downloadable transport volume with a resolved HTTP URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportVolume {
    /// Component identifier to which this patch belongs.
    pub part: String,
    /// Normalized patch source, serialized using the existing version field.
    #[serde(default, rename = "version_from")]
    pub source: PatchSource,
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
