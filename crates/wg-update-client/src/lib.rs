#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

mod catalog;
mod error;
mod installation;

pub use catalog::{
    Asset, BuildSelection, BuildSummary, CatalogError, ClientPart, ClientType, Executable, Game,
    GameMetadata, InspectOptions, ListOptions, ListReport, ListWarning, PROTOCOL_VERSION, Patch,
    PatchFile, PatchManifest, Product, Region, ReleaseError, WebSeed, WgusClient, WgusError,
    inspect, installation_assets, list,
};
pub use error::{Error, Result};
pub use installation::{
    AssetDownloader, DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, DownloadAssetsEvent,
    DownloadAssetsOptions, DownloadAssetsOutcome, DownloadError, DownloadEvent, DownloadOptions,
    DownloadReport, ExtractError, ExtractEvent, ExtractOptions, ExtractOutcome, ExtractionEvent,
    ExtractionOptions, ExtractionReport, INSTALL_MARKER_FILE, InstallEvent, InstallOptions,
    InstallOutcome, InstallPlan, InstallationError, InstallationEvent, InstallationReport,
    InstalledBuild, PackageError, PackageExtractor, PipelineError, STAGING_DIRECTORY, StatusError,
    StatusOptions, StatusReport, StorageError, TransferError, UpdateOptions, UpdateOutcome,
    download, download_with_observer, extract, extract_with_observer, install,
    install_with_observer, status, update, update_with_observer,
};
