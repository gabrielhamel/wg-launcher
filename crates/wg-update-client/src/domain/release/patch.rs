use serde::{Deserialize, Serialize};
use url::Url;

/// One full-install or delta step for a component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
