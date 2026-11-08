//! Options, progress events, and reports for transport-volume consumers.

use serde::Serialize;

/// HTTP volume transfer policy. Defaults to three concurrent files and five attempts.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Maximum simultaneous file transfers; zero is treated as one.
    pub concurrency: usize,
    /// Maximum attempts per file, including the initial request; zero becomes one.
    /// Retries use a quadratic delay of 500 milliseconds times the attempt squared.
    pub attempts: usize,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            concurrency: 3,
            attempts: 5,
        }
    }
}

/// Progress for an individual transport volume.
///
/// Names identify assets across concurrent transfers. Retried attempts may restart
/// the event sequence. Byte counts describe the current attempt and include resumed
/// prefixes. Size-based cache hits do not verify file contents.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DownloadEvent {
    /// A network response is ready and a transfer attempt is starting.
    Started {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
        /// Existing byte prefix retained for this attempt; zero when starting over.
        resumed_at: u64,
    },
    /// Additional response bytes were written for the current attempt.
    Progress {
        /// Relative transport volume name.
        name: String,
        /// Bytes present so far, including the resumed prefix; not a per-event increment.
        downloaded: u64,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
    /// The volume has the expected size and was promoted to its final filename.
    Completed {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
    /// A final file already had the expected size; no transfer was needed.
    Skipped {
        /// Relative transport volume name.
        name: String,
        /// Expected total compressed volume size in bytes.
        size: u64,
    },
}

/// Totals for successful downloads and cache hits.
///
/// No report is returned if any selected file ultimately fails.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DownloadReport {
    /// Volumes completed or promoted from complete partial files.
    /// A promoted partial file counts here even when no network request was needed.
    pub downloaded_files: usize,
    /// Volumes whose final files already had the expected size.
    pub skipped_files: usize,
    /// New bytes retained by successful file attempts, excluding resumed prefixes
    /// and skipped volumes. This is not total network traffic across failed attempts.
    pub downloaded_bytes: u64,
}
