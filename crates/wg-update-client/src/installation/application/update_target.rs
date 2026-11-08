//! Choose unfinished local work before live discovery and seed its starting versions.

use super::status::UpdateTargetSource;
use crate::installation::domain::selection::validate_cached_selection;
use crate::installation::infrastructure::filesystem::{
    InstallationLayout, records::load_pinned_manifest,
};
use crate::installation::{InstalledBuild, PinnedManifest};
use crate::{Result, catalog::resolve_update_release};

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
