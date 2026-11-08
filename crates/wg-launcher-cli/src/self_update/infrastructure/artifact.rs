//! Download and verify a selected launcher archive, then extract its executable.

use super::{
    archive,
    executable::ExtractedCandidate,
    http::{client, fetch},
};
use crate::self_update::domain::{Release, artifact::ReleaseArtifact};
use crate::self_update::progress::{Observer, SelfUpdateEvent, SelfUpdateStage};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, time::Duration};

#[derive(Debug, thiserror::Error)]
pub(crate) enum CandidatePreparationError {
    #[error(transparent)]
    Http(#[from] super::http::HttpError),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Archive(#[from] super::archive::ArchiveError),
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
) -> Result<ExtractedCandidate, CandidatePreparationError> {
    let client = client(Duration::from_secs(120))?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::FetchingChecksum))?;
    let checksum = fetch(&client, release.base.join(&artifact.checksum_name)?, 4096).await?;
    let checksum = std::str::from_utf8(&checksum)?
        .split_whitespace()
        .next()
        .unwrap_or("");
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CandidatePreparationError::InvalidChecksum);
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
            .ok_or(CandidatePreparationError::TooLarge)?;
        if size > 128 * 1024 * 1024 {
            return Err(CandidatePreparationError::TooLarge);
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        observer(SelfUpdateEvent::Advanced(chunk.len() as u64))?;
    }
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::VerifyingArchive))?;
    file.sync_all()?;
    drop(file);
    if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(checksum) {
        return Err(CandidatePreparationError::ChecksumMismatch);
    }
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::OpeningArchive))?;
    let binary = archive::extract(&archive, directory, artifact.format, observer)?;
    Ok(ExtractedCandidate::from_archive(binary))
}
