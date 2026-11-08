//! Record the completed installation and clean up successful work.

use crate::Result;
use crate::installation::infrastructure::filesystem::{
    paths::walk_files, records::save_installed_build, storage::InstallationStorage,
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
    let installed = crate::installation::InstalledBuild::from(manifest);
    save_installed_build(&installed, &storage.installed_state)?;
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
