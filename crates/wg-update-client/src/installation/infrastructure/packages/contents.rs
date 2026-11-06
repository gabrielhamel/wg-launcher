//! Classify decoded patch files before installing replacements and applying deltas.

use super::error::Result;
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

pub(super) enum DeltaFormat {
    Vcdiff,
    Librsync,
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
            if let Some((target, format)) = delta_target(relative) {
                contents.deltas.push(DeltaFile {
                    source,
                    target,
                    format,
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

fn delta_target(path: &Path) -> Option<(PathBuf, DeltaFormat)> {
    let value = path.to_string_lossy();
    let extension = path.extension()?.to_str()?;
    let format = if extension.eq_ignore_ascii_case("xdiff") {
        DeltaFormat::Vcdiff
    } else if extension.eq_ignore_ascii_case("rdiff") {
        DeltaFormat::Librsync
    } else {
        return None;
    };
    let without_extension = value.rsplit_once('.')?.0;
    let (without_checksum, checksum) = without_extension.rsplit_once('.')?;
    if checksum.is_empty()
        || !checksum
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }
    let marker = ".pkg.";
    let position = without_checksum.rfind(marker)?;
    let version = &without_checksum[position + marker.len()..];
    if version.is_empty()
        || !version
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return None;
    }
    Some((PathBuf::from(&without_checksum[..position + 4]), format))
}
