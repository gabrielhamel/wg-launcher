use crate::catalog::{InspectArgs, ListArgs};
use crate::installation::{DownloadArgs, ExtractArgs, InstallArgs, PathArgs, UpdateArgs};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "wg-launcher",
    version,
    about = "Discover, download, install, and update Wargaming game assets"
)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
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
    /// Update the launcher executable to the latest stable release.
    SelfUpdate,
}
