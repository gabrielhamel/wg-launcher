//! Shared streaming and local installation orchestration.

mod execution_plan;
mod extraction;
mod finalization;
mod patch;
mod prefetch;
mod prefetch_budget;
mod reports;
mod session;
mod streaming;

use super::progress::{ExtractionEvent, InstallationEvent};
pub use reports::{ExtractionOptions, ExtractionReport, InstallationReport};

/// Coordinates local extraction and streaming installation of a pinned release.
///
/// Acquires the per-installation lock and runs blocking archive/filesystem work
/// on Tokio's blocking pool. Game-owned inner `.pkg` files remain packed. These
/// lower-level methods accept and pin a resolved manifest without discovery.
/// An error can leave partially changed game files; no whole-install rollback is provided.
pub(crate) struct InstallationPipeline {
    session: session::InstallationSession,
}

impl InstallationPipeline {
    /// Restore checkpoints and resolve the remaining work while holding the lock.
    pub(crate) fn new(
        manifest: crate::installation::PinnedManifest,
        root: impl AsRef<std::path::Path>,
    ) -> crate::Result<Self> {
        let layout = crate::installation::infrastructure::filesystem::InstallationLayout::for_root(
            root.as_ref(),
        );
        Ok(Self {
            session: session::InstallationSession::resume(manifest, layout)?,
        })
    }

    pub(crate) fn plan(&self) -> &crate::installation::InstallPlan {
        &self.session.execution_plan.summary
    }
}

/// Failure of a background installation worker.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// A background installation worker panicked or was cancelled.
    #[error("background installation worker failed: {0}")]
    WorkerFailed(#[source] tokio::task::JoinError),
    /// The download coordinator ended without producing every planned patch.
    #[error("package downloads stopped before all patches were ready")]
    DownloadsStopped,
}
