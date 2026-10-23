mod catalog;
mod download;
mod error;
mod install;
mod launcher;
mod model;
mod service;

pub use catalog::{Product, Region};
pub use download::{AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport};
pub use error::{Error, Result};
pub use install::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, ExtractionEvent, ExtractionOptions,
    ExtractionReport, INSTALL_MARKER_FILE, InstallationEvent, InstallationReport, InstalledBuild,
    PackageExtractor, STAGING_DIRECTORY, installation_assets,
};
pub use launcher::{
    BuildSelection, BuildSummary, DownloadAssetsEvent, DownloadAssetsOptions,
    DownloadAssetsOutcome, ExtractEvent, ExtractOptions, ExtractOutcome, InspectOptions,
    InstallEvent, InstallOptions, InstallOutcome, InstallPlan, ListOptions, ListReport,
    ListWarning, StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, download, extract,
    inspect, install, list, status, update,
};
pub use model::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchManifest,
    WebSeed,
};
pub use service::{PROTOCOL_VERSION, WgusClient};
