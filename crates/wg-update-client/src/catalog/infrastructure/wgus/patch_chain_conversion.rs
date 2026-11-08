//! Normalize publisher patch chains and resolve their HTTP transport volumes.

use super::error::{Result, WgusError as Error};
use super::wire::{self, PatchesChainXml, WebSeedsXml};
use crate::catalog::ComponentVersions;
use crate::catalog::{
    ClientType, Patch, PatchSource, PatchVolume, Product, TransportVolume, WebSeed,
};
use std::collections::HashMap;
use url::Url;

pub(super) struct ConvertedPatchChain {
    pub(super) patches: Vec<Patch>,
    pub(super) volumes: Vec<TransportVolume>,
    pub(super) web_seeds: Vec<WebSeed>,
    pub(super) component_versions: ComponentVersions,
}

pub(super) fn patch_chain_from_wire(
    product: &Product,
    selected: &ClientType,
    chain: Option<PatchesChainXml>,
    seeds: WebSeedsXml,
    component_versions: &ComponentVersions,
) -> Result<ConvertedPatchChain> {
    let chain = match chain {
        Some(chain) => chain,
        None if selected
            .parts
            .iter()
            .all(|part| component_versions.get(&part.id).is_some()) =>
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
    let mut resolved_volumes = Vec::new();
    let mut normalization = PatchChainNormalization::from_known_versions(component_versions);
    for patch in chain.patches {
        let source = normalization.resolve_source_and_advance(
            patch.version_from,
            &patch.part,
            &patch.version_to,
        );
        let volumes = patch
            .files
            .files
            .into_iter()
            .map(|volume| PatchVolume {
                name: volume.name,
                size: volume.size,
                unpacked_size: volume.unpacked_size,
                diffs_size: volume.diffs_size,
            })
            .collect::<Vec<_>>();
        for volume in &volumes {
            resolved_volumes.push(TransportVolume {
                part: patch.part.clone(),
                source: source.clone(),
                version_to: patch.version_to.clone(),
                name: volume.name.clone(),
                url: seed
                    .ok_or_else(|| Error::MissingWebSeed(product.game_id.clone()))?
                    .url
                    .join(&volume.name)?,
                size: volume.size,
                unpacked_size: volume.unpacked_size,
                diffs_size: volume.diffs_size,
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
            volumes,
            torrent_hash,
            torrent_urls,
        });
    }

    let component_versions = selected
        .parts
        .iter()
        .map(|part| {
            normalization
                .reached_versions
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
        volumes: resolved_volumes,
        web_seeds,
        component_versions,
    })
}

/// Publisher-order component progress used only while converting a WGUS chain.
struct PatchChainNormalization {
    reached_versions: HashMap<String, String>,
}

impl PatchChainNormalization {
    fn from_known_versions(known: &ComponentVersions) -> Self {
        Self {
            reached_versions: known
                .iter()
                .map(|(part, version)| (part.clone(), version.clone()))
                .collect(),
        }
    }

    fn resolve_source_and_advance(
        &mut self,
        explicit: Option<String>,
        part: &str,
        target: &str,
    ) -> PatchSource {
        let source = explicit
            .or_else(|| self.reached_versions.get(part).cloned())
            .unwrap_or_else(|| "0".to_owned());
        self.reached_versions
            .insert(part.to_owned(), target.to_owned());
        source.into()
    }
}
