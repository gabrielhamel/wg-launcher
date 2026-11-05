//! High-level workflows, grouped by the operation a consumer requests.

mod discovery;
pub(crate) mod download;
pub(crate) mod extract;
mod install;
pub(crate) mod pipeline;
mod plan;
mod resolution;
pub(crate) mod status;
mod update;

pub use crate::domain::catalog::BuildSelection;
pub use discovery::{
    BuildSummary, InspectOptions, ListOptions, ListReport, ListWarning, inspect, list,
};
pub use download::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, download,
    download_with_observer,
};
pub use extract::{ExtractEvent, ExtractOptions, ExtractOutcome, extract, extract_with_observer};
pub use install::{InstallEvent, InstallOptions, InstallOutcome, install, install_with_observer};
pub use plan::InstallPlan;
pub use status::{StatusOptions, StatusReport, status};
pub use update::{UpdateOptions, UpdateOutcome, update, update_with_observer};
