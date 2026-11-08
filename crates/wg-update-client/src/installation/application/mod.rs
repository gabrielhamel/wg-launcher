//! Local installation workflows and their shared execution pipeline.

mod access;
pub(crate) mod download;
pub(crate) mod extract;
mod install;
mod installed;
pub(crate) mod pipeline;
mod plan;
pub(crate) mod progress;
mod resolution;
pub(crate) mod status;
mod status_report;
pub(crate) mod transfer;
mod update;
mod update_target;

use crate::catalog::BuildSelection;
pub use download::{
    DownloadAssetsEvent, DownloadAssetsOptions, DownloadAssetsOutcome, download,
    download_with_observer,
};
pub use extract::{ExtractEvent, ExtractOptions, ExtractOutcome, extract, extract_with_observer};
pub use install::{InstallEvent, InstallOptions, InstallOutcome, install, install_with_observer};
pub(crate) use installed::InstalledBuildError;
pub use plan::{InstallPlan, PackageSummary};
pub use status::{StatusOptions, StatusReport, status};
pub use update::{UpdateOptions, UpdateOutcome, update, update_with_observer};
pub use update_target::{UpdateTarget, UpdateTargetSource};
