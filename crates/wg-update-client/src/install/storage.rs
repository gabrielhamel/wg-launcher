use super::{InstalledBuild, paths::safe_relative_path};
use crate::{Error, Patch, PatchManifest, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

/// Installation-relative directory containing transport volumes and the pinned manifest.
pub const DOWNLOAD_CACHE_DIRECTORY: &str = ".wg-launcher/download";
/// Pinned manifest filename inside [`DOWNLOAD_CACHE_DIRECTORY`].
pub const DOWNLOAD_MANIFEST_FILE: &str = "manifest.json";
/// Directory beneath an installation root used for temporary extraction and delta work.
pub const STAGING_DIRECTORY: &str = ".wg-launcher/installing";
/// Installed-build JSON path relative to an installation root.
pub const INSTALL_MARKER_FILE: &str = ".wg-launcher/state.json";
const INSTALL_LOCK_FILE: &str = ".wg-launcher/wg-launcher.lock";
#[derive(Debug, Clone)]
pub(crate) struct InstallationStorage {
    pub cache: PathBuf,
    pub work: PathBuf,
    pub marker: PathBuf,
    pub lock: PathBuf,
}

pub(crate) fn installation_storage(install_root: &Path) -> Result<InstallationStorage> {
    Ok(InstallationStorage {
        cache: install_root.join(DOWNLOAD_CACHE_DIRECTORY),
        work: install_root.join(STAGING_DIRECTORY),
        marker: install_root.join(INSTALL_MARKER_FILE),
        lock: install_root.join(INSTALL_LOCK_FILE),
    })
}

pub(super) fn prepare_installation(
    install_root: &Path,
    storage: &InstallationStorage,
) -> Result<()> {
    fs::create_dir_all(install_root)?;
    fs::create_dir_all(&storage.work)?;
    Ok(())
}

pub(super) struct InstallationLock {
    file: File,
}

impl InstallationLock {
    pub(super) fn acquire(storage: &InstallationStorage, install_root: &Path) -> Result<Self> {
        fs::create_dir_all(install_root.join(".wg-launcher"))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&storage.lock)?;
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Self { file }),
            Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                Err(Error::InstallationLocked(install_root.to_path_buf()))
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for InstallationLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

pub(super) fn remove_patch_volumes(cache: &Path, patch: &Patch) -> Result<u64> {
    let mut removed = 0;
    for file in &patch.files {
        let path = cache.join(safe_relative_path(&file.name)?);
        removed += fs::metadata(&path)?.len();
        fs::remove_file(&path)?;
        remove_empty_parents(path.parent(), cache)?;
    }
    Ok(removed)
}

fn remove_empty_parents(mut directory: Option<&Path>, cache: &Path) -> Result<()> {
    while let Some(path) = directory {
        if path == cache || !path.starts_with(cache) {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => directory = path.parent(),
            Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn verify_cache(patches: &[&Patch], cache: &Path) -> Result<()> {
    for file in patches.iter().flat_map(|patch| &patch.files) {
        let path = cache.join(safe_relative_path(&file.name)?);
        let actual = fs::metadata(&path)
            .map(|metadata| {
                if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        if actual != file.size {
            return Err(Error::IncompleteCache {
                path,
                expected: file.size,
                actual,
            });
        }
    }
    Ok(())
}

pub(super) fn write_marker(manifest: &PatchManifest, marker: &Path) -> Result<()> {
    let marker_data = InstalledBuild::from(manifest);
    let bytes = serde_json::to_vec_pretty(&marker_data)
        .map_err(|error| Error::ExtractionTask(error.to_string()))?;
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = marker.with_extension("json.part");
    fs::write(&temporary, bytes)?;
    replace_file(&temporary, marker, &marker.with_extension("json.backup"))?;
    Ok(())
}

pub(super) fn replace_file(source: &Path, target: &Path, backup: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if target.exists() {
        fs::rename(target, backup)?;
    }
    match fs::rename(source, target) {
        Ok(()) => {
            if backup.exists() {
                fs::remove_file(backup)?;
            }
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = fs::rename(backup, target);
            }
            Err(error.into())
        }
    }
}
