use crate::progress::{DiscoveryProgress, DownloadProgress, ExtractionProgress};
use clap::{Args, Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table, presets::UTF8_FULL,
};
use std::{path::PathBuf, process::ExitCode, str::FromStr, sync::Arc};
use wg_launcher::{
    BuildSelection, BuildSummary, DownloadOptions, InstallEvent, InstallOptions, ListOptions,
    Region, StatusOptions, UpdateOptions, install, list, status, update,
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
    List(ListArgs),
    Install(InstallArgs),
    Status(PathArgs),
    Update(UpdateArgs),
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
        Command::Install(args) => run_install(args).await?,
        Command::Status(args) => run_status(args).await?,
        Command::Update(args) => run_update(args).await?,
    }
    Ok(())
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
        download.finish_success(report.download.downloaded_files + report.download.skipped_files);
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
    fn standalone_asset_commands_are_not_exposed() {
        for command in ["download", "extract", "inspect"] {
            let error = Cli::try_parse_from(["wg-launcher", command]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidSubcommand);
        }
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
}
