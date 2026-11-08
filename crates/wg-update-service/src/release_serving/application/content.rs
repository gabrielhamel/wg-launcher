//! Load the content for a release resource through safe filesystem lookup.

use super::routing::ReleaseResource;
use crate::release_serving::infrastructure::filesystem;
use std::path::PathBuf;

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

    pub(super) fn resolve(&self, resource: ReleaseResource<'_>) -> ReleaseContent {
        match resource {
            ReleaseResource::Metadata => ReleaseContent::Metadata(self.metadata.clone()),
            ReleaseResource::PatchChain => ReleaseContent::PatchChain(self.patch_chain.clone()),
            ReleaseResource::Package { relative, kind } => {
                match filesystem::resolve_package(&self.packages, relative) {
                    Some(path) => ReleaseContent::File { path, kind },
                    None => ReleaseContent::Missing,
                }
            }
        }
    }
}
