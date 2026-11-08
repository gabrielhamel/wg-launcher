use super::error::{ReleaseError as Error, Result};
use super::{Patch, ResolvedRelease};
use std::collections::{BTreeMap, HashMap, HashSet};

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
            .filter_map(|patch| patch.source.version().map(|from| (from, patch)))
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
