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
    state: Arc<Mutex<DownloadState>>,
}

struct DownloadState {
    multi: MultiProgress,
    overall: ProgressBar,
    asset_count: usize,
    downloading: bool,
    active_style: ProgressStyle,
    waiting_style: ProgressStyle,
    file_style: ProgressStyle,
    queued_style: ProgressStyle,
    error_style: ProgressStyle,
    files: HashMap<String, FileProgress>,
}

/// Keep a volume's bar, byte position, and activity together through retries.
struct FileProgress {
    bar: ProgressBar,
    position: u64,
    activity: FileActivity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileActivity {
    Queued,
    Transferring,
    Completed,
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

        let mut state = DownloadState {
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
        };
        for (name, size) in assets {
            state.ensure_file(name, size);
        }
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) fn update(&self, event: DownloadEvent) {
        let mut state = self.state.lock().expect("download progress lock");
        match event {
            DownloadEvent::Started {
                name,
                size,
                resumed_at,
            } => {
                state.ensure_file(&name, size);
                state.start_file(&name);
                state.set_position(&name, resumed_at);
            }
            DownloadEvent::Progress {
                name,
                downloaded,
                size,
            } => {
                state.ensure_file(&name, size);
                state.start_file(&name);
                state.set_position(&name, downloaded);
            }
            DownloadEvent::Completed { name, size } | DownloadEvent::Skipped { name, size } => {
                state.ensure_file(&name, size);
                state.set_position(&name, size);
                state.finish_file(&name);
            }
        }
        if state.files.len() == state.asset_count
            && state
                .files
                .values()
                .all(|file| file.activity == FileActivity::Completed)
        {
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
        for file in state.files.values() {
            let bar = &file.bar;
            if file.activity != FileActivity::Completed {
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

impl DownloadState {
    fn refresh_activity(&mut self) {
        let downloading = self
            .files
            .values()
            .any(|file| file.activity == FileActivity::Transferring);
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
        self.files.insert(
            name.to_owned(),
            FileProgress {
                bar,
                position: 0,
                activity: FileActivity::Queued,
            },
        );
    }

    fn start_file(&mut self, name: &str) {
        let file = self.files.get_mut(name).expect("known download");
        if file.activity != FileActivity::Transferring {
            file.activity = FileActivity::Transferring;
            file.bar.set_style(self.file_style.clone());
            file.bar.reset_eta();
            file.bar.enable_steady_tick(Duration::from_millis(100));
        }
    }

    fn finish_file(&mut self, name: &str) {
        let file = self.files.get_mut(name).expect("known download");
        file.activity = FileActivity::Completed;
        file.bar.disable_steady_tick();
        file.bar.finish_and_clear();
        self.multi.remove(&file.bar);
    }

    fn set_position(&mut self, name: &str, position: u64) {
        let file = self.files.get_mut(name).expect("known download");
        if position > file.position {
            self.overall.inc(position - file.position);
        }
        file.position = position;
        file.bar.set_position(position);
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
