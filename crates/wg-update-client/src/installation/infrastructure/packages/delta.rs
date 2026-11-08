//! Reconstruct and replace one game file using a VCDIFF or librsync delta.

use super::contents::{DeltaFile, DeltaFormat};
use super::delta_progress::DeltaWriter;
use super::error::{PackageError as Error, Result};
use crate::installation::infrastructure::filesystem::{
    patch_workspace::PatchWorkspace, replacement::replace_file,
};
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
};

pub(super) fn apply_delta(
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
