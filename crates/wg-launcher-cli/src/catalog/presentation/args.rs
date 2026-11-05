use super::super::infrastructure::read_products;
use clap::Args;
use std::path::PathBuf;
use wg_update_client::{BuildSelection, Game};

#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    #[command(flatten)]
    pub(crate) build: BuildArgs,
}

#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// JSON array of custom products, filtered by --game and --region.
    #[arg(long, value_name = "FILE")]
    pub(crate) product: Option<PathBuf>,
    #[arg(long, default_value = "all")]
    pub(crate) game: String,
    #[arg(long, default_value = "all")]
    pub(crate) region: String,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BuildArgs {
    /// JSON array of custom products; the selected entry is saved for status/update.
    #[arg(long, value_name = "FILE")]
    pub(crate) product: Option<PathBuf>,
    /// Game to inspect, download, or install.
    #[arg(long)]
    pub(crate) game: String,
    #[arg(long)]
    pub(crate) region: Option<String>,

    /// Publisher client variant. Omit for the publisher default or smallest variant.
    #[arg(long)]
    pub(crate) client_type: Option<String>,

    #[arg(long)]
    pub(crate) language: Option<String>,
}

impl TryFrom<BuildArgs> for BuildSelection {
    type Error = Box<dyn std::error::Error>;

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
