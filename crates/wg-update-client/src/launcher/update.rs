use super::{
    BuildSelection, InstallEvent, InstallOutcome,
    install::install_resolved,
    manifest::{load_cache_manifest, load_installed_build, validate_cached_selection},
};
use crate::{DownloadOptions, InstalledBuild, Result, WgusClient, install::installation_storage};
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
/// [`status`] compares with the live catalog. [`update`] may instead compare with
/// a pending pinned manifest to preserve retry consistency.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    /// Absolute installation root.
    pub path: PathBuf,
    /// Identity loaded before any update.
    pub installed: InstalledBuild,
    /// Live catalog version, or pending pinned version during update retries.
    pub latest_version: String,
    /// Whether the build or any component version differs. Versions are compared
    /// for equality; game files are not scanned for integrity.
    pub update_available: bool,
}

/// Configures an in-place update using the installation's recorded selection.
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    /// Existing installation root; use `".".into()` explicitly for the current directory.
    pub path: PathBuf,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Version comparison and optional installation result.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateOutcome {
    /// Status before updating, compared with the live or pending pinned version.
    pub status: StatusReport,
    /// Installation result, or `None` when the compared version was already installed.
    /// An already-current update emits no installation events.
    pub installation: Option<InstallOutcome>,
}

/// Loads the installed-build record and compares it with the live catalog.
///
/// Uses the recorded region, client type, and language. Does not modify files,
/// consult pending manifests, or verify installed file integrity.
///
/// # Errors
///
/// Returns an error if the installation record is missing, unreadable, or invalid,
/// or live manifest resolution fails.
pub async fn status(options: StatusOptions) -> Result<StatusReport> {
    let path = std::path::absolute(options.path)?;
    let installed = load_installed_build(&path).await?;
    let latest =
        super::manifest::resolve(&WgusClient::new()?, &recorded_selection(&installed)).await?;
    Ok(StatusReport {
        path,
        update_available: installed.version != latest.version
            || installed.component_versions != latest.component_versions,
        latest_version: latest.version,
        installed,
    })
}

/// Runs [`update_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`update_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`update_with_observer`].
pub async fn update(options: UpdateOptions) -> Result<UpdateOutcome> {
    update_with_observer(options, |_| {}).await
}

/// Updates an existing installation in place using its recorded build selection.
///
/// A pending pinned manifest takes precedence over the live catalog. When both the
/// build and component versions match, returns without installation or observer
/// events. Otherwise installs in the same directory, preserving its original name.
///
/// Requests component chains from their saved installed versions, skips unchanged
/// components, and checkpoints successful patches for resumable updates. A full
/// package is used when the publisher supplies no chain from the installed version.
///
/// # Errors
///
/// Returns an error for missing or invalid state, a pending selection mismatch,
/// failed catalog resolution, or any error from [`super::install`].
pub async fn update_with_observer<F>(options: UpdateOptions, observer: F) -> Result<UpdateOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let path = std::path::absolute(options.path)?;
    let storage = installation_storage(&path)?;
    let installed = load_installed_build(&path).await?;
    let mut manifest = if let Some(pending) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(&pending, &recorded_selection(&installed))?;
        pending
    } else {
        WgusClient::new()?
            .manifest_from_versions(
                &installed.product,
                &installed.client_type,
                Some(&installed.language),
                &installed.component_versions,
            )
            .await?
    };
    // A package-only download may have pinned a full installation chain. Start
    // that chain at the installed versions, retaining any completed checkpoints.
    for (part, version) in &installed.component_versions {
        manifest
            .base_versions
            .entry(part.clone())
            .or_insert_with(|| version.clone());
    }
    let status = StatusReport {
        path,
        update_available: installed.version != manifest.version
            || installed.component_versions != manifest.component_versions,
        latest_version: manifest.version.clone(),
        installed,
    };
    if !status.update_available {
        return Ok(UpdateOutcome {
            status,
            installation: None,
        });
    }
    let installation =
        install_resolved(manifest, status.path.clone(), options.download, observer).await?;
    Ok(UpdateOutcome {
        status,
        installation: Some(installation),
    })
}

fn recorded_selection(installed: &InstalledBuild) -> BuildSelection {
    BuildSelection {
        game: Some(installed.product.game()),
        region: installed.region.clone(),
        products: Some(vec![installed.product.clone()]),
        client_type: Some(installed.client_type.clone()),
        language: Some(installed.language.clone()),
    }
}
