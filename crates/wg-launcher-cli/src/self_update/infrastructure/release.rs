//! Read cargo-dist's published manifest without GitHub API authentication or rate limits.

use super::http::{client, fetch, validate_url};
use crate::self_update::{Result, domain::Release};
use reqwest::Url;
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleaseError {
    #[error("CLI absent from release manifest")]
    MissingCli,
    #[error("latest release is unstable")]
    Unstable,
    #[error("invalid repository URL")]
    InvalidRepository,
}

const PACKAGE: &str = env!("CARGO_PKG_NAME");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Deserialize)]
struct Manifest {
    announcement_tag: String,
    #[serde(default)]
    announcement_is_prerelease: bool,
    releases: Vec<App>,
}

#[derive(Deserialize)]
struct App {
    app_name: String,
    app_version: String,
    artifacts: Vec<String>,
}

pub(crate) async fn latest() -> Result<Release> {
    let mirror = std::env::var("WG_LAUNCHER_UPDATE_MANIFEST").ok();
    let url = Url::parse(mirror.as_deref().unwrap_or(concat!(
        env!("CARGO_PKG_REPOSITORY"),
        "/releases/latest/download/dist-manifest.json"
    )))?;
    validate_url(&url)?;
    let client = client(Duration::from_secs(2))?;
    let bytes = fetch(&client, url.clone(), 1024 * 1024).await?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    let app = manifest
        .releases
        .into_iter()
        .find(|app| app.app_name == PACKAGE)
        .ok_or(ReleaseError::MissingCli)?;
    let version = Version::parse(
        app.app_version
            .strip_prefix('v')
            .unwrap_or(&app.app_version),
    )?;
    if manifest.announcement_is_prerelease || !version.pre.is_empty() {
        return Err(ReleaseError::Unstable.into());
    }
    let base = if mirror.is_some() {
        url.join("./")?
    } else {
        let mut base = Url::parse(REPOSITORY)?;
        base.path_segments_mut()
            .map_err(|_| ReleaseError::InvalidRepository)?
            .extend(["releases", "download", &manifest.announcement_tag, ""]);
        base
    };
    Ok(Release {
        version,
        base,
        artifacts: app.artifacts,
    })
}
