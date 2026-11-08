//! Resolve the transport volumes for a selected patch chain.

use super::error::{ReleaseError as Error, Result};
use super::{Patch, ResolvedRelease, TransportVolume};
use crate::catalog::ComponentVersions;
use std::collections::HashMap;

/// Clones the transport volumes required by the ordered installation chain.
///
/// Starts at each supplied component version (or `0` for a fresh install), then
/// resolves their volumes by name. Does not read disk or contact the catalog.
///
/// # Errors
///
/// Returns an error for an unreachable component target, cyclic patch chain, or a patch
/// volume absent from the release volumes.
pub fn required_volumes(
    release: &ResolvedRelease,
    base_versions: &ComponentVersions,
) -> Result<Vec<TransportVolume>> {
    Ok(planned_patches(release, base_versions)?
        .into_iter()
        .flat_map(|planned| planned.volumes)
        .collect())
}

/// An ordered patch and the resolved transport volumes needed to apply it.
#[derive(Debug, Clone)]
pub(crate) struct PlannedPatch {
    pub(crate) patch: Patch,
    pub(crate) volumes: Vec<TransportVolume>,
}

pub(crate) fn planned_patches(
    release: &ResolvedRelease,
    base_versions: &ComponentVersions,
) -> Result<Vec<PlannedPatch>> {
    let mut planned = Vec::new();
    for component in release.components() {
        planned.extend(component.planned_patches(base_versions)?);
    }
    Ok(planned)
}

impl super::ComponentRelease<'_> {
    /// Validate the volumes of the selected steps, leaving unused publisher
    /// alternatives alone. Planning does not change the persisted release.
    fn planned_patches(&self, base_versions: &ComponentVersions) -> Result<Vec<PlannedPatch>> {
        let volumes = self
            .transport_volumes
            .iter()
            .map(|volume| (volume.name.as_str(), volume))
            .collect::<HashMap<_, _>>();
        self.ordered_patches(base_versions)?
            .into_iter()
            .map(|patch| {
                let volumes = patch
                    .volumes
                    .iter()
                    .map(|volume| {
                        volumes
                            .get(volume.name.as_str())
                            .copied()
                            .cloned()
                            .ok_or_else(|| Error::MissingPatchAsset(volume.name.clone()))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(PlannedPatch {
                    patch: patch.clone(),
                    volumes,
                })
            })
            .collect()
    }
}
