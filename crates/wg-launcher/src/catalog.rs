use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Supported publisher catalogs. The region also selects a channel and application ID.
///
/// Display and Serde use lowercase identifiers (`eu`, `na`, `asia`, `cn`, `ru`, `ct`).
/// Parsing is case-insensitive and also accepts `china`, `russia`, and `common-test`.
///
/// ```
/// use wg_launcher::Region;
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
    /// Lesta Mir Tankov Russia production.
    Ru,
    /// Wargaming Common Test.
    Ct,
}

impl Region {
    /// All supported catalogs in discovery order.
    pub const ALL: [Self; 6] = [Self::Eu, Self::Na, Self::Asia, Self::Cn, Self::Ru, Self::Ct];

    /// Returns the publisher endpoint and product identity for this region.
    pub const fn product(self) -> Product {
        match self {
            Self::Eu => Product {
                region: self,
                name: "World of Tanks EU",
                host: "wgus-woteu.wargaming.net",
                game_id: "WOT.EU.PRODUCTION",
                channel: "production",
            },
            Self::Na => Product {
                region: self,
                name: "World of Tanks NA",
                host: "wgus-wotna.wargaming.net",
                game_id: "WOT.NA.PRODUCTION",
                channel: "production",
            },
            Self::Asia => Product {
                region: self,
                name: "World of Tanks Asia",
                host: "wgus-wotasia.wargaming.net",
                game_id: "WOT.ASIA.PRODUCTION",
                channel: "production",
            },
            Self::Cn => Product {
                region: self,
                name: "World of Tanks China",
                host: "wgus-cn360.wggames.cn",
                game_id: "WOT.CN.PRODUCTION",
                channel: "production",
            },
            Self::Ru => Product {
                region: self,
                name: "Mir Tankov RU",
                host: "lstus-ru.lesta.ru",
                game_id: "MT.RU.PRODUCTION",
                channel: "production",
            },
            Self::Ct => Product {
                region: self,
                name: "World of Tanks Common Test",
                host: "wgus-wotct.wargaming.net",
                game_id: "WOT.CT.PRODUCTION",
                channel: "common-test",
            },
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
            Self::Ru => "ru",
            Self::Ct => "ct",
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
            "ru" | "russia" => Ok(Self::Ru),
            "ct" | "common-test" => Ok(Self::Ct),
            _ => Err(format!(
                "unknown region '{value}'; expected eu, na, asia, cn, ru, or ct"
            )),
        }
    }
}

/// Publisher endpoint and application identity bound to a catalog region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Product {
    /// Catalog region that owns this product.
    pub region: Region,
    /// Human-readable product name.
    pub name: &'static str,
    /// WGUS-compatible HTTPS hostname, without a scheme or path.
    pub host: &'static str,
    /// Publisher application identifier used for discovery.
    pub game_id: &'static str,
    /// Release channel, such as `production` or `common-test`.
    pub channel: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_lesta_russian_production_catalog() {
        let product = Region::Ru.product();

        assert!(Region::ALL.contains(&Region::Ru));
        assert_eq!("ru".parse(), Ok(Region::Ru));
        assert_eq!(product.host, "lstus-ru.lesta.ru");
        assert_eq!(product.game_id, "MT.RU.PRODUCTION");
        assert_eq!(product.channel, "production");
    }
}
