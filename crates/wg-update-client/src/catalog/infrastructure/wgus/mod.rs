use crate::catalog::ComponentVersions;
mod conversion;
mod error;
mod patch_chain_conversion;
mod request;
mod wire;

use self::error::{Result, WgusError as Error};
pub use error::WgusError;

use crate::catalog::{GameMetadata, Product, ResolvedRelease};
use conversion::{metadata_from_wire, normalize_language, release_from_wire};
use request::{patch_chain_query, product_endpoint, redirect_product};
use reqwest::Client;
use std::time::Duration;
use wire::{MetadataXml, PatchesXml};

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
    /// Returns [`Error::Http`] if the HTTP client cannot be built.
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

    /// Fetches metadata from a caller-supplied product without writing files.
    ///
    /// # Errors
    /// Returns an error for invalid product configuration or failed HTTP/XML requests.
    pub async fn metadata_for_product(&self, product: &Product) -> Result<GameMetadata> {
        let (_, metadata) = self.fetch_metadata(product).await?;
        Ok(metadata)
    }

    /// Resolves a product's full installation chain without downloading assets.
    /// The original product configuration is pinned even after protocol redirects.
    ///
    /// # Errors
    /// Returns errors for invalid configuration or the same protocol failures as [`Self::release_from_versions`].
    pub async fn release_for_product(
        &self,
        product: &Product,
        client_type: &str,
        language: Option<&str>,
    ) -> Result<ResolvedRelease> {
        self.release_from_versions(
            product,
            client_type,
            language,
            &ComponentVersions::default(),
        )
        .await
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
        product.validate_identity()?;
        let mut resolved = product.clone();

        for hop in 0..=MAX_REDIRECTS {
            let (wire_metadata, metadata) = self.fetch_metadata(&resolved).await?;
            let selected = metadata.require_client_type(client_type)?.clone();
            let language = normalize_language(language.unwrap_or(&metadata.default_language));
            metadata.validate_language(&language)?;
            let endpoint = product_endpoint(&resolved, "patches_chain")?;
            let query = patch_chain_query(
                &wire_metadata,
                &metadata,
                &selected,
                &language,
                component_versions,
            );

            let text = self
                .http
                .get(endpoint)
                .query(&query)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let response: PatchesXml = quick_xml::de::from_str(&text)?;

            if let Some(redirected) = redirect_product(&resolved, response.redirect_url())? {
                if hop == MAX_REDIRECTS {
                    return Err(Error::RedirectLimit(product.application_id().to_owned()));
                }
                resolved = redirected;
                continue;
            }

            return release_from_wire(
                product,
                resolved.service_base_url().host_str().unwrap().to_owned(),
                metadata,
                &selected,
                language,
                response,
                component_versions,
            );
        }

        Err(Error::RedirectLimit(product.application_id().to_owned()))
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
