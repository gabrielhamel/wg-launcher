//! Install decoded replacements before applying source-dependent deltas.

use super::{
    contents::PatchContents,
    delta::apply_delta,
    error::{PackageError as Error, Result},
};
use crate::installation::{
    ExtractionEvent,
    infrastructure::filesystem::{patch_workspace::PatchWorkspace, replacement::replace_file},
};
use std::{path::Path, sync::Arc};

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
