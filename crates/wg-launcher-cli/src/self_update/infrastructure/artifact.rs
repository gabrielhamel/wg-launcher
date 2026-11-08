//! Download and verify a selected launcher archive, then extract its executable.

use super::{
    archive,
    http::{client, fetch},
};
use crate::self_update::progress::{Observer, SelfUpdateEvent, SelfUpdateStage};
use crate::self_update::{
    Result,
    domain::{Release, artifact::ReleaseArtifact},
};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ArtifactDownloadError {
    #[error("invalid archive checksum")]
    InvalidChecksum,
    #[error("archive exceeds size limit")]
    TooLarge,
    #[error("archive checksum mismatch")]
    ChecksumMismatch,
}

pub(crate) async fn prepare_candidate(
    release: &Release,
    artifact: &ReleaseArtifact,
    directory: &Path,
    observer: &Observer<'_>,
) -> Result<PathBuf> {
    let client = client(Duration::from_secs(120))?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::FetchingChecksum))?;
    let checksum = fetch(&client, release.base.join(&artifact.checksum_name)?, 4096).await?;
    let checksum = std::str::from_utf8(&checksum)?
        .split_whitespace()
        .next()
        .unwrap_or("");
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ArtifactDownloadError::InvalidChecksum.into());
    }
    let archive = directory.join(&artifact.name);
    let mut file = std::fs::File::create(&archive)?;
    let mut response = client
        .get(release.base.join(&artifact.name)?)
        .send()
        .await?
        .error_for_status()?;
    let mut hasher = Sha256::new();
    let mut size = 0usize;
    observer(SelfUpdateEvent::Transfer {
        stage: SelfUpdateStage::DownloadingArchive,
        total: response.content_length(),
    })?;
    while let Some(chunk) = response.chunk().await? {
        size = size
            .checked_add(chunk.len())
            .ok_or(ArtifactDownloadError::TooLarge)?;
        if size > 128 * 1024 * 1024 {
            return Err(ArtifactDownloadError::TooLarge.into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        observer(SelfUpdateEvent::Advanced(chunk.len() as u64))?;
    }
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::VerifyingArchive))?;
    file.sync_all()?;
    drop(file);
    if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(checksum) {
        return Err(ArtifactDownloadError::ChecksumMismatch.into());
    }
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::OpeningArchive))?;
    archive::extract(&archive, directory, artifact.format, observer)
}
