//! Launcher updates are independent of game workflows and terminal presentation.

use super::Result;
use super::domain::{Release, artifact::ReleaseArtifact, ownership::PackageManager};
use super::infrastructure::{artifact, distribution, executable, release};
use semver::Version;

/// A step in checking and replacing the launcher executable.
#[derive(Clone, Copy)]
pub(crate) enum SelfUpdateStage {
    CheckingRelease,
    FetchingChecksum,
    DownloadingArchive,
    VerifyingArchive,
    OpeningArchive,
    ExtractingExecutable,
    VerifyingExecutable,
    ReplacingExecutable,
}

/// Progress facts; terminal messages and styling belong to presentation.
pub(crate) enum SelfUpdateEvent {
    Stage(SelfUpdateStage),
    ReleaseChecked,
    Transfer {
        stage: SelfUpdateStage,
        total: Option<u64>,
    },
    Advanced(u64),
}

pub(crate) type Observer<'a> = dyn Fn(SelfUpdateEvent) -> Result<()> + 'a;

pub(crate) enum SelfUpdateOutcome {
    Managed(PackageManager),
    Current(Version),
    Updated(Version),
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
    if let Some(manager) = distribution::manager() {
        return Ok(SelfUpdateOutcome::Managed(manager));
    }
    let release = release.map_err(|error| {
        format!("Cannot check the latest CLI release: {error}. Check your connection or release mirror and try again.")
    })?;
    if !release.is_newer() {
        return Ok(SelfUpdateOutcome::Current(release.version));
    }
    let work = executable::staging_directory()?;
    let artifact = ReleaseArtifact::for_host(&release)?;
    let binary = artifact::download(&release, &artifact, work.path(), observer).await?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::VerifyingExecutable))?;
    executable::verify(&binary, &release.version).await?;
    observer(SelfUpdateEvent::Stage(SelfUpdateStage::ReplacingExecutable))?;
    executable::replace(&binary)?;
    Ok(SelfUpdateOutcome::Updated(release.version))
}
