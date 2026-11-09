//! WGUS patch-chain queries retain only the protocol facts needed from metadata.

use super::{PROTOCOL_VERSION, conversion::normalize_language, wire::MetadataXml};
use crate::catalog::ClientVariant;
use crate::catalog::ComponentVersions;

const INSTALLATION_ID: &str = "wg-launcher";

/// Protocol-only facts retained after translating the metadata response.
/// Language spellings stay in publisher order so query encoding preserves its aliases.
pub(super) struct PatchChainRequestContext {
    application_id: String,
    metadata_version: String,
    default_language: String,
    supported_languages: Vec<String>,
}

impl PatchChainRequestContext {
    pub(super) fn from_wire(wire: &MetadataXml) -> Self {
        let predefined = &wire.predefined_section;
        Self {
            application_id: predefined
                .redirect_application_id
                .as_ref()
                .unwrap_or(&predefined.app_id)
                .clone(),
            metadata_version: wire.version.clone(),
            default_language: predefined.default_language.clone(),
            supported_languages: predefined
                .supported_languages
                .split(',')
                .map(str::to_owned)
                .collect(),
        }
    }

    pub(super) fn query(
        &self,
        selected: &ClientVariant,
        language: &str,
        component_versions: &ComponentVersions,
    ) -> Vec<(String, String)> {
        let wire_language = self
            .supported_languages
            .iter()
            .map(String::as_str)
            .find(|candidate| normalize_language(candidate) == language)
            .map(str::trim)
            .unwrap_or(&self.default_language);
        let mut query = vec![
            ("game_id".to_owned(), self.application_id.clone()),
            ("protocol_version".to_owned(), PROTOCOL_VERSION.to_owned()),
            (
                "metadata_protocol_version".to_owned(),
                PROTOCOL_VERSION.to_owned(),
            ),
            ("installation_id".to_owned(), INSTALLATION_ID.to_owned()),
            ("client_type".to_owned(), selected.id.clone()),
            ("lang".to_owned(), wire_language.to_owned()),
            ("metadata_version".to_owned(), self.metadata_version.clone()),
        ];
        for part in &selected.components {
            query.push((
                format!("{}_current_version", part.id),
                component_versions.version_or_zero(&part.id).to_owned(),
            ));
        }
        query
    }
}
