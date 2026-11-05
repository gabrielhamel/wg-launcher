use super::InstalledBuild;
use crate::catalog::{
    Asset, Patch, ReleaseError, ResolvedRelease, installation_assets, ordered_patches,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A resolved release pinned for local work, with completed component checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PinnedManifest {
    #[serde(flatten)]
    pub(crate) release: ResolvedRelease,
    pub(crate) base_versions: BTreeMap<String, String>,
}

impl From<ResolvedRelease> for PinnedManifest {
    fn from(release: ResolvedRelease) -> Self {
        Self {
            release,
            base_versions: BTreeMap::new(),
        }
    }
}

impl PinnedManifest {
    pub(crate) fn installation_assets(&self) -> Result<Vec<Asset>, ReleaseError> {
        installation_assets(&self.release, &self.base_versions)
    }

    pub(crate) fn ordered_patches(&self) -> Result<Vec<&Patch>, ReleaseError> {
        ordered_patches(&self.release, &self.base_versions)
    }

    pub(crate) fn include_installed_versions(&mut self, installed: &InstalledBuild) {
        for (part, version) in &installed.component_versions {
            self.base_versions
                .entry(part.clone())
                .or_insert_with(|| version.clone());
        }
    }
}
