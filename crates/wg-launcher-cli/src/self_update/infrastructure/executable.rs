use super::super::Result;
use semver::Version;
use std::{path::PathBuf, time::Duration};

/// Launcher extracted from an archive whose checksum has been verified.
/// The executable's compatibility and embedded version still need checking.
pub(crate) struct ExtractedCandidate {
    path: PathBuf,
}

impl ExtractedCandidate {
    pub(super) fn from_archive(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Executable that passed the expected-version check; only this value can replace the launcher.
pub(crate) struct VerifiedCandidate {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExecutableError {
    #[error("launcher directory unavailable")]
    MissingDirectory,
    #[error("cannot prepare update in {directory}")]
    Staging {
        directory: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("candidate did not respond")]
    Timeout,
    #[error("candidate failed version check")]
    VersionMismatch,
    #[error("cannot replace executable")]
    Replacement(#[source] std::io::Error),
}

pub(crate) fn staging_directory() -> Result<tempfile::TempDir> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let parent = executable
        .parent()
        .ok_or(ExecutableError::MissingDirectory)?;
    let work = tempfile::Builder::new()
        .prefix(".wg-launcher-self-update-")
        .tempdir_in(parent)
        .map_err(|source| ExecutableError::Staging {
            directory: parent.to_path_buf(),
            source,
        })?;
    Ok(work)
}

pub(crate) async fn verify(
    binary: ExtractedCandidate,
    expected: &Version,
) -> Result<VerifiedCandidate> {
    // Check compatibility and the embedded version before touching the installed executable.
    let mut candidate = tokio::process::Command::new(&binary.path);
    candidate
        .arg("--version")
        .env("WG_LAUNCHER_NO_UPDATE_CHECK", "1")
        .env("CLICOLOR", "0")
        .env("CLICOLOR_FORCE", "0")
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(5), candidate.output())
        .await
        .map_err(|_| ExecutableError::Timeout)??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = super::super::domain::candidate_version(&stdout);
    if !output.status.success() || version != Some(expected.to_string().as_str()) {
        return Err(ExecutableError::VersionMismatch.into());
    }
    Ok(VerifiedCandidate { path: binary.path })
}

pub(crate) fn replace(binary: VerifiedCandidate) -> Result<()> {
    self_replace::self_replace(&binary.path).map_err(ExecutableError::Replacement)?;
    Ok(())
}
