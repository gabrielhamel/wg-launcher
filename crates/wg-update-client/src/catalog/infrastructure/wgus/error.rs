use crate::ErrorKind;
use crate::catalog::{CatalogError, ReleaseError};

/// Failures owned by WGUS discovery.
#[derive(Debug, thiserror::Error)]
pub enum WgusError {
    /// HTTP client creation, request, status, or response-stream failure.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Publisher metadata or patch-chain XML could not be deserialized.
    #[error("WGUS returned invalid XML: {0}")]
    Xml(#[from] quick_xml::DeError),
    /// A publisher URL could not be parsed or resolved.
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    /// The publisher supplied no usable patch chain for this application.
    #[error("WGUS did not publish a patch chain for {0}")]
    MissingPatchChain(String),
    /// The publisher supplied no HTTP seed for this application.
    #[error("WGUS did not publish a web seed for {0}")]
    MissingWebSeed(String),
    /// Protocol redirects exceeded the permitted limit for this application.
    #[error("WGUS redirect limit exceeded for {0}")]
    RedirectLimit(String),
    /// Failure from the underlying catalog operation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// Failure from the underlying release operation.
    #[error(transparent)]
    Release(#[from] ReleaseError),
}

pub(crate) type Result<T> = std::result::Result<T, WgusError>;

impl WgusError {
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            WgusError::Http(_) => ErrorKind::Network,
            WgusError::Catalog(_) => ErrorKind::InvalidSelection,
            WgusError::Release(error) => error.kind(),
            WgusError::Xml(_)
            | WgusError::Url(_)
            | WgusError::MissingPatchChain(_)
            | WgusError::MissingWebSeed(_)
            | WgusError::RedirectLimit(_) => ErrorKind::InvalidResponse,
        }
    }
}

impl From<WgusError> for crate::Error {
    fn from(error: WgusError) -> Self {
        Self::new(error.kind(), error)
    }
}
