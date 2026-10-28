use crate::{DownloadEvent, DownloadReport, PatchManifest};
use serde::{Deserialize, Serialize};

/// Progress through local patch extraction, delta application, and finalization.
///
/// Patch and file indices are one-based. File indices reset for each patch. Paths
/// are relative to the current archive or game tree, depending on the event.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ExtractionEvent {
    /// Required local package volumes are about to be checked.
    CheckingCache {
        /// Number of assets in the supplied manifest, before patch-chain filtering.
        assets: usize,
    },
    /// An ordered patch is about to be extracted.
    ExtractingPatch {
        /// Component being extracted.
        part: String,
        /// Source component version, or `?` if unspecified.
        version_from: String,
        /// Target component version.
        version_to: String,
        /// One-based patch or file index within its reported total.
        index: usize,
        /// Total patches in the chain, or non-directory files in the current archive.
        total: usize,
    },
    /// Progress through one file in the current transport archive.
    ExtractingFile {
        /// Relative archive entry or game file path.
        path: String,
        /// One-based patch or file index within its reported total.
        index: usize,
        /// Total patches in the chain, or non-directory files in the current archive.
        total: usize,
        /// Uncompressed bytes read for the current file, not a per-event increment.
        extracted: u64,
        /// Expected uncompressed file size in bytes.
        size: u64,
    },
    /// A delta is about to be applied to a game file.
    ApplyingDelta {
        /// Relative archive entry or game file path.
        path: String,
    },
    /// A replacement file from a patch is about to be installed.
    InstallingFile {
        /// Relative archive entry or game file path.
        path: String,
    },
    /// The installed-build record and temporary-work cleanup are about to be written.
    Finalizing,
    /// Successful finalization is about to remove the remaining download cache.
    CleaningCache,
    /// Extraction and requested cleanup completed successfully.
    Completed {
        /// Files counted in the final game tree; see [`ExtractionReport::installed_files`].
        files: usize,
        /// Delta files applied during this operation.
        deltas: usize,
        /// Whether finalization removed the download cache.
        cache_removed: bool,
    },
}

/// Results after successful extraction and finalization.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExtractionReport {
    /// Files counted recursively beneath the installation root before finalization.
    /// Includes pre-existing files and launcher state still present at that point;
    /// this is not the number of files newly extracted.
    pub installed_files: usize,
    /// Delta files successfully applied during this operation.
    pub applied_deltas: usize,
    /// Whether successful finalization requested and completed cache removal.
    pub cache_removed: bool,
}

/// Build identity serialized in [`INSTALL_MARKER_FILE`](crate::INSTALL_MARKER_FILE).
///
/// Created from a manifest only after extraction succeeds. Records the current
/// installed build, while the installation directory may retain its original version
/// name. Component versions select subsequent update chains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledBuild {
    /// Publisher application identifier.
    pub game_id: String,
    /// Publisher catalog region.
    pub region: String,
    /// Complete selected product, saved for every built-in or custom installation.
    /// This is one product, never the full input catalog.
    pub product: crate::Product,
    /// Publisher release channel.
    pub channel: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Selected normalized language identifier.
    pub language: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Successfully installed version of each selected component.
    pub component_versions: std::collections::BTreeMap<String, String>,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
}

impl From<&PatchManifest> for InstalledBuild {
    fn from(manifest: &PatchManifest) -> Self {
        Self {
            game_id: manifest.game_id.clone(),
            region: manifest.region.clone(),
            product: manifest.product.clone(),
            channel: manifest.channel.clone(),
            version: manifest.version.clone(),
            client_type: manifest.client_type.clone(),
            language: manifest.language.clone(),
            platform: manifest.platform.clone(),
            architecture: manifest.architecture.clone(),
            metadata_version: manifest.metadata_version.clone(),
            component_versions: manifest.component_versions.clone(),
        }
    }
}

/// Interleaved streaming installation progress.
///
/// Download and extraction notifications may arrive concurrently. Package release
/// is reported only after successful extraction and removal of that patch's volumes.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum InstallationEvent {
    /// An individual transport volume download event.
    Download(DownloadEvent),
    /// A local patch extraction or finalization event.
    Extraction(ExtractionEvent),
    /// Successfully extracted transport volumes were removed from disk.
    ReleasedPackages,
}

/// Combined transfer, extraction, and early package-release totals.
#[derive(Debug, Clone, Default, Serialize)]
pub struct InstallationReport {
    /// Transfer totals accumulated across patches.
    pub download: DownloadReport,
    /// Final extraction and cleanup results.
    pub extraction: ExtractionReport,
    /// Compressed package bytes removed after successful patches, before final
    /// cache cleanup. Zero when cache cleanup is disabled.
    pub released_package_bytes: u64,
}

/// Controls cache retention. The default removes downloaded packages after success.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionOptions {
    /// Remove the cache on successful finalization and release each patch's volumes
    /// after successful extraction. When `false`, retain
    /// packages; temporary `.wg-launcher/installing` work is still removed after success.
    pub clean_download_cache: bool,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            clean_download_cache: true,
        }
    }
}
