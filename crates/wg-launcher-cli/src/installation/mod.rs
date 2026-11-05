//! Local game installation commands and terminal progress.
mod application;
mod presentation;
pub(crate) use application::{run_download, run_extract, run_install, run_status, run_update};
pub(crate) use presentation::args::{DownloadArgs, ExtractArgs, InstallArgs, PathArgs, UpdateArgs};
