mod asset;
mod error;
mod metadata;
pub(crate) mod package_plan;
mod patch;
pub(crate) mod patch_chain;
mod resolved_release;

pub use asset::{Asset, WebSeed};
pub use error::ReleaseError;
pub use metadata::{ClientPart, ClientType, Executable, GameMetadata};
pub use package_plan::installation_assets;
pub(crate) use patch::PatchSource;
pub use patch::{Patch, PatchFile};
pub use resolved_release::ResolvedRelease;
