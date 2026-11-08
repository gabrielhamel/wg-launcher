use super::Patch;
use super::error::{ReleaseError as Error, Result};
use crate::catalog::ComponentVersions;
use std::collections::{HashMap, HashSet};

impl<'a> super::ComponentRelease<'a> {
    pub(super) fn ordered_patches(
        &self,
        base_versions: &ComponentVersions,
    ) -> Result<Vec<&'a Patch>> {
        let mut ordered = Vec::new();
        let component_id = self.id;
        let target = self.require_target()?;
        let mut version = base_versions.version_or_zero(component_id);
        let by_from = self
            .patches()
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
                    part: component_id.to_owned(),
                    version: version.to_owned(),
                });
            }
            let next = by_from
                .get(version)
                .copied()
                .ok_or_else(|| Error::MissingUpdatePatch {
                    part: component_id.to_owned(),
                    version: version.to_owned(),
                    target: target.to_owned(),
                })?;
            ordered.push(next);
            version = &next.version_to;
        }
        Ok(ordered)
    }
}
