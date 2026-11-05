//! Published products, release discovery, selection, and patch planning.

mod application;
mod domain;
mod infrastructure;

pub(crate) use application::resolve;
pub use application::{
    BuildSummary, InspectOptions, ListOptions, ListReport, ListWarning, inspect, list,
};
pub(crate) use domain::release::patch_chain::{assets_for_patches, ordered_patches};
pub use domain::release::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchManifest,
    ReleaseError, WebSeed, installation_assets,
};
pub use domain::{BuildSelection, CatalogError, Game, Product, Region};
pub use infrastructure::wgus::{PROTOCOL_VERSION, WgusClient, WgusError};
