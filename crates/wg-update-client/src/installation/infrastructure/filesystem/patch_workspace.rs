//! Temporary patch files and original sources retained until a checkpoint is saved.

use super::{error::Result, paths::safe_relative_path};
use crate::catalog::PatchIdentity;
use serde::{Serialize, Serializer};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Recovery files for the active patch. Failure leaves the workspace intact.
pub(crate) struct PatchWorkspace {
    active: PathBuf,
}

/// Disk encoding of a domain identity; keep the existing recovery record format.
struct PatchIdentityRecord<'a>(PatchIdentity<'a>);

impl Serialize for PatchIdentityRecord<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        // Keep the existing patch.json array representation byte-for-byte.
        (self.0.component, self.0.source, self.0.target).serialize(serializer)
    }
}

impl PatchWorkspace {
    pub(crate) fn prepare(identity: PatchIdentity<'_>, staging: &Path) -> Result<Self> {
        let active = staging.join("active");
        let identity = serde_json::to_vec(&PatchIdentityRecord(identity))?;
        if fs::read(active.join("patch.json")).ok().as_deref() != Some(identity.as_slice()) {
            if active.exists() {
                fs::remove_dir_all(&active)?;
            }
            fs::create_dir_all(&active)?;
            fs::write(active.join("patch.json"), identity)?;
        }
        let workspace = Self { active };
        workspace.clear_archive()?;
        fs::create_dir_all(workspace.archive())?;
        Ok(workspace)
    }

    pub(crate) fn archive(&self) -> PathBuf {
        self.active.join("archive")
    }

    pub(crate) fn replacement_backup(&self) -> PathBuf {
        self.active.join("replacement-backup")
    }

    pub(crate) fn delta_output(&self) -> PathBuf {
        self.active.join("delta-output")
    }

    pub(crate) fn original_source(&self, relative: &Path) -> Result<PathBuf> {
        Ok(self
            .active
            .join("sources")
            .join(safe_relative_path(&relative.to_string_lossy())?))
    }

    pub(crate) fn preserve_source(&self, game: &Path, relative: &Path) -> Result<PathBuf> {
        let original = self.original_source(relative)?;
        if !original.exists() {
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent)?;
            }
            let temporary = original.with_extension("source.part");
            fs::copy(game.join(relative), &temporary)?;
            fs::rename(temporary, &original)?;
        }
        Ok(original)
    }

    pub(crate) fn clear_archive(&self) -> Result<()> {
        let archive = self.archive();
        if archive.exists() {
            fs::remove_dir_all(archive)?;
        }
        Ok(())
    }

    /// Call only after persisting the patch checkpoint, so failures retain sources.
    pub(crate) fn release(self) -> Result<()> {
        if self.active.exists() {
            fs::remove_dir_all(self.active)?;
        }
        Ok(())
    }
}
