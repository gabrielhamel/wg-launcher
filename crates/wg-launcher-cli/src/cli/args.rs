use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use wg_update_client::{BuildSelection, Game, Product};

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
}

#[derive(Debug, Args)]
pub(super) struct InspectArgs {
    #[command(flatten)]
    pub(super) build: BuildArgs,
}

#[derive(Debug, Args)]
pub(super) struct DownloadArgs {
    #[command(flatten)]
    pub(super) build: BuildArgs,

    #[arg(long)]
    pub(super) output: Option<PathBuf>,

    /// Download only this game part; repeat for multiple parts. Omit to download everything.
    #[arg(long = "part", value_name = "PART")]
    pub(super) parts: Vec<String>,

    #[arg(long, default_value_t = 3)]
    pub(super) concurrency: usize,
}

#[derive(Debug, Args)]
pub(super) struct ExtractArgs {
    /// Downloaded game directory; defaults to the current directory.
    pub(super) directory: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct ListArgs {
    /// JSON array of custom products, filtered by --game and --region.
    #[arg(long, value_name = "FILE")]
    pub(super) product: Option<PathBuf>,
    #[arg(long, default_value = "all")]
    pub(super) game: String,
    #[arg(long, default_value = "all")]
    pub(super) region: String,
}

#[derive(Debug, Clone, Args)]
pub(super) struct BuildArgs {
    /// JSON array of custom products; the selected entry is saved for status/update.
    #[arg(long, value_name = "FILE")]
    pub(super) product: Option<PathBuf>,
    /// Game to inspect, download, or install.
    #[arg(long)]
    pub(super) game: String,
    #[arg(long)]
    pub(super) region: Option<String>,

    /// Publisher client variant. Omit for the publisher default or smallest variant.
    #[arg(long)]
    pub(super) client_type: Option<String>,

    #[arg(long)]
    pub(super) language: Option<String>,
}

impl TryFrom<BuildArgs> for BuildSelection {
    type Error = wg_update_client::Error;

    fn try_from(value: BuildArgs) -> Result<Self, Self::Error> {
        let products = read_products(value.product)?;
        let requested_built_in_game = value.game.parse::<Game>().ok();
        let region = value.region.unwrap_or_else(|| {
            if products.is_none()
                && matches!(
                    requested_built_in_game,
                    Some(Game::Wows | Game::Wowp | Game::Wotb)
                )
            {
                "ww".into()
            } else {
                "eu".into()
            }
        });
        let game = value.game;
        Ok(Self {
            products,
            game: Some(game),
            region,
            client_type: value.client_type,
            language: value.language,
        })
    }
}

pub(super) fn read_products(
    path: Option<PathBuf>,
) -> wg_update_client::Result<Option<Vec<Product>>> {
    path.map(|path| Ok(serde_json::from_slice(&std::fs::read(path)?)?))
        .transpose()
}

#[derive(Debug, Args)]
pub(super) struct InstallArgs {
    #[command(flatten)]
    pub(super) build: BuildArgs,

    #[arg(long)]
    pub(super) output: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    pub(super) concurrency: usize,
}

#[derive(Debug, Args)]
pub(super) struct PathArgs {
    #[arg(long)]
    pub(super) path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct UpdateArgs {
    #[arg(long)]
    pub(super) path: Option<PathBuf>,

    #[arg(long, default_value_t = 3)]
    pub(super) concurrency: usize,
}
