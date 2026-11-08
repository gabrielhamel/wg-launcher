use crate::Result;
use crate::catalog::{BuildInfo, resolve};
use crate::catalog::{BuildSelection, WgusClient};
use crate::installation::PinnedManifest;
use crate::installation::{
    domain::selection::validate_cached_request,
    infrastructure::filesystem::{InstallationStorage, records::load_pinned_manifest},
};
use std::path::PathBuf;

pub(super) async fn resolve_installation(
    client: &WgusClient,
    selection: &BuildSelection,
    requested_root: Option<PathBuf>,
) -> Result<(PinnedManifest, PathBuf)> {
    selection.selected_product()?;
    if let Some(root) = requested_root {
        let storage = InstallationStorage::for_root(&root);
        if let Some(manifest) = load_pinned_manifest(&storage.cache).await? {
            validate_cached_request(&manifest, selection)?;
            return Ok((manifest, root));
        }
        return Ok((resolve(client, selection).await?.into(), root));
    }

    let live = PinnedManifest::from(resolve(client, selection).await?);
    let root = BuildInfo::from_release(&live.release).default_output_directory();
    let storage = InstallationStorage::for_root(&root);
    if let Some(manifest) = load_pinned_manifest(&storage.cache).await? {
        validate_cached_request(&manifest, selection)?;
        return Ok((manifest, root));
    }
    Ok((live, root))
}
