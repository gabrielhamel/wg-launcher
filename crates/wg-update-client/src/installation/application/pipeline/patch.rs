use super::reports::PackageRetention;
use crate::Result;
use crate::catalog::Patch;
use crate::installation::domain::PatchIdentity;
use crate::installation::infrastructure::{
    filesystem::{
        cache::remove_patch_volumes, patch_workspace::PatchWorkspace, records::save_manifest,
        storage::InstallationLayout,
    },
    packages::apply_patch_contents,
};
use crate::installation::{ExtractionEvent, ExtractionOptions, PinnedManifest};
use std::sync::Arc;

/// Position in the remaining chain; the index is zero-based until presented to an observer.
pub(super) struct PatchProgress {
    pub(super) index: usize,
    pub(super) total: usize,
}

/// Apply first, persist the checkpoint second, then release recovery sources and packages.
/// Both staged extraction and streaming installation use this ordering.
pub(super) fn apply_and_checkpoint_patch(
    manifest: &mut PinnedManifest,
    patch: &Patch,
    storage: &InstallationLayout,
    progress: PatchProgress,
    options: ExtractionOptions,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<PatchCompletion> {
    observer(ExtractionEvent::ExtractingPatch {
        part: patch.component_id.clone(),
        version_from: patch.source.version().unwrap_or("?").to_owned(),
        version_to: patch.version_to.clone(),
        index: progress.index + 1,
        total: progress.total,
    });
    let workspace = PatchWorkspace::prepare(PatchIdentity::from(patch), &storage.staging)?;
    let applied_deltas =
        apply_patch_contents(patch, &storage.cache, storage.root(), &workspace, observer)?;
    manifest.checkpoint(patch);
    save_manifest(manifest, storage)?;
    workspace.release()?;
    let released_package_bytes = match options.package_retention() {
        PackageRetention::ReleaseAfterCheckpoint => remove_patch_volumes(&storage.cache, patch)?,
        PackageRetention::KeepDownloaded => 0,
    };
    Ok(PatchCompletion {
        applied_deltas,
        released_package_bytes,
    })
}

pub(super) struct PatchCompletion {
    pub(super) applied_deltas: usize,
    pub(super) released_package_bytes: u64,
}
