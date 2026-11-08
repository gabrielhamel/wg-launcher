use crate::Result;
use crate::catalog::SelectedProductRequest;
use crate::catalog::{BuildInfo, resolve};
use crate::installation::PinnedManifest;
use crate::installation::{
    domain::selection::validate_cached_request,
    infrastructure::filesystem::{InstallationLayout, records::load_pinned_manifest},
};
use std::path::PathBuf;

pub(super) async fn resolve_installation(
    selection: &SelectedProductRequest<'_>,
    requested_root: Option<PathBuf>,
) -> Result<(PinnedManifest, PathBuf)> {
    if let Some(root) = requested_root {
        let storage = InstallationLayout::for_root(&root);
        if let Some(manifest) = load_pinned_manifest(&storage.cache).await? {
            validate_cached_request(&manifest, selection)?;
            return Ok((manifest, root));
        }
        return Ok((resolve(selection).await?.into(), root));
    }

    let live = PinnedManifest::from(resolve(selection).await?);
    let root = BuildInfo::from_release(&live.release).default_output_directory();
    let storage = InstallationLayout::for_root(&root);
    if let Some(manifest) = load_pinned_manifest(&storage.cache).await? {
        validate_cached_request(&manifest, selection)?;
        return Ok((manifest, root));
    }
    Ok((live, root))
}
