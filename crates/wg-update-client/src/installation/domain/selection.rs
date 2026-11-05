use super::error::{InstallationError as Error, Result};
use crate::{BuildSelection, PatchManifest};

pub(crate) fn validate_cached_selection(
    manifest: &PatchManifest,
    selection: &BuildSelection,
) -> Result<()> {
    let product = selection.selected_product()?;
    let selected_region = &product.region;
    let language_matches = selection
        .language
        .as_deref()
        .is_none_or(|language| language.eq_ignore_ascii_case(&manifest.language));
    let client_type_matches = selection
        .client_type
        .as_deref()
        .is_none_or(|client_type| manifest.client_type.eq_ignore_ascii_case(client_type));
    if manifest.product == product
        && manifest.region == *selected_region
        && client_type_matches
        && language_matches
    {
        return Ok(());
    }
    Err(Error::CachedSelectionMismatch {
        cached: format!(
            "{}/{}/{}{}",
            manifest.region,
            manifest.client_type,
            manifest.language,
            product_description(&manifest.product)
        ),
        requested: format!(
            "{}/{}/{}{}",
            selected_region,
            selection
                .client_type
                .as_deref()
                .unwrap_or("publisher default or smallest variant"),
            selection.language.as_deref().unwrap_or("publisher default"),
            product_description(&product)
        ),
    })
}

fn product_description(product: &crate::Product) -> String {
    format!(
        " ({}/{}: {}, {}, {})",
        product.game(),
        product.name,
        product.host,
        product.game_id,
        product.channel
    )
}
