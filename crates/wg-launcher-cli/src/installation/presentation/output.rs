use crate::terminal::human_bytes;

pub(crate) fn print_install_summary(report: &wg_update_client::InstallationReport) {
    println!(
        "complete: {} downloaded, {} already present, {} transferred, {} package cache released during installation",
        report.download.downloaded_files,
        report.download.skipped_files,
        human_bytes(report.download.downloaded_bytes),
        human_bytes(report.released_package_bytes),
    );
}

pub(crate) fn print_plan(plan: &wg_update_client::InstallPlan) {
    println!(
        "{} {} {} {} {}: {} assets, {} -> {}",
        plan.manifest.product.game(),
        plan.manifest.region,
        plan.manifest.version,
        plan.manifest.client_type,
        plan.manifest.language,
        plan.assets.len(),
        human_bytes(plan.compressed_bytes),
        plan.root.display(),
    );
    println!("  package cache: {}", plan.cache.display());
}
