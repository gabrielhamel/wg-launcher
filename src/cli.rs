use crate::progress::{DiscoveryProgress, DownloadProgress, ExtractionProgress};
use clap::{Args, Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table, presets::UTF8_FULL,
};
use std::{path::PathBuf, process::ExitCode, str::FromStr, sync::Arc};
use wg_launcher::{
    BuildSelection, BuildSummary, DownloadAssetsEvent, DownloadAssetsOptions, DownloadOptions,
    ExtractEvent, ExtractOptions, InspectOptions, InstallEvent, InstallOptions, ListOptions,
    PatchManifest, Region, StatusOptions, UpdateOptions, download, extract, inspect, install, list,
    status, update,
};

#[derive(Debug, Parser)]
#[command(
    name = "wg-launcher",
    version,
    about = "Discover, download, and install World of Tanks game assets"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List available game builds across one or all regions.
    List(ListArgs),
    /// Inspect a selected build without changing files.
    Inspect(InspectArgs),
    /// Download package files without extracting them.
    Download(DownloadArgs),
    /// Extract a complete package cache into the game folder.
    Extract(ExtractArgs),
    /// Download and extract a game build in one pipeline.
    Install(InstallArgs),
    /// Show the installed and latest available versions.
    Status(PathArgs),
    /// Update an existing installation in place.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
struct InspectArgs {
    #[command(flatten)]
    build: BuildArgs,
}

#[derive(Debug, Args)]
struct DownloadArgs {
    #[command(flatten)]
    build: BuildArgs,

    #[arg(long)]
    output: Option<PathBuf>,

    /// Download only this game part; repeat for multiple parts. Omit to download everything.
    #[arg(long = "part", value_name = "PART")]
    parts: Vec<String>,

    #[arg(long, default_value_t = 3)]
    concurrency: usize,
}

#[derive(Debug, Args)]
struct ExtractArgs {
    /// Downloaded game directory; defaults to the current directory.
    directory: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, default_value = "all")]
    region: String,
}

#[derive(Debug, Clone, Args)]
struct BuildArgs {
    #[arg(long, default_value = "eu")]
    region: Region,

    #[arg(long, default_value = "sd")]
    client_type: String,

    #[arg(long)]
    language: Option<String>,
}

impl From<BuildArgs> for BuildSelection {
    fn from(value: BuildArgs) -> Self {
        Self {
            region: value.region,
            client_type: value.client_type,
            language: value.language,
        }
    }
}

#[derive(Debug, Args)]
struct InstallArgs {
    #[command(flatten)]
    build: BuildArgs,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    concurrency: usize,
}

#[derive(Debug, Args)]
struct PathArgs {
    #[arg(long)]
    path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct UpdateArgs {
    #[arg(long)]
    path: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    concurrency: usize,
}

pub async fn run() -> ExitCode {
    match execute(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::List(args) => run_list(args).await?,
        Command::Inspect(args) => run_inspect(args).await?,
        Command::Download(args) => run_download(args).await?,
        Command::Extract(args) => run_extract(args).await?,
        Command::Install(args) => run_install(args).await?,
        Command::Status(args) => run_status(args).await?,
        Command::Update(args) => run_update(args).await?,
    }
    Ok(())
}

async fn run_inspect(args: InspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::with_message("Inspecting build")?;
    let result = inspect(InspectOptions {
        selection: args.build.into(),
    })
    .await;
    loader.finish();
    print_manifest(&result?);
    Ok(())
}

async fn run_download(args: DownloadArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let result = download(
        DownloadAssetsOptions {
            selection: args.build.into(),
            output: args.output,
            parts: args.parts,
            download: DownloadOptions {
                concurrency: args.concurrency,
                ..DownloadOptions::default()
            },
        },
        move |event| observe_download(event, &observed_progress),
    )
    .await;

    match result {
        Ok(outcome) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.finish_success();
            }
            println!(
                "downloaded: {} new, {} already present, {} transferred",
                outcome.report.downloaded_files,
                outcome.report.skipped_files,
                human_bytes(outcome.report.downloaded_bytes),
            );
            println!("packages kept in {}", outcome.plan.cache.display());
            Ok(())
        }
        Err(error) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.finish_error();
            }
            Err(error.into())
        }
    }
}

async fn run_extract(args: ExtractArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let result = extract(
        ExtractOptions {
            path: game_path(args.directory)?,
        },
        move |event| observe_extract(event, &observed_progress),
    )
    .await;

    match result {
        Ok(outcome) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.finish_success(&outcome.report);
            }
            println!(
                "complete: {} files, {} deltas applied",
                outcome.report.installed_files, outcome.report.applied_deltas,
            );
            Ok(())
        }
        Err(error) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.finish_error();
            }
            Err(error.into())
        }
    }
}

async fn run_status(args: PathArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::with_message("Checking for updates")?;
    let result = status(StatusOptions {
        path: game_path(args.path)?,
    })
    .await;
    loader.finish();
    let report = result?;
    println!("Installation: {}", report.path.display());
    println!("Installed:    {}", report.installed.version);
    println!("Latest:       {}", report.latest_version);
    if report.update_available {
        let current = std::path::absolute(std::env::current_dir()?)?;
        if current == report.path {
            println!("Update available. Run: wg-launcher update");
        } else {
            println!(
                "Update available. Run: wg-launcher update --path \"{}\"",
                report.path.display()
            );
        }
    } else {
        println!("Up to date.");
    }
    Ok(())
}

async fn run_update(args: UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::with_message("Checking for updates")?;
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let observed_loader = loader.clone();
    let result = update(
        UpdateOptions {
            path: game_path(args.path)?,
            download: DownloadOptions {
                concurrency: args.concurrency,
                ..DownloadOptions::default()
            },
        },
        move |event| {
            observed_loader.finish();
            observe_install(event, &observed_progress);
        },
    )
    .await;
    loader.finish();

    match result {
        Ok(outcome) => match outcome.installation {
            Some(installation) => {
                finish_install_progress(&progress, &installation.report);
                print_install_summary(&installation.report);
                Ok(())
            }
            None => {
                println!("Already up to date ({})", outcome.status.installed.version);
                Ok(())
            }
        },
        Err(error) => {
            fail_install_progress(&progress);
            Err(error.into())
        }
    }
}

fn game_path(path: Option<PathBuf>) -> Result<PathBuf, std::io::Error> {
    path.map_or_else(std::env::current_dir, Ok)
}

async fn run_list(args: ListArgs) -> Result<(), Box<dyn std::error::Error>> {
    let region = if args.region.eq_ignore_ascii_case("all") {
        None
    } else {
        Some(Region::from_str(&args.region).map_err(CliError)?)
    };
    let loader = DiscoveryProgress::new(region)?;
    let result = list(ListOptions { region }).await;
    loader.finish();
    let report = result?;

    for warning in &report.warnings {
        eprintln!(
            "warning: {}/{} unavailable: {}",
            warning.region, warning.client_type, warning.message
        );
    }
    println!("Available World of Tanks builds\n");
    println!("{}", build_table(&report.builds));
    println!("\n* default language for the region");
    println!(
        "Install with: wg-launcher install --region <region> --client-type <sd|hd> --language <language>"
    );
    Ok(())
}

async fn run_install(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let result = install(
        InstallOptions {
            selection: args.build.into(),
            output: args.output,
            download: DownloadOptions {
                concurrency: args.concurrency,
                ..DownloadOptions::default()
            },
        },
        move |event| observe_install(event, &observed_progress),
    )
    .await;

    match result {
        Ok(outcome) => {
            finish_install_progress(&progress, &outcome.report);
            print_install_summary(&outcome.report);
            Ok(())
        }
        Err(error) => {
            fail_install_progress(&progress);
            Err(error.into())
        }
    }
}

type ProgressBars = Option<(DownloadProgress, ExtractionProgress)>;

fn finish_install_progress(
    progress: &std::sync::Mutex<ProgressBars>,
    report: &wg_launcher::InstallationReport,
) {
    if let Some((download, extraction)) = progress.lock().expect("progress lock").as_ref() {
        download.finish_success();
        extraction.finish_success(&report.extraction);
    }
}

fn fail_install_progress(progress: &std::sync::Mutex<ProgressBars>) {
    if let Some((download, extraction)) = progress.lock().expect("progress lock").as_ref() {
        download.finish_error();
        extraction.finish_error();
    }
}

fn print_install_summary(report: &wg_launcher::InstallationReport) {
    println!(
        "complete: {} downloaded, {} already present, {} transferred, {} package cache released during installation",
        report.download.downloaded_files,
        report.download.skipped_files,
        human_bytes(report.download.downloaded_bytes),
        human_bytes(report.released_package_bytes),
    );
}

fn observe_install(event: InstallEvent, progress: &std::sync::Mutex<ProgressBars>) {
    match event {
        InstallEvent::Planned(plan) => {
            let plan = *plan;
            println!(
                "{} {} {} {}: {} assets, {} -> {}",
                plan.manifest.region,
                plan.manifest.version,
                plan.manifest.client_type,
                plan.manifest.language,
                plan.assets.len(),
                human_bytes(plan.compressed_bytes),
                plan.root.display(),
            );
            println!("  package cache: {}", plan.cache.display());
            match DownloadProgress::new(plan.compressed_bytes, plan.assets.len()) {
                Ok(download) => match ExtractionProgress::new(&download) {
                    Ok(extraction) => {
                        *progress.lock().expect("progress lock") = Some((download, extraction));
                    }
                    Err(error) => {
                        eprintln!("warning: terminal progress unavailable: {error}");
                    }
                },
                Err(error) => {
                    eprintln!("warning: terminal progress unavailable: {error}");
                }
            }
        }
        InstallEvent::Progress(event) => {
            let guard = progress.lock().expect("progress lock");
            let Some((download, extraction)) = guard.as_ref() else {
                return;
            };
            match event {
                wg_launcher::InstallationEvent::Download(event) => download.update(event),
                wg_launcher::InstallationEvent::Extraction(event) => extraction.update(event),
                wg_launcher::InstallationEvent::ReleasedPackages { part, files, bytes } => {
                    extraction.released_packages(&part, files, bytes);
                }
            }
        }
    }
}

fn observe_download(
    event: DownloadAssetsEvent,
    progress: &std::sync::Mutex<Option<DownloadProgress>>,
) {
    match event {
        DownloadAssetsEvent::Planned(plan) => {
            print_plan(&plan);
            match DownloadProgress::new(plan.compressed_bytes, plan.assets.len()) {
                Ok(download) => *progress.lock().expect("progress lock") = Some(download),
                Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
            }
        }
        DownloadAssetsEvent::Progress(event) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.update(event);
            }
        }
    }
}

fn observe_extract(event: ExtractEvent, progress: &std::sync::Mutex<Option<ExtractionProgress>>) {
    match event {
        ExtractEvent::Planned(plan) => {
            print_plan(&plan);
            match ExtractionProgress::standalone() {
                Ok(extraction) => *progress.lock().expect("progress lock") = Some(extraction),
                Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
            }
        }
        ExtractEvent::Progress(event) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.update(event);
            }
        }
    }
}

fn print_plan(plan: &wg_launcher::InstallPlan) {
    println!(
        "{} {} {} {}: {} assets, {} -> {}",
        plan.manifest.region,
        plan.manifest.version,
        plan.manifest.client_type,
        plan.manifest.language,
        plan.assets.len(),
        human_bytes(plan.compressed_bytes),
        plan.root.display(),
    );
    println!("  package cache: {}", plan.cache.display());
}

fn print_manifest(manifest: &PatchManifest) {
    let architecture = manifest.architecture.as_deref().unwrap_or("unknown");
    println!("Game:          {}", manifest.game_id);
    println!("Region:        {}", manifest.region);
    println!("Channel:       {}", manifest.channel);
    println!("Version:       {}", manifest.version);
    println!("Client:        {}", manifest.client_type);
    println!("Language:      {}", manifest.language);
    println!("Target:        {} / {architecture}", manifest.platform);
    println!("Parts:         {}", manifest.parts.join(", "));
    println!("Languages:     {}", manifest.available_languages.join(", "));
    println!("Packages:      {}", manifest.assets.len());
    println!("Download size: {}", human_bytes(manifest.download_size()));
    println!(
        "Default path:  {}",
        manifest.default_output_directory().display()
    );
    println!("\nPublished patch chain\n");
    println!("{}", patch_table(manifest));
}

fn patch_table(manifest: &PatchManifest) -> Table {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            ["Part", "From", "To", "Packages", "Download size"].map(|heading| {
                Cell::new(heading)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold)
            }),
        );

    for patch in &manifest.patches {
        let size = patch.files.iter().map(|file| file.size).sum();
        table.add_row(vec![
            Cell::new(&patch.part).add_attribute(Attribute::Bold),
            Cell::new(patch.version_from.as_deref().unwrap_or("unknown")),
            Cell::new(&patch.version_to),
            Cell::new(patch.files.len()).set_alignment(CellAlignment::Right),
            Cell::new(human_bytes(size)).set_alignment(CellAlignment::Right),
        ]);
    }

    table
}

fn build_table(summaries: &[BuildSummary]) -> Table {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            [
                "Region",
                "Channel",
                "Client",
                "Version",
                "Target",
                "Size",
                "Languages",
            ]
            .map(|heading| {
                Cell::new(heading)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold)
            }),
        );

    for item in summaries {
        let languages = item
            .languages
            .iter()
            .map(|language| {
                if language.eq_ignore_ascii_case(&item.default_language) {
                    format!("{language}*")
                } else {
                    language.clone()
                }
            })
            .collect::<Vec<_>>();
        let languages = languages
            .chunks(8)
            .map(|line| line.join(", "))
            .collect::<Vec<_>>()
            .join("\n");
        let target = format!(
            "{} / {}",
            item.platform,
            item.architecture.as_deref().unwrap_or("unknown")
        );
        table.add_row(vec![
            Cell::new(item.region.to_string().to_uppercase()).add_attribute(Attribute::Bold),
            Cell::new(&item.channel),
            Cell::new(item.client_type.to_uppercase()),
            Cell::new(&item.version),
            Cell::new(target),
            Cell::new(human_bytes(item.compressed_bytes)).set_alignment(CellAlignment::Right),
            Cell::new(languages),
        ]);
    }

    table
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

#[derive(Debug)]
struct CliError(String);

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn install_accepts_build_selection() {
        assert!(Cli::try_parse_from(["wg-launcher", "install"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "install", "--client-type", "hd"]).is_ok());
    }

    #[test]
    fn status_and_update_default_to_the_current_directory() {
        assert!(Cli::try_parse_from(["wg-launcher", "status"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "update"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "status", "--path", "games/wot"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "update", "--path", "games/wot"]).is_ok());
    }

    #[test]
    fn standalone_asset_commands_are_exposed() {
        assert!(Cli::try_parse_from(["wg-launcher", "download"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "extract"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "extract", "games/wot"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "download", "--part", "client"]).is_ok());
    }

    #[test]
    fn extract_does_not_accept_catalog_selection_arguments() {
        let error = Cli::try_parse_from(["wg-launcher", "extract", "--region", "eu"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn inspect_uses_the_default_selection_without_arguments() {
        assert!(Cli::try_parse_from(["wg-launcher", "inspect"]).is_ok());
        assert!(Cli::try_parse_from(["wg-launcher", "inspect", "--region", "eu"]).is_ok());
    }

    #[test]
    fn list_does_not_expose_json_output() {
        let error = Cli::try_parse_from(["wg-launcher", "list", "--json"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn build_table_marks_the_default_language_and_formats_sizes() {
        let table = build_table(&[BuildSummary {
            region: Region::Eu,
            channel: "production".to_owned(),
            platform: "windows".to_owned(),
            architecture: Some("x64".to_owned()),
            client_type: "sd".to_owned(),
            version: "2.4.1.5493".to_owned(),
            metadata_version: "1".to_owned(),
            default_language: "EN".to_owned(),
            languages: vec!["EN".to_owned(), "FR".to_owned()],
            parts: vec!["client".to_owned()],
            compressed_bytes: 1_073_741_824,
        }])
        .to_string();

        assert!(table.contains("Region"));
        assert!(table.contains("SD"));
        assert!(table.contains("1.0 GiB"));
        assert!(table.contains("EN*"));
        assert!(table.contains("FR"));
    }

    #[test]
    fn patch_table_shows_versions_packages_and_size() {
        let mut manifest = test_manifest();
        manifest.patches = vec![wg_launcher::Patch {
            part: "locale".to_owned(),
            version_from: Some("0".to_owned()),
            version_to: "2.4.1.5498".to_owned(),
            files: vec![wg_launcher::PatchFile {
                name: "locale.wgpkg".to_owned(),
                size: 1_073_741_824,
                unpacked_size: None,
                diffs_size: None,
            }],
            torrent_hash: None,
            torrent_urls: Vec::new(),
        }];

        let table = patch_table(&manifest).to_string();
        assert!(table.contains("locale"));
        assert!(table.contains("2.4.1.5498"));
        assert!(table.contains("1.0 GiB"));
    }

    fn test_manifest() -> PatchManifest {
        PatchManifest {
            region: Region::Eu,
            channel: "production".to_owned(),
            host: "example.invalid".to_owned(),
            game_id: "WOT.EU.PRODUCTION".to_owned(),
            platform: "windows".to_owned(),
            architecture: Some("x64".to_owned()),
            client_type: "sd".to_owned(),
            language: "EN".to_owned(),
            metadata_version: "1".to_owned(),
            version: "2.4.1.5498".to_owned(),
            available_languages: vec!["EN".to_owned()],
            publisher_ids: Vec::new(),
            parts: vec!["locale".to_owned()],
            patches: Vec::new(),
            web_seeds: Vec::new(),
            assets: Vec::new(),
        }
    }
}
