//! A single stable progress display for CLI executable updates.

use crate::self_update::{
    Result,
    progress::{SelfUpdateEvent, SelfUpdateStage},
};
use crate::terminal::styles::{overall_style, spinner_style};
use indicatif::{ProgressBar, ProgressStyle, style::TemplateError};
use std::time::Duration;

pub(crate) struct SelfUpdateProgress {
    bar: ProgressBar,
}

impl SelfUpdateProgress {
    pub(crate) fn new() -> Self {
        Self {
            bar: ProgressBar::new_spinner(),
        }
    }

    pub(crate) fn stage(&self, message: &str) -> std::result::Result<(), TemplateError> {
        self.bar.reset();
        self.bar.unset_length();
        self.bar.set_style(spinner_style()?);
        self.bar.set_message(message.to_owned());
        self.bar.enable_steady_tick(Duration::from_millis(100));
        Ok(())
    }

    pub(crate) fn transfer(
        &self,
        message: &str,
        total: Option<u64>,
    ) -> std::result::Result<(), TemplateError> {
        self.stage(message)?;
        if let Some(total) = total {
            self.bar.set_length(total);
            self.bar.set_style(overall_style()?);
        } else {
            self.bar.set_style(ProgressStyle::with_template(
                "{spinner:.green} {msg:.bold} {bytes} • {bytes_per_sec}",
            )?);
        }
        Ok(())
    }

    pub(crate) fn advance(&self, bytes: u64) {
        self.bar.inc(bytes);
    }

    pub(crate) fn observe(&self, event: SelfUpdateEvent) -> Result<()> {
        match event {
            SelfUpdateEvent::Stage(stage) => self.stage(message(stage))?,
            SelfUpdateEvent::ReleaseChecked => self.clear(),
            SelfUpdateEvent::Transfer { stage, total } => self.transfer(message(stage), total)?,
            SelfUpdateEvent::Advanced(bytes) => self.advance(bytes),
        }
        Ok(())
    }

    pub(crate) fn clear(&self) {
        self.bar.disable_steady_tick();
        self.bar.finish_and_clear();
    }
}

impl Drop for SelfUpdateProgress {
    fn drop(&mut self) {
        self.clear();
    }
}

fn message(stage: SelfUpdateStage) -> &'static str {
    match stage {
        SelfUpdateStage::CheckingRelease => "Checking the latest CLI release",
        SelfUpdateStage::FetchingChecksum => "Fetching the CLI archive checksum",
        SelfUpdateStage::DownloadingArchive => "Downloading the CLI archive",
        SelfUpdateStage::VerifyingArchive => "Verifying the CLI archive checksum",
        SelfUpdateStage::OpeningArchive => "Opening the CLI archive",
        SelfUpdateStage::ExtractingExecutable => "Extracting the CLI executable",
        SelfUpdateStage::VerifyingExecutable => "Verifying the CLI executable",
        SelfUpdateStage::ReplacingExecutable => "Replacing the CLI executable",
    }
}
