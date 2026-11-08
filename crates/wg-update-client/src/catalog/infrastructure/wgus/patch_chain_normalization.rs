//! Normalize publisher transitions and component targets before resolving package URLs.

use super::error::{Result, WgusError as Error};
use super::wire::{self, PatchesChainXml};
use crate::catalog::{ClientType, ComponentVersions, Patch, PatchSource, PatchVolume, Product};
use std::collections::HashMap;
use url::Url;

/// Publisher patches and targets, independent of HTTP download sources.
pub(super) struct NormalizedPatchChain {
    pub(super) patches: Vec<Patch>,
    pub(super) component_versions: ComponentVersions,
}

impl NormalizedPatchChain {
    pub(super) fn from_wire(
        product: &Product,
        selected: &ClientType,
        chain: Option<PatchesChainXml>,
        component_versions: &ComponentVersions,
    ) -> Result<Self> {
        let chain = require_patch_chain(product, selected, chain, component_versions)?;
        let mut normalization = PatchChainNormalization::from_known_versions(component_versions);
        let mut patches = Vec::with_capacity(chain.patches.len());
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
                .collect();
            let (torrent_hash, torrent_urls) = parse_torrent(patch.torrent)?;
            patches.push(Patch {
                component_id: patch.part,
                source,
                version_to: patch.version_to,
                volumes,
                torrent_hash,
                torrent_urls,
            });
        }
        Ok(Self {
            patches,
            component_versions: normalization.component_targets(selected)?,
        })
    }
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
