//! Associate a selected component with its target and published patch steps.

use super::{ComponentVersions, Patch, ReleaseError, error::Result};

/// Borrowed selected component with a validated, required target version.
/// This view does not change the release's saved representation.
pub(crate) struct ComponentRelease<'a> {
    pub(super) id: &'a str,
    target: &'a str,
    pub(super) published_patches: &'a [Patch],
}

impl<'a> ComponentRelease<'a> {
    pub(super) fn new(
        id: &'a str,
        target: Option<&'a str>,
        published_patches: &'a [Patch],
    ) -> Result<Self> {
        let target = target.ok_or_else(|| ReleaseError::MissingComponentTarget {
            part: id.to_owned(),
        })?;
        Ok(Self {
            id,
            target,
            published_patches,
        })
    }

    pub(crate) fn id(&self) -> &'a str {
        self.id
    }

    pub(crate) fn target_version(&self) -> &'a str {
        self.target
    }

    pub(super) fn patches(&self) -> impl Iterator<Item = &'a Patch> {
        self.published_patches
            .iter()
            .filter(|patch| patch.component_id == self.id)
    }

    /// Select this component's ordered steps from known versions.
    pub(super) fn ordered_patches(
        &self,
        base_versions: &ComponentVersions,
    ) -> Result<Vec<&'a Patch>> {
        super::patch_chain::ordered_patches(self, base_versions)
    }

    pub(crate) fn reached_by(&self, versions: &ComponentVersions) -> bool {
        self.target == versions.version_or_zero(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_target_before_planning_or_checking_completion() {
        assert!(matches!(
            ComponentRelease::new("client", None, &[]),
            Err(ReleaseError::MissingComponentTarget { part }) if part == "client"
        ));
        let component = ComponentRelease::new("client", Some("1.0"), &[]).unwrap();
        let mut versions = ComponentVersions::default();
        assert!(!component.reached_by(&versions));
        versions.insert("client".into(), "1.0".into());
        assert!(component.reached_by(&versions));
        assert!(component.ordered_patches(&versions).unwrap().is_empty());
    }
}
