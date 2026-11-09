//! Workflow failures compose operation-specific adapter errors.

use super::candidate_preparation::CandidatePreparationError;
use crate::self_update::{
    domain::artifact::ArtifactSelectionError,
    infrastructure::{executable::ExecutableError, release::ReleaseDiscoveryError},
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SelfUpdateError {
    #[error("release discovery failed: {0}")]
    ReleaseCheck(#[from] ReleaseDiscoveryError),
    #[error(transparent)]
    ArtifactSelection(#[from] ArtifactSelectionError),
    #[error(transparent)]
    CandidatePreparation(#[from] CandidatePreparationError),
    #[error(transparent)]
    Executable(#[from] ExecutableError),
    #[error(transparent)]
    Progress(#[from] std::io::Error),
}
