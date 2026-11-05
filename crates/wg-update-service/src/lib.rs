//! File-backed WGUS-compatible update service.
//!
//! [`FileService`] is the production server used by the `wg-update-service` binary.

mod release_serving;

pub use release_serving::{FileService, FileServiceOptions};

/// Runs the update-service command-line interface.
pub use release_serving::run;
