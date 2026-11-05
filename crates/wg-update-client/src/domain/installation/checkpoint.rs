use super::error::{InstallationError as Error, Result};
use crate::{InstalledBuild, Patch, PatchManifest};

impl PatchManifest {
    pub(crate) fn resume_from(mut self, pending: Self) -> Result<Self> {
        let mut requested_identity = self.clone();
        let mut pending_identity = pending.clone();
        requested_identity.base_versions.clear();
        pending_identity.base_versions.clear();
        if requested_identity != pending_identity {
            return Err(Error::CachedSelectionMismatch {
                cached: format!("{} {}", pending.game_id, pending.version),
                requested: format!("{} {}", self.game_id, self.version),
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

    pub(crate) fn include_installed_versions(&mut self, installed: &InstalledBuild) {
        for (part, version) in &installed.component_versions {
            self.base_versions
                .entry(part.clone())
                .or_insert_with(|| version.clone());
        }
    }
}
