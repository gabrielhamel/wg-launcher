use super::PinnedManifest;
use super::error::{InstallationError as Error, Result};
use crate::catalog::{Product, SelectedProductRequest};

/// A known product, client type, and language, without discovery defaults or a catalog.
pub(crate) struct ResolvedSelection<'a> {
    pub(crate) product: &'a Product,
    pub(crate) client_type: &'a str,
    pub(crate) language: &'a str,
}

pub(crate) fn validate_cached_request(
    manifest: &PinnedManifest,
    request: &SelectedProductRequest<'_>,
) -> Result<()> {
    validate_cache(
        manifest,
        &request.product,
        request.client_type,
        request.language,
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
    let mut fields = Vec::new();
    if manifest.release.product != *product {
        fields.push(SelectionField::Product);
    }
    if manifest.release.identity.region != product.region {
        fields.push(SelectionField::Region);
    }
    if client_type.is_some_and(|requested| {
        !requested.eq_ignore_ascii_case(&manifest.release.identity.client_type)
    }) {
        fields.push(SelectionField::ClientType);
    }
    if language.is_some_and(|requested| {
        !requested.eq_ignore_ascii_case(&manifest.release.identity.language)
    }) {
        fields.push(SelectionField::Language);
    }
    if fields.is_empty() {
        return Ok(());
    }
    Err(Error::CachedSelectionMismatch(Box::new(
        SelectionMismatch {
            fields,
            cached: SelectionDescription {
                product: manifest.release.product.clone(),
                region: manifest.release.identity.region.clone(),
                client_type: Some(manifest.release.identity.client_type.clone()),
                language: Some(manifest.release.identity.language.clone()),
            },
            requested: SelectionDescription {
                product: product.clone(),
                region: product.region.clone(),
                client_type: client_type.map(str::to_owned),
                language: language.map(str::to_owned),
            },
        },
    )))
}

/// The incompatible selection fields, kept as facts until the error is displayed.
#[derive(Debug)]
pub(super) enum SelectionField {
    Product,
    Region,
    ClientType,
    Language,
}

#[derive(Debug)]
pub(super) struct SelectionDescription {
    pub(super) product: Product,
    pub(super) region: String,
    pub(super) client_type: Option<String>,
    pub(super) language: Option<String>,
}

#[derive(Debug)]
pub struct SelectionMismatch {
    pub(super) fields: Vec<SelectionField>,
    pub(super) cached: SelectionDescription,
    pub(super) requested: SelectionDescription,
}
