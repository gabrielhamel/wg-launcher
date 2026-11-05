mod asset;
mod error;
mod manifest;
mod metadata;
mod patch;
pub(crate) mod patch_chain;

pub use asset::{Asset, WebSeed};
pub use error::ReleaseError;
pub use manifest::PatchManifest;
pub use metadata::{ClientPart, ClientType, Executable, GameMetadata};
pub use patch::{Patch, PatchFile};
pub use patch_chain::installation_assets;
