use super::{
    ExtractionEvent, ExtractionOptions, ExtractionReport,
    archive::extract_archive,
    chain::ordered_patches,
    delta::apply_patch_directory,
    paths::walk_files,
    storage::{
        InstallationStorage, prepare_installation, remove_patch_volumes, verify_cache, write_marker,
    },
};
use crate::{Patch, PatchManifest, Result};
use std::{fs, path::Path, sync::Arc};

#[derive(Clone, Copy)]
pub(super) struct InstallationPaths<'a> {
    pub(super) cache: &'a Path,
    pub(super) game: &'a Path,
    pub(super) work: &'a Path,
}

pub(super) fn extract_patch_sync(
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
    if patch.version_from.as_deref() == Some("0") {
        extract_archive(patch, paths.cache, paths.game, observer)?;
        return Ok(0);
    }

    let patch_dir = paths.work.join(format!("patch-{index}"));
    fs::create_dir_all(&patch_dir)?;
    extract_archive(patch, paths.cache, &patch_dir, observer)?;
    let mut report = ExtractionReport::default();
    apply_patch_directory(&patch_dir, paths.game, paths.work, observer, &mut report)?;
    fs::remove_dir_all(&patch_dir)?;
    Ok(report.applied_deltas)
}

pub(super) fn finalize_installation(
    manifest: &PatchManifest,
    install_root: &Path,
    storage: &InstallationStorage,
    options: ExtractionOptions,
    applied_deltas: usize,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let mut report = ExtractionReport {
        installed_files: walk_files(install_root)?.len(),
        applied_deltas,
        cache_removed: false,
    };
    observer(ExtractionEvent::Finalizing);
    write_marker(manifest, &storage.marker)?;
    if storage.work.exists() {
        fs::remove_dir_all(&storage.work)?;
    }
    if options.clean_download_cache {
        observer(ExtractionEvent::CleaningCache);
        if storage.cache.exists() {
            fs::remove_dir_all(&storage.cache)?;
        }
        report.cache_removed = true;
    }
    observer(ExtractionEvent::Completed {
        files: report.installed_files,
        deltas: report.applied_deltas,
        cache_removed: report.cache_removed,
    });
    Ok(report)
}

pub(super) fn extract_sync(
    manifest: &PatchManifest,
    install_root: &Path,
    storage: &InstallationStorage,
    options: ExtractionOptions,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let cache = &storage.cache;
    observer(ExtractionEvent::CheckingCache {
        assets: manifest.assets.len(),
    });
    let patches = ordered_patches(manifest)?;
    verify_cache(&patches, cache)?;
    prepare_installation(install_root, storage)?;

    let total = patches.len();
    let mut report = ExtractionReport::default();
    let paths = InstallationPaths {
        cache,
        game: install_root,
        work: &storage.work,
    };
    for (index, patch) in patches.into_iter().enumerate() {
        report.applied_deltas += extract_patch_sync(patch, paths, index, total, &observer)?;
        if options.clean_download_cache {
            remove_patch_volumes(cache, patch)?;
        }
    }
    finalize_installation(
        manifest,
        install_root,
        storage,
        options,
        report.applied_deltas,
        observer,
    )
}
