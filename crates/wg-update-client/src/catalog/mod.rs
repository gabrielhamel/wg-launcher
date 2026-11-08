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
pub(crate) use domain::release::patch_chain::ordered_patches;
pub(crate) use domain::release::{
    ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchSource, ReleaseError,
    ResolvedRelease, TransportVolume, WebSeed, required_volumes,
};
pub(crate) use domain::{BuildIdentity, CatalogError};
pub use domain::{BuildSelection, Game, Product, Region};
pub(crate) use infrastructure::wgus::WgusError;
