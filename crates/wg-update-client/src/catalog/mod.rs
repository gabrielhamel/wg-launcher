//! Published products, release discovery, selection, and patch planning.

mod application;
mod domain;
mod infrastructure;

pub(crate) use application::resolve;
pub use application::{
    BuildInfo, BuildSummary, InspectOptions, InspectReport, ListOptions, ListReport, ListWarning,
    PatchSummary, inspect, list,
};
pub(crate) use domain::CatalogError;
pub(crate) use domain::release::patch_chain::{assets_for_patches, ordered_patches};
pub(crate) use domain::release::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchManifest,
    ReleaseError, WebSeed, installation_assets,
};
pub use domain::{BuildSelection, Game, Product, Region};
pub(crate) use infrastructure::wgus::{WgusClient, WgusError};
