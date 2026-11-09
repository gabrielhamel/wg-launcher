//! Package retention policy shared by patch completion and finalization.

/// Lifecycle of successfully applied transport packages.
/// Temporary extraction work is removed after success under either policy.
#[derive(Clone, Copy)]
pub(crate) enum PackageRetention {
    /// Release volumes after each checkpoint and remove the cache at completion.
    ReleaseAfterCheckpoint,
    /// Keep downloaded volumes and the pinned manifest for reuse.
    KeepDownloaded,
}

/// Configuration accepted at the extraction execution boundary.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExtractionOptions {
    /// Release checkpointed packages and remove the cache after success.
    pub(crate) clean_download_cache: bool,
}

impl ExtractionOptions {
    pub(super) fn package_retention(self) -> PackageRetention {
        if self.clean_download_cache {
            PackageRetention::ReleaseAfterCheckpoint
        } else {
            PackageRetention::KeepDownloaded
        }
    }
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            clean_download_cache: true,
        }
    }
}
