mod build_selection;
mod builtin;
mod error;
mod product;
pub(crate) mod release;
mod selection;

pub use build_selection::BuildSelection;
pub use error::CatalogError;

pub(crate) use selection::select_products;

pub use product::Product;

pub use builtin::{Game, Region};
