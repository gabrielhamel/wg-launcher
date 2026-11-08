use super::error::Result;
use super::{InstallationStorage, replacement::replace_file};
use crate::installation::{DOWNLOAD_MANIFEST_FILE, InstalledBuild, PinnedManifest};
use std::{fs, path::Path};

pub(crate) async fn load_installed_build(root: &Path) -> Result<Option<InstalledBuild>> {
    let installed_state = InstallationStorage::for_root(root).installed_state;
    match tokio::fs::read(&installed_state).await {
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

pub(crate) fn save_installed_build(
    installed_build: &InstalledBuild,
    installed_state: &Path,
) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(installed_build)?;
    if let Some(parent) = installed_state.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = installed_state.with_extension("json.part");
    fs::write(&temporary, bytes)?;
    replace_file(
        &temporary,
        installed_state,
        &installed_state.with_extension("json.backup"),
    )?;
    Ok(())
}

pub(crate) fn save_manifest(
    manifest: &PinnedManifest,
    storage: &InstallationStorage,
) -> Result<()> {
    fs::create_dir_all(&storage.cache)?;
    let target = storage.cache.join(DOWNLOAD_MANIFEST_FILE);
    let temporary = storage.cache.join("manifest.json.part");
    fs::write(&temporary, serde_json::to_vec_pretty(manifest)?)?;
    replace_file(
        &temporary,
        &target,
        &storage.cache.join("manifest.json.backup"),
    )
}

pub(crate) fn load_pending_manifest(
    storage: &InstallationStorage,
) -> Result<Option<PinnedManifest>> {
    let bytes = match fs::read(storage.cache.join(DOWNLOAD_MANIFEST_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}
