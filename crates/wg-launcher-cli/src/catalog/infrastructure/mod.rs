use std::path::PathBuf;
use wg_update_client::Product;

pub(crate) fn read_products(
    path: Option<PathBuf>,
) -> Result<Option<Vec<Product>>, Box<dyn std::error::Error>> {
    path.map(|path| Ok(serde_json::from_slice(&std::fs::read(path)?)?))
        .transpose()
}
