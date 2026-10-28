use super::{
    BuildSelection, InstallPlan,
    manifest::{build_plan, resolve_installation},
};
use crate::{
    AssetDownloader, DownloadOptions, InstallationEvent, InstallationReport, PackageExtractor,
    Result, WgusClient, installation_assets,
};
use serde::Serialize;
use std::path::PathBuf;

/// Configures the integrated installation pipeline.
///
/// Requires a game. Other defaults are EU, the publisher's default client type
/// (or the smallest installable variant), the publisher default language, an
/// automatically named directory, and [`DownloadOptions::default`].
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Build selection to resolve or match against the pinned cache.
    pub selection: BuildSelection,
    /// Installation root; `None` derives the name from the live manifest.
    /// Supply the same explicit root when resuming a build after the live version changes.
    pub output: Option<PathBuf>,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Integrated installation or update observer event.
///
/// A single plan precedes progress. An already-current update emits no events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum InstallEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`InstallationEvent`] stream.
    Progress(InstallationEvent),
}

/// Plan and results from a successful integrated installation.
#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    /// Resolved build, destination, and ordered assets.
    pub plan: InstallPlan,
    /// Transfer, extraction, and package-release totals.
    pub report: InstallationReport,
}

/// Runs [`install_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`install_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`install_with_observer`].
pub async fn install(options: InstallOptions) -> Result<InstallOutcome> {
    install_with_observer(options, |_| {}).await
}

/// Downloads and installs a build with overlapping transfer and extraction.
///
/// Acquires the installation lock, pins the manifest for the transfer/extraction
/// pipeline, and reuses cached volumes. Later downloads can overlap ordered patch
/// extraction; packages are removed only after their patch succeeds. Success writes
/// [`INSTALL_MARKER_FILE`](crate::INSTALL_MARKER_FILE) and removes cache and staging data.
///
/// The observer receives a plan, then interleaved download and extraction events.
/// See the [observer contract](crate#observers-and-async-execution).
///
/// # Errors
///
/// Returns an error for discovery, pinned selection mismatch, invalid patch chains,
/// lock contention, transfer, filesystem, archive, or delta failures. Pending downloads
/// are stopped after extraction failure. There is no whole-install rollback; retries
/// preserve component checkpoints and the original sources of an unfinished delta patch.
pub async fn install_with_observer<F>(
    options: InstallOptions,
    observer: F,
) -> Result<InstallOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let client = WgusClient::new()?;
    let (manifest, root) =
        resolve_installation(&client, &options.selection, options.output).await?;
    install_resolved(manifest, root, options.download, observer).await
}

pub(super) async fn install_resolved<F>(
    manifest: crate::PatchManifest,
    root: PathBuf,
    download: DownloadOptions,
    observer: F,
) -> Result<InstallOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let assets = installation_assets(&manifest)?;
    let plan = build_plan(manifest, root, assets)?;
    observer(InstallEvent::Planned(Box::new(plan.clone())));

    let extractor = PackageExtractor::new();
    let report = extractor
        .download_and_extract_with_observer(
            AssetDownloader::new()?,
            plan.manifest.clone(),
            &plan.root,
            download,
            Default::default(),
            move |event| observer(InstallEvent::Progress(event)),
        )
        .await?;

    Ok(InstallOutcome { plan, report })
}
