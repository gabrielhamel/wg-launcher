use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use wg_launcher::{DownloadEvent, ExtractionEvent, ExtractionReport};

#[derive(Clone)]
pub(crate) struct DiscoveryProgress {
    bar: ProgressBar,
}

impl DiscoveryProgress {
    pub(crate) fn new(region: Option<wg_launcher::Region>) -> Result<Self, TemplateError> {
        Self::with_message(match region {
            Some(region) => format!("Loading {} builds", region.to_string().to_uppercase()),
            None => "Loading builds from all regions".to_owned(),
        })
    }

    pub(crate) fn with_message(message: impl Into<String>) -> Result<Self, TemplateError> {
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

fn overall_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "{spinner:.green} {msg:.bold} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} • {bytes_per_sec} • ETA {eta}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

fn spinner_style() -> Result<ProgressStyle, TemplateError> {
    Ok(
        ProgressStyle::with_template("{spinner:.green} {msg:.bold}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    )
}

fn file_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "  {spinner:.green} {msg:38} [{bar:24.cyan/blue}] {bytes:>9}/{total_bytes:9} {bytes_per_sec:>10} ETA {eta:>4}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

fn error_style() -> Result<ProgressStyle, TemplateError> {
    ProgressStyle::with_template("  {msg:.bold.red}")
}

fn extraction_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "{spinner:.green} {msg:.bold} {wide_bar:.cyan/blue} {pos}/{len}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

fn extraction_completed_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg:.bold.green}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_tracks_file_bytes_without_replacing_patch_progress() {
        let progress = ExtractionProgress::standalone().unwrap();
        progress.update(ExtractionEvent::ExtractingPatch {
            part: "client".to_owned(),
            version_from: "0".to_owned(),
            version_to: "1".to_owned(),
            index: 2,
            total: 7,
        });
        for extracted in [0, 40, 100] {
            progress.update(ExtractionEvent::ExtractingFile {
                path: "res/packages/test.pkg".to_owned(),
                index: 3,
                total: 10,
                extracted,
                size: 100,
            });
        }
        assert_eq!(progress.file.position(), 100);
        assert_eq!(progress.file.length(), Some(100));
        assert_eq!(
            progress.file.message(),
            "Extracting file 3/10: res/packages/test.pkg"
        );
        assert_eq!(progress.bar.message(), "[2/7] client: 0 → 1");
        progress.update(ExtractionEvent::ExtractingFile {
            path: "next.pkg".to_owned(),
            index: 4,
            total: 10,
            extracted: 0,
            size: 200,
        });
        assert_eq!(progress.file.position(), 0);
        assert_eq!(progress.file.length(), Some(200));
        assert!(!progress.file.is_finished());
        progress.update(ExtractionEvent::ApplyingDelta {
            path: "test.pkg".to_owned(),
        });
        assert!(progress.file.is_finished());
        assert!(!progress.bar.is_finished());
    }

    #[test]
    fn planned_downloads_are_queued_and_completed_rows_are_cleared() {
        let progress = DownloadProgress::new(
            30,
            [("cached", 10), ("active", 10), ("later-patch", 10)].into_iter(),
        )
        .unwrap();
        {
            let state = progress.state.lock().unwrap();
            assert_eq!(state.files.len(), 3);
            assert!(state.active.is_empty());
            assert!(!state.files["later-patch"].is_finished());
            assert_eq!(state.files["later-patch"].length(), Some(10));
        }
        progress.update(DownloadEvent::Skipped {
            name: "cached".to_owned(),
            size: 10,
        });
        progress.update(DownloadEvent::Started {
            name: "active".to_owned(),
            size: 10,
            resumed_at: 4,
        });
        {
            let state = progress.state.lock().unwrap();
            assert!(state.files["cached"].is_finished());
            assert_eq!(state.active.len(), 1);
            assert!(state.active.contains("active"));
            assert_eq!(state.overall.position(), 14);
        }
        progress.update(DownloadEvent::Completed {
            name: "active".to_owned(),
            size: 10,
        });
        let state = progress.state.lock().unwrap();
        assert!(state.files["active"].is_finished());
        assert!(!state.files["later-patch"].is_finished());
        assert!(state.active.is_empty());
        assert_eq!(state.overall.position(), 20);
        assert!(!state.overall.is_finished());
    }

    #[test]
    fn releasing_packages_hides_extraction_until_the_next_patch() {
        let progress = ExtractionProgress::standalone().unwrap();
        progress.finish_patch();
        assert!(progress.bar.is_finished());
        assert!(progress.file.is_finished());
        progress.update(ExtractionEvent::ExtractingPatch {
            part: "client".to_owned(),
            version_from: "0".to_owned(),
            version_to: "1".to_owned(),
            index: 2,
            total: 7,
        });
        assert!(!progress.bar.is_finished());
        assert_eq!(progress.bar.message(), "[2/7] client: 0 → 1");
        progress.finish_patch();
        progress.update(ExtractionEvent::Finalizing);
        assert!(!progress.bar.is_finished());
        assert_eq!(progress.bar.message(), "Finalizing installation");
    }

    #[test]
    fn download_progress_finishes_only_after_all_assets_are_ready() {
        let progress =
            DownloadProgress::new(20, [("cached", 10), ("downloaded", 10)].into_iter()).unwrap();
        progress.update(DownloadEvent::Skipped {
            name: "cached".to_owned(),
            size: 10,
        });
        assert!(!progress.state.lock().unwrap().overall.is_finished());

        progress.update(DownloadEvent::Progress {
            name: "downloaded".to_owned(),
            downloaded: 10,
            size: 10,
        });
        assert!(!progress.state.lock().unwrap().overall.is_finished());

        progress.update(DownloadEvent::Completed {
            name: "downloaded".to_owned(),
            size: 10,
        });
        assert!(progress.state.lock().unwrap().overall.is_finished());
        let extraction = ExtractionProgress::new(&progress).unwrap();
        assert!(!extraction.bar.is_finished());
        progress.finish_success();
        assert!(progress.state.lock().unwrap().overall.is_finished());
    }

    #[test]
    fn download_progress_finishes_when_there_are_no_assets() {
        let progress = DownloadProgress::new(0, [].into_iter()).unwrap();
        assert!(progress.state.lock().unwrap().overall.is_finished());
    }

    #[test]
    fn streaming_download_waits_between_patches_and_resumes() {
        let progress = DownloadProgress::new(
            30,
            [("first", 10), ("prefetched", 10), ("next", 10)].into_iter(),
        )
        .unwrap();
        for name in ["first", "prefetched"] {
            progress.update(DownloadEvent::Started {
                name: name.to_owned(),
                size: 10,
                resumed_at: 0,
            });
        }
        progress.update(DownloadEvent::Completed {
            name: "first".to_owned(),
            size: 10,
        });
        assert!(progress.state.lock().unwrap().downloading);
        progress.update(DownloadEvent::Completed {
            name: "prefetched".to_owned(),
            size: 10,
        });
        {
            let state = progress.state.lock().unwrap();
            assert!(!state.downloading);
            assert!(!state.overall.is_finished());
            assert_eq!(state.overall.position(), 20);
            assert_eq!(state.overall.message(), "Downloaded packages");
        }
        let extraction = ExtractionProgress::new(&progress).unwrap();
        assert!(!extraction.bar.is_finished());
        progress.update(DownloadEvent::Started {
            name: "next".to_owned(),
            size: 10,
            resumed_at: 0,
        });
        {
            let state = progress.state.lock().unwrap();
            assert!(state.downloading);
            assert_eq!(state.overall.position(), 20);
            assert_eq!(state.overall.message(), "Downloading 3 assets");
        }
        progress.update(DownloadEvent::Completed {
            name: "next".to_owned(),
            size: 10,
        });
        assert!(progress.state.lock().unwrap().overall.is_finished());
    }

    #[test]
    fn short_name_uses_the_filename_and_bounds_its_width() {
        assert_eq!(short_name("directory/file.wgpkg"), "file.wgpkg");

        let shortened = short_name("directory/a-very-long-world-of-tanks-volume-name.wgpkg");
        assert_eq!(shortened.chars().count(), 38);
        assert!(shortened.contains('…'));
        assert!(shortened.ends_with("me.wgpkg"));
    }
}
