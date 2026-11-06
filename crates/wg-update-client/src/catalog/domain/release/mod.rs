mod asset;
mod error;
mod metadata;
mod patch;
pub(crate) mod patch_chain;
mod resolved_release;

pub use asset::{Asset, WebSeed};
pub use error::ReleaseError;
pub use metadata::{ClientPart, ClientType, Executable, GameMetadata};
pub(crate) use patch::PatchSource;
pub use patch::{Patch, PatchFile};
pub use patch_chain::installation_assets;
pub use resolved_release::ResolvedRelease;
