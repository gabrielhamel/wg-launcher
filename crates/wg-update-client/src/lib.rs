#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

mod catalog;
mod error;
mod installation;

pub use catalog::{
    BuildInfo, BuildSelection, BuildSummary, Game, InspectOptions, InspectReport, ListOptions,
    ListReport, ListWarning, PatchSummary, Product, Region, inspect, list,
};
pub use error::{Error, ErrorKind, Result};
pub use installation::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, DownloadEvent,
    DownloadOptions, DownloadReport, ExtractEvent, ExtractOptions, ExtractOutcome, ExtractionEvent,
    ExtractionReport, InstallEvent, InstallOptions, InstallOutcome, InstallPlan, InstallationEvent,
    InstallationReport, PackageSummary, StatusOptions, StatusReport, UpdateOptions, UpdateOutcome,
    download, download_with_observer, extract, extract_with_observer, install,
    install_with_observer, status, update, update_with_observer,
};
