use super::styles::spinner_style;
use indicatif::{ProgressBar, style::TemplateError};
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct DiscoveryProgress {
    bar: ProgressBar,
}

impl DiscoveryProgress {
    pub(crate) fn new(message: impl Into<String>) -> Result<Self, TemplateError> {
        let bar = ProgressBar::new_spinner();
        bar.set_style(spinner_style()?);
        bar.set_message(message.into());
        bar.enable_steady_tick(Duration::from_millis(100));
        Ok(Self { bar })
    }

    pub(crate) fn finish(&self) {
        self.bar.disable_steady_tick();
        self.bar.finish_and_clear();
    }
}
