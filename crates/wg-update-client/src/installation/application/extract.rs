use super::InstallPlan;
use crate::installation::infrastructure::filesystem::records::load_pinned_manifest;
use crate::installation::{ExtractionEvent, ExtractionReport, InstallationPipeline};
use crate::{Result, installation::infrastructure::filesystem::InstallationLayout};
use serde::Serialize;
use std::path::PathBuf;

/// Configures extraction of the complete pinned package cache.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Installation root containing a complete `.wg-launcher/download` cache and pinned manifest.
    /// Use `".".into()` explicitly to select the current directory.
    pub path: PathBuf,
}

/// Local extraction observer event.
///
/// A single plan precedes progress.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum ExtractEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`ExtractionEvent`] stream.
    Progress(ExtractionEvent),
}

/// Plan and results from successful local extraction.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractOutcome {
    /// Pinned build and required cached assets.
    pub plan: InstallPlan,
    /// Installed file count, applied deltas, and cleanup result.
    pub report: ExtractionReport,
}

/// Runs [`extract_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`extract_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`extract_with_observer`].
pub async fn extract(options: ExtractOptions) -> Result<ExtractOutcome> {
    extract_with_observer(options, |_| {}).await
}

/// Installs the complete pinned package cache without contacting the live catalog.
///
/// Reads `.wg-launcher/download/manifest.json`, acquires the installation lock, extracts
/// packages, applies deltas, and writes the installed-build record. Success removes
/// `.wg-launcher/download` and `.wg-launcher/installing`. Inner game-owned `.pkg` files remain intact.
/// The observer receives the plan before extraction progress.
///
/// # Errors
///
/// Returns an error if the manifest is missing or invalid, required volumes are
/// incomplete, the patch chain is invalid, the installation is locked, or filesystem,
/// archive, delta, or background-task operations fail. Failed extraction may leave
/// partially changed game files and temporary work; it is not a transactional rollback.
pub async fn extract_with_observer<F>(
    options: ExtractOptions,
    observer: F,
) -> Result<ExtractOutcome>
where
    F: Fn(ExtractEvent) + Send + Sync + 'static,
{
    let root = std::path::absolute(options.path)?;
    let cache = InstallationLayout::for_root(&root).cache;
    let manifest = load_pinned_manifest(&cache).await?.ok_or_else(|| {
        crate::installation::ExtractError::DownloadManifestNotFound(cache.clone())
    })?;
    let pipeline = InstallationPipeline::new(manifest, &root)?;
    let plan = pipeline.plan().clone();
    observer(ExtractEvent::Planned(Box::new(plan.clone())));
    let report = pipeline
        .extract_with_observer(move |event| {
            observer(ExtractEvent::Progress(event));
        })
        .await?;
    Ok(ExtractOutcome { plan, report })
}

/// Missing pinned input for the local extraction workflow.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The package cache had no pinned download manifest.
    #[error("no downloaded manifest was found in {0:?}; run `wg-launcher download` first")]
    DownloadManifestNotFound(std::path::PathBuf),
}
