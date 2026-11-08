//! Select release descriptions and packages for incoming service requests.

use super::super::{domain::api_route, infrastructure::filesystem};
use std::path::{Path, PathBuf};

/// Content selected by release-serving rules, independent of HTTP representation.
pub(crate) enum ReleaseContent {
    Metadata(Vec<u8>),
    PatchChain(Vec<u8>),
    File { path: PathBuf, kind: ContentKind },
    Missing,
}

pub(crate) enum ContentKind {
    Xml,
    Json,
    Binary,
}

pub(super) struct ReleaseRoutes {
    metadata_route: String,
    patch_chain_route: String,
    metadata: Vec<u8>,
    patch_chain: Vec<u8>,
    packages: PathBuf,
}

impl ReleaseRoutes {
    pub(super) fn new(
        prefix: &str,
        metadata: Vec<u8>,
        patch_chain: Vec<u8>,
        packages: PathBuf,
    ) -> Self {
        Self {
            metadata_route: api_route(prefix, "metadata"),
            patch_chain_route: api_route(prefix, "patches_chain"),
            metadata,
            patch_chain,
            packages,
        }
    }

    pub(super) fn resolve(&self, request_path: &str) -> ReleaseContent {
        if request_path == self.metadata_route {
            return ReleaseContent::Metadata(self.metadata.clone());
        }
        if request_path == self.patch_chain_route {
            return ReleaseContent::PatchChain(self.patch_chain.clone());
        }
        file_resource(&self.packages, request_path)
    }
}

/// Select a release file and its content kind after safe filesystem lookup.
fn file_resource(packages: &Path, request_path: &str) -> ReleaseContent {
    let relative = request_path.trim_start_matches('/');
    let relative = relative.strip_prefix("packages/").unwrap_or(relative);
    let Some(file) = filesystem::resolve_package(packages, Path::new(relative)) else {
        return ReleaseContent::Missing;
    };
    let kind = if relative.ends_with(".xml") {
        ContentKind::Xml
    } else if relative.ends_with(".json") {
        ContentKind::Json
    } else {
        ContentKind::Binary
    };
    ReleaseContent::File { path: file, kind }
}
