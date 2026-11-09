use crate::Result;
use crate::catalog::SelectedProductRequest;
use crate::catalog::{BuildInfo, resolve};
use crate::installation::PinnedManifest;
use crate::installation::{
    domain::selection::validate_cached_request,
    infrastructure::filesystem::{InstallationLayout, records::load_pinned_manifest},
};
use std::path::PathBuf;

/// The pinned release and destination chosen for installation or package download.
pub(super) struct InstallationTarget {
    pub(super) manifest: PinnedManifest,
    pub(super) root: PathBuf,
}

pub(super) async fn resolve_installation_target(
    selection: &SelectedProductRequest<'_>,
    requested_root: Option<PathBuf>,
) -> Result<InstallationTarget> {
    if let Some(root) = requested_root {
        let storage = InstallationLayout::for_root(&root);
        if let Some(manifest) = load_pinned_manifest(&storage).await? {
            validate_cached_request(&manifest.selection(), selection)?;
            return Ok(InstallationTarget { manifest, root });
        }
        return Ok(InstallationTarget {
            manifest: resolve(selection).await?.into(),
            root,
        });
    }

    let live = PinnedManifest::from(resolve(selection).await?);
    let root = BuildInfo::from_release(&live.release).default_output_directory();
    let storage = InstallationLayout::for_root(&root);
    if let Some(manifest) = load_pinned_manifest(&storage).await? {
        validate_cached_request(&manifest.selection(), selection)?;
        return Ok(InstallationTarget { manifest, root });
    }
    Ok(InstallationTarget {
        manifest: live,
        root,
    })
}
