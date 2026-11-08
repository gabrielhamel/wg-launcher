//! Select release descriptions and packages for incoming service requests.

use super::super::{
    domain::api_route,
    infrastructure::{
        filesystem,
        http::{Request, Resource},
    },
};
use std::path::{Path, PathBuf};

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

    pub(super) fn resolve(&self, request: &Request) -> Resource {
        if request.path == self.metadata_route {
            return Resource::Bytes(self.metadata.clone(), "application/xml");
        }
        if request.path == self.patch_chain_route {
            return Resource::Bytes(self.patch_chain.clone(), "application/xml");
        }
        package_resource(&self.packages, &request.path)
    }
}

/// Map supported package routes to HTTP resources after safe filesystem lookup.
fn package_resource(packages: &Path, request_path: &str) -> Resource {
    let relative = request_path.trim_start_matches('/');
    let relative = relative.strip_prefix("packages/").unwrap_or(relative);
    let Some(file) = filesystem::resolve_package(packages, Path::new(relative)) else {
        return Resource::NotFound;
    };
    let content_type = if relative.ends_with(".xml") {
        "application/xml"
    } else if relative.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    };
    Resource::File(file, content_type)
}
