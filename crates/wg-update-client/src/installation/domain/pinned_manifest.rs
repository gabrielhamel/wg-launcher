use super::error::{InstallationError as Error, Result as InstallationResult};
use super::{InstalledBuild, component_progress::ComponentProgress};
use crate::catalog::ComponentVersions;
use crate::catalog::{
    Patch, PlannedPatch, ReleaseError, ResolvedRelease, TransportVolume, planned_patches,
    required_volumes,
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
    /// Produce installed identity only when every selected component reached its target.
    pub(crate) fn complete_installation(&self) -> InstallationResult<InstalledBuild> {
        for component in self.release.components() {
            if !component.reached_by(self.reached_versions()) {
                return Err(Error::IncompleteInstallation {
                    component: component.id().to_owned(),
                    reached: self
                        .reached_versions()
                        .version_or_zero(component.id())
                        .to_owned(),
                    target: component.target_version().unwrap_or("unknown").to_owned(),
                });
            }
        }
        Ok(InstalledBuild {
            identity: self.release.identity.clone(),
            product: self.release.product.clone(),
            component_versions: self.release.component_versions.clone(),
        })
    }

    /// Component versions reached locally, including saved patch completions.
    pub(crate) fn reached_versions(&self) -> &ComponentVersions {
        self.progress.versions()
    }

    pub(crate) fn required_volumes(
        &self,
    ) -> std::result::Result<Vec<TransportVolume>, ReleaseError> {
        required_volumes(&self.release, self.progress.versions())
    }

    pub(crate) fn planned_patches(&self) -> std::result::Result<Vec<PlannedPatch>, ReleaseError> {
        planned_patches(&self.release, self.progress.versions())
    }

    /// Seed starting versions without overwriting completed patch checkpoints.
    pub(crate) fn seed_missing_component_versions(&mut self, installed: &InstalledBuild) {
        self.progress.seed_missing(&installed.component_versions);
    }

    pub(crate) fn resume_from(mut self, pending: Self) -> InstallationResult<Self> {
        let differences =
            super::release_compatibility::compare_releases(&self.release, &pending.release);
        if !differences.is_empty() {
            return Err(Error::PinnedReleaseMismatch { differences });
        }
        // Persisted checkpoints are authoritative when retrying an original plan.
        self.progress.restore_saved_checkpoints(pending.progress);
        Ok(self)
    }

    pub(crate) fn checkpoint(&mut self, patch: &Patch) {
        self.progress
            .record_completion(&patch.component_id, &patch.version_to);
    }
}
