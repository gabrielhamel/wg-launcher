use crate::catalog::ResolvedRelease;
use serde::Serialize;
use std::path::PathBuf;

/// Identity of a selected or installed game build, independent of launcher storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildInfo {
    /// Normalized game identifier.
    pub game: String,
    /// Human-readable product name.
    pub name: String,
    /// Publisher application identifier.
    pub game_id: String,
    /// Publisher catalog region.
    pub region: String,
    /// Publisher release channel.
    pub channel: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher client variant identifier.
    pub client_type: String,
    /// Selected normalized language identifier.
    pub language: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, when published.
    pub architecture: Option<String>,
}

impl BuildInfo {
    pub(crate) fn from_release(release: &ResolvedRelease) -> Self {
        Self {
            game: release.product.game(),
            name: release.product.name.clone(),
            game_id: release.game_id.clone(),
            region: release.region.clone(),
            channel: release.channel.clone(),
            version: release.version.clone(),
            client_type: release.client_type.clone(),
            language: release.language.clone(),
            platform: release.platform.clone(),
            architecture: release.architecture.clone(),
        }
    }
}

/// One published component patch, without transport URLs or archive internals.
#[derive(Debug, Clone, Serialize)]
pub struct PatchSummary {
    /// Component identifier.
    pub part: String,
    /// Source component version; `0` denotes a full-install patch.
    pub version_from: Option<String>,
    /// Target component version.
    pub version_to: String,
    /// Number of transport package volumes in this patch.
    pub package_count: usize,
    /// Total compressed package bytes in this patch.
    pub compressed_bytes: u64,
}

/// Read-only information about a live build, returned by [`crate::inspect`].
#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    /// Selected build identity.
    pub build: BuildInfo,
    /// Normalized languages advertised for this build.
    pub available_languages: Vec<String>,
    /// Selected component identifiers, usable as download filters.
    pub parts: Vec<String>,
    /// Published component patch summaries in publisher order.
    pub patches: Vec<PatchSummary>,
    /// Number of published transport package volumes.
    pub package_count: usize,
    /// Total compressed bytes across the published volumes, before cache reuse.
    pub compressed_bytes: u64,
    /// Suggested relative installation directory; no directory is created.
    pub default_output_directory: PathBuf,
}

impl InspectReport {
    pub(crate) fn from_release(release: &ResolvedRelease) -> Self {
        Self {
            build: BuildInfo::from_release(release),
            available_languages: release.available_languages.clone(),
            parts: release.parts.clone(),
            patches: release
                .patches
                .iter()
                .map(|patch| PatchSummary {
                    part: patch.part.clone(),
                    version_from: patch.version_from.clone(),
                    version_to: patch.version_to.clone(),
                    package_count: patch.files.len(),
                    compressed_bytes: patch.files.iter().map(|file| file.size).sum(),
                })
                .collect(),
            package_count: release.assets.len(),
            compressed_bytes: release.download_size(),
            default_output_directory: BuildInfo::from_release(release).default_output_directory(),
        }
    }
}
