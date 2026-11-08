//! Mutable execution state shared by local extraction and streaming installation.

use super::execution_plan::ExecutionPlan;
use super::finalization::finalize_installation;
use super::patch::{PatchCompletion, PatchProgress, apply_and_checkpoint_patch};
use super::{ExtractionEvent, ExtractionOptions, ExtractionReport};
use crate::installation::{
    PinnedManifest,
    infrastructure::filesystem::{
        InstallationLayout, InstallationLock, records, storage::prepare_installation,
    },
};
use crate::{Result, catalog::Patch};
use std::sync::Arc;

/// One execution's layout, authoritative checkpoints, and completed delta count.
/// Owns exclusive access even while a blocking worker outlives its awaiting future.
pub(super) struct InstallationSession {
    pub(super) manifest: PinnedManifest,
    pub(super) layout: InstallationLayout,
    pub(super) execution_plan: ExecutionPlan,
    applied_deltas: usize,
    lock: Arc<InstallationLock>,
}

impl InstallationSession {
    /// Acquire exclusive access before restoring authoritative saved checkpoints.
    pub(super) fn resume(manifest: PinnedManifest, layout: InstallationLayout) -> Result<Self> {
        let lock = Arc::new(InstallationLock::acquire(&layout)?);
        let manifest = match records::load_pinned_manifest_sync(&layout)? {
            Some(pending) => manifest.resume_from(pending)?,
            None => manifest,
        };
        let execution_plan = ExecutionPlan::resolve(&manifest, layout.root().to_path_buf())?;
        Ok(Self {
            execution_plan,
            manifest,
            layout,
            applied_deltas: 0,
            lock,
        })
    }

    /// Keep downloads protected until cancellation completes, even if patch execution fails.
    pub(super) fn retain_lock(&self) -> Arc<InstallationLock> {
        Arc::clone(&self.lock)
    }

    pub(super) fn prepare(&self) -> Result<()> {
        records::save_manifest(&self.manifest, &self.layout)?;
        prepare_installation(&self.layout)?;
        Ok(())
    }

    pub(super) fn apply_patch(
        &mut self,
        patch: &Patch,
        progress: PatchProgress,
        options: ExtractionOptions,
        observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    ) -> Result<PatchCompletion> {
        let completion = apply_and_checkpoint_patch(
            &mut self.manifest,
            patch,
            &self.layout,
            progress,
            options,
            observer,
        )?;
        self.applied_deltas += completion.applied_deltas;
        Ok(completion)
    }

    pub(super) fn finalize(
        self,
        options: ExtractionOptions,
        observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    ) -> Result<ExtractionReport> {
        finalize_installation(
            &self.manifest,
            &self.layout,
            options,
            self.applied_deltas,
            observer,
        )
    }
}
