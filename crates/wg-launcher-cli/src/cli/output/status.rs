use super::human_bytes;
use console::style;
use std::path::Path;
use wg_update_client::StatusReport;

pub(in crate::cli) fn print_status(report: &StatusReport, current: &Path) {
    println!("Installation: {}", report.path.display());
    println!("Game:         {}", report.installed.product.name);
    println!("Region:       {}", report.installed.region);
    if report.update_available {
        println!(
            "Installed:    {}",
            style(&report.installed.version).yellow()
        );
        println!(
            "Latest:       {}",
            style(&report.latest_version).green().bold()
        );
        println!(
            "Update size:  ~{}",
            human_bytes(report.update_download_bytes)
        );
        let available = style("Update available.").green().bold();
        if current == report.path {
            println!("{available} Run: wg-launcher update");
        } else {
            println!(
                "{available} Run: wg-launcher update --path \"{}\"",
                report.path.display()
            );
        }
    } else {
        println!("Installed:    {}", report.installed.version);
        println!("Latest:       {}", report.latest_version);
        println!("{}", style("Up to date.").green());
    }
}
