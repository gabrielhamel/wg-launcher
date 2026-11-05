//! A single stable progress display for CLI executable updates.

use crate::terminal::styles::{overall_style, spinner_style};
use indicatif::{ProgressBar, ProgressStyle, style::TemplateError};
use std::{io, time::Duration};

pub(crate) struct SelfUpdateProgress {
    bar: ProgressBar,
}

impl SelfUpdateProgress {
    pub(crate) fn new() -> Result<Self, TemplateError> {
        let progress = Self {
            bar: ProgressBar::new_spinner(),
        };
        progress.stage("Checking the latest CLI release")?;
        Ok(progress)
    }

    pub(crate) fn stage(&self, message: &str) -> Result<(), TemplateError> {
        self.bar.reset();
        self.bar.unset_length();
        self.bar.set_style(spinner_style()?);
        self.bar.set_message(message.to_owned());
        self.bar.enable_steady_tick(Duration::from_millis(100));
        Ok(())
    }

    pub(crate) fn transfer(&self, message: &str, total: Option<u64>) -> Result<(), TemplateError> {
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

    pub(crate) fn copy(
        &self,
        reader: impl io::Read,
        writer: &mut impl io::Write,
    ) -> io::Result<u64> {
        io::copy(&mut self.bar.wrap_read(reader), writer)
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
