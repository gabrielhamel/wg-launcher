use crate::terminal::human_bytes;

pub(crate) fn print_install_summary(report: &wg_update_client::InstallationReport) {
    println!(
        "complete: {} volumes completed, {} reused, {} newly retained, {} package cache released during installation",
        report.download.completed_volumes,
        report.download.reused_volumes,
        human_bytes(report.download.new_bytes_retained),
        human_bytes(report.released_package_bytes),
    );
}

pub(crate) fn print_plan(plan: &wg_update_client::InstallPlan) {
    println!(
        "{} {} {} {} {}: {} assets, {} -> {}",
        plan.build.game,
        plan.build.region,
        plan.build.version,
        plan.build.client_type,
        plan.build.language,
        plan.packages.len(),
        human_bytes(plan.compressed_bytes),
        plan.root.display(),
    );
    println!("  package cache: {}", plan.cache.display());
}

pub(crate) fn print_download_summary(outcome: &wg_update_client::DownloadAssetsOutcome) {
    println!(
        "download complete: {} volumes completed, {} reused, {} newly retained",
        outcome.report.completed_volumes,
        outcome.report.reused_volumes,
        human_bytes(outcome.report.new_bytes_retained),
    );
    println!("packages kept in {}", outcome.plan.cache.display());
}

pub(crate) fn print_extract_summary(report: &wg_update_client::ExtractionReport) {
    println!(
        "complete: {} files in installation before finalization (including launcher metadata), {} deltas applied",
        report.files_before_finalization, report.applied_deltas,
    );
}

pub(super) fn print_install_plan(plan: &wg_update_client::InstallPlan) {
    println!(
        "{} {} {} {}: {} assets, {} -> {}",
        plan.build.region,
        plan.build.version,
        plan.build.client_type,
        plan.build.language,
        plan.packages.len(),
        human_bytes(plan.compressed_bytes),
        plan.root.display(),
    );
    println!("  package cache: {}", plan.cache.display());
}
