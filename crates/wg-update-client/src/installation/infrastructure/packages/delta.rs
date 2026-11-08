use super::contents::{DeltaFile, DeltaFormat, PatchContents};
use super::delta_progress::DeltaWriter;
use super::error::{PackageError as Error, Result};
use crate::installation::ExtractionEvent;
use crate::installation::infrastructure::filesystem::{
    patch_workspace::PatchWorkspace, replacement::replace_file,
};
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
    sync::Arc,
};

pub(crate) fn apply_patch_directory(
    patch_dir: &Path,
    game: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<usize> {
    let contents = PatchContents::read(patch_dir)?;
    for replacement in contents.replacements {
        let target = game.join(&replacement.relative);
        observer(ExtractionEvent::InstallingFile {
            path: replacement.relative.to_string_lossy().into_owned(),
        });
        replace_file(
            &replacement.source,
            &target,
            &workspace.replacement_backup(),
        )?;
    }
    let mut applied_deltas = 0;
    for delta in contents.deltas {
        let target = game.join(&delta.target);
        // A killed replacement can leave the target absent between renames.
        // The preserved original remains authoritative until the patch checkpoint.
        if !target.is_file() && !workspace.original_source(&delta.target)?.is_file() {
            return Err(Error::MissingDeltaTarget(target));
        }
        observer(ExtractionEvent::PreparingDelta {
            path: delta.target.to_string_lossy().into_owned(),
        });
        let original = workspace.preserve_source(game, &delta.target)?;
        observer(ExtractionEvent::ApplyingDelta {
            path: delta.target.to_string_lossy().into_owned(),
        });
        apply_delta(&original, &target, &delta, workspace, |written| {
            observer(ExtractionEvent::DeltaProgress {
                path: delta.target.to_string_lossy().into_owned(),
                written,
            });
        })?;
        applied_deltas += 1;
    }
    Ok(applied_deltas)
}

fn apply_delta(
    source: &Path,
    target: &Path,
    delta: &DeltaFile,
    workspace: &PatchWorkspace,
    progress: impl FnMut(u64),
) -> Result<()> {
    let temporary = workspace.delta_output();
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let mut writer = DeltaWriter::new(File::create(&temporary)?, progress);
    match delta.format {
        DeltaFormat::Vcdiff => apply_vcdiff(source, &delta.source, &mut writer)?,
        DeltaFormat::Librsync => apply_rsync_delta(source, &delta.source, &mut writer)?,
    }
    writer.finish()?;
    drop(writer);
    if !fs::metadata(&temporary).is_ok_and(|metadata| metadata.len() > 0) {
        return Err(Error::EmptyDeltaOutput(temporary));
    }
    replace_file(&temporary, target, &workspace.replacement_backup())?;
    Ok(())
}

fn apply_vcdiff(source: &Path, delta: &Path, writer: &mut impl Write) -> Result<()> {
    let source_file = File::open(source)?;
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

fn apply_rsync_delta(source: &Path, delta: &Path, writer: &mut impl Write) -> Result<()> {
    let source_file = File::open(source)?;
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
