//! Launcher updates are independent of game workflows and installation state.

mod archive;
mod distribution;
mod release;

use crate::progress::SelfUpdateProgress;
use std::{error::Error, time::Duration};

pub(super) use release::Release;
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const CURRENT: &str = env!("CARGO_PKG_VERSION");

pub(super) async fn check() -> Option<Release> {
    if std::env::var_os("WG_LAUNCHER_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    // Startup must remain usable offline, including local-only game extraction.
    release::latest().await.ok()
}

pub(super) fn notify_if_newer(release: Option<&Release>) {
    if let Some(release) = release.filter(|release| release.is_newer()) {
        super::output::print_cli_update(
            CURRENT,
            &release.version.to_string(),
            &distribution::update_instruction(true),
        );
    }
}

pub(super) async fn print_version() {
    let latest = check().await;
    let update_available = latest.as_ref().is_some_and(Release::is_newer);
    let version = latest.as_ref().map(|release| release.version.to_string());
    super::output::print_cli_version(
        CURRENT,
        version.as_deref(),
        update_available,
        &distribution::update_instruction(false),
    );
    if let Some(reason) = distribution::unsupported_reason(false) {
        println!("{reason}");
    }
}

pub(super) async fn run() -> Result<()> {
    let progress = SelfUpdateProgress::new()?;
    let release = release::latest().await;
    progress.clear();
    if let Some(reason) = distribution::unsupported_reason(true) {
        return Err(reason.into());
    }
    let release = release.map_err(|error| {
        format!("Cannot check the latest CLI release: {error}. Check your connection or release mirror and try again.")
    })?;
    if !release.is_newer() {
        println!(
            "wg-launcher is already up to date ({CURRENT}; latest: {}).",
            release.version
        );
        return Ok(());
    }
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
    let binary = release.download(work.path(), &progress).await?;
    progress.stage("Verifying the CLI executable")?;
    // Check compatibility and the embedded version before touching the installed executable.
    let mut candidate = tokio::process::Command::new(&binary);
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
    let mut lines = stdout.lines();
    let first = lines.next().unwrap_or_default();
    // Released CLIs use the original one-line format; newer builds use status rows.
    let version = if first == "wg-launcher" {
        lines
            .next()
            .and_then(|line| line.strip_prefix("Installed:"))
            .map(str::trim)
    } else {
        first
            .strip_prefix("wg-launcher ")
            .and_then(|line| line.split_whitespace().next())
    };
    if !output.status.success() || version != Some(release.version.to_string().as_str()) {
        return Err(
            "The downloaded CLI failed its version check; the installed executable was preserved."
                .into(),
        );
    }
    progress.stage("Replacing the CLI executable")?;
    self_replace::self_replace(&binary).map_err(|error| {
        format!("Cannot replace the CLI executable: {error}. Check directory permissions.")
    })?;
    progress.clear();
    super::output::print_cli_updated(&release.version.to_string());
    Ok(())
}
