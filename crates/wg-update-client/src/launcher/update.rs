use super::{
    BuildSelection, InstallEvent, InstallOptions, InstallOutcome, install_with_observer,
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
    /// Whether recorded and compared versions differ; versions are not ordered
    /// semantically and game files are not scanned for integrity.
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
        update_available: installed.version != latest.version,
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
/// A pending pinned manifest takes precedence over the live catalog. If its version
/// (or the live version when no manifest is pinned) equals the installed version,
/// returns without installation or observer events. Otherwise delegates to [`install_with_observer`]
/// using the same directory, preserving the original directory name.
///
/// Updates follow the publisher's installation chain. Per-component installed
/// versions are not stored, so this is not a fully minimal component delta update.
///
/// # Errors
///
/// Returns an error for missing or invalid state, a pending selection mismatch,
/// failed catalog resolution, or any error from [`install_with_observer`].
pub async fn update_with_observer<F>(options: UpdateOptions, observer: F) -> Result<UpdateOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let path = std::path::absolute(options.path)?;
    let storage = installation_storage(&path)?;
    let installed = load_installed_build(&path).await?;
    let status = if let Some(pending) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(&pending, &recorded_selection(&installed))?;
        StatusReport {
            path,
            update_available: installed.version != pending.version,
            latest_version: pending.version,
            installed,
        }
    } else {
        status(StatusOptions { path }).await?
    };
    if !status.update_available {
        return Ok(UpdateOutcome {
            status,
            installation: None,
        });
    }
    let installation = install_with_observer(
        InstallOptions {
            selection: recorded_selection(&status.installed),
            output: Some(status.path.clone()),
            download: options.download,
        },
        observer,
    )
    .await?;
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
