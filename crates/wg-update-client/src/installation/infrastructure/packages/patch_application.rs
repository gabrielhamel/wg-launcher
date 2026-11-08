//! Install decoded replacements before applying source-dependent deltas.

use super::{
    archive::{decode_archive, decode_patch_contents},
    contents::PatchContents,
    delta::apply_delta,
    error::{PackageError as Error, Result},
};
use crate::installation::{
    ExtractionEvent,
    infrastructure::filesystem::{
        cache::LocalPatch, patch_workspace::PatchWorkspace, replacement::replace_file,
    },
};
use std::{fs, path::Path, sync::Arc};

/// Decode full content safely before replacing each live game file.
pub(crate) fn install_full_component(
    patch: &LocalPatch,
    game: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<()> {
    fs::create_dir_all(game)?;
    decode_archive(patch, &workspace.archive(), observer, |relative, output| {
        let target = game.join(relative);
        if output.is_dir() {
            fs::create_dir_all(target)?;
        } else {
            replace_file(output, &target, &workspace.replacement_backup())?;
        }
        Ok(())
    })
}

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

pub(crate) fn apply_patch_contents(
    patch: &LocalPatch,
    game: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<usize> {
    if patch.patch().provides_full_component() {
        install_full_component(patch, game, workspace, observer)?;
        return Ok(0);
    }

    let patch_dir = workspace.archive();
    decode_patch_contents(patch, workspace, observer)?;
    let applied_deltas = apply_patch_directory(&patch_dir, game, workspace, observer)?;
    workspace.clear_archive()?;
    Ok(applied_deltas)
}
