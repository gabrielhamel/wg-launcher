//! Translate WGUS wire responses into catalog releases, without network or local state.

use super::error::{Result, WgusError as Error};
use super::wire::{self, MetadataXml, PatchesXml};
use crate::catalog::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, Product,
    ResolvedRelease, WebSeed,
};
use std::collections::{BTreeMap, HashMap};
use url::Url;

pub(super) fn release_from_wire(
    product: &Product,
    host: String,
    metadata: GameMetadata,
    selected: &ClientType,
    language: String,
    response: PatchesXml,
    component_versions: &BTreeMap<String, String>,
) -> Result<ResolvedRelease> {
    let chain = match response.patches_chain {
        Some(chain) => chain,
        None if selected
            .parts
            .iter()
            .all(|part| component_versions.contains_key(&part.id)) =>
        {
            wire::PatchesChainXml {
                patches: Vec::new(),
            }
        }
        None => return Err(Error::MissingPatchChain(product.game_id.to_owned())),
    };
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
    let seed = web_seeds.first();

    let mut patches = Vec::with_capacity(chain.patches.len());
    let mut assets = Vec::new();
    let mut latest_version_by_part = component_versions
        .iter()
        .map(|(part, version)| (part.clone(), version.clone()))
        .collect::<HashMap<_, _>>();
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
                url: seed
                    .ok_or_else(|| Error::MissingWebSeed(product.game_id.clone()))?
                    .url
                    .join(&file.name)?,
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

    let version = response
        .version_name
        .filter(|version| !version.trim().is_empty())
        .or_else(|| patches.first().map(|patch| patch.version_to.clone()))
        .unwrap_or_else(|| "unknown".to_owned());

    Ok(ResolvedRelease {
        region: product.region.clone(),
        product: product.clone(),
        channel: product.channel.to_owned(),
        host,
        game_id: metadata.game_id.clone(),
        platform: "windows".to_owned(),
        architecture: selected.architecture.clone(),
        client_type: selected.id.clone(),
        language,
        metadata_version: metadata.metadata_version,
        version,
        available_languages: metadata.supported_languages,
        publisher_ids: metadata.publisher_ids,
        parts: selected.parts.iter().map(|part| part.id.clone()).collect(),
        component_versions: selected
            .parts
            .iter()
            .map(|part| {
                latest_version_by_part
                    .get(&part.id)
                    .cloned()
                    .map(|version| (part.id.clone(), version))
                    .ok_or_else(|| crate::catalog::ReleaseError::MissingFullPatch {
                        part: part.id.clone(),
                    })
            })
            .collect::<std::result::Result<_, crate::catalog::ReleaseError>>()?,
        patches,
        web_seeds,
        assets,
    })
}

pub(super) fn metadata_from_wire(wire: &MetadataXml) -> GameMetadata {
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

pub(super) fn normalize_language(language: &str) -> String {
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
