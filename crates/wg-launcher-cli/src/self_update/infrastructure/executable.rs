use super::super::Result;
use semver::Version;
use std::{path::Path, time::Duration};

pub(crate) fn staging_directory() -> Result<tempfile::TempDir> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let parent = executable
        .parent()
        .ok_or("Cannot locate the launcher directory.")?;
    let work = tempfile::Builder::new()
        .prefix(".wg-launcher-self-update-")
        .tempdir_in(parent)
        .map_err(|error| {
            format!(
                "Cannot prepare the CLI update in {}: {error}. Check directory permissions.",
                parent.display()
            )
        })?;
    Ok(work)
}

pub(crate) async fn verify(binary: &Path, expected: &Version) -> Result<()> {
    // Check compatibility and the embedded version before touching the installed executable.
    let mut candidate = tokio::process::Command::new(binary);
    candidate
        .arg("--version")
        .env("WG_LAUNCHER_NO_UPDATE_CHECK", "1")
        .env("CLICOLOR", "0")
        .env("CLICOLOR_FORCE", "0")
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(5), candidate.output())
        .await
        .map_err(
            |_| "The downloaded CLI did not respond; the installed executable was preserved.",
        )??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = super::super::domain::candidate_version(&stdout);
    if !output.status.success() || version != Some(expected.to_string().as_str()) {
        return Err(
            "The downloaded CLI failed its version check; the installed executable was preserved."
                .into(),
        );
    }
    Ok(())
}

pub(crate) fn replace(binary: &Path) -> Result<()> {
    self_replace::self_replace(binary).map_err(|error| {
        format!("Cannot replace the CLI executable: {error}. Check directory permissions.")
    })?;
    Ok(())
}
