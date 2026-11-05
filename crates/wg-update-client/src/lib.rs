#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

mod application;
mod domain;
mod error;
mod infrastructure;

pub use application::pipeline::{
    ExtractionEvent, ExtractionOptions, ExtractionReport, InstallationEvent, InstallationReport,
    PackageExtractor,
};
pub use application::{
    BuildSelection, BuildSummary, DownloadAssetsEvent, DownloadAssetsOptions,
    DownloadAssetsOutcome, ExtractEvent, ExtractOptions, ExtractOutcome, InspectOptions,
    InstallEvent, InstallOptions, InstallOutcome, InstallPlan, ListOptions, ListReport,
    ListWarning, StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, download,
    download_with_observer, extract, extract_with_observer, inspect, install,
    install_with_observer, list, status, update, update_with_observer,
};
pub use domain::catalog::{Game, Product, Region};
pub use domain::release::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchManifest,
    WebSeed,
};
pub use domain::{installation::InstalledBuild, release::installation_assets};
pub use error::{Error, Result};
pub use infrastructure::installation_fs::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, INSTALL_MARKER_FILE, STAGING_DIRECTORY,
};
pub use infrastructure::transfer::{
    AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport,
};
pub use infrastructure::wgus::{PROTOCOL_VERSION, WgusClient};

pub use application::{
    download::DownloadError, extract::ExtractError, pipeline::PipelineError, status::StatusError,
};
pub use domain::{catalog::CatalogError, installation::InstallationError, release::ReleaseError};
pub use infrastructure::{
    installation_fs::StorageError, packages::PackageError, transfer::TransferError, wgus::WgusError,
};
