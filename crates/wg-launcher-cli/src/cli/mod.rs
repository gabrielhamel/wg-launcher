//! CLI dispatch; parsing, output, and observer adapters live in focused modules.

mod args;
mod observers;
mod output;

use crate::progress::DiscoveryProgress;
use args::{
    Cli, Command, DownloadArgs, ExtractArgs, InspectArgs, InstallArgs, ListArgs, PathArgs,
    UpdateArgs,
};
use clap::Parser;
use observers::{
    fail_install_progress, finish_install_progress, observe_download, observe_extract,
    observe_install,
};
use output::{build_table, human_bytes, print_install_summary, print_manifest, print_status};
use std::{path::PathBuf, process::ExitCode, sync::Arc};
use wg_update_client::{
    DownloadAssetsOptions, DownloadOptions, ExtractOptions, InspectOptions, InstallOptions,
    ListOptions, StatusOptions, UpdateOptions, download_with_observer, extract_with_observer,
    inspect, install_with_observer, list, status, update_with_observer,
};

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
    let loader = DiscoveryProgress::new("Inspecting build")?;
    let result = inspect(InspectOptions {
        selection: args.build.try_into()?,
    })
    .await;
    loader.finish();
    print_manifest(&result?);
    Ok(())
}

async fn run_download(args: DownloadArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let result = download_with_observer(
        DownloadAssetsOptions {
            selection: args.build.try_into()?,
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
    let result = extract_with_observer(
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
    let loader = DiscoveryProgress::new("Checking for updates")?;
    let result = status(StatusOptions {
        path: game_path(args.path)?,
    })
    .await;
    loader.finish();
    let report = result?;
    let current = std::path::absolute(std::env::current_dir()?)?;
    print_status(&report, &current);
    Ok(())
}

async fn run_update(args: UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::new("Checking for updates")?;
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let observed_loader = loader.clone();
    let result = update_with_observer(
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
    let products = args::read_products(args.product.clone())?;
    let game = if args.game.eq_ignore_ascii_case("all") {
        None
    } else {
        Some(args.game)
    };
    let region = if args.region.eq_ignore_ascii_case("all") {
        None
    } else {
        Some(args.region)
    };
    let loader = DiscoveryProgress::new(match (&game, &region) {
        (Some(game), Some(region)) => {
            format!("Loading {game} builds for {}", region.to_uppercase())
        }
        (Some(game), None) => format!("Loading {game} builds from all regions"),
        (None, Some(region)) => {
            format!("Loading all games for {}", region.to_uppercase())
        }
        (None, None) => "Loading builds for all games and regions".to_owned(),
    })?;
    let result = list(ListOptions {
        game,
        region,
        products,
    })
    .await;

    loader.finish();

    let report = result?;

    for warning in &report.warnings {
        eprintln!(
            "warning: {}/{}/{} unavailable: {}",
            warning.game, warning.region, warning.client_type, warning.message
        );
    }
    println!("Available game builds\n");
    println!("{}", build_table(&report.builds));
    println!("\n* default language for the region");
    if let Some(path) = args.product {
        println!(
            "Install with: wg-launcher install --product \"{}\" --game <game> --region <region> --client-type <client-type> --language <language>",
            path.display()
        );
    } else {
        println!(
            "Install with: wg-launcher install --game <game> --region <region> --client-type <client-type> --language <language>"
        );
    }
    Ok(())
}

async fn run_install(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Arc::new(std::sync::Mutex::new(None));
    let observed_progress = Arc::clone(&progress);
    let result = install_with_observer(
        InstallOptions {
            selection: args.build.try_into()?,
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
