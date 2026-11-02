//! Launcher updates are independent of game workflows and installation state.

mod archive;
mod distribution;
mod release;

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

pub(super) fn warn_if_newer(release: Option<&Release>) {
    if let Some(release) = release.filter(|release| release.is_newer()) {
        eprintln!(
            "warning: wg-launcher {} is available (installed: {CURRENT}). {}",
            release.version,
            distribution::update_instruction(),
        );
    }
}

pub(super) async fn print_version() {
    let latest = check().await;
    let latest = latest
        .as_ref()
        .map(|release| release.version.to_string())
        .unwrap_or_else(|| "unavailable".into());
    println!("wg-launcher {CURRENT} (latest: {latest})");
    if let Some(reason) = distribution::unsupported_reason() {
        println!("{reason}");
    }
}

pub(super) async fn run() -> Result<()> {
    let release = release::latest().await;
    warn_if_newer(release.as_ref().ok());
    if let Some(reason) = distribution::unsupported_reason() {
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
    println!("Updating wg-launcher {CURRENT} to {}...", release.version);
    let binary = release.download(work.path()).await?;
    // Check compatibility and the embedded version before touching the installed executable.
    let mut candidate = tokio::process::Command::new(&binary);
    candidate
        .arg("--version")
        .env("WG_LAUNCHER_NO_UPDATE_CHECK", "1")
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(5), candidate.output())
        .await
        .map_err(
            |_| "The downloaded CLI did not respond; the installed executable was preserved.",
        )??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut words = stdout.split_whitespace();
    if !output.status.success()
        || words.next() != Some("wg-launcher")
        || words.next() != Some(release.version.to_string().as_str())
    {
        return Err(
            "The downloaded CLI failed its version check; the installed executable was preserved."
                .into(),
        );
    }
    self_replace::self_replace(&binary).map_err(|error| {
        format!("Cannot replace the CLI executable: {error}. Check directory permissions.")
    })?;
    println!("Updated wg-launcher to {}.", release.version);
    Ok(())
}
