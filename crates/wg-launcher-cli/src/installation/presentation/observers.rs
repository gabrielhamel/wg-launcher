//! Presentation adapters own shared progress state and its event/finish lifecycle.

use super::output::print_plan;
use super::progress::{DownloadProgress, ExtractionProgress};
use crate::terminal::human_bytes;
use std::sync::{Arc, Mutex};
use wg_update_client::{
    DownloadAssetsEvent, ExtractEvent, ExtractionReport, InstallEvent, InstallationReport,
};

#[derive(Clone, Default)]
pub(super) struct InstallObserver {
    state: Arc<Mutex<Option<(DownloadProgress, ExtractionProgress)>>>,
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
                match DownloadProgress::new(
                    plan.compressed_bytes,
                    plan.packages
                        .iter()
                        .map(|asset| (asset.name.as_str(), asset.size)),
                ) {
                    Ok(download) => match ExtractionProgress::new(&download) {
                        Ok(extraction) => {
                            *self.state.lock().expect("progress lock") =
                                Some((download, extraction));
                        }
                        Err(error) => {
                            eprintln!("warning: terminal progress unavailable: {error}");
                        }
                    },
                    Err(error) => {
                        eprintln!("warning: terminal progress unavailable: {error}");
                    }
                }
            }
            InstallEvent::Progress(event) => {
                let guard = self.state.lock().expect("progress lock");
                let Some((download, extraction)) = guard.as_ref() else {
                    return;
                };
                match event {
                    wg_update_client::InstallationEvent::Download(event) => download.update(event),
                    wg_update_client::InstallationEvent::Extraction(event) => {
                        extraction.update(event)
                    }
                    wg_update_client::InstallationEvent::ReleasedPackages => {
                        extraction.finish_patch();
                    }
                }
            }
        }
    }

    pub(super) fn finish_success(&self, report: &InstallationReport) {
        if let Some((download, extraction)) = self.state.lock().expect("progress lock").as_ref() {
            download.finish_success();
            extraction.finish_success(&report.extraction);
        }
    }

    pub(super) fn finish_error(&self) {
        if let Some((download, extraction)) = self.state.lock().expect("progress lock").as_ref() {
            download.finish_error();
            extraction.finish_error();
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
