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

impl ReleaseArtifact {
    pub(crate) fn for_host(release: &Release) -> Result<Self> {
        let (target, format) = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("windows", "x86_64") => ("x86_64-pc-windows-msvc", ArchiveFormat::Zip),
            ("macos", "aarch64") => ("aarch64-apple-darwin", ArchiveFormat::TarGzip),
            ("macos", "x86_64") => ("x86_64-apple-darwin", ArchiveFormat::TarGzip),
            ("linux", "x86_64") if cfg!(target_env = "gnu") => ("x86_64-unknown-linux-gnu", ArchiveFormat::TarGzip),
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
