//! Metadata discovered for a product, reused while resolving its variants.

use super::{
    MAX_REDIRECTS, WgusClient,
    conversion::{normalize_language, release_from_wire},
    error::{Result, WgusError},
    request::{patch_chain_query, product_endpoint, redirect_product},
    wire::{MetadataXml, PatchesXml},
};
use crate::catalog::{ComponentVersions, GameMetadata, Product, ResolvedRelease};

/// A metadata response and the exact product endpoint it describes.
/// Redirected responses are local to each release resolution, not a global cache.
pub(crate) struct DiscoveredProduct {
    product: Product,
    wire: MetadataXml,
    metadata: GameMetadata,
}

impl DiscoveredProduct {
    pub(super) fn new(product: Product, wire: MetadataXml, metadata: GameMetadata) -> Self {
        Self {
            product,
            wire,
            metadata,
        }
    }

    pub(crate) fn metadata(&self) -> &GameMetadata {
        &self.metadata
    }

    pub(crate) async fn resolve_release(
        &self,
        client: &WgusClient,
        client_type: &str,
        language: Option<&str>,
        component_versions: &ComponentVersions,
    ) -> Result<ResolvedRelease> {
        let mut redirected = None;
        for hop in 0..=MAX_REDIRECTS {
            let current = redirected.as_ref().unwrap_or(self);
            let metadata = &current.metadata;
            let selected = metadata.require_client_type(client_type)?;
            let language = normalize_language(language.unwrap_or(&metadata.default_language));
            metadata.validate_language(&language)?;
            let endpoint = product_endpoint(&current.product, "patches_chain")?;
            let query = patch_chain_query(
                &current.wire,
                metadata,
                selected,
                &language,
                component_versions,
            );
            let text = client
                .http
                .get(endpoint)
                .query(&query)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let response: PatchesXml = quick_xml::de::from_str(&text)?;
            if let Some(product) = redirect_product(&current.product, response.redirect_url())? {
                if hop == MAX_REDIRECTS {
                    return Err(WgusError::RedirectLimit(
                        self.product.application_id().to_owned(),
                    ));
                }
                redirected = Some(client.discover(&product).await?);
                continue;
            }
            return release_from_wire(
                &self.product,
                current
                    .product
                    .service_base_url()
                    .host_str()
                    .unwrap()
                    .to_owned(),
                metadata.clone(),
                selected,
                language,
                response,
                component_versions,
            );
        }
        Err(WgusError::RedirectLimit(
            self.product.application_id().to_owned(),
        ))
    }
}
