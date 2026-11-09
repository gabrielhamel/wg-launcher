//! Fetch launcher artifacts and verify their SHA-256 checksums.

use super::http::{client, fetch};
use crate::self_update::progress::{Observer, SelfUpdateEvent};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ArtifactError {
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
    #[error("invalid archive checksum")]
    InvalidChecksum,
    #[error("archive exceeds size limit")]
    TooLarge,
    #[error("archive checksum mismatch")]
    ChecksumMismatch,
}

/// Downloaded archive together with the digest computed while writing it.
pub(crate) struct DownloadedArchive {
    path: PathBuf,
    digest: String,
}

impl DownloadedArchive {
    pub(crate) fn verify(self, checksum: &str) -> Result<PathBuf, ArtifactError> {
        if !self.digest.eq_ignore_ascii_case(checksum) {
            return Err(ArtifactError::ChecksumMismatch);
        }
        Ok(self.path)
    }
}

pub(crate) fn artifact_url(base: &Url, name: &str) -> Result<Url, ArtifactError> {
    Ok(base.join(name)?)
}

pub(crate) fn download_client() -> Result<Client, ArtifactError> {
    Ok(client(Duration::from_secs(120))?)
}

pub(crate) async fn fetch_checksum(client: &Client, url: Url) -> Result<String, ArtifactError> {
    let checksum = fetch(client, url, 4096).await?;
    let checksum = std::str::from_utf8(&checksum)?
        .split_whitespace()
        .next()
        .unwrap_or("");
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ArtifactError::InvalidChecksum);
    }
    Ok(checksum.to_owned())
}

pub(crate) async fn download_archive(
    client: &Client,
    url: Url,
    archive: &Path,
    observer: &Observer<'_>,
) -> Result<DownloadedArchive, ArtifactError> {
    let mut file = std::fs::File::create(archive)?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut hasher = Sha256::new();
    let mut size = 0usize;
    observer(SelfUpdateEvent::Transfer {
        stage: crate::self_update::progress::SelfUpdateStage::DownloadingArchive,
        total: response.content_length(),
    })?;
    while let Some(chunk) = response.chunk().await? {
        size = size
            .checked_add(chunk.len())
            .ok_or(ArtifactError::TooLarge)?;
        if size > 128 * 1024 * 1024 {
            return Err(ArtifactError::TooLarge);
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        observer(SelfUpdateEvent::Advanced(chunk.len() as u64))?;
    }
    // Flushing belongs to transfer completion; the application verifies the digest next.
    file.sync_all()?;
    Ok(DownloadedArchive {
        path: archive.to_owned(),
        digest: format!("{:x}", hasher.finalize()),
    })
}
