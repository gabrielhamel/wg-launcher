use super::domain::{ReleaseError, api_route, normalize_prefix, render_text, validate_public_url};
use super::infrastructure::{
    access_log, filesystem,
    http::{self, Request, Resource, ServerHandle},
};
use std::{io, net::SocketAddr, path::PathBuf, sync::Arc};

const METADATA_FILE: &str = "metadata.xml";
const PATCH_CHAIN_FILE: &str = "patches_chain.xml";

/// Configuration for a file-backed update service.
#[derive(Clone, Debug)]
pub struct FileServiceOptions {
    /// Release root containing `metadata.xml`, `patches_chain.xml`, and `packages/`.
    pub input: PathBuf,
    /// Address to bind. Port zero selects an ephemeral port.
    pub listen: SocketAddr,
    /// Path prefix used for the WGUS API routes.
    pub prefix: String,
    /// Public HTTP(S) origin substituted for `__BASE_URL__` in XML files.
    pub public_url: Option<String>,
    /// Optional append-only tab-separated HTTP access log.
    pub access_log: Option<PathBuf>,
}

/// Running file-backed WGUS-compatible update service.
///
/// Metadata is read and rendered at startup. Package bodies are opened and streamed
/// for each request, so large release volumes are never retained in memory.
pub struct FileService {
    handle: ServerHandle,
    prefix: String,
    public_url: String,
}

impl FileService {
    /// Validates the release directory, binds the listener, and starts serving.
    pub fn start(options: FileServiceOptions) -> io::Result<Self> {
        let files = filesystem::load_release(&options.input)?;
        let packages = files.packages;
        let metadata = files.metadata;
        let patch_chain = files.patch_chain;
        let prefix = normalize_prefix(&options.prefix)?;
        let listener = http::bind(options.listen)?;
        let address = listener.local_addr()?;
        let public_url = match options.public_url {
            Some(url) => validate_public_url(&url)?,
            None => format!("http://{address}"),
        };
        let metadata = render_text(metadata, &public_url, METADATA_FILE)?;
        let patch_chain = render_text(patch_chain, &public_url, PATCH_CHAIN_FILE)?;
        let metadata_route = api_route(&prefix, "metadata");
        let patch_chain_route = api_route(&prefix, "patches_chain");
        let access_log = access_log::open(options.access_log)?;
        let handler = Arc::new(move |request: &Request| {
            access_log::record(&access_log, request);
            if request.path == metadata_route {
                return Resource::Bytes(metadata.clone(), "application/xml");
            }
            if request.path == patch_chain_route {
                return Resource::Bytes(patch_chain.clone(), "application/xml");
            }
            http::package_resource(&packages, &request.path)
        });
        let handle = ServerHandle::start(listener, handler);
        Ok(Self {
            handle,
            prefix,
            public_url,
        })
    }

    /// Public origin used when rendering the release metadata.
    pub fn url(&self) -> &str {
        &self.public_url
    }

    /// URL to store in a launcher's custom product configuration.
    pub fn product_host(&self) -> String {
        if self.prefix.is_empty() {
            self.public_url.clone()
        } else {
            format!("{}/{}", self.public_url, self.prefix)
        }
    }

    /// Bound socket address, including the selected port when port zero was used.
    pub fn local_addr(&self) -> SocketAddr {
        self.handle.address
    }
}

impl From<ReleaseError> for io::Error {
    fn from(error: ReleaseError) -> Self {
        io::Error::new(io::ErrorKind::InvalidInput, error)
    }
}
