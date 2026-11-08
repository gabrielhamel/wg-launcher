//! Record the completed installation and clean up successful work.

use crate::Result;
use crate::installation::infrastructure::filesystem::{
    paths::walk_files, records::save_installed_build, storage::InstallationLayout,
};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport, PinnedManifest};
use std::{fs, path::Path, sync::Arc};

pub(super) fn finalize_installation(
    manifest: &PinnedManifest,
    storage: &InstallationLayout,
    options: ExtractionOptions,
    applied_deltas: usize,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let files_before_finalization = count_files_before_finalization(storage.root())?;
    let mut report = ExtractionReport {
        files_before_finalization,
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
        files: report.files_before_finalization,
        deltas: report.applied_deltas,
        cache_removed: report.cache_removed,
    });
    Ok(report)
}

/// Count the whole tree before state writes and cleanup, including existing game
/// files and launcher metadata; it does not count newly extracted files.
fn count_files_before_finalization(install_root: &Path) -> Result<usize> {
    Ok(walk_files(install_root)?.len())
}
