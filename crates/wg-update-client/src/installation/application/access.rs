//! Adopt authoritative local work while holding exclusive installation access.

use crate::installation::{
    PinnedManifest,
    domain::selection::validate_cached_request,
    infrastructure::filesystem::{InstallationLayout, InstallationLock, records},
};
use crate::{Result, catalog::SelectedProductRequest};
use std::sync::Arc;

pub(super) struct LockedInstallation {
    pub(super) layout: InstallationLayout,
    lock: Arc<InstallationLock>,
}

impl LockedInstallation {
    pub(super) fn open(layout: InstallationLayout) -> Result<Self> {
        let lock = Arc::new(InstallationLock::acquire(&layout)?);
        Ok(Self { layout, lock })
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
