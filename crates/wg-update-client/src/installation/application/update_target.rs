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

pub(super) async fn resolve_update_target(
    installed: &InstalledBuild,
    storage: &InstallationLayout,
) -> Result<(PinnedManifest, UpdateTargetSource)> {
    let (mut manifest, target_source) =
        if let Some(pending) = load_pinned_manifest(&storage.cache).await? {
            validate_cached_selection(&pending, &installed.selection())?;
            (pending, UpdateTargetSource::PinnedManifest)
        } else {
            (
                crate::installation::PinnedManifest::from(
                    resolve_update_release(
                        &installed.product,
                        &installed.identity.client_type,
                        &installed.identity.language,
                        &installed.component_versions,
                    )
                    .await?,
                ),
                UpdateTargetSource::LiveCatalog,
            )
        };
    // A package-only download may have pinned a full installation chain. Start
    // that chain at the installed versions, retaining any completed checkpoints.
    manifest.seed_missing_component_versions(installed);
    Ok((manifest, target_source))
}
