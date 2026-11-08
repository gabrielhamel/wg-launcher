//! Filesystem mechanics for successful installation cleanup and reporting.

use super::{InstallationLayout, error::Result, paths::walk_files};
use std::fs;

/// Count the whole tree before state writes and cleanup, including existing game
/// files and launcher metadata; it does not measure files installed by this operation.
pub(crate) fn count_files_before_finalization(storage: &InstallationLayout) -> Result<usize> {
    Ok(walk_files(storage.root())?.len())
}

pub(crate) fn remove_staging(storage: &InstallationLayout) -> Result<()> {
    if storage.staging.exists() {
        fs::remove_dir_all(&storage.staging)?;
    }
    Ok(())
}

pub(crate) fn remove_download_cache(storage: &InstallationLayout) -> Result<()> {
    if storage.cache.exists() {
        fs::remove_dir_all(&storage.cache)?;
    }
    Ok(())
}
