use super::super::infrastructure::wgus::WgusClient;
use crate::catalog::{BuildSelection, GameMetadata, Product, ResolvedRelease, installation_assets};
use crate::{Error, Result};

pub(crate) async fn resolve(selection: &BuildSelection) -> Result<ResolvedRelease> {
    let product = selection.selected_product()?;
    let client = WgusClient::new()?;
    if let Some(client_type) = selection.client_type.as_deref() {
        return client
            .release_for_product(&product, client_type, selection.language.as_deref())
            .await
            .map_err(Into::into);
    }

    let metadata = client.metadata_for_product(&product).await?;
    if let Some(default_client_type) = metadata.publisher_default_client_type() {
        return client
            .release_for_product(&product, default_client_type, selection.language.as_deref())
            .await
            .map_err(Into::into);
    }

    resolve_smallest_installable_variant(
        &client,
        &product,
        &metadata,
        selection.language.as_deref(),
    )
    .await
}

/// Resolve candidates in publisher order; equal sizes keep the first candidate,
/// and total failure preserves the first resolution or planning error.
async fn resolve_smallest_installable_variant(
    client: &WgusClient,
    product: &Product,
    metadata: &GameMetadata,
    language: Option<&str>,
) -> Result<ResolvedRelease> {
    let available = metadata
        .client_types
        .iter()
        .map(|client_type| client_type.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut best: Option<(u64, ResolvedRelease)> = None;
    let mut first_error: Option<Error> = None;
    for client_type in &metadata.client_types {
        match client
            .release_for_product(product, &client_type.id, language)
            .await
        {
            Ok(release) => match installation_assets(&release, &Default::default()) {
                Ok(assets) => {
                    let size = assets.iter().map(|asset| asset.size).sum();
                    if best.as_ref().is_none_or(|(best_size, _)| size < *best_size) {
                        best = Some((size, release));
                    }
                }
                Err(error) => {
                    first_error.get_or_insert(error.into());
                }
            },
            Err(error) => {
                first_error.get_or_insert(error.into());
            }
        };
    }
    best.map(|(_, release)| release).ok_or_else(|| {
        first_error.unwrap_or_else(|| {
            crate::catalog::ReleaseError::UnknownClientType {
                requested: "publisher default or smallest variant".into(),
                available,
            }
            .into()
        })
    })
}

/// Discover the publisher's update chain from the caller's known component versions.
/// Local checkpoints and pending-manifest precedence remain installation concerns.
pub(crate) async fn resolve_update_release(
    product: &crate::catalog::Product,
    client_type: &str,
    language: &str,
    component_versions: &std::collections::BTreeMap<String, String>,
) -> Result<ResolvedRelease> {
    Ok(WgusClient::new()?
        .release_from_versions(product, client_type, Some(language), component_versions)
        .await?)
}
