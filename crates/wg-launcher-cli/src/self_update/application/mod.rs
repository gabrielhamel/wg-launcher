//! Launcher updates are independent of game workflows and terminal presentation.

mod error;
pub(crate) use error::SelfUpdateError;

use super::Result;
use super::domain::{
    Release,
    artifact::{HostTarget, ReleaseArtifact},
    ownership::{ExecutableOwnership, PackageManager},
};
use super::infrastructure::{artifact, distribution, executable, release};
use super::progress::{Observer, SelfUpdateEvent, SelfUpdateStage};
use semver::Version;

pub(crate) enum SelfUpdateOutcome {
    Managed(PackageManager),
    Current(Version),
    Updated(Version),
}

/// Resolve executable ownership once for an application interaction.
pub(crate) fn executable_ownership() -> ExecutableOwnership {
    distribution::ownership()
}

pub(crate) async fn check() -> Option<Release> {
    if std::env::var_os("WG_LAUNCHER_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    // Startup must remain usable offline, including local-only game extraction.
    release::latest().await.ok()
}

pub(crate) async fn run(observer: &Observer<'_>) -> Result<SelfUpdateOutcome> {
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::CheckingRelease))?;
    let release = release::latest().await;
    observer(SelfUpdateEvent::ReleaseChecked)?;
    if let ExecutableOwnership::Managed(manager) = executable_ownership() {
        return Ok(SelfUpdateOutcome::Managed(manager));
    }
    let release = release.map_err(|cause| SelfUpdateError::ReleaseCheck(Box::new(cause)))?;
    if !is_newer(&release) {
        return Ok(SelfUpdateOutcome::Current(release.version));
    }
    let work = executable::staging_directory()?;
    let host = HostTarget {
        os: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        environment: if cfg!(target_env = "gnu") { "gnu" } else { "" },
    };
    let artifact = ReleaseArtifact::for_host(&release, &host)?;
    let binary = artifact::prepare_candidate(&release, &artifact, work.path(), observer).await?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::VerifyingExecutable))?;
    let binary = executable::verify(binary, &release.version).await?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::ReplacingExecutable))?;
    executable::replace(binary)?;
    Ok(SelfUpdateOutcome::Updated(release.version))
}

/// Compare a release with the running executable's compiled version.
pub(crate) fn is_newer(release: &Release) -> bool {
    Version::parse(super::CURRENT).is_ok_and(|current| release.is_newer_than(&current))
}
