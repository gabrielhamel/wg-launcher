//! Prepare the selected launcher for executable verification.

use crate::self_update::{
    domain::{Release, artifact::ReleaseArtifact},
    infrastructure::{archive, artifact, executable::ExtractedCandidate},
    progress::{Observer, SelfUpdateEvent, SelfUpdateStage},
};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CandidatePreparationError {
    #[error(transparent)]
    Artifact(#[from] artifact::ArtifactError),
    #[error(transparent)]
    Archive(#[from] archive::ArchiveError),
    #[error(transparent)]
    Progress(#[from] std::io::Error),
}

pub(super) async fn prepare_candidate(
    release: &Release,
    selected: &ReleaseArtifact,
    directory: &Path,
    observer: &Observer<'_>,
) -> Result<ExtractedCandidate, CandidatePreparationError> {
    let client = artifact::download_client()?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::FetchingChecksum))?;
    let checksum = artifact::fetch_checksum(
        &client,
        artifact::artifact_url(&release.base, &selected.checksum_name)?,
    )
    .await?;
    let downloaded = artifact::download_archive(
        &client,
        artifact::artifact_url(&release.base, &selected.name)?,
        &directory.join(&selected.name),
        observer,
    )
    .await?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::VerifyingArchive))?;
    let verified = downloaded.verify(&checksum)?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::OpeningArchive))?;
    let binary = archive::extract(&verified, directory, selected.format, observer)?;
    Ok(ExtractedCandidate::from_archive(binary))
}
