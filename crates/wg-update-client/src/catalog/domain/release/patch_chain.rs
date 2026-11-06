use super::error::{ReleaseError as Error, Result};
use super::{Asset, Patch, ResolvedRelease};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Clones the transport assets required by the ordered installation chain.
///
/// Starts at each supplied component version (or `0` for a fresh install), then
/// resolves their volumes by name. Does not read disk or contact the catalog.
///
/// # Errors
///
/// Returns an error for an unreachable component target, cyclic patch chain, or a patch
/// volume absent from the release assets.
pub fn installation_assets(
    release: &ResolvedRelease,
    base_versions: &BTreeMap<String, String>,
) -> Result<Vec<Asset>> {
    let patches = ordered_patches(release, base_versions)?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    Ok(assets_for_patches(release, &patches)?
        .into_iter()
        .flatten()
        .collect())
}

pub(crate) fn assets_for_patches(
    release: &ResolvedRelease,
    patches: &[Patch],
) -> Result<Vec<Vec<Asset>>> {
    let by_name = release
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

pub(crate) fn ordered_patches<'a>(
    release: &'a ResolvedRelease,
    base_versions: &BTreeMap<String, String>,
) -> Result<Vec<&'a Patch>> {
    let mut ordered = Vec::new();
    for part in &release.parts {
        let target = release
            .component_versions
            .get(part)
            .ok_or_else(|| Error::MissingFullPatch { part: part.clone() })?;
        let mut version = base_versions.get(part).map(String::as_str).unwrap_or("0");
        let by_from = release
            .patches
            .iter()
            .filter(|patch| &patch.part == part)
            .filter_map(|patch| patch.version_from.as_deref().map(|from| (from, patch)))
            .collect::<HashMap<_, _>>();
        // A publisher may replace an obsolete delta chain with a full package.
        if version != target && !by_from.contains_key(version) && by_from.contains_key("0") {
            version = "0";
        }
        let mut seen = HashSet::new();
        while version != target {
            if !seen.insert(version) {
                return Err(Error::BrokenPatchChain {
                    part: part.clone(),
                    version: version.to_owned(),
                });
            }
            let next = by_from
                .get(version)
                .copied()
                .ok_or_else(|| Error::MissingUpdatePatch {
                    part: part.clone(),
                    version: version.to_owned(),
                    target: target.clone(),
                })?;
            ordered.push(next);
            version = &next.version_to;
        }
    }
    Ok(ordered)
}
