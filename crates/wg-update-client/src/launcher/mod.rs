//! High-level workflows, grouped by the operation a consumer requests.

mod discovery;
mod download;
mod extract;
mod install;
mod manifest;
mod update;

pub use discovery::{
    BuildSelection, BuildSummary, InspectOptions, ListOptions, ListReport, ListWarning, inspect,
    list,
};
pub use download::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, download,
    download_with_observer,
};
pub use extract::{ExtractEvent, ExtractOptions, ExtractOutcome, extract, extract_with_observer};
pub use install::{InstallEvent, InstallOptions, InstallOutcome, install, install_with_observer};
pub use manifest::InstallPlan;
pub use update::{
    StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, status, update, update_with_observer,
};
