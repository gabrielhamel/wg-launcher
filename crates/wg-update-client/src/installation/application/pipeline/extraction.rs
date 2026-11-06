use super::patch::{PatchProgress, apply_and_checkpoint_patch};
use crate::Result;
use crate::installation::infrastructure::filesystem::{
    cache::verify_cache,
    paths::walk_files,
    records::{save_installed_build, save_manifest},
    retry::resume_manifest,
    storage::{InstallationStorage, prepare_installation},
};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport, PinnedManifest};
use std::{fs, path::Path, sync::Arc};

pub(super) fn finalize_installation(
    manifest: &PinnedManifest,
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
    save_installed_build(manifest, &storage.installed_state)?;
    if storage.staging.exists() {
        fs::remove_dir_all(&storage.staging)?;
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
    manifest: &PinnedManifest,
    install_root: &Path,
    storage: &InstallationStorage,
    options: ExtractionOptions,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let cache = &storage.cache;
    observer(ExtractionEvent::CheckingCache {
        assets: manifest.release.assets.len(),
    });
    let mut manifest = resume_manifest(manifest.clone(), storage)?;
    let patches = manifest
        .ordered_patches()?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    verify_cache(&patches.iter().collect::<Vec<_>>(), cache)?;
    save_manifest(&manifest, storage)?;
    prepare_installation(install_root, storage)?;

    let total = patches.len();
    let mut report = ExtractionReport::default();
    for (index, patch) in patches.into_iter().enumerate() {
        let completion = apply_and_checkpoint_patch(
            &mut manifest,
            &patch,
            storage,
            install_root,
            PatchProgress { index, total },
            options,
            &observer,
        )?;
        report.applied_deltas += completion.applied_deltas;
    }
    finalize_installation(
        &manifest,
        install_root,
        storage,
        options,
        report.applied_deltas,
        observer,
    )
}
