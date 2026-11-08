//! Stable, shared download and extraction bars for the terminal.

mod download;
mod extraction;
mod installation;

pub(crate) use installation::InstallationProgress;

pub(crate) use download::DownloadProgress;
pub(crate) use extraction::ExtractionProgress;
