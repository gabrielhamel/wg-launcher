//! Local game installation commands and terminal progress.
mod presentation;
pub(crate) use presentation::args::{DownloadArgs, ExtractArgs, InstallArgs, PathArgs, UpdateArgs};
pub(crate) use presentation::commands::{
    run_download, run_extract, run_install, run_status, run_update,
};
