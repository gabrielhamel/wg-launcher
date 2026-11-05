/// Failures owned by catalog validation.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// A custom product has an invalid endpoint or identity.
    #[error("invalid product: {0}")]
    InvalidProduct(String),
}

pub(crate) type Result<T> = std::result::Result<T, CatalogError>;
