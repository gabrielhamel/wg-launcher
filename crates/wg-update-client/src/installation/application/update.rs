use super::status::StatusReport;
use super::update_target::resolve_update_target;
use super::{InstallEvent, InstallOutcome, install::install_resolved};
use super::{installed::load_installed_build, status_report::build_status_report};
use crate::installation::DownloadOptions;
use crate::{Result, installation::infrastructure::filesystem::InstallationLayout};
use serde::Serialize;
use std::path::PathBuf;

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
    let storage = InstallationLayout::for_root(&path);
    let installed = load_installed_build(&path).await?;
    let (manifest, target_source) = resolve_update_target(&installed, &storage).await?;
    let status = build_status_report(
        path,
        &installed,
        &manifest.release,
        manifest.reached_versions(),
        target_source,
    )?;
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
