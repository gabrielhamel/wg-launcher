use super::BuildSelection;
use crate::{
    Asset, DOWNLOAD_MANIFEST_FILE, Error, InstalledBuild, PatchManifest, Result, WgusClient,
    install::{installation_assets, installation_storage},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Resolved build and filesystem plan emitted before transfer or extraction.
///
/// A plan event is informational; it does not guarantee the operation will succeed.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    /// Complete pinned build manifest, even for a filtered download.
    pub manifest: PatchManifest,
    /// Installation root, which can be relative to the current directory.
    pub root: PathBuf,
    /// Package cache directory beneath the root; relative when the root is relative.
    pub cache: PathBuf,
    /// Selected transport volumes in installation order.
    pub assets: Vec<Asset>,
    /// Total compressed bytes in `assets`, before subtracting cached data.
    pub compressed_bytes: u64,
}

pub(super) async fn load_installed_build(root: &Path) -> Result<InstalledBuild> {
    let marker = installation_storage(root)?.marker;
    match tokio::fs::read(&marker).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(crate::Error::InstallationNotFound(root.to_path_buf()))
        }
        Err(error) => Err(error.into()),
    }
}

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

pub(super) fn build_plan(
    manifest: PatchManifest,
    root: PathBuf,
    assets: Vec<Asset>,
) -> Result<InstallPlan> {
    let compressed_bytes = assets.iter().map(|asset| asset.size).sum();
    let cache = installation_storage(&root)?.cache;
    Ok(InstallPlan {
        manifest,
        root,
        cache,
        assets,
        compressed_bytes,
    })
}

pub(super) async fn resolve(
    client: &WgusClient,
    selection: &BuildSelection,
) -> Result<PatchManifest> {
    let product = selection.selected_product()?;
    if let Some(client_type) = selection.client_type.as_deref() {
        return client
            .manifest_for_product(&product, client_type, selection.language.as_deref())
            .await;
    }

    let metadata = client.metadata_for_product(&product).await?;
    if metadata
        .client_type(&metadata.default_client_type)
        .is_some()
    {
        return client
            .manifest_for_product(
                &product,
                &metadata.default_client_type,
                selection.language.as_deref(),
            )
            .await;
    }

    let available = metadata
        .client_types
        .iter()
        .map(|client_type| client_type.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut best: Option<(u64, PatchManifest)> = None;
    let mut first_error = None;
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
                    first_error.get_or_insert(error);
                }
            },
            Err(error) => {
                first_error.get_or_insert(error);
            }
        };
    }
    best.map(|(_, manifest)| manifest).ok_or_else(|| {
        first_error.unwrap_or_else(|| Error::UnknownClientType {
            requested: "publisher default or smallest variant".into(),
            available,
        })
    })
}

pub(super) async fn load_cache_manifest(cache: &Path) -> Result<Option<PatchManifest>> {
    let path = cache.join(DOWNLOAD_MANIFEST_FILE);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn save_cache_manifest(manifest: &PatchManifest, cache: &Path) -> Result<()> {
    tokio::fs::create_dir_all(cache).await?;
    let mut bytes = serde_json::to_vec_pretty(manifest)?;
    bytes.push(b'\n');
    let target = cache.join(DOWNLOAD_MANIFEST_FILE);
    let temporary = cache.join(format!("{DOWNLOAD_MANIFEST_FILE}.part"));
    tokio::fs::write(&temporary, bytes).await?;
    if cfg!(windows) && target.exists() {
        tokio::fs::remove_file(&target).await?;
    }
    tokio::fs::rename(temporary, target).await?;
    Ok(())
}

pub(super) fn validate_cached_selection(
    manifest: &PatchManifest,
    selection: &BuildSelection,
) -> Result<()> {
    let product = selection.selected_product()?;
    let selected_region = &product.region;
    let language_matches = selection
        .language
        .as_deref()
        .is_none_or(|language| language.eq_ignore_ascii_case(&manifest.language));
    let client_type_matches = selection
        .client_type
        .as_deref()
        .is_none_or(|client_type| manifest.client_type.eq_ignore_ascii_case(client_type));
    if manifest.product == product
        && manifest.region == *selected_region
        && client_type_matches
        && language_matches
    {
        return Ok(());
    }
    Err(crate::Error::CachedSelectionMismatch {
        cached: format!(
            "{}/{}/{}{}",
            manifest.region,
            manifest.client_type,
            manifest.language,
            product_description(&manifest.product)
        ),
        requested: format!(
            "{}/{}/{}{}",
            selected_region,
            selection
                .client_type
                .as_deref()
                .unwrap_or("publisher default or smallest variant"),
            selection.language.as_deref().unwrap_or("publisher default"),
            product_description(&product)
        ),
    })
}

fn product_description(product: &crate::Product) -> String {
    format!(
        " ({}/{}: {}, {}, {})",
        product.game(),
        product.name,
        product.host,
        product.game_id,
        product.channel
    )
}
