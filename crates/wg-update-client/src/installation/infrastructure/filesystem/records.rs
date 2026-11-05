use super::error::Result;
use super::installation_storage;
use crate::installation::{DOWNLOAD_MANIFEST_FILE, InstalledBuild, PinnedManifest};
use std::path::Path;

pub(crate) async fn load_installed_build(root: &Path) -> Result<Option<InstalledBuild>> {
    let marker = installation_storage(root)?.marker;
    match tokio::fs::read(&marker).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn load_cache_manifest(cache: &Path) -> Result<Option<PinnedManifest>> {
    let path = cache.join(DOWNLOAD_MANIFEST_FILE);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn save_cache_manifest(manifest: &PinnedManifest, cache: &Path) -> Result<()> {
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
