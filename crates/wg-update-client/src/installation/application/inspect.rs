use crate::catalog::resolve;
use crate::installation::domain::initial_directory_name;
use crate::{
    Result,
    catalog::{BuildSelection, InspectReport},
};

/// Selects a build whose resolved WGUS release should be inspected.
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    /// Build to resolve; requires a game and otherwise uses EU, the publisher's
    /// default client type (or smallest installable variant), and default language.
    pub selection: BuildSelection,
}

/// Resolves a live build without downloading packages or changing files.
///
/// # Errors
///
/// Returns an error for failed network requests, invalid protocol responses,
/// unsupported client types or languages, or missing patch chains or web seeds.
pub async fn inspect(options: InspectOptions) -> Result<InspectReport> {
    let request = options.selection.select_product()?;
    let release = resolve(&request).await?;
    let directory = initial_directory_name(&release.product, &release.identity);
    Ok(InspectReport::from_release(&release, directory))
}
