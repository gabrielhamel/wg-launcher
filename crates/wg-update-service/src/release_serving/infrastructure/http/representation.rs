//! HTTP response descriptions, independent of release-resource roles.

use std::path::{Path, PathBuf};

pub(crate) enum HttpResponse {
    Bytes {
        body: Vec<u8>,
        content_type: ContentType,
    },
    File {
        path: PathBuf,
        content_type: ContentType,
    },
    NotFound,
}

pub(crate) enum ContentType {
    Xml,
    Json,
    Binary,
}

impl ContentType {
    /// Preserve the service's case-sensitive filename classification.
    pub(crate) fn for_path(path: &Path) -> Self {
        let name = path.as_os_str().to_string_lossy();
        if name.ends_with(".xml") {
            Self::Xml
        } else if name.ends_with(".json") {
            Self::Json
        } else {
            Self::Binary
        }
    }

    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Xml => "application/xml",
            Self::Json => "application/json",
            Self::Binary => "application/octet-stream",
        }
    }
}
