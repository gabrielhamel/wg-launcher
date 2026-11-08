use crate::Result;
use crate::catalog::Patch;
use crate::installation::infrastructure::{
    filesystem::{
        cache::remove_patch_volumes, patch_workspace::PatchWorkspace, records::save_manifest,
        storage::InstallationStorage,
    },
    packages::{
        archive::{decode_patch_contents, install_full_patch},
        patch_application::apply_patch_directory,
    },
};
use crate::installation::{ExtractionEvent, ExtractionOptions, PinnedManifest};
use std::{path::Path, sync::Arc};

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
    storage: &InstallationStorage,
    install_root: &Path,
    progress: PatchProgress,
    options: ExtractionOptions,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<PatchCompletion> {
    observer(ExtractionEvent::ExtractingPatch {
        part: patch.part.clone(),
        version_from: patch.source.version().unwrap_or("?").to_owned(),
        version_to: patch.version_to.clone(),
        index: progress.index + 1,
        total: progress.total,
    });
    let workspace = PatchWorkspace::prepare(patch, &storage.staging)?;
    let applied_deltas =
        extract_patch_sync(patch, &storage.cache, install_root, &workspace, observer)?;
    manifest.checkpoint(patch);
    save_manifest(manifest, storage)?;
    workspace.release()?;
    let released_package_bytes = if options.clean_download_cache {
        remove_patch_volumes(&storage.cache, patch)?
    } else {
        0
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

fn extract_patch_sync(
    patch: &Patch,
    cache: &Path,
    game: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<usize> {
    if patch.is_full_install() {
        install_full_patch(patch, cache, game, workspace, observer)?;
        return Ok(0);
    }

    let patch_dir = workspace.archive();
    decode_patch_contents(patch, cache, workspace, observer)?;
    let applied_deltas = apply_patch_directory(&patch_dir, game, workspace, observer)?;
    workspace.clear_archive()?;
    Ok(applied_deltas)
}
