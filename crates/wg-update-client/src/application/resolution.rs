use crate::domain::installation::selection::validate_cached_selection;
use crate::infrastructure::installation_fs::{installation_storage, records::load_cache_manifest};
use crate::{BuildSelection, Error, PatchManifest, Result, WgusClient, installation_assets};
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

pub(super) async fn resolve(
    client: &WgusClient,
    selection: &BuildSelection,
) -> Result<PatchManifest> {
    let product = selection.selected_product()?;
    if let Some(client_type) = selection.client_type.as_deref() {
        return client
            .manifest_for_product(&product, client_type, selection.language.as_deref())
            .await
            .map_err(Into::into);
    }

    let metadata = client.metadata_for_product(&product).await?;
    if let Some(default_client_type) = metadata.publisher_default_client_type() {
        return client
            .manifest_for_product(&product, default_client_type, selection.language.as_deref())
            .await
            .map_err(Into::into);
    }

    let available = metadata
        .client_types
        .iter()
        .map(|client_type| client_type.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut best: Option<(u64, PatchManifest)> = None;
    let mut first_error: Option<Error> = None;
    for client_type in &metadata.client_types {
        match client
            .manifest_for_product(&product, &client_type.id, selection.language.as_deref())
            .await
        {
            Ok(manifest) => match installation_assets(&manifest) {
                Ok(assets) => {
                    let size = assets.iter().map(|asset| asset.size).sum();
                    if best.as_ref().is_none_or(|(best_size, _)| size < *best_size) {
                        best = Some((size, manifest));
                    }
                }
                Err(error) => {
                    first_error.get_or_insert(error.into());
                }
            },
            Err(error) => {
                first_error.get_or_insert(error.into());
            }
        };
    }
    best.map(|(_, manifest)| manifest).ok_or_else(|| {
        first_error.unwrap_or_else(|| {
            crate::ReleaseError::UnknownClientType {
                requested: "publisher default or smallest variant".into(),
                available,
            }
            .into()
        })
    })
}
