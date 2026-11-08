//! Select the launcher archive and checksum published for the running host.

use super::Release;
use crate::self_update::Result;

pub(crate) enum ArchiveFormat {
    Zip,
    TarGzip,
}

pub(crate) struct ReleaseArtifact {
    pub(crate) name: String,
    pub(crate) checksum_name: String,
    pub(crate) format: ArchiveFormat,
}

/// Host facts supplied by the application, rather than discovered by domain rules.
pub(crate) struct HostTarget<'a> {
    pub(crate) os: &'a str,
    pub(crate) architecture: &'a str,
    pub(crate) environment: &'a str,
}

impl ReleaseArtifact {
    pub(crate) fn for_host(release: &Release, host: &HostTarget<'_>) -> Result<Self> {
        let (target, format) = match (host.os, host.architecture) {
            ("windows", "x86_64") => ("x86_64-pc-windows-msvc", ArchiveFormat::Zip),
            ("macos", "aarch64") => ("aarch64-apple-darwin", ArchiveFormat::TarGzip),
            ("macos", "x86_64") => ("x86_64-apple-darwin", ArchiveFormat::TarGzip),
            ("linux", "x86_64") if host.environment == "gnu" => ("x86_64-unknown-linux-gnu", ArchiveFormat::TarGzip),
            _ => return Err("Self-update not supported for this platform; install a compatible CLI build manually.".into()),
        };
        let extension = match format {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::TarGzip => "tar.gz",
        };
        let name = format!("{}-{target}.{extension}", env!("CARGO_PKG_NAME"));
        let checksum_name = format!("{name}.sha256");
        if !release.artifacts.contains(&name) || !release.artifacts.contains(&checksum_name) {
            return Err("The latest release has no archive and checksum for this platform.".into());
        }
        Ok(Self {
            name,
            checksum_name,
            format,
        })
    }
}
