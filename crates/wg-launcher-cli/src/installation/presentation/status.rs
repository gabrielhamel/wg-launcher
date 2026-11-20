use crate::terminal::{command, human_bytes};
use console::style;
use std::path::Path;
use wg_update_client::{StatusReport, UpdateTargetSource};

pub(crate) fn print_status(report: &StatusReport, current: &Path, preload: bool) {
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
        let mut invocation = "wg-launcher update".to_owned();
        if preload {
            invocation.push_str(" --preload");
        }
        if current != report.path {
            invocation.push_str(&format!(" --path \"{}\"", report.path.display()));
        }
        println!("{available} Run: {}", command(&invocation));
    } else {
        println!("Installed:    {}", report.installed.version);
        println!("{target_label:<14}{}", report.target.version);
        println!("{}", style("Up to date.").green());
    }
}

pub(crate) fn print_current_target(report: &StatusReport) {
    match report.target.source {
        UpdateTargetSource::LiveCatalog => {
            println!("Already up to date ({})", report.installed.version)
        }
        UpdateTargetSource::PinnedManifest => {
            println!("Already at pinned target ({})", report.target.version)
        }
    }
}
