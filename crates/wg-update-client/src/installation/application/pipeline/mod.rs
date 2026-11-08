//! Shared streaming and local installation orchestration.

mod extraction;
mod patch;
mod prefetch;
mod streaming;
mod types;

pub use types::{
    ExtractionEvent, ExtractionOptions, ExtractionReport, InstallationEvent, InstallationReport,
};

/// Coordinates local extraction and streaming installation of a pinned release.
///
/// Acquires the per-installation lock and runs blocking archive/filesystem work
/// on Tokio's blocking pool. Game-owned inner `.pkg` files remain packed. These
/// lower-level methods accept and pin a resolved manifest without discovery.
/// An error can leave partially changed game files; no whole-install rollback is provided.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallationPipeline;

/// Failure of a background installation worker.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// A background extraction or download-coordination task failed.
    #[error("background extraction task failed: {0}")]
    ExtractionTask(String),
}

/// Restore saved component checkpoints under the installation lock.
fn resume_manifest(
    manifest: crate::installation::PinnedManifest,
    storage: &crate::installation::infrastructure::filesystem::storage::InstallationStorage,
) -> crate::Result<crate::installation::PinnedManifest> {
    use crate::installation::infrastructure::filesystem::records::load_pending_manifest;
    match load_pending_manifest(storage)? {
        Some(pending) => Ok(manifest.resume_from(pending)?),
        None => Ok(manifest),
    }
}
