use crate::catalog::BuildArgs;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(crate) struct DownloadArgs {
    #[command(flatten)]
    pub(crate) build: BuildArgs,

    #[arg(long)]
    pub(crate) output: Option<PathBuf>,

    /// Download only this game part; repeat for multiple parts. Omit to download everything.
    #[arg(long = "part", value_name = "PART")]
    pub(crate) parts: Vec<String>,

    #[arg(long, default_value_t = 3)]
    pub(crate) concurrency: usize,
}

#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// Downloaded game directory; defaults to the current directory.
    pub(crate) directory: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct InstallArgs {
    #[command(flatten)]
    pub(crate) build: BuildArgs,

    #[arg(long)]
    pub(crate) output: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    pub(crate) concurrency: usize,
}

#[derive(Debug, Args)]
pub(crate) struct PathArgs {
    #[arg(long)]
    pub(crate) path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateArgs {
    #[arg(long)]
    pub(crate) path: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    pub(crate) concurrency: usize,
}
