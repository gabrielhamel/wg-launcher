//! Installation identity, asset transfer, package application, and recovery.

mod application;
mod domain;
mod infrastructure;

pub use application::pipeline::{
    ExtractionEvent, ExtractionOptions, ExtractionReport, InstallationEvent, InstallationReport,
    PackageExtractor,
};
pub use application::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, ExtractEvent,
    ExtractOptions, ExtractOutcome, InstallEvent, InstallOptions, InstallOutcome, InstallPlan,
    StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, download, download_with_observer,
    extract, extract_with_observer, install, install_with_observer, status, update,
    update_with_observer,
};
pub use application::{
    download::DownloadError, extract::ExtractError, pipeline::PipelineError, status::StatusError,
};
pub use domain::{InstallationError, InstalledBuild};
pub use infrastructure::filesystem::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, INSTALL_MARKER_FILE, STAGING_DIRECTORY,
    StorageError,
};
pub use infrastructure::packages::PackageError;
pub use infrastructure::transfer::{
    AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport, TransferError,
};
