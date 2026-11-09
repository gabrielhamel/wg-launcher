use crate::ErrorKind;

/// Failures owned by release selection and patch planning.
#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    /// The requested client variant was not advertised.
    #[error("unknown client type '{requested}' (available: {})", .available.join(", "))]
    UnknownClientType {
        /// Requested identifier or build selection.
        requested: String,
        /// Supported identifiers in publisher order.
        available: Vec<String>,
    },
    /// The requested language was not advertised.
    #[error("unsupported language '{requested}' (available: {})", .available.join(", "))]
    UnsupportedLanguage {
        /// Requested identifier or build selection.
        requested: String,
        /// Supported identifiers in publisher order.
        available: Vec<String>,
    },
    /// A selected component has no published target version.
    #[error("cannot plan {part}: the release has no target version for this component")]
    MissingComponentTarget {
        /// Affected component identifier.
        part: String,
    },
    /// The publisher's chain cannot reach the selected component target.
    #[error(
        "cannot update {part} from {version} to {target}: no applicable patch; obtain a complete installation chain"
    )]
    MissingUpdatePatch {
        /// Affected component identifier.
        part: String,
        /// Component version with no successor.
        version: String,
        /// Required component version.
        target: String,
    },
    /// A patch volume name was absent from the resolved asset list.
    #[error("manifest asset is missing for patch volume: {0}")]
    MissingPatchAsset(String),
    /// A selected volume has ambiguous sources or disagrees with its patch declaration.
    #[error("inconsistent or ambiguous download source for patch volume: {0}")]
    ConflictingPatchAsset(String),
    /// More than one published patch starts at a required component version.
    #[error("ambiguous patch chain for {part} at version {version}: multiple patches start here")]
    AmbiguousPatchTransition {
        /// Affected component identifier.
        part: String,
        /// Source version shared by the conflicting patches.
        version: String,
    },
    /// A component patch chain revisited an already processed version.
    #[error("cyclic patch chain for {part} at version {version}")]
    BrokenPatchChain {
        /// Affected component identifier.
        part: String,
        /// Component version where the cycle was detected.
        version: String,
    },
}

pub(crate) type Result<T> = std::result::Result<T, ReleaseError>;

impl ReleaseError {
    pub(crate) fn kind(&self) -> ErrorKind {
        match self {
            ReleaseError::UnknownClientType { .. } | ReleaseError::UnsupportedLanguage { .. } => {
                ErrorKind::InvalidSelection
            }
            ReleaseError::MissingComponentTarget { .. }
            | ReleaseError::MissingUpdatePatch { .. }
            | ReleaseError::MissingPatchAsset(_)
            | ReleaseError::ConflictingPatchAsset(_)
            | ReleaseError::AmbiguousPatchTransition { .. }
            | ReleaseError::BrokenPatchChain { .. } => ErrorKind::InvalidResponse,
        }
    }
}

impl From<ReleaseError> for crate::Error {
    fn from(error: ReleaseError) -> Self {
        Self::new(error.kind(), error)
    }
}
