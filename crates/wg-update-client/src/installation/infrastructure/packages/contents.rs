//! Classify decoded patch files before installing replacements and applying deltas.

use super::{
    delta_filename::{DeltaFileName, DeltaFormat},
    error::Result,
};
use crate::installation::infrastructure::filesystem::{StorageError, paths::walk_files};
use std::path::{Path, PathBuf};

pub(super) struct PatchContents {
    pub(super) replacements: Vec<ReplacementFile>,
    pub(super) deltas: Vec<DeltaFile>,
}

pub(super) struct ReplacementFile {
    pub(super) source: PathBuf,
    pub(super) relative: PathBuf,
}

pub(super) struct DeltaFile {
    pub(super) source: PathBuf,
    pub(super) target: PathBuf,
    pub(super) format: DeltaFormat,
}

impl PatchContents {
    pub(super) fn read(directory: &Path) -> Result<Self> {
        let mut contents = Self {
            replacements: Vec::new(),
            deltas: Vec::new(),
        };
        for source in walk_files(directory)? {
            let relative = source
                .strip_prefix(directory)
                .map_err(|_| StorageError::UnsafeAssetPath(source.display().to_string()))?;
            if let Some(name) = DeltaFileName::parse(relative) {
                contents.deltas.push(DeltaFile {
                    source,
                    target: name.target,
                    format: name.format,
                });
            } else {
                let relative = relative.to_path_buf();
                contents
                    .replacements
                    .push(ReplacementFile { source, relative });
            }
        }
        Ok(contents)
    }
}
