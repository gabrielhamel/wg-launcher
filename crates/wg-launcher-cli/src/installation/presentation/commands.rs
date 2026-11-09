use super::{
    args::{DownloadArgs, ExtractArgs, InstallArgs, PathArgs, UpdateArgs},
    observers::{DownloadObserver, ExtractObserver, InstallObserver},
    output::{print_download_summary, print_extract_summary, print_install_summary},
    status::{print_current_target, print_status},
};
use crate::terminal::DiscoveryProgress;
use std::path::PathBuf;
use wg_update_client::{
    DownloadAssetsOptions, DownloadOptions, ExtractOptions, InstallOptions, StatusOptions,
    UpdateOptions, download_with_observer, extract_with_observer, install_with_observer, status,
    update_with_observer,
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
            print_download_summary(&outcome);
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
            print_extract_summary(&outcome.report);
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
                print_current_target(&outcome.status);
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
