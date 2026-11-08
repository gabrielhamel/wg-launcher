//! Published products, release discovery, selection, and patch planning.

mod application;
mod domain;
mod infrastructure;

pub use application::{
    BuildInfo, BuildSummary, InspectOptions, InspectReport, ListOptions, ListReport, ListWarning,
    PatchSummary, inspect, list,
};
pub(crate) use application::{resolve, resolve_update_release};
pub(crate) use domain::release::package_plan::{PlannedPatch, planned_patches};
pub(crate) use domain::release::{
    ClientPart, ClientType, ComponentVersions, Executable, GameMetadata, Patch, PatchSource,
    PatchVolume, ReleaseError, ResolvedRelease, TransportVolume, WebSeed, required_volumes,
};
pub(crate) use domain::{BuildIdentity, CatalogError};
pub use domain::{BuildSelection, Game, Product, Region};
pub(crate) use infrastructure::wgus::WgusError;
