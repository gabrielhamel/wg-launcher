//! Metadata discovered for a product, reused while resolving its variants.

use super::{
    MAX_REDIRECTS, WgusClient,
    conversion::{metadata_from_wire, normalize_language, release_from_wire},
    endpoint::DiscoveryEndpoint,
    error::{Result, WgusError},
    request::PatchChainRequestContext,
    wire::{MetadataXml, PatchesXml},
};
use crate::catalog::{ComponentVersions, GameMetadata, Product, ResolvedRelease};

/// A metadata response and the exact product endpoint it describes.
/// Redirected responses are local to each release resolution, not a global cache.
pub(crate) struct DiscoveredProduct {
    product: Product,
    endpoint: DiscoveryEndpoint,
    request_context: PatchChainRequestContext,
    metadata: GameMetadata,
}

impl DiscoveredProduct {
    pub(super) fn from_wire(
        product: Product,
        endpoint: DiscoveryEndpoint,
        wire: MetadataXml,
    ) -> Self {
        let metadata = metadata_from_wire(&wire);
        let request_context = PatchChainRequestContext::from_wire(&wire);
        Self {
            product,
            endpoint,
            request_context,
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
            let endpoint = current.endpoint.route("patches_chain")?;
            let query = current
                .request_context
                .query(selected, &language, component_versions);
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
            if let Some(endpoint) = current.endpoint.redirect(response.redirect_url())? {
                if hop == MAX_REDIRECTS {
                    return Err(WgusError::RedirectLimit(
                        self.product.application_id().to_owned(),
                    ));
                }
                redirected = Some(client.discover_endpoint(&self.product, endpoint).await?);
                continue;
            }
            return release_from_wire(
                &self.product,
                current.endpoint.hostname().to_owned(),
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
