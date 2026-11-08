use super::plan::build_plan;
use super::{BuildSelection, InstallPlan, resolution::resolve_installation};
use crate::installation::domain::selection::validate_cached_request;
use crate::installation::infrastructure::filesystem::records::{
    load_pinned_manifest, save_manifest,
};
use crate::{
    Result,
    catalog::TransportVolume,
    installation::infrastructure::filesystem::{InstallationLayout, InstallationLock},
    installation::{DownloadEvent, DownloadOptions, DownloadReport, VolumeDownloader},
};
use serde::Serialize;
use std::path::PathBuf;

/// Configures a package-only download. The complete manifest is pinned inside
/// `<root>/.wg-launcher/download` even when `parts` limits the transferred package set.
#[derive(Debug, Clone, Default)]
pub struct DownloadAssetsOptions {
    /// Build to download; requires a game and otherwise uses EU, the publisher's
    /// default client type (or smallest installable variant), and default language.
    pub selection: BuildSelection,
    /// Installation root; `None` derives a relative path from the resolved build.
    /// An explicit path allows a pinned manifest to be reused before live resolution.
    pub output: Option<PathBuf>,
    /// Exact component identifiers to transfer; empty selects the complete installation.
    /// The complete manifest is pinned even when only some components are transferred.
    pub parts: Vec<String>,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Package-only download observer event.
///
/// A single plan precedes progress.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum DownloadAssetsEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`DownloadEvent`] stream.
    Progress(DownloadEvent),
}

/// Plan and transfer totals from a successful package-only download.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadAssetsOutcome {
    /// Resolved build, destination, and selected assets.
    pub plan: InstallPlan,
    /// Successful transfers and cache hits.
    pub report: DownloadReport,
}

/// Runs [`download_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`download_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`download_with_observer`].
pub async fn download(options: DownloadAssetsOptions) -> Result<DownloadAssetsOutcome> {
    download_with_observer(options, |_| {}).await
}

/// Downloads packages into the installation's `.wg-launcher/download` directory without extracting.
///
/// Reuses a matching pinned manifest, complete cached files, and resumable partial
/// files. Does not write an installed-build record or clean the cache. A filtered
/// download is insufficient for extraction until all required packages are present.
/// Acquires the per-installation mutation lock before pinning or transferring files.
/// Competing download, extraction, or installation workflows fail without modifying the cache.
///
/// The observer receives a plan before progress. Callbacks may run concurrently;
/// see the [crate-level observer contract](crate#observers-and-async-execution).
///
/// # Errors
///
/// Returns an error for lock contention, discovery or cache I/O failures, a pinned selection mismatch,
/// an invalid patch chain, an empty asset selection, unsafe paths, or exhausted
/// transfer retries. Cached and partial files may remain after failure.
pub async fn download_with_observer<F>(
    options: DownloadAssetsOptions,
    observer: F,
) -> Result<DownloadAssetsOutcome>
where
    F: Fn(DownloadAssetsEvent) + Send + Sync + 'static,
{
    let (manifest, root) = resolve_installation(&options.selection, options.output).await?;
    let storage = InstallationLayout::for_root(&root);
    let _lock = InstallationLock::acquire(&storage)?;
    // Discovery happens before locking; another workflow could have checkpointed
    // this installation meanwhile. Read its authoritative manifest under the lock.
    let manifest = if let Some(pending) = load_pinned_manifest(&storage.cache).await? {
        validate_cached_request(&pending, &options.selection)?;
        pending
    } else {
        manifest
    };
    let all_assets = manifest.required_volumes()?;
    let assets = select_download_volumes(all_assets, &options.parts);
    if assets.is_empty() {
        return Err(
            crate::installation::DownloadError::EmptySelection(options.parts.join(", ")).into(),
        );
    }
    let plan = build_plan(&manifest, root, &assets);
    observer(DownloadAssetsEvent::Planned(Box::new(plan.clone())));
    save_manifest(&manifest, &storage)?;
    let report = VolumeDownloader::new()?
        .download_with_observer(assets, &plan.cache, options.download, move |event| {
            observer(DownloadAssetsEvent::Progress(event));
        })
        .await?;
    Ok(DownloadAssetsOutcome { plan, report })
}

pub(super) fn select_download_volumes(
    assets: Vec<TransportVolume>,
    parts: &[String],
) -> Vec<TransportVolume> {
    if parts.is_empty() {
        return assets;
    }
    assets
        .into_iter()
        .filter(|asset| parts.iter().any(|part| part == &asset.component_id))
        .collect()
}

/// Component selection failure in the package download workflow.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// No installation assets matched the component filter.
    #[error("no assets match the selected parts: {0}")]
    EmptySelection(String),
}
