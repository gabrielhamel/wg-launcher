//! Mutable execution state shared by local extraction and streaming installation.

use super::finalization::finalize_installation;
use super::patch::{PatchCompletion, PatchProgress, apply_and_checkpoint_patch};
use super::{ExtractionEvent, ExtractionOptions, ExtractionReport};
use crate::installation::{
    PinnedManifest,
    infrastructure::filesystem::{InstallationLayout, records, storage::prepare_installation},
};
use crate::{Result, catalog::Patch};
use std::sync::Arc;

/// One execution's layout, authoritative checkpoints, and completed delta count.
/// Callers retain the installation lock for the entire session lifetime.
pub(super) struct InstallationSession {
    pub(super) manifest: PinnedManifest,
    pub(super) layout: InstallationLayout,
    applied_deltas: usize,
}

impl InstallationSession {
    /// Restore authoritative saved checkpoints while the caller holds the lock.
    pub(super) fn resume(manifest: PinnedManifest, layout: InstallationLayout) -> Result<Self> {
        let manifest = match records::load_pinned_manifest_sync(&layout)? {
            Some(pending) => manifest.resume_from(pending)?,
            None => manifest,
        };
        Ok(Self {
            manifest,
            layout,
            applied_deltas: 0,
        })
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
