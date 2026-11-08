use super::error::{InstallationError as Error, Result as InstallationResult};
use super::{InstalledBuild, component_progress::ComponentProgress};
use crate::catalog::{
    Asset, Patch, PlannedPatch, ReleaseError, ResolvedRelease, installation_assets,
    ordered_patches, planned_patches,
};
use serde::{Deserialize, Serialize};

/// A resolved release pinned for local work, with completed component checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PinnedManifest {
    #[serde(flatten)]
    pub(crate) release: ResolvedRelease,
    #[serde(rename = "base_versions")]
    progress: ComponentProgress,
}

impl From<ResolvedRelease> for PinnedManifest {
    fn from(release: ResolvedRelease) -> Self {
        Self {
            release,
            progress: ComponentProgress::default(),
        }
    }
}

impl PinnedManifest {
    pub(crate) fn installation_assets(&self) -> std::result::Result<Vec<Asset>, ReleaseError> {
        installation_assets(&self.release, self.progress.versions())
    }

    pub(crate) fn ordered_patches(&self) -> std::result::Result<Vec<&Patch>, ReleaseError> {
        ordered_patches(&self.release, self.progress.versions())
    }

    pub(crate) fn planned_patches(&self) -> std::result::Result<Vec<PlannedPatch>, ReleaseError> {
        planned_patches(&self.release, self.progress.versions())
    }

    /// Seed starting versions without overwriting completed patch checkpoints.
    pub(crate) fn seed_missing_component_versions(&mut self, installed: &InstalledBuild) {
        self.progress.seed_missing(&installed.component_versions);
    }

    pub(crate) fn resume_from(mut self, pending: Self) -> InstallationResult<Self> {
        if self.release != pending.release {
            return Err(Error::CachedSelectionMismatch {
                cached: format!(
                    "{} {}",
                    pending.release.identity.game_id, pending.release.identity.version
                ),
                requested: format!(
                    "{} {}",
                    self.release.identity.game_id, self.release.identity.version
                ),
            });
        }
        // Persisted checkpoints are authoritative when retrying an original plan.
        self.progress.retain_saved(pending.progress);
        Ok(self)
    }

    pub(crate) fn checkpoint(&mut self, patch: &Patch) {
        self.progress
            .record_completion(&patch.part, &patch.version_to);
    }
}
