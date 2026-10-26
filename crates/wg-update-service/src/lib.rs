//! File-backed WGUS-compatible update service.
//!
//! [`FileService`] is the production server used by the `wg-update-service` binary.

mod server;

pub use server::{FileService, FileServiceOptions};
