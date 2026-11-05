use std::path::PathBuf;
use wg_update_client::Product;

pub(crate) fn read_products(
    path: Option<PathBuf>,
) -> wg_update_client::Result<Option<Vec<Product>>> {
    path.map(|path| Ok(serde_json::from_slice(&std::fs::read(path)?)?))
        .transpose()
}
