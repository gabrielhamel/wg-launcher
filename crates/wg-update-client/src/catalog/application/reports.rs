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
        Self::from_identity(&release.product, &release.identity)
    }

    pub(crate) fn from_identity(
        product: &crate::catalog::Product,
        identity: &crate::catalog::BuildIdentity,
    ) -> Self {
        Self {
            game: product.game(),
            name: product.name.clone(),
            game_id: identity.application_id.clone(),
            region: identity.region.clone(),
            channel: identity.channel.clone(),
            version: identity.version.clone(),
            client_type: identity.client_type.clone(),
            language: identity.language.clone(),
            platform: identity.platform.clone(),
            architecture: identity.architecture.clone(),
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
            parts: release.component_ids.clone(),
            patches: release
                .patches
                .iter()
                .map(|patch| PatchSummary {
                    part: patch.component_id.clone(),
                    version_from: patch.source.version().map(str::to_owned),
                    version_to: patch.version_to.clone(),
                    package_count: patch.volumes.len(),
                    compressed_bytes: patch.volumes.iter().map(|volume| volume.size).sum(),
                })
                .collect(),
            package_count: release.volumes.len(),
            compressed_bytes: release.published_volume_bytes(),
            default_output_directory: BuildInfo::from_release(release).default_output_directory(),
        }
    }
}
