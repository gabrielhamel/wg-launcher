use crate::installation::{DownloadEvent, DownloadReport};
use serde::Serialize;

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
    /// The original game file is being preserved for a resumable delta application.
    PreparingDelta {
        /// Relative game file path.
        path: String,
    },
    /// Progress reconstructing a game file from a delta.
    ///
    /// The output size is unknown in advance. Notifications are throttled, with
    /// an initial zero and a final byte count after successful output flushing.
    /// The final count does not mean the whole patch has been checkpointed.
    DeltaProgress {
        /// Relative game file path.
        path: String,
        /// Reconstructed output bytes, not a per-event increment.
        written: u64,
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
        /// Files counted before state writes and cleanup; see [`ExtractionReport::files_before_finalization`].
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
    /// this is not the number of files newly extracted. The serialized field uses
    /// the same `files_before_finalization` name.
    pub files_before_finalization: usize,
    /// Delta files successfully applied during this operation.
    pub applied_deltas: usize,
    /// Whether successful finalization requested and completed cache removal.
    pub cache_removed: bool,
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
