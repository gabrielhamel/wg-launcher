//! Resolve the transport packages for a selected patch chain.

use super::error::{ReleaseError as Error, Result};
use super::{Asset, Patch, ResolvedRelease};
use std::collections::BTreeMap;

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
    Ok(planned_patches(release, base_versions)?
        .into_iter()
        .flat_map(|planned| planned.packages)
        .collect())
}

/// An ordered patch and the resolved transport packages needed to apply it.
#[derive(Debug, Clone)]
pub(crate) struct PlannedPatch {
    pub(crate) patch: Patch,
    pub(crate) packages: Vec<Asset>,
}

pub(crate) fn planned_patches(
    release: &ResolvedRelease,
    base_versions: &BTreeMap<String, String>,
) -> Result<Vec<PlannedPatch>> {
    let mut planned = Vec::new();
    for component in release.components() {
        planned.extend(component?.planned_patches(base_versions)?);
    }
    Ok(planned)
}

impl super::resolved_release::ComponentRelease<'_> {
    /// Validate the packages of the selected steps, leaving unused publisher
    /// alternatives alone. Planning does not change the persisted release.
    fn planned_patches(
        &self,
        base_versions: &BTreeMap<String, String>,
    ) -> Result<Vec<PlannedPatch>> {
        self.ordered_patches(base_versions)?
            .into_iter()
            .map(|patch| {
                let packages = patch
                    .files
                    .iter()
                    .map(|file| {
                        self.packages
                            .get(file.name.as_str())
                            .copied()
                            .cloned()
                            .ok_or_else(|| Error::MissingPatchAsset(file.name.clone()))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(PlannedPatch {
                    patch: patch.clone(),
                    packages,
                })
            })
            .collect()
    }
}
