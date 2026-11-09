//! Terminal commands for CLI version discovery and executable updates.

use super::{distribution, output, progress::SelfUpdateProgress};
use crate::self_update::{CURRENT, application, domain::Release};
use application::SelfUpdateOutcome;

pub(crate) fn notify_if_newer(release: Option<&Release>) {
    if let Some(release) = release.filter(|release| application::is_newer(release)) {
        output::print_cli_update(
            CURRENT,
            &release.version.to_string(),
            &distribution::update_instruction(application::executable_ownership(), true),
        );
    }
}

pub(crate) async fn print_version() {
    let ownership = application::executable_ownership();
    let latest = application::check_startup_release_notice().await;
    let update_available = latest.as_ref().is_some_and(application::is_newer);
    let version = latest.as_ref().map(|release| release.version.to_string());
    output::print_cli_version(
        CURRENT,
        version.as_deref(),
        update_available,
        &distribution::update_instruction(ownership, false),
    );
    if let Some(reason) = distribution::unsupported_reason(ownership, false) {
        println!("{reason}");
    }
}

pub(crate) async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let progress = SelfUpdateProgress::new();
    let outcome = application::update_executable(&|event| progress.observe(event))
        .await
        .map_err(|error| super::error::describe(&error))?;
    progress.clear();
    match outcome {
        SelfUpdateOutcome::Managed(manager) => {
            return Err(distribution::managed_reason(manager, true).into());
        }
        SelfUpdateOutcome::Current(version) => {
            println!("wg-launcher is already up to date ({CURRENT}; latest: {version}).")
        }
        SelfUpdateOutcome::Updated(version) => output::print_cli_updated(&version.to_string()),
    }
    Ok(())
}
