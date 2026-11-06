use super::{
    error::{Result, StorageError as Error},
    paths::safe_relative_path,
};
use crate::catalog::Patch;
use std::{fs, path::Path};

pub(crate) fn remove_patch_volumes(cache: &Path, patch: &Patch) -> Result<u64> {
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

pub(crate) fn verify_cache(patches: &[&Patch], cache: &Path) -> Result<()> {
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
