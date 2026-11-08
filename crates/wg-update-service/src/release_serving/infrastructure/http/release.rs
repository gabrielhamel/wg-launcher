//! Route release requests and describe their content through safe filesystem lookup.

use super::{
    Request,
    routing::{ReleaseResource, ReleaseRoutes},
};
use crate::release_serving::domain::RoutePrefix;
use crate::release_serving::infrastructure::{
    filesystem,
    http::{ContentType, HttpResponse},
};
use std::path::PathBuf;

pub(crate) struct ReleaseHandler {
    routes: ReleaseRoutes,
    metadata: Vec<u8>,
    patch_chain: Vec<u8>,
    packages: PathBuf,
}

impl ReleaseHandler {
    pub(crate) fn new(
        prefix: &RoutePrefix,
        metadata: Vec<u8>,
        patch_chain: Vec<u8>,
        packages: PathBuf,
    ) -> Self {
        Self {
            routes: ReleaseRoutes::new(prefix),
            metadata,
            patch_chain,
            packages,
        }
    }

    pub(crate) fn respond(&self, request: &Request) -> HttpResponse {
        match self.routes.resource(&request.path) {
            ReleaseResource::Metadata => HttpResponse::Bytes {
                body: self.metadata.clone(),
                content_type: ContentType::Xml,
            },
            ReleaseResource::PatchChain => HttpResponse::Bytes {
                body: self.patch_chain.clone(),
                content_type: ContentType::Xml,
            },
            ReleaseResource::Package { relative } => {
                match filesystem::resolve_package(&self.packages, relative) {
                    Some(path) => HttpResponse::File {
                        path,
                        content_type: ContentType::for_path(relative),
                    },
                    None => HttpResponse::NotFound,
                }
            }
        }
    }
}
