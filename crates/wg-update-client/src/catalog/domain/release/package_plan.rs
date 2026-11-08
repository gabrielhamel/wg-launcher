//! Resolve the transport volumes for a selected patch chain.

use super::error::Result;
use super::{Patch, ResolvedRelease, TransportVolume, volume_index::VolumeIndex};
use crate::catalog::ComponentVersions;

/// Clones the transport volumes required by the ordered installation chain.
///
/// Starts at each supplied component version (or `0` for a fresh install), then
/// binds their volume declarations to matching patch identities and download sources.
/// Does not read disk or contact the catalog.
///
/// # Errors
///
/// Returns an error for an unreachable component target, cyclic patch chain, or a patch
/// volume with a missing, ambiguous, or inconsistent download source.
pub fn required_volumes(
    release: &ResolvedRelease,
    base_versions: &ComponentVersions,
) -> Result<Vec<TransportVolume>> {
    Ok(planned_patches(release, base_versions)?
        .into_iter()
        .flat_map(PlannedPatch::into_volumes)
        .collect())
}

/// An ordered patch and the resolved transport volumes needed to apply it.
/// Private fields preserve the binding established by patch planning.
#[derive(Debug, Clone)]
pub(crate) struct PlannedPatch {
    patch: Patch,
    volumes: Vec<TransportVolume>,
}

impl PlannedPatch {
    pub(crate) fn patch(&self) -> &Patch {
        &self.patch
    }

    pub(crate) fn volumes(&self) -> &[TransportVolume] {
        &self.volumes
    }

    pub(crate) fn into_volumes(self) -> Vec<TransportVolume> {
        self.volumes
    }
}

pub(crate) fn planned_patches(
    release: &ResolvedRelease,
    base_versions: &ComponentVersions,
) -> Result<Vec<PlannedPatch>> {
    let volumes = VolumeIndex::new(release.contents.volumes());
    let mut planned = Vec::new();
    for component in release.components() {
        for patch in component?.ordered_patches(base_versions)? {
            planned.push(PlannedPatch {
                patch: patch.clone(),
                volumes: patch
                    .volumes
                    .iter()
                    .map(|declaration| volumes.bind(patch, declaration).cloned())
                    .collect::<Result<_>>()?,
            });
        }
    }
    Ok(planned)
}
