use super::PinnedManifest;
use super::error::{InstallationError as Error, Result};
use crate::catalog::{BuildSelection, Product};

/// A known product, client type, and language, without discovery defaults or a catalog.
pub(crate) struct ResolvedSelection<'a> {
    pub(crate) product: &'a Product,
    pub(crate) client_type: &'a str,
    pub(crate) language: &'a str,
}

pub(crate) fn validate_cached_request(
    manifest: &PinnedManifest,
    request: &BuildSelection,
) -> Result<()> {
    validate_cache(
        manifest,
        &request.selected_product()?,
        request.client_type.as_deref(),
        request.language.as_deref(),
    )
}

pub(crate) fn validate_cached_selection(
    manifest: &PinnedManifest,
    selection: &ResolvedSelection<'_>,
) -> Result<()> {
    validate_cache(
        manifest,
        selection.product,
        Some(selection.client_type),
        Some(selection.language),
    )
}

fn validate_cache(
    manifest: &PinnedManifest,
    product: &Product,
    client_type: Option<&str>,
    language: Option<&str>,
) -> Result<()> {
    let selected_region = &product.region;
    let language_matches =
        language.is_none_or(|language| language.eq_ignore_ascii_case(&manifest.release.language));
    let client_type_matches = client_type.is_none_or(|client_type| {
        manifest
            .release
            .client_type
            .eq_ignore_ascii_case(client_type)
    });
    if manifest.release.product == *product
        && manifest.release.region == *selected_region
        && client_type_matches
        && language_matches
    {
        return Ok(());
    }
    Err(Error::CachedSelectionMismatch {
        cached: format!(
            "{}/{}/{}{}",
            manifest.release.region,
            manifest.release.client_type,
            manifest.release.language,
            product_description(&manifest.release.product)
        ),
        requested: format!(
            "{}/{}/{}{}",
            selected_region,
            client_type.unwrap_or("publisher default or smallest variant"),
            language.unwrap_or("publisher default"),
            product_description(product)
        ),
    })
}

fn product_description(product: &crate::catalog::Product) -> String {
    format!(
        " ({}/{}: {}, {}, {})",
        product.game(),
        product.name,
        product.host,
        product.game_id,
        product.channel
    )
}
