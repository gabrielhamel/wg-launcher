use crate::{
    Asset, ClientPart, ClientType, Error, Executable, GameMetadata, Patch, PatchFile,
    PatchManifest, Product, Region, Result, WebSeed,
    model::{MetadataXml, PatchesXml},
};
use reqwest::Client;
use std::{collections::HashMap, time::Duration};
use url::Url;

/// WGUS protocol version sent in metadata and patch-chain requests.
pub const PROTOCOL_VERSION: &str = "100500.6969696";
const INSTALLATION_ID: &str = "wg-launcher";
const MAX_REDIRECTS: usize = 3;

/// Reusable client for live WGUS-compatible metadata and full installation chains.
///
/// Supports the catalogs in [`Region`], including Lesta RU. Resolves transport URLs
/// without downloading package contents. Use [`AssetDownloader`](crate::AssetDownloader)
/// for transfers or the high-level workflows for installation lifecycle management.
#[derive(Clone)]
pub struct WgusClient {
    http: Client,
}

impl WgusClient {
    /// Creates a protocol client with a 15-second connection timeout, 60-second
    /// request timeout, and the crate version in its user agent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if the HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("wg-launcher/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http })
    }

    /// Uses a caller-configured HTTP client without adding default client settings.
    ///
    /// Useful for application-specific TLS, timeout, proxy, or user-agent policies.
    pub fn with_http_client(http: Client) -> Self {
        Self { http }
    }

    /// Fetches metadata for a region's configured publisher application.
    ///
    /// Does not fetch package contents or write local state.
    ///
    /// # Errors
    ///
    /// Returns an error for failed HTTP requests or invalid metadata XML.
    pub async fn metadata(&self, region: Region) -> Result<GameMetadata> {
        let product = region.product();
        let (_, metadata) = self.fetch_metadata(product.host, product.game_id).await?;
        Ok(metadata)
    }

    /// Resolves a full installation manifest for a client variant and optional language.
    ///
    /// Client type matching is exact. Language is normalized; `None` selects the
    /// publisher default. Requests component chains from version `0`, follows bounded
    /// protocol host redirects, and resolves asset URLs using the first HTTP seed.
    /// Omitted source versions are inferred per component, starting at `0`.
    /// No packages are downloaded and no filesystem state is changed.
    ///
    /// # Errors
    ///
    /// Returns an error for HTTP/XML/URL failures, unknown client variants, unsupported
    /// languages, missing patch chains or web seeds, or an exceeded redirect limit.
    pub async fn manifest(
        &self,
        region: Region,
        client_type: &str,
        language: Option<&str>,
    ) -> Result<PatchManifest> {
        let product = region.product();
        let mut host = product.host.to_owned();

        for hop in 0..=MAX_REDIRECTS {
            let (wire_metadata, metadata) = self.fetch_metadata(&host, product.game_id).await?;
            let selected = metadata.client_type(client_type).cloned().ok_or_else(|| {
                Error::UnknownClientType {
                    requested: client_type.to_owned(),
                    available: metadata
                        .client_types
                        .iter()
                        .map(|value| value.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                }
            })?;
            let language = normalize_language(language.unwrap_or(&metadata.default_language));
            if !metadata.supported_languages.is_empty()
                && !metadata
                    .supported_languages
                    .iter()
                    .any(|item| item == &language)
            {
                return Err(Error::UnsupportedLanguage {
                    requested: language,
                    available: metadata.supported_languages.join(", "),
                });
            }
            let wire_language = wire_metadata
                .predefined_section
                .supported_languages
                .split(',')
                .find(|candidate| normalize_language(candidate) == language)
                .map(str::trim)
                .unwrap_or(&wire_metadata.predefined_section.default_language)
                .to_owned();

            let app_id = wire_metadata
                .predefined_section
                .redirect_application_id
                .as_deref()
                .unwrap_or(&wire_metadata.predefined_section.app_id);
            let endpoint = format!("https://{host}/api/v1/patches_chain/");
            let mut query = vec![
                ("game_id".to_owned(), app_id.to_owned()),
                ("protocol_version".to_owned(), PROTOCOL_VERSION.to_owned()),
                (
                    "metadata_protocol_version".to_owned(),
                    PROTOCOL_VERSION.to_owned(),
                ),
                ("installation_id".to_owned(), INSTALLATION_ID.to_owned()),
                ("client_type".to_owned(), client_type.to_owned()),
                ("lang".to_owned(), wire_language),
                (
                    "metadata_version".to_owned(),
                    metadata.metadata_version.clone(),
                ),
            ];
            for part in &selected.parts {
                query.push((format!("{}_current_version", part.id), "0".to_owned()));
            }

            let text = self
                .http
                .get(endpoint)
                .query(&query)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let response: PatchesXml = quick_xml::de::from_str(&text)?;

            if let Some(redirect) = response.redirect_url() {
                let redirected_host = Url::parse(redirect)?
                    .host_str()
                    .ok_or(url::ParseError::EmptyHost)?
                    .to_owned();
                if redirected_host != host {
                    if hop == MAX_REDIRECTS {
                        return Err(Error::RedirectLimit(product.game_id.to_owned()));
                    }
                    host = redirected_host;
                    continue;
                }
            }

            return Self::build_manifest(product, host, metadata, &selected, language, response);
        }

        Err(Error::RedirectLimit(product.game_id.to_owned()))
    }

    async fn fetch_metadata(
        &self,
        host: &str,
        game_id: &str,
    ) -> Result<(MetadataXml, GameMetadata)> {
        let endpoint = format!("https://{host}/api/v1/metadata/");
        let text = self
            .http
            .get(endpoint)
            .query(&[
                ("guid", game_id),
                ("chain_id", "unknown"),
                ("protocol_version", PROTOCOL_VERSION),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let wire: MetadataXml = quick_xml::de::from_str(&text)?;
        let metadata = metadata_from_wire(&wire);
        Ok((wire, metadata))
    }

    fn build_manifest(
        product: Product,
        host: String,
        metadata: GameMetadata,
        selected: &ClientType,
        language: String,
        response: PatchesXml,
    ) -> Result<PatchManifest> {
        let chain = response
            .patches_chain
            .ok_or_else(|| Error::MissingPatchChain(product.game_id.to_owned()))?;
        let web_seeds = response
            .web_seeds
            .urls
            .into_iter()
            .map(|seed| {
                Ok(WebSeed {
                    url: Url::parse(seed.url.trim())?,
                    name: seed.name,
                    threads: seed.threads,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let seed = web_seeds
            .first()
            .ok_or_else(|| Error::MissingWebSeed(product.game_id.to_owned()))?;

        let mut patches = Vec::with_capacity(chain.patches.len());
        let mut assets = Vec::new();
        let mut latest_version_by_part = HashMap::new();
        for patch in chain.patches {
            let version_from = inferred_version_from(
                patch.version_from,
                &patch.part,
                &patch.version_to,
                &mut latest_version_by_part,
            );
            let files = patch
                .files
                .files
                .into_iter()
                .map(|file| PatchFile {
                    name: file.name,
                    size: file.size,
                    unpacked_size: file.unpacked_size,
                    diffs_size: file.diffs_size,
                })
                .collect::<Vec<_>>();
            for file in &files {
                assets.push(Asset {
                    part: patch.part.clone(),
                    version_from: version_from.clone(),
                    version_to: patch.version_to.clone(),
                    name: file.name.clone(),
                    url: seed.url.join(&file.name)?,
                    size: file.size,
                    unpacked_size: file.unpacked_size,
                    diffs_size: file.diffs_size,
                });
            }
            let torrent_hash = patch
                .torrent
                .as_ref()
                .and_then(|torrent| torrent.hash.clone());
            let torrent_urls = patch
                .torrent
                .map(|torrent| {
                    torrent
                        .urls
                        .urls
                        .into_iter()
                        .map(|url| Url::parse(&url).map_err(Error::from))
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            patches.push(Patch {
                part: patch.part,
                version_from,
                version_to: patch.version_to,
                files,
                torrent_hash,
                torrent_urls,
            });
        }

        Ok(PatchManifest {
            region: product.region,
            channel: product.channel.to_owned(),
            host,
            game_id: metadata.game_id.clone(),
            platform: "windows".to_owned(),
            architecture: selected.architecture.clone(),
            client_type: selected.id.clone(),
            language,
            metadata_version: metadata.metadata_version,
            version: response
                .version_name
                .unwrap_or_else(|| "unknown".to_owned()),
            available_languages: metadata.supported_languages,
            publisher_ids: metadata.publisher_ids,
            parts: selected.parts.iter().map(|part| part.id.clone()).collect(),
            patches,
            web_seeds,
            assets,
        })
    }
}

fn metadata_from_wire(wire: &MetadataXml) -> GameMetadata {
    let predefined = &wire.predefined_section;
    GameMetadata {
        metadata_version: wire.version.clone(),
        game_id: predefined.app_id.clone(),
        name: predefined.name.clone(),
        filesystem_name: predefined.filesystem_name.clone(),
        environment: predefined.environment.clone(),
        default_language: normalize_language(&predefined.default_language),
        supported_languages: predefined
            .supported_languages
            .split(',')
            .map(normalize_language)
            .filter(|language| !language.is_empty())
            .collect(),
        default_client_type: predefined.client_types.default.clone(),
        client_types: predefined
            .client_types
            .types
            .iter()
            .map(|client| ClientType {
                id: client.id.clone(),
                architecture: client.architecture.clone(),
                parts: client
                    .client_parts
                    .parts
                    .iter()
                    .map(|part| ClientPart {
                        id: part.id.clone(),
                        app_type: part.app_type.clone(),
                        language_specific: part.language_specific,
                        integrity_checked: part.integrity_checked,
                    })
                    .collect(),
            })
            .collect(),
        executables: predefined
            .executables
            .executables
            .iter()
            .map(|executable| Executable {
                path: executable.path.clone(),
                architecture: executable.architecture.clone(),
                emulation: executable.emulation.clone(),
            })
            .collect(),
        publisher_ids: wire
            .publisher_ids
            .split(',')
            .map(str::trim)
            .filter(|publisher| !publisher.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

fn normalize_language(language: &str) -> String {
    language.trim().replace('_', "-").to_ascii_uppercase()
}

fn inferred_version_from(
    explicit: Option<String>,
    part: &str,
    version_to: &str,
    latest_version_by_part: &mut HashMap<String, String>,
) -> Option<String> {
    let version_from = explicit
        .or_else(|| latest_version_by_part.get(part).cloned())
        .unwrap_or_else(|| "0".to_owned());
    latest_version_by_part.insert(part.to_owned(), version_to.to_owned());
    Some(version_from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata_and_normalizes_languages() {
        let xml = r#"<protocol wgc_publisher_id="wargaming,steam"><version>42</version><predefined_section><app_id>WOT.EU.PRODUCTION</app_id><supported_languages>EN,PT_BR</supported_languages><default_language>EN</default_language><name>World of Tanks</name><fs_name>World_of_Tanks_EU</fs_name><environment_name>EU</environment_name><client_types default="sd"><client_type id="sd" arch="x64"><client_parts><client_part id="locale" lang="true" integrity="true"/><client_part id="client" integrity="true"/></client_parts></client_type></client_types><executables><executable arch="x64">win64\WorldOfTanks.exe</executable></executables></predefined_section></protocol>"#;
        let wire: MetadataXml = quick_xml::de::from_str(xml).unwrap();
        let metadata = metadata_from_wire(&wire);

        assert_eq!(metadata.supported_languages, ["EN", "PT-BR"]);
        assert_eq!(metadata.client_types[0].parts.len(), 2);
        assert!(metadata.client_types[0].parts[0].language_specific);
        assert_eq!(metadata.publisher_ids, ["wargaming", "steam"]);
    }

    #[test]
    fn infers_versions_for_lesta_install_chains() {
        let mut latest = HashMap::new();

        assert_eq!(
            inferred_version_from(None, "client", "1.0", &mut latest).as_deref(),
            Some("0")
        );
        assert_eq!(
            inferred_version_from(None, "client", "1.1", &mut latest).as_deref(),
            Some("1.0")
        );
        assert_eq!(
            inferred_version_from(Some("0.9".to_owned()), "client", "1.2", &mut latest).as_deref(),
            Some("0.9")
        );
    }
}
