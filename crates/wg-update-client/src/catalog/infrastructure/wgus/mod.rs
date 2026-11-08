use crate::catalog::ComponentVersions;
mod conversion;
mod discovery;
pub(crate) use discovery::DiscoveredProduct;
mod error;
mod patch_chain_conversion;
mod patch_chain_normalization;
mod request;
mod wire;

use self::error::Result;
pub use error::WgusError;

use crate::catalog::{GameMetadata, Product, ResolvedRelease};
use conversion::metadata_from_wire;
use request::product_endpoint;
use reqwest::Client;
use std::time::Duration;
use wire::MetadataXml;

/// WGUS protocol version sent in metadata and patch-chain requests.
pub const PROTOCOL_VERSION: &str = "100500.6969696";
const MAX_REDIRECTS: usize = 3;

/// Reusable client for live WGUS-compatible metadata and full installation chains.
///
/// Supports built-in catalogs, including Lesta RU. Resolves transport URLs
/// without downloading package contents. Use [`VolumeDownloader`](crate::installation::VolumeDownloader)
/// for transfers or the high-level workflows for installation lifecycle management.
#[derive(Clone)]
pub struct WgusClient {
    http: Client,
}

impl WgusClient {
    /// Creates a protocol client with a 15-second connection timeout, 60-second
    /// request timeout, and the crate version in its user agent.
    ///
    /// # Errors
    ///
    /// Returns [`WgusError::Http`] if the HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http })
    }

    /// Discover a product once so its variants share one metadata response.
    pub(crate) async fn discover(&self, product: &Product) -> Result<DiscoveredProduct> {
        product.validate_identity()?;
        let (wire, metadata) = self.fetch_metadata(product).await?;
        Ok(DiscoveredProduct::new(product.clone(), wire, metadata))
    }

    /// Resolves update packages using the installed version of each component.
    ///
    /// Missing components start at `0`. Unchanged components retain their installed
    /// versions even when the publisher returns no patches for them. Original product
    /// configuration and component versions survive protocol redirects.
    ///
    /// # Errors
    /// Returns the configuration, HTTP, XML, redirect, and patch-chain errors.
    pub async fn release_from_versions(
        &self,
        product: &Product,
        client_type: &str,
        language: Option<&str>,
        component_versions: &ComponentVersions,
    ) -> Result<ResolvedRelease> {
        self.discover(product)
            .await?
            .resolve_release(self, client_type, language, component_versions)
            .await
    }

    async fn fetch_metadata(&self, product: &Product) -> Result<(MetadataXml, GameMetadata)> {
        let endpoint = product_endpoint(product, "metadata")?;
        let text = self
            .http
            .get(endpoint)
            .query(&[
                ("guid", product.application_id()),
                ("chain_id", "unknown"),
                ("protocol_version", PROTOCOL_VERSION),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let wire: MetadataXml = quick_xml::de::from_str(&text)?;
        let metadata = metadata_from_wire(&wire);
        Ok((wire, metadata))
    }
}
