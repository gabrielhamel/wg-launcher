//! Pinned component checkpoints and original delta sources for patch retries.

use super::{
    paths::safe_relative_path,
    storage::{DOWNLOAD_MANIFEST_FILE, InstallationStorage, replace_file},
};
use crate::{Error, Patch, PatchManifest, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn resume_manifest(
    mut manifest: PatchManifest,
    storage: &InstallationStorage,
) -> Result<PatchManifest> {
    let bytes = match fs::read(storage.cache.join(DOWNLOAD_MANIFEST_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(manifest),
        Err(error) => return Err(error.into()),
    };
    let pending: PatchManifest = serde_json::from_slice(&bytes)?;
    let mut requested_identity = serde_json::to_value(&manifest)?;
    let mut pending_identity = serde_json::to_value(&pending)?;
    requested_identity
        .as_object_mut()
        .unwrap()
        .remove("base_versions");
    pending_identity
        .as_object_mut()
        .unwrap()
        .remove("base_versions");
    if requested_identity != pending_identity {
        return Err(Error::CachedSelectionMismatch {
            cached: format!("{} {}", pending.game_id, pending.version),
            requested: format!("{} {}", manifest.game_id, manifest.version),
        });
    }
    // Low-level callers may retry with their original manifest. Never rewind
    // component checkpoints that have already been committed under this lock.
    manifest.base_versions.extend(pending.base_versions);
    Ok(manifest)
}

pub(super) fn save_manifest(manifest: &PatchManifest, storage: &InstallationStorage) -> Result<()> {
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

pub(super) fn checkpoint_patch(
    manifest: &mut PatchManifest,
    patch: &Patch,
    storage: &InstallationStorage,
) -> Result<()> {
    manifest
        .base_versions
        .insert(patch.part.clone(), patch.version_to.clone());
    save_manifest(manifest, storage)?;
    let active = storage.work.join("active");
    if active.exists() {
        fs::remove_dir_all(active)?;
    }
    Ok(())
}

pub(super) fn prepare_patch(patch: &Patch, work: &Path) -> Result<PathBuf> {
    let active = work.join("active");
    let identity =
        serde_json::to_vec(&(patch.part.as_str(), &patch.version_from, &patch.version_to))?;
    if fs::read(active.join("patch.json")).ok().as_deref() != Some(identity.as_slice()) {
        if active.exists() {
            fs::remove_dir_all(&active)?;
        }
        fs::create_dir_all(&active)?;
        fs::write(active.join("patch.json"), identity)?;
    }
    let archive = active.join("archive");
    if archive.exists() {
        fs::remove_dir_all(&archive)?;
    }
    fs::create_dir_all(&archive)?;
    Ok(active)
}

pub(super) fn delta_source(game: &Path, relative: &Path, active: &Path) -> Result<PathBuf> {
    let original = active
        .join("sources")
        .join(safe_relative_path(&relative.to_string_lossy())?);
    if !original.exists() {
        if let Some(parent) = original.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = original.with_extension("source.part");
        fs::copy(game.join(relative), &temporary)?;
        fs::rename(temporary, &original)?;
    }
    Ok(original)
}
