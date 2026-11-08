use serde::Serialize;

/// Publisher metadata describing the available client variants and languages.
#[derive(Debug, Clone, Serialize)]
pub struct GameMetadata {
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
    /// Publisher application identifier.
    #[serde(rename = "game_id")]
    pub application_id: String,
    /// Human-readable game name.
    pub name: String,
    /// Publisher-suggested filesystem name.
    pub filesystem_name: String,
    /// Publisher environment label.
    pub environment: String,
    /// Normalized publisher default language.
    pub default_language: String,
    /// Normalized languages advertised by the publisher.
    pub supported_languages: Vec<String>,
    /// Publisher default client variant identifier; used when it names an advertised variant.
    pub default_client_type: String,
    /// Client variants and their component definitions.
    pub client_types: Vec<ClientType>,
    /// Executable metadata for the payload; the launcher does not run these files.
    pub executables: Vec<Executable>,
    /// Publisher identifiers advertised in metadata.
    pub publisher_ids: Vec<String>,
}

impl GameMetadata {
    /// Finds a client variant by its exact, case-sensitive identifier.
    ///
    /// Returns `None` when the publisher did not advertise that identifier.
    pub fn client_type(&self, id: &str) -> Option<&ClientType> {
        self.client_types.iter().find(|client| client.id == id)
    }
}

/// A publisher client variant and the components it requires.
#[derive(Debug, Clone, Serialize)]
pub struct ClientType {
    /// Exact publisher client variant identifier.
    pub id: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Components required by this variant.
    pub parts: Vec<ClientPart>,
}

/// A component referenced by a client variant.
///
/// The flags describe publisher metadata; they do not enable local integrity scans.
#[derive(Debug, Clone, Serialize)]
pub struct ClientPart {
    /// Publisher component identifier used in patch requests.
    pub id: String,
    /// Optional publisher application type.
    pub app_type: Option<String>,
    /// Whether the publisher marks this component as language-specific.
    pub language_specific: bool,
    /// Whether the publisher marks this component for integrity checking.
    pub integrity_checked: bool,
}

/// Publisher information about a game executable; no launch behavior is provided.
#[derive(Debug, Clone, Serialize)]
pub struct Executable {
    /// Executable path as advertised by the publisher.
    pub path: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Optional publisher emulation hint.
    pub emulation: Option<String>,
}

impl GameMetadata {
    pub(crate) fn require_client_type(&self, id: &str) -> super::error::Result<&ClientType> {
        self.client_type(id)
            .ok_or_else(|| super::ReleaseError::UnknownClientType {
                requested: id.to_owned(),
                available: self
                    .client_types
                    .iter()
                    .map(|client| client.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }

    pub(crate) fn validate_language(&self, language: &str) -> super::error::Result<()> {
        if !self.supported_languages.is_empty()
            && !self.supported_languages.iter().any(|item| item == language)
        {
            return Err(super::ReleaseError::UnsupportedLanguage {
                requested: language.to_owned(),
                available: self.supported_languages.join(", "),
            });
        }
        Ok(())
    }

    pub(crate) fn publisher_default_client_type(&self) -> Option<&str> {
        self.client_type(&self.default_client_type)
            .map(|client| client.id.as_str())
    }
}
