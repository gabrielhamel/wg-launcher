//! Resolve HTTP transport volumes for normalized publisher patch chains.

use super::error::{Result, WgusError as Error};
use super::patch_chain_normalization::NormalizedPatchChain;
use super::wire::{PatchesChainXml, WebSeedsXml};
use crate::catalog::ComponentVersions;
use crate::catalog::{ClientVariant, Patch, Product, TransportVolume, WebSeed};
use url::Url;

pub(super) struct ConvertedPatchChain {
    pub(super) patches: Vec<Patch>,
    pub(super) volumes: Vec<TransportVolume>,
    pub(super) web_seeds: Vec<WebSeed>,
    pub(super) component_versions: ComponentVersions,
}

pub(super) fn patch_chain_from_wire(
    product: &Product,
    selected: &ClientVariant,
    chain: Option<PatchesChainXml>,
    seeds: WebSeedsXml,
    component_versions: &ComponentVersions,
) -> Result<ConvertedPatchChain> {
    let web_seeds = parse_web_seeds(seeds)?;
    let normalized = NormalizedPatchChain::from_wire(product, selected, chain, component_versions)?;
    let mut volumes = Vec::new();
    for patch in &normalized.patches {
        volumes.extend(resolve_transport_volumes(
            product,
            patch,
            web_seeds.first(),
        )?);
    }
    Ok(ConvertedPatchChain {
        patches: normalized.patches,
        volumes,
        web_seeds,
        component_versions: normalized.component_versions,
    })
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
    patch: &Patch,
    seed: Option<&WebSeed>,
) -> Result<Vec<TransportVolume>> {
    patch
        .volumes
        .iter()
        .map(|volume| {
            Ok(TransportVolume {
                component_id: patch.component_id.clone(),
                source: patch.source.clone(),
                version_to: patch.version_to.clone(),
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
