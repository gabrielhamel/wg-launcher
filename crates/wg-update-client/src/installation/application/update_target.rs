//! Choose unfinished local work before live discovery and seed its starting versions.

use crate::installation::domain::selection::validate_cached_selection;
use crate::installation::infrastructure::filesystem::{
    InstallationLayout, records::load_pinned_manifest,
};
use crate::installation::{InstalledBuild, PinnedManifest};
use crate::{Result, catalog::resolve_update_release};
use serde::Serialize;

/// Source of the build chosen for comparison with an installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateTargetSource {
    /// The publisher's current release, resolved from the live catalog.
    LiveCatalog,
    /// The saved release for unfinished work, which may differ from the live catalog.
    PinnedManifest,
}

/// Build version chosen for comparison, together with how it was selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateTarget {
    /// Publisher game build version; component versions are compared separately.
    pub version: String,
    /// Whether this target came from live discovery or unfinished local work.
    pub source: UpdateTargetSource,
}

/// A chosen release, its local progress, and its discovery origin kept together.
pub(super) struct ResolvedUpdateTarget {
    manifest: PinnedManifest,
    source: UpdateTargetSource,
}

impl ResolvedUpdateTarget {
    fn new(
        mut manifest: PinnedManifest,
        source: UpdateTargetSource,
        installed: &InstalledBuild,
    ) -> Self {
        // A package-only download may pin a full chain. Preserve saved checkpoints
        // while supplying installed versions for components not yet checkpointed.
        manifest.seed_missing_component_versions(installed);
        Self { manifest, source }
    }

    pub(super) async fn live(installed: &InstalledBuild) -> Result<Self> {
        let release = resolve_update_release(
            &installed.product,
            &installed.identity.client_type,
            &installed.identity.language,
            &installed.component_versions,
        )
        .await?;
        Ok(Self::new(
            release.into(),
            UpdateTargetSource::LiveCatalog,
            installed,
        ))
    }

    pub(super) fn status_report(
        &self,
        path: std::path::PathBuf,
        installed: &InstalledBuild,
    ) -> Result<super::status::StatusReport> {
        super::status_report::build_status_report(
            path,
            installed,
            &self.manifest.release,
            self.manifest.reached_versions(),
            self.source,
        )
    }

    pub(super) fn into_manifest(self) -> PinnedManifest {
        self.manifest
    }
}

pub(super) async fn resolve_update_target(
    installed: &InstalledBuild,
    storage: &InstallationLayout,
) -> Result<ResolvedUpdateTarget> {
    if let Some(pending) = load_pinned_manifest(storage).await? {
        validate_cached_selection(&pending, &installed.selection())?;
        Ok(ResolvedUpdateTarget::new(
            pending,
            UpdateTargetSource::PinnedManifest,
            installed,
        ))
    } else {
        ResolvedUpdateTarget::live(installed).await
    }
}
