use super::{
    args::{DownloadArgs, ExtractArgs, InstallArgs, PathArgs, UpdateArgs},
    observers::{DownloadObserver, ExtractObserver, InstallObserver},
    output::print_install_summary,
    status::print_status,
};
use crate::terminal::{DiscoveryProgress, human_bytes};
use std::path::PathBuf;
use wg_update_client::{
    DownloadAssetsOptions, DownloadOptions, ExtractOptions, InstallOptions, StatusOptions,
    UpdateOptions, UpdateTargetSource, download_with_observer, extract_with_observer,
    install_with_observer, status, update_with_observer,
};

pub(crate) async fn run_download(args: DownloadArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = DownloadObserver::default();
    let observed_progress = progress.clone();
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
        move |event| observed_progress.observe(event),
    )
    .await;

    match result {
        Ok(outcome) => {
            progress.finish_success();
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
            progress.finish_error();
            Err(error.into())
        }
    }
}

pub(crate) async fn run_extract(args: ExtractArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = ExtractObserver::default();
    let observed_progress = progress.clone();
    let result = extract_with_observer(
        ExtractOptions {
            path: game_path(args.directory)?,
        },
        move |event| observed_progress.observe(event),
    )
    .await;

    match result {
        Ok(outcome) => {
            progress.finish_success(&outcome.report);
            println!(
                "complete: {} files counted before cleanup, {} deltas applied",
                outcome.report.files_before_finalization, outcome.report.applied_deltas,
            );
            Ok(())
        }
        Err(error) => {
            progress.finish_error();
            Err(error.into())
        }
    }
}

pub(crate) async fn run_status(args: PathArgs) -> Result<(), Box<dyn std::error::Error>> {
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

pub(crate) async fn run_update(args: UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::new("Checking for updates")?;
    let progress = InstallObserver::default();
    let observed_progress = progress.clone();
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
            observed_progress.observe(event);
        },
    )
    .await;
    loader.finish();

    match result {
        Ok(outcome) => match outcome.installation {
            Some(installation) => {
                progress.finish_success(&installation.report);
                print_install_summary(&installation.report);
                Ok(())
            }
            None => {
                match outcome.status.target.source {
                    UpdateTargetSource::LiveCatalog => {
                        println!("Already up to date ({})", outcome.status.installed.version)
                    }
                    UpdateTargetSource::PinnedManifest => println!(
                        "Already at pinned target ({})",
                        outcome.status.target.version
                    ),
                }
                Ok(())
            }
        },
        Err(error) => {
            progress.finish_error();
            Err(error.into())
        }
    }
}

fn game_path(path: Option<PathBuf>) -> Result<PathBuf, std::io::Error> {
    path.map_or_else(std::env::current_dir, Ok)
}

pub(crate) async fn run_install(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    let progress = InstallObserver::default();
    let observed_progress = progress.clone();
    let result = install_with_observer(
        InstallOptions {
            selection: args.build.try_into()?,
            output: args.output,
            download: DownloadOptions {
                concurrency: args.concurrency,
                ..DownloadOptions::default()
            },
        },
        move |event| observed_progress.observe(event),
    )
    .await;

    match result {
        Ok(outcome) => {
            progress.finish_success(&outcome.report);
            print_install_summary(&outcome.report);
            Ok(())
        }
        Err(error) => {
            progress.finish_error();
            Err(error.into())
        }
    }
}
