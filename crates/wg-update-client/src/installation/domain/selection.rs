use super::error::{InstallationError as Error, Result};
use crate::catalog::{Product, SelectedProductRequest};

/// Known product, recorded region, client type, and language, without discovery defaults.
pub(crate) struct ResolvedSelection<'a> {
    pub(crate) product: &'a Product,
    pub(crate) region: &'a str,
    pub(crate) client_type: &'a str,
    pub(crate) language: &'a str,
}

pub(crate) fn validate_cached_request(
    cached: &ResolvedSelection<'_>,
    request: &SelectedProductRequest<'_>,
) -> Result<()> {
    validate_compatibility(
        cached,
        &request.product,
        request.client_type,
        request.language,
    )
}

pub(crate) fn validate_cached_selection(
    cached: &ResolvedSelection<'_>,
    selection: &ResolvedSelection<'_>,
) -> Result<()> {
    validate_compatibility(
        cached,
        selection.product,
        Some(selection.client_type),
        Some(selection.language),
    )
}

fn validate_compatibility(
    cached: &ResolvedSelection<'_>,
    product: &Product,
    client_type: Option<&str>,
    language: Option<&str>,
) -> Result<()> {
    let mut fields = Vec::new();
    if cached.product != product {
        fields.push(SelectionField::Product);
    }
    if cached.region != product.region {
        fields.push(SelectionField::Region);
    }
    if client_type.is_some_and(|requested| !requested.eq_ignore_ascii_case(cached.client_type)) {
        fields.push(SelectionField::ClientType);
    }
    if language.is_some_and(|requested| !requested.eq_ignore_ascii_case(cached.language)) {
        fields.push(SelectionField::Language);
    }
    if fields.is_empty() {
        return Ok(());
    }
    Err(Error::CachedSelectionMismatch(Box::new(
        SelectionMismatch {
            fields,
            cached: SelectionDescription {
                product: cached.product.clone(),
                region: cached.region.to_owned(),
                client_type: Some(cached.client_type.to_owned()),
                language: Some(cached.language.to_owned()),
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

impl std::fmt::Display for SelectionDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{} ({}/{}: {}, {}, {})",
            self.region,
            self.client_type
                .as_deref()
                .unwrap_or("publisher default or smallest variant"),
            self.language.as_deref().unwrap_or("publisher default"),
            self.product.game(),
            self.product.name,
            self.product.service_base_url(),
            self.product.application_id(),
            self.product.channel
        )
    }
}

impl std::fmt::Display for SelectionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the cached build is {} but the requested build is {} (different ",
            self.cached, self.requested
        )?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            f.write_str(match field {
                SelectionField::Product => "product",
                SelectionField::Region => "region",
                SelectionField::ClientType => "client type",
                SelectionField::Language => "language",
            })?;
        }
        f.write_str("); use matching options or a different output directory")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_choices_accept_cached_defaults_and_explicit_choices_ignore_case() {
        let product = crate::Region::Eu.product();
        let cached = ResolvedSelection {
            product: &product,
            region: "eu",
            client_type: "hd",
            language: "en",
        };
        assert!(validate_compatibility(&cached, &product, None, None).is_ok());
        assert!(validate_compatibility(&cached, &product, Some("HD"), Some("EN")).is_ok());
    }

    #[test]
    fn reports_product_recorded_region_and_explicit_choice_mismatches() {
        let product = crate::Region::Eu.product();
        let mut other = product.clone();
        other.host = "http://localhost/other".parse().unwrap();
        let cached = ResolvedSelection {
            product: &product,
            region: "inconsistent",
            client_type: "hd",
            language: "en",
        };
        let Error::CachedSelectionMismatch(mismatch) =
            validate_compatibility(&cached, &other, Some("sd"), Some("fr")).unwrap_err()
        else {
            panic!("expected a selection mismatch")
        };
        assert!(matches!(
            mismatch.fields.as_slice(),
            [
                SelectionField::Product,
                SelectionField::Region,
                SelectionField::ClientType,
                SelectionField::Language
            ]
        ));
        assert_eq!(mismatch.cached.region, "inconsistent");
        assert_eq!(mismatch.requested.region, "eu");
    }
}
