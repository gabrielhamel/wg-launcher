//! Load the content for a release resource through safe filesystem lookup.

use super::routing::ReleaseResource;
use crate::release_serving::infrastructure::{
    filesystem,
    http::{ContentType, HttpResponse},
};
use std::path::PathBuf;

pub(super) struct ReleaseContents {
    metadata: Vec<u8>,
    patch_chain: Vec<u8>,
    packages: PathBuf,
}

impl ReleaseContents {
    pub(super) fn new(metadata: Vec<u8>, patch_chain: Vec<u8>, packages: PathBuf) -> Self {
        Self {
            metadata,
            patch_chain,
            packages,
        }
    }

    pub(super) fn resolve(&self, resource: ReleaseResource<'_>) -> HttpResponse {
        match resource {
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
