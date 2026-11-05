//! Read cargo-dist's published manifest without GitHub API authentication or rate limits.

use super::archive;
use crate::self_update::presentation::progress::SelfUpdateProgress;
use crate::self_update::{Result, domain::Release};
use reqwest::{Client, Url};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

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

impl Release {
    pub(crate) async fn download(
        &self,
        directory: &Path,
        progress: &SelfUpdateProgress,
    ) -> Result<PathBuf> {
        let (target, extension) = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("windows", "x86_64") => ("x86_64-pc-windows-msvc", "zip"),
            ("macos", "aarch64") => ("aarch64-apple-darwin", "tar.gz"),
            ("macos", "x86_64") => ("x86_64-apple-darwin", "tar.gz"),
            ("linux", "x86_64") if cfg!(target_env = "gnu") => ("x86_64-unknown-linux-gnu", "tar.gz"),
            _ => return Err("Self-update not supported for this platform; install a compatible CLI build manually.".into()),
        };
        let name = format!("{PACKAGE}-{target}.{extension}");
        let checksum_name = format!("{name}.sha256");
        if !self.artifacts.contains(&name) || !self.artifacts.contains(&checksum_name) {
            return Err("The latest release has no archive and checksum for this platform.".into());
        }
        let client = client(Duration::from_secs(120))?;
        progress.stage("Fetching the CLI archive checksum")?;
        let checksum = fetch(&client, self.base.join(&checksum_name)?, 4096).await?;
        let checksum = std::str::from_utf8(&checksum)?
            .split_whitespace()
            .next()
            .unwrap_or("");
        if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(
                "The CLI archive checksum is invalid; the installed executable was preserved."
                    .into(),
            );
        }
        let archive = directory.join(&name);
        let mut file = std::fs::File::create(&archive)?;
        let mut response = client
            .get(self.base.join(&name)?)
            .send()
            .await?
            .error_for_status()?;
        let mut hasher = Sha256::new();
        let mut size = 0usize;
        progress.transfer("Downloading the CLI archive", response.content_length())?;
        while let Some(chunk) = response.chunk().await? {
            size = size
                .checked_add(chunk.len())
                .ok_or("CLI archive is too large.")?;
            if size > 128 * 1024 * 1024 {
                return Err(
                    "CLI archive is too large; the installed executable was preserved.".into(),
                );
            }
            hasher.update(&chunk);
            file.write_all(&chunk)?;
            progress.advance(chunk.len() as u64);
        }
        progress.stage("Verifying the CLI archive checksum")?;
        file.sync_all()?;
        drop(file);
        if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(checksum) {
            return Err(
                "CLI archive checksum mismatch; the installed executable was preserved.".into(),
            );
        }
        progress.stage("Opening the CLI archive")?;
        archive::extract(&archive, directory, extension == "zip", progress)
    }
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
        .ok_or("The release manifest does not contain this CLI.")?;
    let version = Version::parse(
        app.app_version
            .strip_prefix('v')
            .unwrap_or(&app.app_version),
    )?;
    if manifest.announcement_is_prerelease || !version.pre.is_empty() {
        return Err("The latest CLI release is not a stable version.".into());
    }
    let base = if mirror.is_some() {
        url.join("./")?
    } else {
        let mut base = Url::parse(REPOSITORY)?;
        base.path_segments_mut()
            .map_err(|_| "Invalid CLI repository URL.")?
            .extend(["releases", "download", &manifest.announcement_tag, ""]);
        base
    };
    Ok(Release {
        version,
        base,
        artifacts: app.artifacts,
    })
}

fn validate_url(url: &Url) -> Result<()> {
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() == "https" || (url.scheme() == "http" && local) {
        Ok(())
    } else {
        Err("CLI updates require HTTPS (HTTP is allowed only for loopback mirrors).".into())
    }
}

fn client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("wg-launcher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(2))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many CLI update redirects")
            } else if validate_url(attempt.url()).is_ok() {
                attempt.follow()
            } else {
                attempt.error("Unsafe CLI update redirect")
            }
        }))
        .build()?)
}

async fn fetch(client: &Client, url: Url, limit: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("CLI update metadata is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
