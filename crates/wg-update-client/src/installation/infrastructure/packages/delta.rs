use super::delta_progress::DeltaWriter;
use super::error::{PackageError as Error, Result};
use crate::installation::infrastructure::filesystem::{
    paths::walk_files, retry::delta_source, storage::replace_file,
};
use crate::installation::{ExtractionEvent, ExtractionReport};
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) fn apply_patch_directory(
    patch_dir: &Path,
    game: &Path,
    work: &Path,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    report: &mut ExtractionReport,
) -> Result<()> {
    let files = walk_files(patch_dir)?;
    for source in files.iter().filter(|path| delta_target(path).is_none()) {
        let relative = source.strip_prefix(patch_dir).map_err(|_| {
            crate::installation::StorageError::UnsafeAssetPath(source.display().to_string())
        })?;
        let target = game.join(relative);
        observer(ExtractionEvent::InstallingFile {
            path: relative.to_string_lossy().into_owned(),
        });
        replace_file(source, &target, &work.join("replacement-backup"))?;
    }
    for delta in files.iter().filter(|path| delta_target(path).is_some()) {
        let relative = delta.strip_prefix(patch_dir).map_err(|_| {
            crate::installation::StorageError::UnsafeAssetPath(delta.display().to_string())
        })?;
        let target_relative = delta_target(relative)
            .ok_or_else(|| Error::InvalidDeltaPath(relative.display().to_string()))?;
        let target = game.join(&target_relative);
        if !target.is_file() {
            return Err(Error::MissingDeltaTarget(target));
        }
        observer(ExtractionEvent::PreparingDelta {
            path: target_relative.to_string_lossy().into_owned(),
        });
        let original = delta_source(game, &target_relative, work)?;
        observer(ExtractionEvent::ApplyingDelta {
            path: target_relative.to_string_lossy().into_owned(),
        });
        apply_delta(&original, &target, delta, work, |written| {
            observer(ExtractionEvent::DeltaProgress {
                path: target_relative.to_string_lossy().into_owned(),
                written,
            });
        })?;
        report.applied_deltas += 1;
    }
    Ok(())
}

fn apply_delta(
    source: &Path,
    target: &Path,
    delta: &Path,
    work: &Path,
    progress: impl FnMut(u64),
) -> Result<()> {
    fs::create_dir_all(work)?;
    let temporary = work.join("delta-output");
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let mut writer = DeltaWriter::new(File::create(&temporary)?, progress);
    match delta.extension().and_then(OsStr::to_str) {
        Some(extension) if extension.eq_ignore_ascii_case("xdiff") => {
            apply_vcdiff(source, delta, &mut writer)?
        }
        Some(extension) if extension.eq_ignore_ascii_case("rdiff") => {
            apply_rsync_delta(source, delta, &mut writer)?
        }
        _ => return Err(Error::InvalidDeltaPath(delta.display().to_string())),
    }
    writer.finish()?;
    drop(writer);
    if !fs::metadata(&temporary).is_ok_and(|metadata| metadata.len() > 0) {
        return Err(Error::EmptyDeltaOutput(temporary));
    }
    replace_file(&temporary, target, &work.join("replacement-backup"))?;
    Ok(())
}

fn apply_vcdiff(target: &Path, delta: &Path, writer: &mut impl Write) -> Result<()> {
    let source_file = File::open(target)?;
    let source = unsafe { memmap2::MmapOptions::new().map(&source_file)? };
    let mut source = &source[..];
    let patch = BufReader::new(File::open(delta)?);

    oxidelta::compress::decoder::DeltaDecoder::new(patch)
        .decode_to(&mut source, writer)
        .map_err(|error| Error::DeltaDecode {
            format: "VCDIFF",
            path: delta.to_path_buf(),
            detail: error.to_string(),
        })?;
    Ok(())
}

fn apply_rsync_delta(target: &Path, delta: &Path, writer: &mut impl Write) -> Result<()> {
    let source_file = File::open(target)?;
    let delta_file = File::open(delta)?;
    let source = unsafe { memmap2::MmapOptions::new().map(&source_file)? };
    let patch = unsafe { memmap2::MmapOptions::new().map(&delta_file)? };
    fast_rsync::apply(&source, &patch, writer).map_err(|error| Error::DeltaDecode {
        format: "librsync",
        path: delta.to_path_buf(),
        detail: error.to_string(),
    })?;
    Ok(())
}

fn delta_target(path: &Path) -> Option<PathBuf> {
    let value = path.to_string_lossy();
    let extension = path.extension()?.to_str()?;
    if !extension.eq_ignore_ascii_case("xdiff") && !extension.eq_ignore_ascii_case("rdiff") {
        return None;
    }
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
    Some(PathBuf::from(&without_checksum[..position + 4]))
}
