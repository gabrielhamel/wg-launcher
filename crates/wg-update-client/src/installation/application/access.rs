//! Adopt authoritative local work while holding exclusive installation access.

use crate::installation::{
    InstalledBuild, PinnedManifest,
    domain::selection::validate_cached_request,
    infrastructure::filesystem::{InstallationLayout, InstallationLock, records},
};
use crate::{Result, catalog::SelectedProductRequest};
use std::sync::Arc;

pub(super) struct LockedInstallation {
    layout: InstallationLayout,
    lock: Arc<InstallationLock>,
}

impl LockedInstallation {
    pub(super) fn open(layout: InstallationLayout) -> Result<Self> {
        let lock = Arc::new(InstallationLock::acquire(&layout)?);
        Ok(Self { layout, lock })
    }

    pub(super) fn layout(&self) -> &InstallationLayout {
        &self.layout
    }

    /// Persist pinned work only while this installation is exclusively owned.
    pub(super) fn save_manifest(&self, manifest: &PinnedManifest) -> Result<()> {
        records::save_manifest(manifest, &self.layout)?;
        Ok(())
    }

    /// Publish completed identity under the same lock as patch execution.
    pub(super) fn save_installed_build(&self, installed: &InstalledBuild) -> Result<()> {
        records::save_installed_build(installed, &self.layout)?;
        Ok(())
    }

    /// Downloads adopt compatible pinned work, even if discovery found another release.
    pub(super) async fn adopt_matching_download(
        &self,
        discovered: PinnedManifest,
        request: &SelectedProductRequest<'_>,
    ) -> Result<PinnedManifest> {
        match records::load_pinned_manifest(&self.layout).await? {
            Some(pending) => {
                validate_cached_request(&pending, request)?;
                Ok(pending)
            }
            None => Ok(discovered),
        }
    }

    /// Execution restores checkpoints only when the entire pinned release matches.
    pub(super) fn resume_exact_release(&self, manifest: PinnedManifest) -> Result<PinnedManifest> {
        match records::load_pinned_manifest_sync(&self.layout)? {
            Some(pending) => Ok(manifest.resume_from(pending)?),
            None => Ok(manifest),
        }
    }

    /// Keep exclusive access until blocking work and download cancellation have ended.
    pub(super) fn retain_lock(&self) -> Arc<InstallationLock> {
        Arc::clone(&self.lock)
    }
}
