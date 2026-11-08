//! Validated public origin and API route prefix used to construct service addresses.

use super::error::{ReleaseError, Result, invalid_input};

pub(crate) struct RoutePrefix(String);

impl RoutePrefix {
    pub(crate) fn parse(prefix: &str) -> Result<Self> {
        let prefix = prefix.trim_matches('/');
        if prefix
            .split('/')
            .any(|part| part == "." || part == ".." || part.contains(['?', '#']))
        {
            return Err(invalid_input(format!("invalid URL path prefix '{prefix}'")));
        }
        Ok(Self(prefix.to_owned()))
    }

    pub(crate) fn api_route(&self, route: &str) -> String {
        if self.0.is_empty() {
            format!("/api/v1/{route}/")
        } else {
            format!("/{}/api/v1/{route}/", self.0)
        }
    }
}

pub(crate) struct PublicOrigin(String);

impl PublicOrigin {
    pub(crate) fn parse(url: &str) -> Result<Self> {
        let url = url.trim_end_matches('/');
        let parsed = url::Url::parse(url).map_err(|_| {
            ReleaseError::InvalidPublicOrigin("public origin must be an absolute HTTP(S) origin")
        })?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(ReleaseError::InvalidPublicOrigin(
                "public origin must be an absolute HTTP(S) origin without credentials, path, query, or fragment",
            ));
        }
        Ok(Self(url.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn product_host(&self, prefix: &RoutePrefix) -> String {
        if prefix.0.is_empty() {
            self.0.clone()
        } else {
            format!("{}/{}", self.0, prefix.0)
        }
    }
}
