//! Normalize publisher patch chains and resolve their HTTP transport volumes.

use super::error::{Result, WgusError as Error};
use super::wire::{self, PatchesChainXml, WebSeedsXml};
use crate::catalog::{
    ClientType, Patch, PatchFile, PatchSource, Product, TransportVolume, WebSeed,
};
use std::collections::{BTreeMap, HashMap};
use url::Url;

pub(super) struct ConvertedPatchChain {
    pub(super) patches: Vec<Patch>,
    pub(super) assets: Vec<TransportVolume>,
    pub(super) web_seeds: Vec<WebSeed>,
    pub(super) component_versions: BTreeMap<String, String>,
}

pub(super) fn patch_chain_from_wire(
    product: &Product,
    selected: &ClientType,
    chain: Option<PatchesChainXml>,
    seeds: WebSeedsXml,
    component_versions: &BTreeMap<String, String>,
) -> Result<ConvertedPatchChain> {
    let chain = match chain {
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
    let web_seeds = seeds
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
        let source = normalize_patch_source_and_record_target(
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
            assets.push(TransportVolume {
                part: patch.part.clone(),
                source: source.clone(),
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
            source,
            version_to: patch.version_to,
            files,
            torrent_hash,
            torrent_urls,
        });
    }

    let component_versions = selected
        .parts
        .iter()
        .map(|part| {
            latest_version_by_part
                .get(&part.id)
                .cloned()
                .map(|version| (part.id.clone(), version))
                .ok_or_else(|| crate::catalog::ReleaseError::MissingComponentTarget {
                    part: part.id.clone(),
                })
        })
        .collect::<std::result::Result<_, crate::catalog::ReleaseError>>()?;
    Ok(ConvertedPatchChain {
        patches,
        assets,
        web_seeds,
        component_versions,
    })
}

fn normalize_patch_source_and_record_target(
    explicit: Option<String>,
    part: &str,
    version_to: &str,
    latest_version_by_part: &mut HashMap<String, String>,
) -> PatchSource {
    let version_from = explicit
        .or_else(|| latest_version_by_part.get(part).cloned())
        .unwrap_or_else(|| "0".to_owned());
    latest_version_by_part.insert(part.to_owned(), version_to.to_owned());
    version_from.into()
}
