//! Inspect cached volumes and prepare resumable files, without making HTTP requests.

use super::error::{Result, TransferError as Error};
use crate::catalog::TransportVolume;
use crate::installation::infrastructure::filesystem::paths::safe_relative_path;
use std::path::{Path, PathBuf};
use tokio::fs;

pub(super) enum DownloadPreparation {
    ReuseComplete,
    PromotePartial(VolumePaths),
    TransferFrom { paths: VolumePaths, offset: u64 },
}

pub(super) struct VolumePaths {
    pub(super) target: PathBuf,
    pub(super) partial: PathBuf,
}

impl VolumePaths {
    pub(super) async fn promote(self) -> Result<()> {
        fs::rename(self.partial, self.target).await?;
        Ok(())
    }

    pub(super) async fn verify_and_promote(self, expected: u64) -> Result<()> {
        let actual = fs::metadata(&self.partial).await?.len();
        if actual != expected {
            return Err(Error::SizeMismatch {
                path: self.partial,
                expected,
                actual,
            });
        }
        self.promote().await
    }
}

pub(super) async fn prepare(asset: &TransportVolume, output: &Path) -> Result<DownloadPreparation> {
    let target = output.join(safe_relative_path(&asset.name)?);
    if let Ok(metadata) = fs::metadata(&target).await {
        if metadata.len() == asset.size {
            return Ok(DownloadPreparation::ReuseComplete);
        }
        fs::remove_file(&target).await?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }
    let partial = target.with_extension(format!(
        "{}.part",
        target
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
    ));
    let mut offset = fs::metadata(&partial)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);
    if offset > asset.size {
        fs::remove_file(&partial).await?;
        offset = 0;
    }
    let paths = VolumePaths { target, partial };
    if offset == asset.size {
        Ok(DownloadPreparation::PromotePartial(paths))
    } else {
        Ok(DownloadPreparation::TransferFrom { paths, offset })
    }
}
