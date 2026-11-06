use super::{
    error::{Result, StorageError as Error},
    storage::InstallationStorage,
};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

pub(crate) struct InstallationLock {
    file: File,
}

impl InstallationLock {
    pub(crate) fn acquire(storage: &InstallationStorage, install_root: &Path) -> Result<Self> {
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
