use super::{
    BuildSelection, InstallPlan,
    manifest::{build_plan, resolve_installation, save_cache_manifest},
};
use crate::{
    Asset, AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport, Result, WgusClient,
    installation_assets,
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
/// This package-only workflow does not acquire the extraction installation lock;
/// callers must serialize access to the same destination.
///
/// The observer receives a plan before progress. Callbacks may run concurrently;
/// see the [crate-level observer contract](crate#observers-and-async-execution).
///
/// # Errors
///
/// Returns an error for discovery or cache I/O failures, a pinned selection mismatch,
/// an invalid patch chain, an empty asset selection, unsafe paths, or exhausted
/// transfer retries. Cached and partial files may remain after failure.
pub async fn download_with_observer<F>(
    options: DownloadAssetsOptions,
    observer: F,
) -> Result<DownloadAssetsOutcome>
where
    F: Fn(DownloadAssetsEvent) + Send + Sync + 'static,
{
    let client = WgusClient::new()?;
    let (manifest, root) =
        resolve_installation(&client, &options.selection, options.output).await?;
    let all_assets = installation_assets(&manifest)?;
    let assets = select_download_assets(all_assets, &options.parts);
    if assets.is_empty() {
        return Err(crate::Error::EmptySelection(options.parts.join(", ")));
    }
    let plan = build_plan(manifest, root, assets)?;
    observer(DownloadAssetsEvent::Planned(Box::new(plan.clone())));
    save_cache_manifest(&plan.manifest, &plan.cache).await?;
    let report = AssetDownloader::new()?
        .download_with_observer(
            plan.assets.clone(),
            &plan.cache,
            options.download,
            move |event| {
                observer(DownloadAssetsEvent::Progress(event));
            },
        )
        .await?;
    Ok(DownloadAssetsOutcome { plan, report })
}

pub(super) fn select_download_assets(assets: Vec<Asset>, parts: &[String]) -> Vec<Asset> {
    if parts.is_empty() {
        return assets;
    }
    assets
        .into_iter()
        .filter(|asset| parts.iter().any(|part| part == &asset.part))
        .collect()
}
