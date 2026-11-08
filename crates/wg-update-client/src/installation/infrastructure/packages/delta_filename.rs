//! Interpret publisher delta names without changing how unrecognized files are classified.

use std::path::{Path, PathBuf};

pub(super) enum DeltaFormat {
    Vcdiff,
    Librsync,
}

/// A recognized delta filename identifies a game-owned package and delta encoding.
/// Its version and checksum segments are syntax markers, not verified file identity.
pub(super) struct DeltaFileName {
    pub(super) target: PathBuf,
    pub(super) format: DeltaFormat,
}

impl DeltaFileName {
    pub(super) fn parse(path: &Path) -> Option<Self> {
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
        Some(Self {
            target: PathBuf::from(&without_checksum[..position + 4]),
            format,
        })
    }
}
