use super::error::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Installation-relative directory containing transport volumes and the pinned manifest.
pub const DOWNLOAD_CACHE_DIRECTORY: &str = ".wg-launcher/download";
/// Pinned manifest filename inside [`DOWNLOAD_CACHE_DIRECTORY`].
pub const DOWNLOAD_MANIFEST_FILE: &str = "manifest.json";
/// Directory beneath an installation root used for temporary extraction and delta work.
pub const STAGING_DIRECTORY: &str = ".wg-launcher/installing";
/// Installed-build JSON path relative to an installation root.
pub const INSTALLED_STATE_FILE: &str = ".wg-launcher/state.json";
const INSTALL_LOCK_FILE: &str = ".wg-launcher/wg-launcher.lock";
/// The game root and all launcher-owned paths derived from that root.
#[derive(Debug, Clone)]
pub(crate) struct InstallationLayout {
    root: PathBuf,
    pub cache: PathBuf,
    pub staging: PathBuf,
    pub installed_state: PathBuf,
    pub lock: PathBuf,
}

impl InstallationLayout {
    /// Describes the installation layout without reading or creating files.
    pub(crate) fn for_root(install_root: &Path) -> Self {
        Self {
            root: install_root.to_path_buf(),
            cache: install_root.join(DOWNLOAD_CACHE_DIRECTORY),
            staging: install_root.join(STAGING_DIRECTORY),
            installed_state: install_root.join(INSTALLED_STATE_FILE),
            lock: install_root.join(INSTALL_LOCK_FILE),
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

pub(crate) fn prepare_installation(storage: &InstallationLayout) -> Result<()> {
    fs::create_dir_all(storage.root())?;
    fs::create_dir_all(&storage.staging)?;
    Ok(())
}
