use super::DownloadProgress;
use crate::terminal::styles::{error_style, extraction_completed_style, extraction_style};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};
use std::time::Duration;
use wg_update_client::{ExtractionEvent, ExtractionReport};

#[derive(Clone)]
pub(crate) struct ExtractionProgress {
    bar: ProgressBar,
    file: ProgressBar,
    file_style: ProgressStyle,
    delta_style: ProgressStyle,
}

impl ExtractionProgress {
    pub(crate) fn new(download: &DownloadProgress) -> Result<Self, TemplateError> {
        Self::with_multi(&download.multi_progress())
    }

    fn with_multi(multi: &MultiProgress) -> Result<Self, TemplateError> {
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(extraction_style()?);
        bar.set_message("Preparing extraction");
        bar.enable_steady_tick(Duration::from_millis(100));
        let file = multi.add(ProgressBar::new(0));
        let file_style =
            ProgressStyle::with_template("  {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}")?
                .progress_chars("━━╸");
        let delta_style = ProgressStyle::with_template(
            "  {spinner:.green} {msg} • {bytes} • {bytes_per_sec} • elapsed {elapsed_precise}",
        )?;
        file.set_style(file_style.clone());
        file.finish_and_clear();
        Ok(Self {
            bar,
            file,
            file_style,
            delta_style,
        })
    }

    pub(crate) fn standalone() -> Result<Self, TemplateError> {
        Self::with_multi(&MultiProgress::new())
    }

    pub(crate) fn update(&self, event: ExtractionEvent) {
        match event {
            ExtractionEvent::CheckingCache { assets } => {
                self.bar
                    .set_message(format!("Checking {assets} cached assets"));
            }
            ExtractionEvent::ExtractingPatch {
                part,
                version_from,
                version_to,
                index,
                total,
            } => {
                self.file.finish_and_clear();
                self.bar.reset();
                self.bar.enable_steady_tick(Duration::from_millis(100));
                self.bar.set_length(total as u64);
                self.bar.set_position(index.saturating_sub(1) as u64);
                self.bar.set_message(format!(
                    "[{index}/{total}] {part}: {version_from} → {version_to}"
                ));
            }
            ExtractionEvent::ExtractingFile {
                path,
                index,
                total,
                extracted,
                size,
            } => {
                if extracted == 0 {
                    self.file.reset();
                    self.file.set_style(self.file_style.clone());
                }
                self.file.set_length(size);
                self.file
                    .set_message(format!("Extracting file {index}/{total}: {path}"));
                self.file.set_position(extracted);
            }
            ExtractionEvent::PreparingDelta { path } => {
                self.file.finish_and_clear();
                self.bar
                    .set_message(format!("Preserving original file: {path}"));
            }
            ExtractionEvent::DeltaProgress { path: _, written } => {
                if written == 0 {
                    self.file.reset();
                    self.file.set_style(self.delta_style.clone());
                    self.file.set_message("Reconstructed");
                    self.file.enable_steady_tick(Duration::from_millis(100));
                }
                self.file.set_position(written);
            }
            ExtractionEvent::ApplyingDelta { path } => {
                self.file.finish_and_clear();
                self.bar.set_message(format!("Applying delta to {path}"));
            }
            ExtractionEvent::InstallingFile { path } => {
                self.file.finish_and_clear();
                self.bar.set_message(format!("Installing {path}"));
            }
            ExtractionEvent::Finalizing => {
                self.file.finish_and_clear();
                self.bar.reset();
                self.bar.enable_steady_tick(Duration::from_millis(100));
                self.bar.set_message("Finalizing installation");
            }
            ExtractionEvent::CleaningCache => {
                self.file.finish_and_clear();
                self.bar.set_message("Removing downloaded packages");
            }
            ExtractionEvent::Completed {
                files,
                deltas,
                cache_removed,
            } => {
                self.file.finish_and_clear();
                if let Some(length) = self.bar.length() {
                    self.bar.set_position(length);
                }
                self.finish_bar(files, deltas, cache_removed);
            }
        }
    }

    pub(crate) fn finish_success(&self, report: &ExtractionReport) {
        self.file.finish_and_clear();
        if !self.bar.is_finished() {
            self.finish_bar(
                report.files_before_finalization,
                report.applied_deltas,
                report.cache_removed,
            );
        }
    }

    fn finish_bar(&self, files_before_finalization: usize, deltas: usize, cache_removed: bool) {
        self.bar.disable_steady_tick();
        self.bar.set_style(extraction_completed_style());
        let cleanup = if cache_removed {
            ", package cache removed"
        } else {
            ""
        };
        self.bar.finish_with_message(format!(
            "✓ Installation ready ({files_before_finalization} files in installation before finalization (including launcher metadata), {deltas} deltas applied{cleanup})"
        ));
    }

    pub(crate) fn finish_error(&self) {
        self.file.finish_and_clear();
        self.bar.disable_steady_tick();
        self.bar
            .set_style(error_style().unwrap_or_else(|_| ProgressStyle::default_spinner()));
        self.bar.abandon_with_message("✗ Extraction failed");
    }

    pub(crate) fn finish_patch(&self) {
        self.file.finish_and_clear();
        self.bar.disable_steady_tick();
        self.bar.finish_and_clear();
    }
}
