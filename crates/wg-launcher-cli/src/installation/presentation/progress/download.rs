use super::download_accounting::{DownloadAccounting, FileActivity};
use crate::terminal::styles::{error_style, file_style, overall_style};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use wg_update_client::DownloadEvent;

#[derive(Clone)]
pub(crate) struct DownloadProgress {
    state: Arc<Mutex<DownloadDisplay>>,
}

struct DownloadDisplay {
    multi: MultiProgress,
    overall: ProgressBar,
    asset_count: usize,
    downloading: bool,
    active_style: ProgressStyle,
    waiting_style: ProgressStyle,
    file_style: ProgressStyle,
    queued_style: ProgressStyle,
    error_style: ProgressStyle,
    files: HashMap<String, ProgressBar>,
    accounting: DownloadAccounting,
}

impl DownloadProgress {
    pub(crate) fn new<'a>(
        total_bytes: u64,
        assets: impl ExactSizeIterator<Item = (&'a str, u64)>,
    ) -> Result<Self, TemplateError> {
        Self::with_multi(MultiProgress::new(), total_bytes, assets)
    }

    pub(super) fn with_multi<'a>(
        multi: MultiProgress,
        total_bytes: u64,
        assets: impl ExactSizeIterator<Item = (&'a str, u64)>,
    ) -> Result<Self, TemplateError> {
        let asset_count = assets.len();
        let overall = multi.add(ProgressBar::new(total_bytes));
        let active_style = overall_style()?;
        let waiting_style = ProgressStyle::with_template(
            "{spinner:.green} {msg:.bold} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} • Extraction in progress",
        )?
        .progress_chars("━━╸");
        overall.set_style(active_style.clone());
        overall.set_message(format!("Downloading {asset_count} assets"));
        overall.enable_steady_tick(Duration::from_millis(100));
        if asset_count == 0 {
            overall.disable_steady_tick();
            overall.finish_and_clear();
        }

        let mut state = DownloadDisplay {
            multi,
            overall,
            asset_count,
            downloading: false,
            active_style,
            waiting_style,
            file_style: file_style()?,
            queued_style: ProgressStyle::with_template("  {msg:38.dim} queued • {total_bytes}")?,
            error_style: error_style()?,
            files: HashMap::new(),
            accounting: DownloadAccounting::new(asset_count),
        };
        for (name, size) in assets {
            state.accounting.register(name);
            state.ensure_file(name, size);
        }
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) fn update(&self, event: DownloadEvent) {
        let mut state = self.state.lock().expect("download progress lock");
        let update = state.accounting.observe(event);
        state.ensure_file(&update.name, update.size);
        let bar = &state.files[&update.name];
        if update.became_active {
            bar.set_style(state.file_style.clone());
            bar.reset_eta();
            bar.enable_steady_tick(Duration::from_millis(100));
        }
        state.overall.inc(update.advanced_bytes);
        bar.set_position(update.position);
        if update.activity == FileActivity::Completed {
            bar.disable_steady_tick();
            bar.finish_and_clear();
            state.multi.remove(bar);
        }
        if state.accounting.all_complete() {
            state.finish_success();
        } else {
            state.refresh_activity();
        }
    }

    pub(crate) fn finish_success(&self) {
        let state = self.state.lock().expect("download progress lock");
        state.finish_success();
    }

    pub(crate) fn finish_error(&self) {
        let state = self.state.lock().expect("download progress lock");
        for (name, bar) in &state.files {
            if !state.accounting.is_complete(name) {
                bar.set_style(state.error_style.clone());
                bar.abandon_with_message(format!("✗ {}", bar.message()));
            }
        }
        if !state.overall.is_finished() {
            state.overall.disable_steady_tick();
            state.overall.set_style(state.error_style.clone());
            state.overall.abandon_with_message("✗ Download failed");
        }
    }
}

impl DownloadDisplay {
    fn refresh_activity(&mut self) {
        let downloading = self.accounting.is_transferring();
        if downloading == self.downloading {
            return;
        }
        self.downloading = downloading;
        if downloading {
            self.overall.reset_eta();
            self.overall.set_style(self.active_style.clone());
            self.overall
                .set_message(format!("Downloading {} assets", self.asset_count));
        } else {
            self.overall.set_style(self.waiting_style.clone());
            self.overall.set_message("Downloaded packages");
        }
    }

    fn finish_success(&self) {
        self.overall.disable_steady_tick();
        self.overall.finish_and_clear();
    }

    fn ensure_file(&mut self, name: &str, size: u64) {
        if self.files.contains_key(name) {
            return;
        }
        let bar = self
            .multi
            .insert_before(&self.overall, ProgressBar::new(size));
        bar.set_style(self.queued_style.clone());
        bar.set_message(short_name(name));
        self.files.insert(name.to_owned(), bar);
    }
}

fn short_name(name: &str) -> String {
    let name = Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(name);
    const MAX_CHARS: usize = 38;
    if name.chars().count() <= MAX_CHARS {
        return name.to_owned();
    }

    let start = name.chars().take(19).collect::<String>();
    let end = name
        .chars()
        .rev()
        .take(18)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}…{end}")
}
