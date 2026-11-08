//! Shared observer contract for package execution and installation consumers.

use crate::installation::DownloadEvent;
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
        /// Files counted before state writes and cleanup; see [`crate::ExtractionReport::files_before_finalization`].
        files: usize,
        /// Delta files applied during this operation.
        deltas: usize,
        /// Whether finalization removed the download cache.
        cache_removed: bool,
    },
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
