//! Remaining patch work resolved once from the session's authoritative checkpoints.

use crate::installation::application::plan::build_plan;
use crate::installation::{InstallPlan, PinnedManifest};
use crate::{Result, catalog::PlannedPatch};
use std::path::PathBuf;

pub(super) struct ExecutionPlan {
    pub(super) patches: Vec<PlannedPatch>,
    pub(super) summary: InstallPlan,
}

impl ExecutionPlan {
    pub(super) fn resolve(manifest: &PinnedManifest, root: PathBuf) -> Result<Self> {
        let patches = manifest.planned_patches()?;
        let summary = build_plan(
            manifest,
            root,
            patches
                .iter()
                .flat_map(|patch| patch.volumes.iter().map(|volume| &volume.transport)),
        );
        Ok(Self { patches, summary })
    }
}
