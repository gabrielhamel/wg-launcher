use super::{
    error::{Result, StorageError as Error},
    paths::safe_relative_path,
};
use crate::catalog::{Patch, PlannedPatch};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A planned patch bound to local package paths, without changing its published identity.
/// Paths are safe cache-relative names; file presence and contents are checked separately.
pub(crate) struct LocalPatch {
    pub(crate) patch: Patch,
    volumes: Vec<LocalVolume>,
}

struct LocalVolume {
    path: PathBuf,
    expected_bytes: u64,
}

impl LocalPatch {
    pub(crate) fn from_plan(planned: &PlannedPatch, cache: &Path) -> Result<Self> {
        // Each planned volume carries the declared size used for cache verification.
        let volumes = planned
            .volumes
            .iter()
            .map(|volume| {
                Ok(LocalVolume {
                    path: cache.join(safe_relative_path(&volume.name)?),
                    expected_bytes: volume.size,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            patch: planned.patch.clone(),
            volumes,
        })
    }

    pub(crate) fn volume_paths(&self) -> impl Iterator<Item = &Path> {
        self.volumes.iter().map(|volume| volume.path.as_path())
    }
}

pub(crate) fn remove_patch_volumes(cache: &Path, patch: &LocalPatch) -> Result<u64> {
    let mut removed = 0;
    for volume in &patch.volumes {
        let path = &volume.path;
        removed += fs::metadata(path)?.len();
        fs::remove_file(path)?;
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

pub(crate) fn verify_cache(patches: &[LocalPatch]) -> Result<()> {
    for volume in patches.iter().flat_map(|patch| &patch.volumes) {
        let path = &volume.path;
        let actual = fs::metadata(path)
            .map(|metadata| {
                if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        if actual != volume.expected_bytes {
            return Err(Error::IncompleteCache {
                path: path.to_path_buf(),
                expected: volume.expected_bytes,
                actual,
            });
        }
    }
    Ok(())
}
