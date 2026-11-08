//! Exhaustive self-update failures, preserving concrete causes for diagnostics.

use crate::self_update::{
    domain::artifact::ArtifactSelectionError,
    infrastructure::{
        ArchiveError, HttpError, artifact::ArtifactDownloadError, executable::ExecutableError,
        release::ReleaseError,
    },
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SelfUpdateError {
    #[error("release discovery failed: {0}")]
    ReleaseCheck(#[source] Box<SelfUpdateError>),
    #[error(transparent)]
    ArtifactSelection(#[from] ArtifactSelectionError),
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    ArtifactDownload(#[from] ArtifactDownloadError),
    #[error(transparent)]
    Executable(#[from] ExecutableError),
    #[error(transparent)]
    Release(#[from] ReleaseError),
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Version(#[from] semver::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}
