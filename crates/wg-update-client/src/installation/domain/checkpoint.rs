use super::PinnedManifest;
use super::error::{InstallationError as Error, Result};
use crate::catalog::Patch;

impl PinnedManifest {
    pub(crate) fn resume_from(mut self, pending: Self) -> Result<Self> {
        if self.release != pending.release {
            return Err(Error::CachedSelectionMismatch {
                cached: format!("{} {}", pending.release.game_id, pending.release.version),
                requested: format!("{} {}", self.release.game_id, self.release.version),
            });
        }
        // Persisted checkpoints are authoritative when retrying an original plan.
        self.base_versions.extend(pending.base_versions);
        Ok(self)
    }

    pub(crate) fn checkpoint(&mut self, patch: &Patch) {
        self.base_versions
            .insert(patch.part.clone(), patch.version_to.clone());
    }
}
