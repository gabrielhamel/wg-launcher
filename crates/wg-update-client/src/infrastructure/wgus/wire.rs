//! WGUS XML wire format; kept separate from public, protocol-independent models.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "protocol")]
pub(crate) struct MetadataXml {
    #[serde(rename = "@wgc_publisher_id", default)]
    pub publisher_ids: String,
    pub version: String,
    pub predefined_section: PredefinedSectionXml,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PredefinedSectionXml {
    pub app_id: String,
    #[serde(default)]
    pub redirect_application_id: Option<String>,
    #[serde(default)]
    pub supported_languages: String,
    pub default_language: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "fs_name", default)]
    pub filesystem_name: String,
    #[serde(rename = "environment_name", default)]
    pub environment: String,
    pub client_types: ClientTypesXml,
    #[serde(default)]
    pub executables: ExecutablesXml,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientTypesXml {
    #[serde(rename = "@default")]
    pub default: String,
    #[serde(rename = "client_type", default)]
    pub types: Vec<ClientTypeXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientTypeXml {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@arch")]
    pub architecture: Option<String>,
    pub client_parts: ClientPartsXml,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientPartsXml {
    #[serde(rename = "client_part", default)]
    pub parts: Vec<ClientPartXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientPartXml {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@app_type")]
    pub app_type: Option<String>,
    #[serde(rename = "@lang", default, deserialize_with = "xml_bool")]
    pub language_specific: bool,
    #[serde(rename = "@integrity", default, deserialize_with = "xml_bool")]
    pub integrity_checked: bool,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ExecutablesXml {
    #[serde(rename = "executable", default)]
    pub executables: Vec<ExecutableXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExecutableXml {
    #[serde(rename = "@arch")]
    pub architecture: Option<String>,
    #[serde(rename = "@emul")]
    pub emulation: Option<String>,
    #[serde(rename = "$text")]
    pub path: String,
}

fn xml_bool<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.is_some_and(|value| value.eq_ignore_ascii_case("true") || value == "1"))
}

#[derive(Debug, Deserialize)]
#[serde(rename = "protocol")]
pub(crate) struct PatchesXml {
    #[serde(default)]
    pub patches_chain: Option<PatchesChainXml>,
    #[serde(default)]
    pub web_seeds: WebSeedsXml,
    #[serde(default)]
    pub version_name: Option<String>,
    #[serde(default)]
    pub redirect_url: Option<String>,
    #[serde(default)]
    pub changed_game_info: Option<ChangedGameInfoXml>,
}

impl PatchesXml {
    pub fn redirect_url(&self) -> Option<&str> {
        self.redirect_url.as_deref().or_else(|| {
            self.changed_game_info
                .as_ref()
                .and_then(|info| info.redirect_url.as_deref())
        })
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChangedGameInfoXml {
    #[serde(default)]
    pub redirect_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchesChainXml {
    #[serde(rename = "patch", default)]
    pub patches: Vec<PatchXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchXml {
    pub files: PatchFilesXml,
    #[serde(default)]
    pub torrent: Option<TorrentXml>,
    pub part: String,
    #[serde(default)]
    pub version_from: Option<String>,
    pub version_to: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchFilesXml {
    #[serde(rename = "file", default)]
    pub files: Vec<PatchFileXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchFileXml {
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub unpacked_size: Option<u64>,
    #[serde(default)]
    pub diffs_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TorrentXml {
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub urls: TorrentUrlsXml,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct TorrentUrlsXml {
    #[serde(rename = "url", default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct WebSeedsXml {
    #[serde(rename = "url", default)]
    pub urls: Vec<WebSeedXml>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebSeedXml {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@threads")]
    pub threads: Option<usize>,
    #[serde(rename = "$text")]
    pub url: String,
}
