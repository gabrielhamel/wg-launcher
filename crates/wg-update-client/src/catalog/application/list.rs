use super::super::infrastructure::wgus::WgusClient;
use crate::Result;
use crate::catalog::{Product, required_volumes};
use serde::Serialize;

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
    /// Variants omitted because release resolution or patch ordering failed.
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

async fn list_with_client(client: &WgusClient, options: ListOptions) -> Result<ListReport> {
    let products = crate::catalog::domain::select_products(
        options.products.as_deref(),
        options.game.as_deref(),
        options.region.as_deref(),
    )?;
    let mut builds = Vec::new();
    let mut warnings = Vec::new();

    for product in products {
        let discovered = client.discover(&product).await?;
        let metadata = discovered.metadata();
        for client_type in &metadata.client_types {
            let resolved = discovered
                .resolve_release(
                    client,
                    &client_type.id,
                    Some(&metadata.default_language),
                    &Default::default(),
                )
                .await
                .map_err(crate::Error::from)
                .and_then(|release| {
                    BuildSummary::from_release(&product, &metadata.default_language, release)
                });
            match resolved {
                Ok(build) => builds.push(build),
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

impl BuildSummary {
    fn from_release(
        product: &Product,
        default_language: &str,
        release: crate::catalog::ResolvedRelease,
    ) -> Result<Self> {
        let volumes = required_volumes(&release, &Default::default())?;
        Ok(Self {
            game: product.game(),
            region: product.region.clone(),
            channel: release.identity.channel,
            platform: release.identity.platform,
            architecture: release.identity.architecture,
            client_type: release.identity.client_type,
            version: release.identity.version,
            metadata_version: release.identity.metadata_version,
            default_language: default_language.to_owned(),
            languages: release.available_languages,
            parts: release.contents.component_ids().to_vec(),
            compressed_bytes: volumes.iter().map(|volume| volume.size).sum(),
        })
    }
}
