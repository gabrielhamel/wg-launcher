use crate::{Asset, Error, Patch, PatchManifest, Result};
use std::collections::{HashMap, HashSet};

/// Clones the transport assets required by the ordered installation chain.
///
/// Orders full-install and subsequent patches for each selected component, then
/// resolves their volumes by name. Does not read disk or contact the catalog.
///
/// # Errors
///
/// Returns an error for a missing full-install patch, cyclic patch chain, or a patch
/// volume absent from the manifest assets.
pub fn installation_assets(manifest: &PatchManifest) -> Result<Vec<Asset>> {
    let patches = ordered_patches(manifest)?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    Ok(assets_for_patches(manifest, &patches)?
        .into_iter()
        .flatten()
        .collect())
}

pub(super) fn assets_for_patches(
    manifest: &PatchManifest,
    patches: &[Patch],
) -> Result<Vec<Vec<Asset>>> {
    let by_name = manifest
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset))
        .collect::<HashMap<_, _>>();
    patches
        .iter()
        .map(|patch| {
            patch
                .files
                .iter()
                .map(|file| {
                    by_name
                        .get(file.name.as_str())
                        .copied()
                        .cloned()
                        .ok_or_else(|| Error::MissingPatchAsset(file.name.clone()))
                })
                .collect()
        })
        .collect()
}

pub(super) fn ordered_patches(manifest: &PatchManifest) -> Result<Vec<&Patch>> {
    let mut ordered = Vec::new();
    for part in &manifest.parts {
        let part_patches = manifest
            .patches
            .iter()
            .filter(|patch| &patch.part == part)
            .collect::<Vec<_>>();
        let full = part_patches
            .iter()
            .copied()
            .find(|patch| patch.version_from.as_deref() == Some("0"))
            .ok_or_else(|| Error::MissingFullPatch { part: part.clone() })?;
        ordered.push(full);
        let by_from = part_patches
            .into_iter()
            .filter_map(|patch| patch.version_from.as_deref().map(|from| (from, patch)))
            .collect::<HashMap<_, _>>();
        let mut version = full.version_to.as_str();
        let mut seen = HashSet::new();
        while let Some(next) = by_from.get(version).copied() {
            if !seen.insert(version.to_owned()) {
                return Err(Error::BrokenPatchChain {
                    part: part.clone(),
                    version: version.to_owned(),
                });
            }
            ordered.push(next);
            version = &next.version_to;
        }
    }
    Ok(ordered)
}
