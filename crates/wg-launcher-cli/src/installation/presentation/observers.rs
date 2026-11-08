//! Presentation adapters own shared progress state and its event/finish lifecycle.

use super::output::print_plan;
use super::progress::{DownloadProgress, ExtractionProgress, InstallationProgress};
use crate::terminal::human_bytes;
use std::sync::{Arc, Mutex};
use wg_update_client::{
    DownloadAssetsEvent, ExtractEvent, ExtractionReport, InstallEvent, InstallationReport,
};

#[derive(Clone, Default)]
pub(super) struct InstallObserver {
    state: Arc<Mutex<Option<InstallationProgress>>>,
}

impl InstallObserver {
    pub(super) fn observe(&self, event: InstallEvent) {
        match event {
            InstallEvent::Planned(plan) => {
                let plan = *plan;
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
                match InstallationProgress::new(&plan) {
                    Ok(progress) => *self.state.lock().expect("progress lock") = Some(progress),
                    Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
                }
            }
            InstallEvent::Progress(event) => {
                if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
                    progress.update(event);
                }
            }
        }
    }

    pub(super) fn finish_success(&self, report: &InstallationReport) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_success(report);
        }
    }

    pub(super) fn finish_error(&self) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_error();
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct DownloadObserver {
    state: Arc<Mutex<Option<DownloadProgress>>>,
}

impl DownloadObserver {
    pub(super) fn observe(&self, event: DownloadAssetsEvent) {
        match event {
            DownloadAssetsEvent::Planned(plan) => {
                print_plan(&plan);
                match DownloadProgress::new(
                    plan.compressed_bytes,
                    plan.packages
                        .iter()
                        .map(|asset| (asset.name.as_str(), asset.size)),
                ) {
                    Ok(download) => *self.state.lock().expect("progress lock") = Some(download),
                    Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
                }
            }
            DownloadAssetsEvent::Progress(event) => {
                if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
                    progress.update(event);
                }
            }
        }
    }

    pub(super) fn finish_success(&self) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_success();
        }
    }

    pub(super) fn finish_error(&self) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_error();
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct ExtractObserver {
    state: Arc<Mutex<Option<ExtractionProgress>>>,
}

impl ExtractObserver {
    pub(super) fn observe(&self, event: ExtractEvent) {
        match event {
            ExtractEvent::Planned(plan) => {
                print_plan(&plan);
                match ExtractionProgress::standalone() {
                    Ok(extraction) => *self.state.lock().expect("progress lock") = Some(extraction),
                    Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
                }
            }
            ExtractEvent::Progress(event) => {
                if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
                    progress.update(event);
                }
            }
        }
    }

    pub(super) fn finish_success(&self, report: &ExtractionReport) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_success(report);
        }
    }

    pub(super) fn finish_error(&self) {
        if let Some(progress) = self.state.lock().expect("progress lock").as_ref() {
            progress.finish_error();
        }
    }
}
