//! Stable, shared download and extraction bars for the terminal.

mod download;
mod extraction;

pub(crate) use download::DownloadProgress;
pub(crate) use extraction::ExtractionProgress;
