use crate::infrastructure::installation_fs::installation_storage;
use crate::{Asset, PatchManifest, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Resolved build and filesystem plan emitted before transfer or extraction.
///
/// A plan event is informational; it does not guarantee the operation will succeed.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    /// Complete pinned build manifest, even for a filtered download.
    pub manifest: PatchManifest,
    /// Installation root, which can be relative to the current directory.
    pub root: PathBuf,
    /// Package cache directory beneath the root; relative when the root is relative.
    pub cache: PathBuf,
    /// Selected transport volumes in installation order.
    pub assets: Vec<Asset>,
    /// Total compressed bytes in `assets`, before subtracting cached data.
    pub compressed_bytes: u64,
}

pub(super) fn build_plan(
    manifest: PatchManifest,
    root: PathBuf,
    assets: Vec<Asset>,
) -> Result<InstallPlan> {
    let compressed_bytes = assets.iter().map(|asset| asset.size).sum();
    let cache = installation_storage(&root)?.cache;
    Ok(InstallPlan {
        manifest,
        root,
        cache,
        assets,
        compressed_bytes,
    })
}
