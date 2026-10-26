use crate::{Error, Result};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file() {
                output.push(entry.path());
            }
        }
    }
    output.sort();
    Ok(output)
}

pub(super) fn safe_relative_path(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::UnsafeAssetPath(name.to_owned()));
    }
    Ok(path.to_path_buf())
}

pub(super) fn safe_archive_path(name: &str) -> Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    let relative = safe_relative_path(&normalized)?;
    if relative.components().next().is_some_and(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".wg-launcher")
    }) {
        return Err(Error::UnsafeAssetPath(name.to_owned()));
    }
    Ok(relative)
}
