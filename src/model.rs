use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct GameMetadata {
    pub metadata_version: String,
    pub game_id: String,
    pub name: String,
    pub filesystem_name: String,
    pub environment: String,
    pub default_language: String,
    pub supported_languages: Vec<String>,
    pub default_client_type: String,
    pub client_types: Vec<ClientType>,
    pub executables: Vec<Executable>,
    pub publisher_ids: Vec<String>,
}

impl GameMetadata {
    pub fn client_type(&self, id: &str) -> Option<&ClientType> {
        self.client_types.iter().find(|client| client.id == id)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientType {
    pub id: String,
    pub architecture: Option<String>,
    pub parts: Vec<ClientPart>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientPart {
    pub id: String,
    pub app_type: Option<String>,
    pub language_specific: bool,
    pub integrity_checked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Executable {
    pub path: String,
    pub architecture: Option<String>,
    pub emulation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchManifest {
    pub region: crate::Region,
    pub channel: String,
    pub host: String,
    pub game_id: String,
    pub platform: String,
    pub architecture: Option<String>,
    pub client_type: String,
    pub language: String,
    pub metadata_version: String,
    pub version: String,
    pub available_languages: Vec<String>,
    pub publisher_ids: Vec<String>,
    pub parts: Vec<String>,
    pub patches: Vec<Patch>,
    pub web_seeds: Vec<WebSeed>,
    pub assets: Vec<Asset>,
}

impl PatchManifest {
    pub fn assets_for_parts(&self, parts: &[String]) -> Vec<Asset> {
        if parts.is_empty() {
            return self.assets.clone();
        }
        let selected: BTreeSet<&str> = parts.iter().map(String::as_str).collect();
        self.assets
            .iter()
            .filter(|asset| selected.contains(asset.part.as_str()))
            .cloned()
            .collect()
    }

    pub fn download_size(&self) -> u64 {
        self.assets.iter().map(|asset| asset.size).sum()
    }

    pub fn default_output_directory(&self) -> PathBuf {
        let architecture = self.architecture.as_deref().unwrap_or("unknown-arch");
        PathBuf::from(format!(
            "wot-{}-{}-{}-{}-{}-{}",
            slug(&self.region.to_string()),
            slug(&self.channel),
            slug(&self.platform),
            slug(architecture),
            slug(&self.client_type),
            slug(&self.language),
        ))
    }

    pub(crate) fn legacy_output_directory(&self) -> PathBuf {
        let architecture = self.architecture.as_deref().unwrap_or("unknown-arch");
        PathBuf::from(format!(
            "wot-{}-{}-{}-{}-{}-{}-{}",
            slug(&self.region.to_string()),
            slug(&self.channel),
            slug(&self.version),
            slug(&self.platform),
            slug(architecture),
            slug(&self.client_type),
            slug(&self.language),
        ))
    }
}

fn slug(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous_separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '.' {
            output.push(character);
            previous_separator = false;
        } else if !previous_separator && !output.is_empty() {
            output.push('-');
            previous_separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub part: String,
    pub version_from: Option<String>,
    pub version_to: String,
    pub files: Vec<PatchFile>,
    pub torrent_hash: Option<String>,
    pub torrent_urls: Vec<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchFile {
    pub name: String,
    pub size: u64,
    pub unpacked_size: Option<u64>,
    pub diffs_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSeed {
    pub url: Url,
    pub name: Option<String>,
    pub threads: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub part: String,
    pub version_from: Option<String>,
    pub version_to: String,
    pub name: String,
    pub url: Url,
    pub size: u64,
    pub unpacked_size: Option<u64>,
    pub diffs_size: Option<u64>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_directory_identifies_the_installation_variant() {
        let manifest = PatchManifest {
            region: crate::Region::Eu,
            channel: "production".to_owned(),
            host: "example.invalid".to_owned(),
            game_id: "WOT.EU.PRODUCTION".to_owned(),
            platform: "windows".to_owned(),
            architecture: Some("x64".to_owned()),
            client_type: "sd".to_owned(),
            language: "PT-BR".to_owned(),
            metadata_version: "metadata".to_owned(),
            version: "2.4.1.5493".to_owned(),
            available_languages: vec![],
            publisher_ids: vec![],
            parts: vec![],
            patches: vec![],
            web_seeds: vec![],
            assets: vec![],
        };

        assert_eq!(
            manifest.default_output_directory(),
            PathBuf::from("wot-eu-production-windows-x64-sd-pt-br")
        );

        let encoded = serde_json::to_vec(&manifest).unwrap();
        let decoded: PatchManifest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.version, "2.4.1.5493");
        assert_eq!(decoded.language, "PT-BR");
        assert_eq!(
            decoded.default_output_directory(),
            manifest.default_output_directory()
        );
    }
}
