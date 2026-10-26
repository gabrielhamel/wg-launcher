use super::manifest::resolve;
use crate::{PatchManifest, Product, Region, Result, WgusClient, installation_assets};
use serde::Serialize;

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
    pub(super) fn selected_product(&self) -> Result<Product> {
        let game = self.game.as_deref().ok_or_else(|| {
            crate::Error::InvalidProduct(
                "a game must be selected before inspecting, downloading, or installing".into(),
            )
        })?;
        Ok(crate::catalog::select_products(
            self.products.as_deref(),
            Some(game),
            Some(&self.region),
        )?
        .remove(0))
    }
}

/// Configures discovery. By default every supported region is queried.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Filter either catalog by game identifier, or query every configured game with `None`.
    pub game: Option<String>,
    /// Filter either catalog by region label, or query every configured region with `None`.
    pub region: Option<String>,
    /// Replacement catalog, filtered by `region`. `None` uses built-ins.
    /// Empty catalogs and duplicate region labels (ignoring case) are rejected.
    pub products: Option<Vec<Product>>,
}

/// One installable client variant discovered by [`list`].
#[derive(Debug, Clone, Serialize)]
pub struct BuildSummary {
    /// Normalized game identifier.
    pub game: String,
    /// Publisher catalog region.
    pub region: String,
    /// Publisher release channel.
    pub channel: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
    /// Publisher default language used for size resolution.
    pub default_language: String,
    /// Advertised normalized language identifiers.
    pub languages: Vec<String>,
    /// Required component identifiers.
    pub parts: Vec<String>,
    /// Compressed byte total for the ordered installation assets in the default language.
    pub compressed_bytes: u64,
}

/// A client variant that could not be resolved or ordered during discovery.
#[derive(Debug, Clone, Serialize)]
pub struct ListWarning {
    /// Normalized game identifier.
    pub game: String,
    /// Publisher catalog region.
    pub region: String,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Human-readable reason the variant is unavailable.
    pub message: String,
}

/// Discovered builds and per-variant failures.
#[derive(Debug, Clone, Serialize)]
pub struct ListReport {
    /// Successfully resolved, installable variants.
    pub builds: Vec<BuildSummary>,
    /// Variants omitted because manifest resolution or patch ordering failed.
    pub warnings: Vec<ListWarning>,
}

/// Discovers installable variants using the live catalog, without writing files.
///
/// Variant failures are collected in [`ListReport::warnings`] so other variants
/// can still be returned.
///
/// # Errors
///
/// Returns an error for an invalid catalog or missing region selection,
/// if the HTTP client cannot be created, or if a region's metadata
/// cannot be fetched or parsed. Such a failure stops discovery.
pub async fn list(options: ListOptions) -> Result<ListReport> {
    list_with_client(&WgusClient::new()?, options).await
}

/// Selects a build whose resolved WGUS manifest should be inspected.
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    /// Build to resolve; requires a game and otherwise uses EU, the publisher's
    /// default client type (or smallest installable variant), and default language.
    pub selection: BuildSelection,
}

/// Resolves a live build without downloading packages or changing files.
///
/// # Errors
///
/// Returns an error for failed network requests, invalid protocol responses,
/// unsupported client types or languages, or missing patch chains or web seeds.
pub async fn inspect(options: InspectOptions) -> Result<PatchManifest> {
    resolve(&WgusClient::new()?, &options.selection).await
}

async fn list_with_client(client: &WgusClient, options: ListOptions) -> Result<ListReport> {
    let products = crate::catalog::select_products(
        options.products.as_deref(),
        options.game.as_deref(),
        options.region.as_deref(),
    )?;
    let mut builds = Vec::new();
    let mut warnings = Vec::new();

    for product in products {
        let metadata = client.metadata_for_product(&product).await?;
        for client_type in &metadata.client_types {
            match client
                .manifest_for_product(&product, &client_type.id, Some(&metadata.default_language))
                .await
            {
                Ok(manifest) => match installation_assets(&manifest) {
                    Ok(assets) => builds.push(BuildSummary {
                        game: product.game(),
                        region: product.region.clone(),
                        channel: manifest.channel,
                        platform: manifest.platform,
                        architecture: manifest.architecture,
                        client_type: manifest.client_type,
                        version: manifest.version,
                        metadata_version: manifest.metadata_version,
                        default_language: metadata.default_language.clone(),
                        languages: manifest.available_languages,
                        parts: manifest.parts,
                        compressed_bytes: assets.iter().map(|asset| asset.size).sum(),
                    }),
                    Err(error) => warnings.push(ListWarning {
                        game: product.game(),
                        region: product.region.clone(),
                        client_type: client_type.id.clone(),
                        message: error.to_string(),
                    }),
                },
                Err(error) => warnings.push(ListWarning {
                    game: product.game(),
                    region: product.region.clone(),
                    client_type: client_type.id.clone(),
                    message: error.to_string(),
                }),
            }
        }
    }

    Ok(ListReport { builds, warnings })
}
