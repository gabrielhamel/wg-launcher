//! Shared streaming and local installation orchestration.

mod execution_plan;
mod extraction;
mod finalization;
mod options;
mod patch;
mod prefetch;
mod prefetch_budget;
mod reports;
mod session;
mod streaming;

use super::progress::{ExtractionEvent, InstallationEvent};
pub(crate) use options::{ExtractionOptions, PackageRetention};
pub use reports::{ExtractionReport, InstallationReport};

/// Coordinates local extraction and streaming installation of a pinned release.
///
/// Acquires the per-installation lock and runs blocking archive/filesystem work
/// on Tokio's blocking pool. Game-owned inner `.pkg` files remain packed. These
/// lower-level methods accept and pin a resolved manifest without discovery.
/// An error can leave partially changed game files; no whole-install rollback is provided.
pub(crate) struct InstallationPipeline {
    session: session::InstallationSession,
    execution_plan: execution_plan::ExecutionPlan,
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
        let session = session::InstallationSession::resume(manifest, layout)?;
        let execution_plan = execution_plan::ExecutionPlan::resolve(
            &session.manifest,
            session.access.layout().root().to_path_buf(),
        )?;
        Ok(Self {
            session,
            execution_plan,
        })
    }

    pub(crate) fn plan(&self) -> &crate::installation::InstallPlan {
        &self.execution_plan.summary
    }

    /// Consume the planned pipeline once, handing its remaining patches to execution.
    fn into_execution(
        self,
    ) -> (
        session::InstallationSession,
        Vec<crate::catalog::PlannedPatch>,
    ) {
        (self.session, self.execution_plan.patches)
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

impl From<PipelineError> for crate::Error {
    fn from(error: PipelineError) -> Self {
        Self::new(crate::ErrorKind::Internal, error)
    }
}
