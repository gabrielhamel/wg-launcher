//! Resumable transport-volume downloads.

mod attempt;
mod downloader;
mod error;
mod local;

use crate::installation::application::transfer::{DownloadEvent, DownloadOptions, DownloadReport};
pub use downloader::VolumeDownloader;
