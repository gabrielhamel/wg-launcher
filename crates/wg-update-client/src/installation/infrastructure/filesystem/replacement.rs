use super::error::Result;
use std::{fs, path::Path};

pub(crate) fn replace_file(source: &Path, target: &Path, backup: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if target.exists() {
        if !fs::symlink_metadata(target)?.file_type().is_file() {
            return Err(std::io::Error::other(format!(
                "cannot replace {}: destination is not a regular file; move it aside and retry",
                target.display()
            ))
            .into());
        }
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
