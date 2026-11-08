//! WGUS query encoding and redirect targets; configured product identity stays unchanged.

use super::{PROTOCOL_VERSION, conversion::normalize_language, error::Result, wire::MetadataXml};
use crate::catalog::{ClientType, GameMetadata, Product};
use std::collections::BTreeMap;
use url::Url;

const INSTALLATION_ID: &str = "wg-launcher";

pub(super) fn patch_chain_query(
    wire: &MetadataXml,
    metadata: &GameMetadata,
    selected: &ClientType,
    language: &str,
    component_versions: &BTreeMap<String, String>,
) -> Vec<(String, String)> {
    let predefined = &wire.predefined_section;
    let wire_language = predefined
        .supported_languages
        .split(',')
        .find(|candidate| normalize_language(candidate) == language)
        .map(str::trim)
        .unwrap_or(&predefined.default_language);
    let app_id = predefined
        .redirect_application_id
        .as_deref()
        .unwrap_or(&predefined.app_id);
    let mut query = vec![
        ("game_id".to_owned(), app_id.to_owned()),
        ("protocol_version".to_owned(), PROTOCOL_VERSION.to_owned()),
        (
            "metadata_protocol_version".to_owned(),
            PROTOCOL_VERSION.to_owned(),
        ),
        ("installation_id".to_owned(), INSTALLATION_ID.to_owned()),
        ("client_type".to_owned(), selected.id.clone()),
        ("lang".to_owned(), wire_language.to_owned()),
        (
            "metadata_version".to_owned(),
            metadata.metadata_version.clone(),
        ),
    ];
    for part in &selected.parts {
        query.push((
            format!("{}_current_version", part.id),
            component_versions
                .get(&part.id)
                .cloned()
                .unwrap_or_else(|| "0".to_owned()),
        ));
    }
    query
}

/// Ignore redirects back to the current endpoint, as some publishers return them.
pub(super) fn redirect_product(
    current: &Product,
    redirect: Option<&str>,
) -> Result<Option<Product>> {
    let Some(redirect) = redirect else {
        return Ok(None);
    };
    let mut redirected = current.clone();
    redirected.host = Url::parse(redirect)?;
    redirected.validate_identity()?;
    if product_endpoint(&redirected, "metadata")? == product_endpoint(current, "metadata")? {
        return Ok(None);
    }
    Ok(Some(redirected))
}

pub(super) fn product_endpoint(product: &Product, route: &str) -> Result<Url> {
    product.validate_identity()?;
    Ok(Url::parse(&format!(
        "{}/api/v1/{route}/",
        product.host.as_str().trim_end_matches('/')
    ))?)
}
