use super::{Game, Product};
use crate::{Error, Result};
use std::collections::HashSet;

pub(crate) fn select_products(
    products: Option<&[Product]>,
    game: Option<&str>,
    region: Option<&str>,
) -> Result<Vec<Product>> {
    let Some(products) = products else {
        let games = match game {
            Some(game) => vec![game.parse::<Game>().map_err(Error::InvalidProduct)?],
            None => Game::ALL.to_vec(),
        };
        let products = games
            .into_iter()
            .flat_map(Game::products)
            .filter(|product| {
                region.is_none_or(|region| product.region.eq_ignore_ascii_case(region))
            })
            .collect::<Vec<_>>();
        return require_match(products, game, region, None);
    };
    if products.is_empty() {
        return Err(Error::InvalidProduct(
            "the custom product catalog must not be empty".into(),
        ));
    }
    let mut selections = HashSet::new();
    for product in products {
        product.validate()?;
        let selection = (product.game(), product.region.to_ascii_lowercase());
        if !selections.insert(selection) {
            return Err(Error::InvalidProduct(format!(
                "duplicate region '{}' for game '{}'; configure one product per game and region",
                product.region,
                product.game()
            )));
        }
    }
    let selected: Vec<_> = products
        .iter()
        .filter(|product| {
            game.is_none_or(|game| product.game().eq_ignore_ascii_case(game))
                && region.is_none_or(|region| product.region.eq_ignore_ascii_case(region))
        })
        .cloned()
        .collect();
    require_match(selected, game, region, Some(products))
}

fn require_match(
    selected: Vec<Product>,
    game: Option<&str>,
    region: Option<&str>,
    configured: Option<&[Product]>,
) -> Result<Vec<Product>> {
    if !selected.is_empty() {
        return Ok(selected);
    }
    let available = configured
        .map(|products| {
            products
                .iter()
                .map(|product| format!("{}/{}", product.game(), product.region))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| {
            Game::ALL
                .into_iter()
                .flat_map(Game::products)
                .map(|product| format!("{}/{}", product.game(), product.region))
                .collect::<Vec<_>>()
                .join(", ")
        });
    Err(Error::InvalidProduct(format!(
        "no product configured for region '{}' and game '{}'; available selections: {available}",
        region.unwrap_or("all"),
        game.unwrap_or("all")
    )))
}
