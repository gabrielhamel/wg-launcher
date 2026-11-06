//! Filesystem persistence and original delta sources for patch retries.

use super::{
    error::Result,
    paths::safe_relative_path,
    records::save_manifest,
    storage::{DOWNLOAD_MANIFEST_FILE, InstallationStorage},
};
use crate::catalog::Patch;
use crate::installation::PinnedManifest;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn resume_manifest(
    manifest: PinnedManifest,
    storage: &InstallationStorage,
) -> Result<PinnedManifest> {
    let bytes = match fs::read(storage.cache.join(DOWNLOAD_MANIFEST_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(manifest),
        Err(error) => return Err(error.into()),
    };
    let pending: PinnedManifest = serde_json::from_slice(&bytes)?;
    Ok(manifest.resume_from(pending)?)
}

pub(crate) fn checkpoint_patch(
    manifest: &mut PinnedManifest,
    patch: &Patch,
    storage: &InstallationStorage,
) -> Result<()> {
    manifest.checkpoint(patch);
    save_manifest(manifest, storage)?;
    let active = storage.staging.join("active");
    if active.exists() {
        fs::remove_dir_all(active)?;
    }
    Ok(())
}

pub(crate) fn prepare_patch(patch: &Patch, work: &Path) -> Result<PathBuf> {
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

pub(crate) fn delta_source(game: &Path, relative: &Path, active: &Path) -> Result<PathBuf> {
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
