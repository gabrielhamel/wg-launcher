//! Catalog commands and build-selection presentation.
mod infrastructure;
mod presentation;
pub(crate) use presentation::args::{BuildArgs, InspectArgs, ListArgs};
pub(crate) use presentation::commands::{run_inspect, run_list};
