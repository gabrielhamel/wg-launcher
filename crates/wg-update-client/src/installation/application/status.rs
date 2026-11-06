use crate::Result;
use crate::catalog::{BuildInfo, WgusClient};
use crate::installation::{InstalledBuild, PinnedManifest};
use serde::Serialize;
use std::path::PathBuf;

/// Selects an existing installation for a live version comparison.
#[derive(Debug, Clone)]
pub struct StatusOptions {
    /// Installation root containing the installed-build record; use `".".into()`
    /// explicitly for the current directory.
    pub path: PathBuf,
}

/// Recorded build identity and the version it was compared against.
///
/// [`status`] compares with the live catalog. [`update`](crate::update) may instead compare with
/// a pending pinned manifest to preserve retry consistency.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    /// Absolute installation root.
    pub path: PathBuf,
    /// Identity loaded before any update.
    pub installed: BuildInfo,
    /// Live catalog version, or pending pinned version during update retries.
    pub latest_version: String,
    /// Whether the build or any component version differs. Versions are compared
    /// for equality; game files are not scanned for integrity.
    pub update_available: bool,
    /// Compressed bytes in the applicable remaining patch chain, before subtracting
    /// cached packages or partial downloads. Zero when no packages are needed.
    pub update_download_bytes: u64,
}

/// Loads the installed-build record and compares it with the live catalog.
///
/// Uses the recorded region, client type, and language. Does not modify files,
/// consult pending manifests, or verify installed file integrity. Resolves the live
/// component update chain to estimate compressed download bytes without transferring
/// packages; cached data is not subtracted from that estimate.
///
/// # Errors
///
/// Returns an error if the installation record is missing, unreadable, or invalid,
/// or live manifest resolution fails.
pub async fn status(options: StatusOptions) -> Result<StatusReport> {
    let path = std::path::absolute(options.path)?;
    let installed = load_installed_build(&path).await?;
    let latest = WgusClient::new()?
        .release_from_versions(
            &installed.product,
            &installed.client_type,
            Some(&installed.language),
            &installed.component_versions,
        )
        .await?;
    let mut latest = PinnedManifest::from(latest);
    latest.include_installed_versions(&installed);
    build_status(path, installed, &latest)
}

pub(super) fn build_status(
    path: PathBuf,
    installed: InstalledBuild,
    manifest: &PinnedManifest,
) -> Result<StatusReport> {
    let update_available = installed.differs_from(manifest);
    let update_download_bytes = if update_available {
        manifest
            .installation_assets()?
            .iter()
            .map(|asset| asset.size)
            .sum()
    } else {
        0
    };
    Ok(StatusReport {
        path,
        installed: installed.build_info(),
        latest_version: manifest.release.version.clone(),
        update_available,
        update_download_bytes,
    })
}

/// Missing installed identity for status and update workflows.
#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    /// The installation root had no installed-build record.
    #[error("no wg-launcher installation record was found for {0:?}")]
    InstallationNotFound(std::path::PathBuf),
}

pub(super) async fn load_installed_build(
    root: &std::path::Path,
) -> crate::Result<crate::installation::InstalledBuild> {
    Ok(
        crate::installation::infrastructure::filesystem::records::load_installed_build(root)
            .await?
            .ok_or_else(|| StatusError::InstallationNotFound(root.to_path_buf()))?,
    )
}
