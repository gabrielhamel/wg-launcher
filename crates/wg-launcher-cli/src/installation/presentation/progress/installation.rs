//! Coordinate download and extraction renderers on one stable terminal surface.

use super::{DownloadProgress, ExtractionProgress};
use indicatif::{MultiProgress, style::TemplateError};
use wg_update_client::{InstallPlan, InstallationEvent, InstallationReport};

pub(crate) struct InstallationProgress {
    _multi: MultiProgress,
    download: DownloadProgress,
    extraction: ExtractionProgress,
}

impl InstallationProgress {
    pub(crate) fn new(plan: &InstallPlan) -> Result<Self, TemplateError> {
        let multi = MultiProgress::new();
        let download = DownloadProgress::with_multi(
            multi.clone(),
            plan.compressed_bytes,
            plan.packages
                .iter()
                .map(|package| (package.name.as_str(), package.size)),
        )?;
        let extraction = ExtractionProgress::with_multi(&multi)?;
        Ok(Self {
            _multi: multi,
            download,
            extraction,
        })
    }

    pub(crate) fn update(&self, event: InstallationEvent) {
        match event {
            InstallationEvent::Download(event) => self.download.update(event),
            InstallationEvent::Extraction(event) => self.extraction.update(event),
            InstallationEvent::ReleasedPackages => self.extraction.finish_patch(),
        }
    }

    pub(crate) fn finish_success(&self, report: &InstallationReport) {
        self.download.finish_success();
        self.extraction.finish_success(&report.extraction);
    }

    pub(crate) fn finish_error(&self) {
        self.download.finish_error();
        self.extraction.finish_error();
    }
}
