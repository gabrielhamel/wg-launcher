//! CLI executable updates, independent of game installation state.
mod application;
mod domain;
mod infrastructure;
mod presentation;

pub(crate) use application::{check, notify_if_newer, print_version, run};
pub(crate) const CURRENT: &str = env!("CARGO_PKG_VERSION");
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
