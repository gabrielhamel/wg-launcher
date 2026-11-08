//! Translate WGUS wire responses into catalog releases, without network or local state.

use super::error::Result;
use super::patch_chain_conversion::patch_chain_from_wire;
use super::wire::{MetadataXml, PatchesXml};
use crate::catalog::ComponentVersions;
use crate::catalog::{
    ClientPart, ClientType, Executable, GameMetadata, Product, ReleaseContents, ResolvedRelease,
};

pub(super) fn release_from_wire(
    product: &Product,
    resolved_hostname: String,
    metadata: GameMetadata,
    selected: &ClientType,
    language: String,
    response: PatchesXml,
    component_versions: &ComponentVersions,
) -> Result<ResolvedRelease> {
    let chain = patch_chain_from_wire(
        product,
        selected,
        response.patches_chain,
        response.web_seeds,
        component_versions,
    )?;
    let patches = chain.patches;
    let version = response
        .version_name
        .filter(|version| !version.trim().is_empty())
        .or_else(|| patches.first().map(|patch| patch.version_to.clone()))
        .unwrap_or_else(|| "unknown".to_owned());

    Ok(ResolvedRelease {
        identity: crate::catalog::BuildIdentity {
            region: product.region.clone(),
            channel: product.channel.clone(),
            application_id: metadata.application_id.clone(),
            platform: "windows".to_owned(),
            architecture: selected.architecture.clone(),
            client_type: selected.id.clone(),
            language,
            metadata_version: metadata.metadata_version,
            version,
        },
        product: product.clone(),
        resolved_hostname,
        available_languages: metadata.supported_languages,
        publisher_ids: metadata.publisher_ids,
        contents: ReleaseContents::new(
            selected.parts.iter().map(|part| part.id.clone()).collect(),
            chain.component_versions,
            patches,
            chain.volumes,
        ),
        web_seeds: chain.web_seeds,
    })
}

pub(super) fn metadata_from_wire(wire: &MetadataXml) -> GameMetadata {
    let predefined = &wire.predefined_section;
    GameMetadata {
        metadata_version: wire.version.clone(),
        application_id: predefined.app_id.clone(),
        name: predefined.name.clone(),
        filesystem_name: predefined.filesystem_name.clone(),
        environment: predefined.environment.clone(),
        default_language: normalize_language(&predefined.default_language),
        supported_languages: predefined
            .supported_languages
            .split(',')
            .map(normalize_language)
            .filter(|language| !language.is_empty())
            .collect(),
        default_client_type: predefined.client_types.default.clone(),
        client_types: predefined
            .client_types
            .types
            .iter()
            .map(|client| ClientType {
                id: client.id.clone(),
                architecture: client.architecture.clone(),
                parts: client
                    .client_parts
                    .parts
                    .iter()
                    .map(|part| ClientPart {
                        id: part.id.clone(),
                        app_type: part.app_type.clone(),
                        language_specific: part.language_specific,
                        integrity_checked: part.integrity_checked,
                    })
                    .collect(),
            })
            .collect(),
        executables: predefined
            .executables
            .executables
            .iter()
            .map(|executable| Executable {
                path: executable.path.clone(),
                architecture: executable.architecture.clone(),
                emulation: executable.emulation.clone(),
            })
            .collect(),
        publisher_ids: wire
            .publisher_ids
            .split(',')
            .map(str::trim)
            .filter(|publisher| !publisher.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

pub(super) fn normalize_language(language: &str) -> String {
    language.trim().replace('_', "-").to_ascii_uppercase()
}
