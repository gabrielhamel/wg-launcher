//! Launcher updates are independent of game workflows and installation state.

use super::domain::Release;
use super::infrastructure::{executable, release};
use super::presentation::{distribution, output, progress::SelfUpdateProgress};
use super::{CURRENT, Result};

pub(crate) async fn check() -> Option<Release> {
    if std::env::var_os("WG_LAUNCHER_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    // Startup must remain usable offline, including local-only game extraction.
    release::latest().await.ok()
}

pub(crate) fn notify_if_newer(release: Option<&Release>) {
    if let Some(release) = release.filter(|release| release.is_newer()) {
        output::print_cli_update(
            CURRENT,
            &release.version.to_string(),
            &distribution::update_instruction(true),
        );
    }
}

pub(crate) async fn print_version() {
    let latest = check().await;
    let update_available = latest.as_ref().is_some_and(Release::is_newer);
    let version = latest.as_ref().map(|release| release.version.to_string());
    output::print_cli_version(
        CURRENT,
        version.as_deref(),
        update_available,
        &distribution::update_instruction(false),
    );
    if let Some(reason) = distribution::unsupported_reason(false) {
        println!("{reason}");
    }
}

pub(crate) async fn run() -> Result<()> {
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
    let work = executable::staging_directory()?;
    let binary = release.download(work.path(), &progress).await?;
    progress.stage("Verifying the CLI executable")?;
    executable::verify(&binary, &release.version).await?;
    progress.stage("Replacing the CLI executable")?;
    executable::replace(&binary)?;
    progress.clear();
    output::print_cli_updated(&release.version.to_string());
    Ok(())
}
