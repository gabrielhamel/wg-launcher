use crate::installation::infrastructure::filesystem::installation_storage;
use crate::{Asset, BuildInfo, PatchManifest, Result};
use serde::Serialize;
use std::path::PathBuf;

/// A package identified in download progress, without its transport URL.
#[derive(Debug, Clone, Serialize)]
pub struct PackageSummary {
    /// Relative package name, matching [`crate::DownloadEvent`] names.
    pub name: String,
    /// Expected compressed package size in bytes.
    pub size: u64,
}

/// Build, destination, and transfer summary emitted before installation work.
///
/// Informational only; receiving this plan does not guarantee success.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    /// Selected build identity.
    pub build: BuildInfo,
    /// Installation root, possibly relative to the current directory.
    pub root: PathBuf,
    /// Package cache directory, useful for staged installation and offline transfer.
    pub cache: PathBuf,
    /// Selected packages in installation order, usable to initialize progress displays.
    pub packages: Vec<PackageSummary>,
    /// Total compressed package bytes before subtracting cached data.
    pub compressed_bytes: u64,
}

pub(super) fn build_plan(
    manifest: &PatchManifest,
    root: PathBuf,
    assets: &[Asset],
) -> Result<InstallPlan> {
    let compressed_bytes = assets.iter().map(|asset| asset.size).sum();
    let cache = installation_storage(&root)?.cache;
    Ok(InstallPlan {
        build: BuildInfo::from_manifest(manifest),
        root,
        cache,
        packages: assets
            .iter()
            .map(|asset| PackageSummary {
                name: asset.name.clone(),
                size: asset.size,
            })
            .collect(),
        compressed_bytes,
    })
}
