mod product;
mod selection;

pub(crate) use selection::select_products;

pub use product::Product;

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Games available in the built-in Wargaming catalog.
///
/// Display and Serde use the launcher identifiers `wot`, `wows`, `wowp`, and `wotb`.
/// Parsing is case-insensitive and also accepts the full English game names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    /// World of Tanks, including the compatible Lesta Mir Tankov catalogs.
    Wot,
    /// World of Warships.
    Wows,
    /// World of Warplanes.
    Wowp,
    /// World of Tanks Blitz.
    Wotb,
}

impl Game {
    /// All games in the built-in catalog, in discovery order.
    pub const ALL: [Self; 4] = [Self::Wot, Self::Wows, Self::Wowp, Self::Wotb];

    /// Human-readable game name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Wot => "World of Tanks",
            Self::Wows => "World of Warships",
            Self::Wowp => "World of Warplanes",
            Self::Wotb => "World of Tanks Blitz",
        }
    }

    pub(crate) fn products(self) -> Vec<Product> {
        match self {
            Self::Wot => Region::ALL.into_iter().map(Region::product).collect(),
            Self::Wows => vec![worldwide_product("WOWS.WW.PRODUCTION", "World of Warships")],
            Self::Wowp => vec![worldwide_product(
                "WOWP.WW.PRODUCTION",
                "World of Warplanes",
            )],
            Self::Wotb => vec![worldwide_product(
                "WOTB.WW.PRODUCTION",
                "World of Tanks Blitz",
            )],
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wot => "wot",
            Self::Wows => "wows",
            Self::Wowp => "wowp",
            Self::Wotb => "wotb",
        })
    }
}

impl FromStr for Game {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wot" | "world-of-tanks" | "world of tanks" => Ok(Self::Wot),
            "wows" | "world-of-warships" | "world of warships" => Ok(Self::Wows),
            "wowp" | "world-of-warplanes" | "world of warplanes" => Ok(Self::Wowp),
            "wotb" | "world-of-tanks-blitz" | "world of tanks blitz" => Ok(Self::Wotb),
            _ => Err(format!(
                "unknown game '{value}'; expected wot, wows, wowp, or wotb"
            )),
        }
    }
}

fn worldwide_product(game_id: &str, name: &str) -> Product {
    Product {
        game: game_id
            .split('.')
            .next()
            .expect("built-in application family")
            .to_ascii_lowercase(),
        region: "ww".into(),
        name: name.into(),
        host: "https://wgus-eu.wargaming.net/"
            .parse()
            .expect("built-in catalog URL"),
        game_id: game_id.into(),
        channel: "production".into(),
    }
}

/// Supported publisher catalogs. The region also selects a channel and application ID.
///
/// Display and Serde use lowercase identifiers (`eu`, `na`, `asia`, `cn`, `ct`, `ru`, `pt-ru`).
/// Parsing is case-insensitive and also accepts `china`, `russia` and `common-test`.
///
/// ```
/// use wg_update_client::Region;
///
/// assert_eq!("russia".parse::<Region>().unwrap(), Region::Ru);
/// assert_eq!(Region::Ct.to_string(), "ct");
/// assert_eq!(Region::Ru.product().game_id, "MT.RU.PRODUCTION");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    /// Wargaming Europe production.
    Eu,
    /// Wargaming North America production.
    Na,
    /// Wargaming Asia production.
    Asia,
    /// China production.
    Cn,
    /// Wargaming common test.
    Ct,
    /// Lesta Mir Tankov Russia production.
    Ru,
    /// Lesta Mir Tankov Russia Public test.
    PtRu,
}

impl Region {
    /// All supported catalogs in discovery order.
    pub const ALL: [Self; 7] = [
        Self::Eu,
        Self::Na,
        Self::Asia,
        Self::Cn,
        Self::Ct,
        Self::Ru,
        Self::PtRu,
    ];

    /// Returns the publisher endpoint and product identity for this region.
    pub fn product(self) -> Product {
        let (name, host, game_id, channel) = match self {
            Self::Eu => (
                "World of Tanks EU",
                "wgus-woteu.wargaming.net",
                "WOT.EU.PRODUCTION",
                "production",
            ),
            Self::Na => (
                "World of Tanks NA",
                "wgus-wotna.wargaming.net",
                "WOT.NA.PRODUCTION",
                "production",
            ),
            Self::Asia => (
                "World of Tanks Asia",
                "wgus-wotasia.wargaming.net",
                "WOT.ASIA.PRODUCTION",
                "production",
            ),
            Self::Cn => (
                "World of Tanks China",
                "wgus-cn360.wggames.cn",
                "WOT.CN.PRODUCTION",
                "production",
            ),
            Self::Ct => (
                "World of Tanks Common Test",
                "wgus-wotct.wargaming.net",
                "WOT.CT.PRODUCTION",
                "common-test",
            ),
            Self::Ru => (
                "Mir Tankov RU",
                "lstus-ru.lesta.ru",
                "MT.RU.PRODUCTION",
                "production",
            ),
            Self::PtRu => (
                "Mir Tankov Public Test RU",
                "lstus-ru.lesta.ru",
                "MT.PT.PRODUCTION",
                "public-test",
            ),
        };
        Product {
            game: "wot".into(),
            region: self.to_string(),
            name: name.into(),
            host: format!("https://{host}/")
                .parse()
                .expect("built-in catalog URL"),
            game_id: game_id.into(),
            channel: channel.into(),
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Eu => "eu",
            Self::Na => "na",
            Self::Asia => "asia",
            Self::Cn => "cn",
            Self::Ct => "ct",
            Self::Ru => "ru",
            Self::PtRu => "pt-ru",
        })
    }
}

impl FromStr for Region {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "eu" => Ok(Self::Eu),
            "na" => Ok(Self::Na),
            "asia" => Ok(Self::Asia),
            "cn" | "china" => Ok(Self::Cn),
            "ct" | "common-test" => Ok(Self::Ct),
            "ru" | "russia" => Ok(Self::Ru),
            "pt-ru" | "public-test-russia" => Ok(Self::PtRu),
            _ => Err(format!(
                "unknown region '{value}'; expected eu, na, asia, cn, ct, ru, or pt-ru"
            )),
        }
    }
}
