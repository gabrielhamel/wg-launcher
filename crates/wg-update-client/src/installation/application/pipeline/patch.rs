use crate::Result;
use crate::catalog::Patch;
use crate::installation::infrastructure::{
    filesystem::{
        cache::remove_patch_volumes,
        retry::{checkpoint_patch, prepare_patch},
        storage::InstallationStorage,
    },
    packages::{archive::extract_archive, delta::apply_patch_directory},
};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport, PinnedManifest};
use std::{fs, path::Path, sync::Arc};

/// Position in the remaining chain; the index is zero-based until presented to an observer.
pub(super) struct PatchProgress {
    pub(super) index: usize,
    pub(super) total: usize,
}

#[derive(Clone, Copy)]
struct InstallationPaths<'a> {
    cache: &'a Path,
    game: &'a Path,
    staging: &'a Path,
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
    let paths = InstallationPaths {
        cache: &storage.cache,
        game: install_root,
        staging: &storage.staging,
    };
    let applied_deltas =
        extract_patch_sync(patch, paths, progress.index, progress.total, observer)?;
    checkpoint_patch(manifest, patch, storage)?;
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
    paths: InstallationPaths<'_>,
    index: usize,
    total: usize,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<usize> {
    observer(ExtractionEvent::ExtractingPatch {
        part: patch.part.clone(),
        version_from: patch.version_from.clone().unwrap_or_else(|| "?".to_owned()),
        version_to: patch.version_to.clone(),
        index: index + 1,
        total,
    });
    let active = prepare_patch(patch, paths.staging)?;
    if patch.is_full_install() {
        extract_archive(patch, paths.cache, paths.game, Some(&active), observer)?;
        return Ok(0);
    }

    let patch_dir = active.join("archive");
    extract_archive(patch, paths.cache, &patch_dir, None, observer)?;
    let mut report = ExtractionReport::default();
    apply_patch_directory(&patch_dir, paths.game, &active, observer, &mut report)?;
    fs::remove_dir_all(&patch_dir)?;
    Ok(report.applied_deltas)
}
