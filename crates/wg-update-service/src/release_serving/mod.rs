//! Serving published releases through the WGUS-compatible HTTP boundary.
mod application;
mod domain;
mod infrastructure;
mod presentation;

pub use application::{FileService, FileServiceOptions};
pub use presentation::run;
