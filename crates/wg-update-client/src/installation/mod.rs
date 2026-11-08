//! Installation identity, asset transfer, package application, and recovery.

mod application;
mod domain;
mod infrastructure;

pub use application::pipeline::{
    ExtractionEvent, ExtractionReport, InstallationEvent, InstallationReport,
};
pub(crate) use application::pipeline::{ExtractionOptions, InstallationPipeline};
pub use application::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, ExtractEvent,
    ExtractOptions, ExtractOutcome, InstallEvent, InstallOptions, InstallOutcome, InstallPlan,
    PackageSummary, StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, UpdateTarget,
    UpdateTargetSource, download, download_with_observer, extract, extract_with_observer, install,
    install_with_observer, status, update, update_with_observer,
};
pub(crate) use application::{
    InstalledBuildError, download::DownloadError, extract::ExtractError, pipeline::PipelineError,
};
pub(crate) use domain::{InstallationError, InstalledBuild, PinnedManifest};
pub(crate) use infrastructure::filesystem::{DOWNLOAD_MANIFEST_FILE, StorageError};
pub(crate) use infrastructure::packages::PackageError;
pub(crate) use infrastructure::transfer::{AssetDownloader, TransferError};
pub use infrastructure::transfer::{DownloadEvent, DownloadOptions, DownloadReport};
