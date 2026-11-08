use crate::terminal::{command, human_bytes};
use console::style;
use std::path::Path;
use wg_update_client::{StatusReport, UpdateTargetSource};

pub(crate) fn print_status(report: &StatusReport, current: &Path) {
    println!("Installation: {}", report.path.display());
    println!("Game:         {}", report.installed.name);
    println!("Region:       {}", report.installed.region);
    let target_label = match report.target.source {
        UpdateTargetSource::LiveCatalog => "Latest:",
        UpdateTargetSource::PinnedManifest => "Pinned target:",
    };
    if report.update_available {
        println!(
            "Installed:    {}",
            style(&report.installed.version).yellow()
        );
        println!(
            "{target_label:<14}{}",
            style(&report.target.version).green().bold()
        );
        println!(
            "Update size:  ~{}",
            human_bytes(report.update_download_bytes)
        );
        let available = style("Update available.").green().bold();
        if current == report.path {
            println!("{available} Run: {}", command("wg-launcher update"));
        } else {
            println!(
                "{available} Run: {}",
                command(&format!(
                    "wg-launcher update --path \"{}\"",
                    report.path.display()
                ))
            );
        }
    } else {
        println!("Installed:    {}", report.installed.version);
        println!("{target_label:<14}{}", report.target.version);
        println!("{}", style("Up to date.").green());
    }
}
