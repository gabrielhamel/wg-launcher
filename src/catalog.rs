use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    Eu,
    Na,
    Asia,
    Cn,
    Ru,
    Ct,
}

impl Region {
    pub const ALL: [Self; 6] = [Self::Eu, Self::Na, Self::Asia, Self::Cn, Self::Ru, Self::Ct];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Product {
    pub region: Region,
    pub name: &'static str,
    pub host: &'static str,
    pub game_id: &'static str,
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
