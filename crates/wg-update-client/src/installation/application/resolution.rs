use crate::catalog::resolve;
use crate::installation::{
    domain::selection::validate_cached_selection,
    infrastructure::filesystem::{installation_storage, records::load_cache_manifest},
};
use crate::{BuildSelection, PatchManifest, Result, WgusClient};
use std::path::PathBuf;

pub(super) async fn resolve_installation(
    client: &WgusClient,
    selection: &BuildSelection,
    requested_root: Option<PathBuf>,
) -> Result<(PatchManifest, PathBuf)> {
    selection.selected_product()?;
    if let Some(root) = requested_root {
        let storage = installation_storage(&root)?;
        if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
            validate_cached_selection(&manifest, selection)?;
            return Ok((manifest, root));
        }
        return Ok((resolve(client, selection).await?, root));
    }

    let live = resolve(client, selection).await?;
    let root = live.default_output_directory();
    let storage = installation_storage(&root)?;
    if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(&manifest, selection)?;
        return Ok((manifest, root));
    }
    Ok((live, root))
}
