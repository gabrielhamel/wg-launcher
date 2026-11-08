//! Associate a selected component with its target and published patch steps.

use super::{ComponentVersions, Patch, ReleaseError, error::Result};

/// Borrowed component data; missing targets remain visible to each consumer's rules.
/// This view does not change the release's saved representation.
pub(crate) struct ComponentRelease<'a> {
    pub(super) id: &'a str,
    pub(super) target: Option<&'a str>,
    pub(super) published_patches: &'a [Patch],
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

    /// Select this component's ordered steps from known versions.
    pub(super) fn ordered_patches(
        &self,
        base_versions: &ComponentVersions,
    ) -> Result<Vec<&'a Patch>> {
        super::patch_chain::ordered_patches(self, base_versions)
    }

    pub(crate) fn reached_by(&self, versions: &ComponentVersions) -> bool {
        self.target == Some(versions.version_or_zero(self.id))
    }
}
