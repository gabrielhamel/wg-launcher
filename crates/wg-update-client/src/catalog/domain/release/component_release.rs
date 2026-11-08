//! Associate a selected component with its target and published patch steps.

use super::{ComponentVersions, Patch, ReleaseError, TransportVolume, error::Result};

/// Borrowed component data; missing targets remain visible to each consumer's rules.
/// This view does not change the release's saved representation.
pub(crate) struct ComponentRelease<'a> {
    pub(super) id: &'a str,
    pub(super) target: Option<&'a str>,
    pub(super) published_patches: &'a [Patch],
    pub(super) transport_volumes: &'a [TransportVolume],
}

impl<'a> ComponentRelease<'a> {
    pub(crate) fn id(&self) -> &'a str {
        self.id
    }

    pub(crate) fn target_version(&self) -> Option<&'a str> {
        self.target
    }

    pub(super) fn patches(&self) -> impl Iterator<Item = &'a Patch> {
        self.published_patches
            .iter()
            .filter(|patch| patch.component_id == self.id)
    }

    pub(super) fn require_target(&self) -> Result<&'a str> {
        self.target
            .ok_or_else(|| ReleaseError::MissingComponentTarget {
                part: self.id.to_owned(),
            })
    }

    /// Plan this component from known versions, resolving only the chosen steps' volumes.
    pub(super) fn planned_patches(
        &self,
        base_versions: &ComponentVersions,
    ) -> Result<Vec<super::package_plan::PlannedPatch>> {
        let patches = super::patch_chain::ordered_patches(self, base_versions)?;
        super::package_plan::resolve_patch_volumes(patches, self.transport_volumes)
    }

    pub(crate) fn reached_by(&self, versions: &ComponentVersions) -> bool {
        self.target == Some(versions.version_or_zero(self.id))
    }
}
