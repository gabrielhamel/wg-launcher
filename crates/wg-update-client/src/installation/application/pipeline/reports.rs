use crate::installation::DownloadReport;
use serde::Serialize;

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
