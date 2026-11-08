//! Select release descriptions and packages for incoming service requests.

use super::super::domain::RoutePrefix;
use std::path::Path;

/// A requested release resource, before checking whether its package file exists.
pub(super) enum ReleaseResource<'a> {
    Metadata,
    PatchChain,
    Package { relative: &'a Path },
}

pub(super) struct ReleaseRoutes {
    metadata_route: String,
    patch_chain_route: String,
}

impl ReleaseRoutes {
    pub(super) fn new(prefix: &RoutePrefix) -> Self {
        Self {
            metadata_route: prefix.api_route("metadata"),
            patch_chain_route: prefix.api_route("patches_chain"),
        }
    }

    pub(super) fn resource<'a>(&self, request_path: &'a str) -> ReleaseResource<'a> {
        if request_path == self.metadata_route {
            return ReleaseResource::Metadata;
        }
        if request_path == self.patch_chain_route {
            return ReleaseResource::PatchChain;
        }
        let relative = request_path.trim_start_matches('/');
        let relative = relative.strip_prefix("packages/").unwrap_or(relative);
        ReleaseResource::Package {
            relative: Path::new(relative),
        }
    }
}
