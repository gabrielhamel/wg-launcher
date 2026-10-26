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

    /// Validates the service URL and required identity fields without network access.
    ///
    /// # Errors
    /// Returns [`crate::Error::InvalidProduct`] for an unsupported URL or empty identity.
    pub fn validate(&self) -> crate::Result<()> {
        if !matches!(self.host.scheme(), "http" | "https")
            || self.host.host_str().is_none()
            || !self.host.username().is_empty()
            || self.host.password().is_some()
            || self.host.query().is_some()
            || self.host.fragment().is_some()
        {
            return Err(crate::Error::InvalidProduct(
                "host must be an absolute HTTP(S) base URL without credentials, query, or fragment"
                    .into(),
            ));
        }
        for (field, value) in [
            ("game", &self.game),
            ("region", &self.region),
            ("name", &self.name),
            ("game_id", &self.game_id),
            ("channel", &self.channel),
        ] {
            if value.trim().is_empty() {
                return Err(crate::Error::InvalidProduct(format!(
                    "{field} must not be empty"
                )));
            }
        }
        if !self.game.chars().any(|c| c.is_ascii_alphanumeric())
            || !self.region.chars().any(|c| c.is_ascii_alphanumeric())
            || !self.channel.chars().any(|c| c.is_ascii_alphanumeric())
        {
            return Err(crate::Error::InvalidProduct(
                "game, region, and channel must contain an ASCII letter or digit".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn endpoint(&self, route: &str) -> crate::Result<url::Url> {
        self.validate()?;
        Ok(url::Url::parse(&format!(
            "{}/api/v1/{route}/",
            self.host.as_str().trim_end_matches('/')
        ))?)
    }
}

fn default_game() -> String {
    "wot".into()
}
