use super::InstallationPipeline;
use super::patch::PatchProgress;
use super::session::InstallationSession;
use crate::Result;
use crate::installation::infrastructure::filesystem::cache::{LocalPatch, verify_cache};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport};
use std::sync::Arc;

pub(super) fn extract_sync(
    mut session: InstallationSession,
    patches: Vec<crate::catalog::PlannedPatch>,
    options: ExtractionOptions,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let cache = &session.access.layout.cache;
    observer(ExtractionEvent::CheckingCache {
        assets: session.manifest.release.volumes.len(),
    });
    let patches = patches
        .into_iter()
        .map(|planned| LocalPatch::from_plan(&planned, cache))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    verify_cache(&patches)?;
    session.prepare()?;

    let total = patches.len();
    for (index, patch) in patches.into_iter().enumerate() {
        session.apply_patch(&patch, PatchProgress { index, total }, options, &observer)?;
    }
    session.finalize(options, observer)
}

impl InstallationPipeline {
    /// Extracts an already downloaded manifest with default cache cleanup.
    ///
    /// Equivalent to [`Self::extract_with_options_and_observer`] with [`ExtractionOptions::default`].
    /// Reads volumes from the session's `.wg-launcher/download` without network access and writes
    /// the installed-build record on success.
    ///
    /// # Errors
    ///
    /// Returns the lock, cache, patch-chain, filesystem, archive, delta, or worker
    /// errors described by [`Self::extract_with_options_and_observer`].
    pub async fn extract_with_observer<F>(self, observer: F) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        self.extract_with_options_and_observer(ExtractionOptions::default(), observer)
            .await
    }

    /// Extracts local packages with an explicit cache-retention policy.
    ///
    /// Retains the session's installation lock, verifies required volume sizes, applies the
    /// remaining patches, and records the installed build. Pins the supplied manifest
    /// and preserves completed checkpoints and original delta sources on retry.
    /// Callbacks run on the blocking extraction worker; see the
    /// [observer contract](crate#observers-and-async-execution).
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, incomplete cache, invalid patch chains or
    /// paths, filesystem failures, invalid archives, failed deltas, or worker failure.
    /// Cache verification uses sizes rather than content hashes.
    pub async fn extract_with_options_and_observer<F>(
        self,
        options: ExtractionOptions,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        let (session, patches) = self.into_execution();
        let observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = Arc::new(observer);
        tokio::task::spawn_blocking(move || extract_sync(session, patches, options, observer))
            .await
            .map_err(crate::installation::PipelineError::WorkerFailed)?
    }
}
