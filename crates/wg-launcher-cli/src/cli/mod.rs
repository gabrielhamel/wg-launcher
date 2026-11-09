//! Executable argument parsing and cross-context dispatch.

mod args;
use crate::{
    catalog::{run_inspect, run_list},
    installation::{run_download, run_extract, run_install, run_status, run_update},
    self_update,
};
use args::{Cli, Command};
use clap::Parser;
use std::process::ExitCode;

pub async fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayVersion {
                self_update::print_version().await;
                return ExitCode::SUCCESS;
            }
            if error.kind() == clap::error::ErrorKind::DisplayHelp {
                self_update::notify_if_newer(
                    self_update::check_startup_release_notice().await.as_ref(),
                );
            }
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };
    match execute(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(cli.command, Command::SelfUpdate) {
        self_update::notify_if_newer(self_update::check_startup_release_notice().await.as_ref());
    }
    match cli.command {
        Command::List(args) => run_list(args).await?,
        Command::Inspect(args) => run_inspect(args).await?,
        Command::Download(args) => run_download(args).await?,
        Command::Extract(args) => run_extract(args).await?,
        Command::Install(args) => run_install(args).await?,
        Command::Status(args) => run_status(args).await?,
        Command::Update(args) => run_update(args).await?,
        Command::SelfUpdate => self_update::run().await?,
    }
    Ok(())
}
