//! Presentation adapters own shared progress state and its event/finish lifecycle.

use super::output::print_plan;
use super::progress::{DownloadProgress, ExtractionProgress, InstallationProgress};
use super::progress_state::ProgressState;
use crate::terminal::human_bytes;
use wg_update_client::{
    DownloadAssetsEvent, ExtractEvent, ExtractionReport, InstallEvent, InstallationReport,
};

#[derive(Clone, Default)]
pub(super) struct InstallObserver {
    state: ProgressState<InstallationProgress>,
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
                self.state.initialize(InstallationProgress::new(&plan));
            }
            InstallEvent::Progress(event) => {
                self.state.with(|progress| {
                    progress.update(event);
                });
            }
        }
    }

    pub(super) fn finish_success(&self, report: &InstallationReport) {
        self.state.with(|progress| {
            progress.finish_success(report);
        });
    }

    pub(super) fn finish_error(&self) {
        self.state.with(|progress| {
            progress.finish_error();
        });
    }
}

#[derive(Clone, Default)]
pub(super) struct DownloadObserver {
    state: ProgressState<DownloadProgress>,
}

impl DownloadObserver {
    pub(super) fn observe(&self, event: DownloadAssetsEvent) {
        match event {
            DownloadAssetsEvent::Planned(plan) => {
                print_plan(&plan);
                self.state.initialize(DownloadProgress::new(
                    plan.compressed_bytes,
                    plan.packages
                        .iter()
                        .map(|asset| (asset.name.as_str(), asset.size)),
                ));
            }
            DownloadAssetsEvent::Progress(event) => {
                self.state.with(|progress| {
                    progress.update(event);
                });
            }
        }
    }

    pub(super) fn finish_success(&self) {
        self.state.with(|progress| {
            progress.finish_success();
        });
    }

    pub(super) fn finish_error(&self) {
        self.state.with(|progress| {
            progress.finish_error();
        });
    }
}

#[derive(Clone, Default)]
pub(super) struct ExtractObserver {
    state: ProgressState<ExtractionProgress>,
}

impl ExtractObserver {
    pub(super) fn observe(&self, event: ExtractEvent) {
        match event {
            ExtractEvent::Planned(plan) => {
                print_plan(&plan);
                self.state.initialize(ExtractionProgress::standalone());
            }
            ExtractEvent::Progress(event) => {
                self.state.with(|progress| {
                    progress.update(event);
                });
            }
        }
    }

    pub(super) fn finish_success(&self, report: &ExtractionReport) {
        self.state.with(|progress| {
            progress.finish_success(report);
        });
    }

    pub(super) fn finish_error(&self) {
        self.state.with(|progress| {
            progress.finish_error();
        });
    }
}
