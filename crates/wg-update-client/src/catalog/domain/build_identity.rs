use serde::{Deserialize, Serialize};

/// Published build identity shared by release discovery and installed records.
/// The configured product remains separate from the publisher application identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BuildIdentity {
    /// Publisher application identifier.
    #[serde(rename = "game_id")]
    pub application_id: String,
    /// Publisher catalog region.
    pub region: String,
    /// Publisher release channel.
    pub channel: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Selected normalized language identifier.
    pub language: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
}
