//! Stable, shared download and extraction bars for the terminal.

mod discovery;
mod styles;
pub(crate) use discovery::DiscoveryProgress;
use styles::{
    error_style, extraction_completed_style, extraction_style, file_style, overall_style,
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use wg_update_client::{DownloadEvent, ExtractionEvent, ExtractionReport};

#[derive(Clone)]
pub(crate) struct DownloadProgress {
    state: Arc<Mutex<State>>,
}

struct State {
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
    positions: HashMap<String, u64>,
    active: HashSet<String>,
}

impl DownloadProgress {
    pub(crate) fn new<'a>(
        total_bytes: u64,
        assets: impl ExactSizeIterator<Item = (&'a str, u64)>,
    ) -> Result<Self, TemplateError> {
        let asset_count = assets.len();
        let multi = MultiProgress::new();
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

        let mut state = State {
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
            positions: HashMap::new(),
            active: HashSet::new(),
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
            DownloadEvent::Completed { name, size } => {
                state.ensure_file(&name, size);
                state.set_position(&name, size);
                state.finish_file(&name);
            }
            DownloadEvent::Skipped { name, size } => {
                state.ensure_file(&name, size);
                state.set_position(&name, size);
                state.finish_file(&name);
            }
        }
        if state.files.len() == state.asset_count
            && state.files.values().all(ProgressBar::is_finished)
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
        for bar in state.files.values() {
            if !bar.is_finished() {
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

#[derive(Clone)]
pub(crate) struct ExtractionProgress {
    bar: ProgressBar,
    file: ProgressBar,
}

impl ExtractionProgress {
    pub(crate) fn new(download: &DownloadProgress) -> Result<Self, TemplateError> {
        let state = download.state.lock().expect("download progress lock");
        Self::with_multi(&state.multi)
    }

    fn with_multi(multi: &MultiProgress) -> Result<Self, TemplateError> {
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(extraction_style()?);
        bar.set_message("Preparing extraction");
        bar.enable_steady_tick(Duration::from_millis(100));
        let file = multi.add(ProgressBar::new(0));
        file.set_style(
            ProgressStyle::with_template("  {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}")?
                .progress_chars("━━╸"),
        );
        file.finish_and_clear();
        Ok(Self { bar, file })
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
                }
                self.file.set_length(size);
                self.file
                    .set_message(format!("Extracting file {index}/{total}: {path}"));
                self.file.set_position(extracted);
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
                self.bar.disable_steady_tick();
                self.bar.set_style(extraction_completed_style());
                let cleanup = if cache_removed {
                    ", package cache removed"
                } else {
                    ""
                };
                self.bar.finish_with_message(format!(
                    "✓ Installation ready ({files} files, {deltas} deltas applied{cleanup})"
                ));
            }
        }
    }

    pub(crate) fn finish_success(&self, report: &ExtractionReport) {
        self.file.finish_and_clear();
        if !self.bar.is_finished() {
            self.bar.disable_steady_tick();
            self.bar.set_style(extraction_completed_style());
            let cleanup = if report.cache_removed {
                ", package cache removed"
            } else {
                ""
            };
            self.bar.finish_with_message(format!(
                "✓ Installation ready ({} files, {} deltas applied{cleanup})",
                report.installed_files, report.applied_deltas,
            ));
        }
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

impl State {
    fn refresh_activity(&mut self) {
        let downloading = !self.active.is_empty();
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

    fn start_file(&mut self, name: &str) {
        if self.active.insert(name.to_owned()) {
            let bar = &self.files[name];
            bar.set_style(self.file_style.clone());
            bar.reset_eta();
            bar.enable_steady_tick(Duration::from_millis(100));
        }
    }

    fn finish_file(&mut self, name: &str) {
        self.active.remove(name);
        let bar = &self.files[name];
        bar.disable_steady_tick();
        bar.finish_and_clear();
        self.multi.remove(bar);
    }

    fn set_position(&mut self, name: &str, position: u64) {
        let previous = self
            .positions
            .insert(name.to_owned(), position)
            .unwrap_or(0);
        if position > previous {
            self.overall.inc(position - previous);
        }
        if let Some(bar) = self.files.get(name) {
            bar.set_position(position);
        }
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
