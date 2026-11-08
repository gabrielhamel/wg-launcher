//! Mutable execution state shared by local extraction and streaming installation.

use super::super::access::LockedInstallation;
use super::finalization::finalize_installation;
use super::patch::{PatchCompletion, PatchProgress, apply_and_checkpoint_patch};
use super::{ExtractionEvent, ExtractionOptions, ExtractionReport};
use crate::Result;
use crate::installation::{
    PinnedManifest,
    infrastructure::filesystem::{
        InstallationLayout, InstallationLock, cache::LocalPatch, records,
        storage::prepare_installation,
    },
};
use std::sync::Arc;

/// One execution's layout, authoritative checkpoints, and completed delta count.
/// Owns exclusive access even while a blocking worker outlives its awaiting future.
pub(super) struct InstallationSession {
    pub(super) manifest: PinnedManifest,
    pub(super) access: LockedInstallation,
    applied_deltas: usize,
}

impl InstallationSession {
    /// Acquire exclusive access before restoring authoritative saved checkpoints.
    pub(super) fn resume(manifest: PinnedManifest, layout: InstallationLayout) -> Result<Self> {
        let access = LockedInstallation::open(layout)?;
        let manifest = access.resume_exact_release(manifest)?;
        Ok(Self {
            manifest,
            access,
            applied_deltas: 0,
        })
    }

    /// Keep downloads protected until cancellation completes, even if patch execution fails.
    pub(super) fn retain_lock(&self) -> Arc<InstallationLock> {
        self.access.retain_lock()
    }

    pub(super) fn prepare(&self) -> Result<()> {
        records::save_manifest(&self.manifest, &self.access.layout)?;
        prepare_installation(&self.access.layout)?;
        Ok(())
    }

    pub(super) fn apply_patch(
        &mut self,
        patch: &LocalPatch,
        progress: PatchProgress,
        options: ExtractionOptions,
        observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    ) -> Result<PatchCompletion> {
        let completion = apply_and_checkpoint_patch(
            &mut self.manifest,
            patch,
            &self.access.layout,
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
            &self.access.layout,
            options,
            self.applied_deltas,
            observer,
        )
    }
}
