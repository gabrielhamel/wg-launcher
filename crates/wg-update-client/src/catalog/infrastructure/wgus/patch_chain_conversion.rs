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
    let chain = require_patch_chain(product, selected, chain, component_versions)?;
    let web_seeds = parse_web_seeds(seeds)?;
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
        resolved_volumes.extend(resolve_transport_volumes(
            product,
            &patch.part,
            &source,
            &patch.version_to,
            &volumes,
            seed,
        )?);
        let (torrent_hash, torrent_urls) = parse_torrent(patch.torrent)?;
        patches.push(Patch {
            part: patch.part,
            source,
            version_to: patch.version_to,
            volumes,
            torrent_hash,
            torrent_urls,
        });
    }

    let component_versions = normalization.component_targets(selected)?;
    Ok(ConvertedPatchChain {
        patches,
        volumes: resolved_volumes,
        web_seeds,
        component_versions,
    })
}

fn require_patch_chain(
    product: &Product,
    selected: &ClientType,
    chain: Option<PatchesChainXml>,
    component_versions: &ComponentVersions,
) -> Result<PatchesChainXml> {
    match chain {
        Some(chain) => Ok(chain),
        None if selected
            .parts
            .iter()
            .all(|part| component_versions.get(&part.id).is_some()) =>
        {
            Ok(wire::PatchesChainXml {
                patches: Vec::new(),
            })
        }
        None => Err(Error::MissingPatchChain(
            product.application_id().to_owned(),
        )),
    }
}

fn parse_web_seeds(seeds: WebSeedsXml) -> Result<Vec<WebSeed>> {
    seeds
        .urls
        .into_iter()
        .map(|seed| {
            Ok(WebSeed {
                url: Url::parse(seed.url.trim())?,
                name: seed.name,
                threads: seed.threads,
            })
        })
        .collect()
}

fn resolve_transport_volumes(
    product: &Product,
    part: &str,
    source: &PatchSource,
    version_to: &str,
    volumes: &[PatchVolume],
    seed: Option<&WebSeed>,
) -> Result<Vec<TransportVolume>> {
    volumes
        .iter()
        .map(|volume| {
            Ok(TransportVolume {
                part: part.to_owned(),
                source: source.clone(),
                version_to: version_to.to_owned(),
                name: volume.name.clone(),
                url: seed
                    .ok_or_else(|| Error::MissingWebSeed(product.application_id().to_owned()))?
                    .url
                    .join(&volume.name)?,
                size: volume.size,
                unpacked_size: volume.unpacked_size,
                diffs_size: volume.diffs_size,
            })
        })
        .collect()
}

fn parse_torrent(torrent: Option<wire::TorrentXml>) -> Result<(Option<String>, Vec<Url>)> {
    let torrent_hash = torrent.as_ref().and_then(|torrent| torrent.hash.clone());
    let torrent_urls = torrent
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
    Ok((torrent_hash, torrent_urls))
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

    fn component_targets(&self, selected: &ClientType) -> Result<ComponentVersions> {
        let targets = selected
            .parts
            .iter()
            .map(|part| {
                self.reached_versions
                    .get(&part.id)
                    .cloned()
                    .map(|version| (part.id.clone(), version))
                    .ok_or_else(|| crate::catalog::ReleaseError::MissingComponentTarget {
                        part: part.id.clone(),
                    })
            })
            .collect::<std::result::Result<_, crate::catalog::ReleaseError>>()?;
        Ok(targets)
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
