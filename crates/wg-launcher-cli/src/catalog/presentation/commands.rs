use super::super::infrastructure::read_products;
use super::{
    args::{InspectArgs, ListArgs},
    output::{build_table, print_inspection, print_install_hint},
};
use crate::terminal::DiscoveryProgress;
use wg_update_client::{InspectOptions, ListOptions, inspect, list};

pub(crate) async fn run_inspect(args: InspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let loader = DiscoveryProgress::new("Inspecting build")?;
    let result = inspect(InspectOptions {
        selection: args.build.try_into()?,
    })
    .await;
    loader.finish();
    print_inspection(&result?);
    Ok(())
}

pub(crate) async fn run_list(args: ListArgs) -> Result<(), Box<dyn std::error::Error>> {
    let products = read_products(args.product.clone())?;
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
    print_install_hint(args.product.as_deref());
    Ok(())
}
