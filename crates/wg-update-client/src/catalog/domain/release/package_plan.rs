//! Resolve the transport volumes for a selected patch chain.

use super::error::{ReleaseError as Error, Result};
use super::{Patch, PatchVolume, ResolvedRelease, TransportVolume};
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
        .flat_map(|planned| planned.volumes.into_iter().map(|volume| volume.transport))
        .collect())
}

/// An ordered patch and the resolved transport volumes needed to apply it.
#[derive(Debug, Clone)]
pub(crate) struct PlannedPatch {
    pub(crate) patch: Patch,
    pub(crate) volumes: Vec<PlannedVolume>,
}

/// A selected volume keeps its declared archive contract beside its HTTP source.
#[derive(Debug, Clone)]
pub(crate) struct PlannedVolume {
    pub(crate) declaration: PatchVolume,
    pub(crate) transport: TransportVolume,
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

/// Look up transport volumes only for selected patches; unused alternatives stay unchecked.
pub(super) fn resolve_patch_volumes(
    patches: Vec<&Patch>,
    transport_volumes: &[TransportVolume],
) -> Result<Vec<PlannedPatch>> {
    let volumes = transport_volumes
        .iter()
        .map(|volume| (volume.name.as_str(), volume))
        .collect::<HashMap<_, _>>();
    patches
        .into_iter()
        .map(|patch| {
            let volumes = patch
                .volumes
                .iter()
                .map(|volume| {
                    volumes
                        .get(volume.name.as_str())
                        .copied()
                        .map(|transport| PlannedVolume {
                            declaration: volume.clone(),
                            transport: transport.clone(),
                        })
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
