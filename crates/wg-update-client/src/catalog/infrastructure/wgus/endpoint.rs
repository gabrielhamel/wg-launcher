//! Transient discovery addresses, independent of configured product identity.

use super::error::Result;
use crate::catalog::{Product, validate_service_base_url};
use url::Url;

pub(super) struct DiscoveryEndpoint {
    base_url: Url,
}

impl DiscoveryEndpoint {
    pub(super) fn for_product(product: &Product) -> Result<Self> {
        product.validate_identity()?;
        Ok(Self {
            base_url: product.service_base_url().clone(),
        })
    }

    pub(super) fn route(&self, route: &str) -> Result<Url> {
        Ok(Url::parse(&format!(
            "{}/api/v1/{route}/",
            self.base_url.as_str().trim_end_matches('/')
        ))?)
    }

    pub(super) fn hostname(&self) -> &str {
        self.base_url
            .host_str()
            .expect("validated HTTP(S) endpoint has a host")
    }

    /// Ignore redirects to the current metadata endpoint, as some publishers return them.
    pub(super) fn redirect(&self, redirect: Option<&str>) -> Result<Option<Self>> {
        let Some(redirect) = redirect else {
            return Ok(None);
        };
        let base_url = Url::parse(redirect)?;
        validate_service_base_url(&base_url)?;
        let redirected = Self { base_url };
        if redirected.route("metadata")? == self.route("metadata")? {
            return Ok(None);
        }
        Ok(Some(redirected))
    }
}
