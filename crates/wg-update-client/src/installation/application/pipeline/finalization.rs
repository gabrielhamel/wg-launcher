//! Record the completed installation and clean up successful work.

use super::reports::PackageRetention;
use crate::Result;
use crate::installation::infrastructure::filesystem::{
    cleanup::{count_files_before_finalization, remove_download_cache, remove_staging},
    records::save_installed_build,
    storage::InstallationLayout,
};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport, PinnedManifest};
use std::sync::Arc;

pub(super) fn finalize_installation(
    manifest: &PinnedManifest,
    storage: &InstallationLayout,
    options: ExtractionOptions,
    applied_deltas: usize,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let installed = manifest.complete_installation()?;
    let files_before_finalization = count_files_before_finalization(storage)?;
    let mut report = ExtractionReport {
        files_before_finalization,
        applied_deltas,
        cache_removed: false,
    };
    observer(ExtractionEvent::Finalizing);
    save_installed_build(&installed, storage)?;
    remove_staging(storage)?;
    if let PackageRetention::ReleaseAfterCheckpoint = options.package_retention() {
        observer(ExtractionEvent::CleaningCache);
        remove_download_cache(storage)?;
        report.cache_removed = true;
    }
    observer(ExtractionEvent::Completed {
        files: report.files_before_finalization,
        deltas: report.applied_deltas,
        cache_removed: report.cache_removed,
    });
    Ok(report)
}
