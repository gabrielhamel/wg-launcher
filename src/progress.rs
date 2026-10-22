use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};
use std::{
    collections::HashMap,
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
    file_style: ProgressStyle,
    skipped_style: ProgressStyle,
    error_style: ProgressStyle,
    files: HashMap<String, ProgressBar>,
    positions: HashMap<String, u64>,
}

impl DownloadProgress {
    pub(crate) fn new(total_bytes: u64, asset_count: usize) -> Result<Self, TemplateError> {
        let multi = MultiProgress::new();
        let overall = multi.add(ProgressBar::new(total_bytes));
        overall.set_style(overall_style()?);
        overall.set_message(format!("Downloading {asset_count} assets"));
        overall.enable_steady_tick(Duration::from_millis(100));
        if asset_count == 0 {
            overall.disable_steady_tick();
            overall.finish_and_clear();
        }

        Ok(Self {
            state: Arc::new(Mutex::new(State {
                multi,
                overall,
                asset_count,
                file_style: file_style()?,
                skipped_style: skipped_style()?,
                error_style: error_style()?,
                files: HashMap::new(),
                positions: HashMap::new(),
            })),
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
                state.set_position(&name, resumed_at);
            }
            DownloadEvent::Progress {
                name,
                downloaded,
                size,
            } => {
                state.ensure_file(&name, size);
                state.set_position(&name, downloaded);
            }
            DownloadEvent::Completed { name, size } => {
                state.ensure_file(&name, size);
                state.set_position(&name, size);
                if let Some(bar) = state.files.get(&name) {
                    bar.finish_with_message(short_name(&name));
                }
            }
            DownloadEvent::Skipped { name, size } => {
                state.ensure_file(&name, size);
                state.set_position(&name, size);
                if let Some(bar) = state.files.get(&name) {
                    bar.set_style(state.skipped_style.clone());
                    bar.finish_with_message(format!(
                        "✓ {} (already downloaded)",
                        short_name(&name)
                    ));
                }
            }
        }
        if state.files.len() == state.asset_count
            && state.files.values().all(ProgressBar::is_finished)
        {
            state.finish_success();
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
}

impl ExtractionProgress {
    pub(crate) fn new(download: &DownloadProgress) -> Result<Self, TemplateError> {
        let state = download.state.lock().expect("download progress lock");
        let bar = state.multi.add(ProgressBar::new_spinner());
        bar.set_style(extraction_style()?);
        bar.set_message("Preparing extraction");
        bar.enable_steady_tick(Duration::from_millis(100));
        Ok(Self { bar })
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
                self.bar.set_length(total as u64);
                self.bar.set_position(index.saturating_sub(1) as u64);
                self.bar.set_message(format!(
                    "[{index}/{total}] {part}: {version_from} → {version_to}"
                ));
            }
            ExtractionEvent::ApplyingDelta { path } => {
                self.bar.set_message(format!("Applying delta to {path}"));
            }
            ExtractionEvent::InstallingFile { path } => {
                self.bar.set_message(format!("Installing {path}"));
            }
            ExtractionEvent::Finalizing => {
                self.bar.set_message("Finalizing installation");
            }
            ExtractionEvent::CleaningCache => {
                self.bar.set_message("Removing downloaded packages");
            }
            ExtractionEvent::Completed {
                files,
                deltas,
                cache_removed,
            } => {
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
        self.bar.disable_steady_tick();
        self.bar
            .set_style(error_style().unwrap_or_else(|_| ProgressStyle::default_spinner()));
        self.bar.abandon_with_message("✗ Extraction failed");
    }

    pub(crate) fn released_packages(&self, part: &str, files: usize, bytes: u64) {
        self.bar.set_message(format!(
            "Released {files} {part} package volume(s) ({})",
            human_bytes(bytes)
        ));
    }
}

impl State {
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
        bar.set_style(self.file_style.clone());
        bar.set_message(short_name(name));
        bar.enable_steady_tick(Duration::from_millis(100));
        self.files.insert(name.to_owned(), bar);
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

fn skipped_style() -> Result<ProgressStyle, TemplateError> {
    Ok(
        ProgressStyle::with_template("  {msg:60.dim} [{bar:24.green}] {bytes:>9}/{total_bytes:9}")?
            .progress_chars("━━╸"),
    )
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

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_progress_finishes_only_after_all_assets_are_ready() {
        let progress = DownloadProgress::new(20, 2).unwrap();
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
        let progress = DownloadProgress::new(0, 0).unwrap();
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
