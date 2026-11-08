use super::comparison::build_status_report;
use super::installed::load_installed_build;
use super::update_target::{UpdateTarget, UpdateTargetSource};
use crate::Result;
use crate::catalog::{BuildInfo, resolve_update_release};
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
    /// Build chosen for comparison, with its live or pinned origin.
    pub target: UpdateTarget,
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
    let latest = resolve_update_release(
        &installed.product,
        &installed.identity.client_type,
        &installed.identity.language,
        &installed.component_versions,
    )
    .await?;
    build_status_report(
        path,
        &installed,
        &latest,
        &installed.component_versions,
        UpdateTargetSource::LiveCatalog,
    )
}
