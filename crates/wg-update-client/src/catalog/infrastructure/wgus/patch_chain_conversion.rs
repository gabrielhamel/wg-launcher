//! Resolve HTTP transport volumes for normalized publisher patch chains.

use super::error::{Result, WgusError as Error};
use super::patch_chain_normalization::NormalizedPatchChain;
use super::wire::{PatchXml, PatchesChainXml, WebSeedsXml};
use crate::catalog::ComponentVersions;
use crate::catalog::{ClientVariant, Patch, Product, ReleaseMode, TransportVolume, WebSeed};
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
    chains: Vec<PatchesChainXml>,
    seeds: WebSeedsXml,
    component_versions: &ComponentVersions,
    mode: ReleaseMode,
) -> Result<ConvertedPatchChain> {
    let web_seeds = parse_web_seeds(seeds)?;
    let patches = selected_patches(chains, mode);
    let normalized =
        NormalizedPatchChain::from_wire(product, selected, patches, component_versions)?;
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

fn selected_patches(mut chains: Vec<PatchesChainXml>, mode: ReleaseMode) -> Option<Vec<PatchXml>> {
    // Prefer the explicit install chain, with an untyped fallback for compatible services.
    let install = chains
        .iter()
        .position(|chain| chain.chain_type.as_deref() == Some("install"))
        .or_else(|| chains.iter().position(|chain| chain.chain_type.is_none()));
    let mut patches = install.map(|index| chains.remove(index).patches);
    if mode == ReleaseMode::IncludePreload {
        for chain in chains
            .into_iter()
            .filter(|chain| chain.chain_type.as_deref() == Some("preload"))
        {
            patches.get_or_insert_with(Vec::new).extend(chain.patches);
        }
    }
    patches
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
