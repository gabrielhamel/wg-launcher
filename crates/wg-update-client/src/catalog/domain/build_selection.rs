use super::error::{CatalogError as Error, Result};
use super::{Product, Region};

/// Selects a catalog build.
///
/// A game must be selected. Other defaults are EU, the publisher's default client
/// type (or the smallest installable variant), and the publishing region's default language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSelection {
    /// Game identifier to select from the custom or built-in catalog.
    pub game: Option<String>,
    /// Region label to select from the custom or built-in catalog; defaults to `eu`.
    pub region: String,
    /// Replacement catalog. `None` uses built-ins; an empty custom catalog is invalid.
    pub products: Option<Vec<Product>>,
    /// Publisher client variant identifier, such as `sd` or `hd`. `None` selects
    /// the publisher default, falling back to the smallest installable variant.
    pub client_type: Option<String>,
    /// Requested language; `None` uses the publisher default.
    pub language: Option<String>,
}

impl Default for BuildSelection {
    fn default() -> Self {
        Self {
            game: None,
            region: Region::Eu.to_string(),
            products: None,
            client_type: None,
            language: None,
        }
    }
}

impl BuildSelection {
    pub(crate) fn selected_product(&self) -> Result<Product> {
        let game = self.game.as_deref().ok_or_else(|| {
            Error::InvalidProduct(
                "a game must be selected before inspecting, downloading, or installing".into(),
            )
        })?;
        Ok(
            super::select_products(self.products.as_deref(), Some(game), Some(&self.region))?
                .remove(0),
        )
    }
}
