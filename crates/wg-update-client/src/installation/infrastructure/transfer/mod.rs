//! Resumable transport-volume downloads.

mod attempt;
mod contracts;
mod downloader;
mod error;
mod local;

pub use contracts::{DownloadEvent, DownloadOptions, DownloadReport};
pub use downloader::VolumeDownloader;
pub use error::TransferError;
