use super::error::{CatalogError as Error, Result};
use serde::{Deserialize, Serialize};

/// Owned publisher endpoint and application identity for built-in or custom catalogs.
///
/// Use [`crate::Region::product`] for built-in defaults or construct this value for another
/// WGUS-compatible service. Custom products are persisted with installations so
/// status/update do not require the original configuration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    /// Stable game identifier used by catalog filters and installation directory names.
    /// Older product files without this field default to `wot`.
    #[serde(default = "default_game")]
    pub game: String,
    /// Region label used in reports and output directory names; need not be a [`crate::Region`].
    pub region: String,
    /// Human-readable product name.
    pub name: String,
    /// Absolute HTTP(S) service base URL, including an optional port and path prefix.
    /// A trailing slash is optional. Credentials, query strings, and fragments are rejected.
    pub host: url::Url,
    /// Publisher application identifier used for metadata discovery.
    pub game_id: String,
    /// Release channel used in reports and output directory names.
    pub channel: String,
}

impl Product {
    /// Returns the normalized game identifier used for selection and display.
    pub fn game(&self) -> String {
        self.game.trim().to_ascii_lowercase()
    }

    /// Publisher application identity, distinct from the catalog's game label.
    pub(crate) fn application_id(&self) -> &str {
        &self.game_id
    }

    /// Complete service base URL, including any configured port and path prefix.
    pub(crate) fn service_base_url(&self) -> &url::Url {
        &self.host
    }

    /// Validates the service URL and required identity fields without network access.
    ///
    /// # Errors
    /// Returns [`crate::ErrorKind::InvalidSelection`] for an unsupported URL or empty identity.
    pub fn validate(&self) -> crate::Result<()> {
        self.validate_identity().map_err(Into::into)
    }

    pub(crate) fn validate_identity(&self) -> Result<()> {
        validate_service_base_url(&self.host)?;
        for (field, value) in [
            ("game", &self.game),
            ("region", &self.region),
            ("name", &self.name),
            ("game_id", &self.game_id),
            ("channel", &self.channel),
        ] {
            if value.trim().is_empty() {
                return Err(Error::InvalidProduct(format!("{field} must not be empty")));
            }
        }
        if !self.game.chars().any(|c| c.is_ascii_alphanumeric())
            || !self.region.chars().any(|c| c.is_ascii_alphanumeric())
            || !self.channel.chars().any(|c| c.is_ascii_alphanumeric())
        {
            return Err(Error::InvalidProduct(
                "game, region, and channel must contain an ASCII letter or digit".into(),
            ));
        }
        Ok(())
    }
}

fn default_game() -> String {
    "wot".into()
}

/// Validate a configured or redirected service address using the same URL rules.
pub(crate) fn validate_service_base_url(host: &url::Url) -> Result<()> {
    if !matches!(host.scheme(), "http" | "https")
        || host.host_str().is_none()
        || !host.username().is_empty()
        || host.password().is_some()
        || host.query().is_some()
        || host.fragment().is_some()
    {
        return Err(Error::InvalidProduct(
            "host must be an absolute HTTP(S) base URL without credentials, query, or fragment"
                .into(),
        ));
    }
    Ok(())
}
