use std::path::PathBuf;

/// Errors from catalog discovery, transport, extraction, and installation state.
///
/// High-level workflows share this error type. Match variants for structured
/// handling; display messages provide context for humans. A failed mutating operation
/// may leave cached downloads, temporary work, or partially changed game files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP client creation, request, status, or response-stream failure.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Publisher metadata or patch-chain XML could not be deserialized.
    #[error("WGUS returned invalid XML: {0}")]
    Xml(#[from] quick_xml::DeError),
    /// A publisher URL could not be parsed or resolved.
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    /// Pinned manifest or installed-build JSON could not be deserialized.
    #[error("invalid cached manifest: {0}")]
    Json(#[from] serde_json::Error),
    /// Filesystem or other I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The requested client variant was not advertised.
    #[error("unknown client type '{requested}' (available: {available})")]
    UnknownClientType {
        /// Requested identifier or build selection.
        requested: String,
        /// Human-readable list of supported identifiers.
        available: String,
    },
    /// The requested language was not advertised.
    #[error("unsupported language '{requested}' (available: {available})")]
    UnsupportedLanguage {
        /// Requested identifier or build selection.
        requested: String,
        /// Human-readable list of supported identifiers.
        available: String,
    },
    /// The publisher supplied no usable patch chain for this application.
    #[error("WGUS did not publish a patch chain for {0}")]
    MissingPatchChain(String),
    /// The publisher supplied no HTTP seed for this application.
    #[error("WGUS did not publish a web seed for {0}")]
    MissingWebSeed(String),
    /// Protocol redirects exceeded the permitted limit for this application.
    #[error("WGUS redirect limit exceeded for {0}")]
    RedirectLimit(String),
    /// A transport or archive path was unsafe for local extraction.
    #[error("unsafe asset path returned by WGUS: {0}")]
    UnsafeAssetPath(String),
    /// A completed transfer did not match the advertised byte size.
    #[error("downloaded file has the wrong size: {path:?} (expected {expected}, got {actual})")]
    SizeMismatch {
        /// Affected local file path.
        path: PathBuf,
        /// Advertised or required byte size.
        expected: u64,
        /// Observed byte size; zero can indicate a missing cache file.
        actual: u64,
    },
    /// A resumed request received an unacceptable HTTP response.
    #[error("invalid response while resuming {path:?}: HTTP {status}")]
    ResumeRejected {
        /// Affected local file path.
        path: PathBuf,
        /// HTTP status code returned by the server.
        status: u16,
    },
    /// A volume failed after exhausting its retry budget.
    #[error("download failed after {attempts} attempts: {url}: {source}")]
    DownloadRetries {
        /// Number of attempts performed.
        attempts: usize,
        /// Download URL that failed.
        url: url::Url,
        /// Final error from the last transfer attempt.
        #[source]
        source: Box<Error>,
    },
    /// No installation assets matched the component filter.
    #[error("no assets match the selected parts: {0}")]
    EmptySelection(String),
    /// A required cached volume was missing or had the wrong size.
    #[error(
        "cached asset is missing or incomplete: {path:?} (expected {expected} bytes, got {actual})"
    )]
    IncompleteCache {
        /// Affected local file path.
        path: PathBuf,
        /// Advertised or required byte size.
        expected: u64,
        /// Observed byte size; zero can indicate a missing cache file.
        actual: u64,
    },
    /// A component had no usable full-install patch starting at version `0`.
    #[error("cannot extract {part}: no full-install patch starts at version 0")]
    MissingFullPatch {
        /// Affected component identifier.
        part: String,
    },
    /// A patch volume name was absent from the resolved asset list.
    #[error("manifest asset is missing for patch volume: {0}")]
    MissingPatchAsset(String),
    /// A component patch chain revisited an already processed version.
    #[error("cyclic patch chain for {part} at version {version}")]
    BrokenPatchChain {
        /// Affected component identifier.
        part: String,
        /// Component version where the cycle was detected.
        version: String,
    },
    /// Transport archive decoding or entry extraction failed.
    #[error("package archive extraction failed: {0}")]
    ArchiveExtraction(String),
    /// A delta could not be decoded or applied.
    #[error("failed to apply {format} delta {path:?}: {detail}")]
    DeltaDecode {
        /// Delta format identifier.
        format: &'static str,
        /// Affected local file path.
        path: PathBuf,
        /// Decoder or validation failure description.
        detail: String,
    },
    /// A delta package entry did not describe a valid target path.
    #[error("invalid delta path in package: {0}")]
    InvalidDeltaPath(String),
    /// The game file required as a delta source was missing.
    #[error("delta target does not exist: {0:?}")]
    MissingDeltaTarget(PathBuf),
    /// A delta decoder unexpectedly produced an empty output file.
    #[error("delta decoder produced no output: {0:?}")]
    EmptyDeltaOutput(PathBuf),
    /// Another process holds the installation mutation lock.
    #[error("another installation is already running for {0:?}")]
    InstallationLocked(PathBuf),
    /// The installation root had no installed-build record.
    #[error("no wg-launcher installation record was found for {0:?}")]
    InstallationNotFound(PathBuf),
    /// The package cache had no pinned download manifest.
    #[error("no downloaded manifest was found in {0:?}; run `wg-launcher download` first")]
    DownloadManifestNotFound(PathBuf),
    /// A background extraction or download-coordination task failed.
    #[error("background extraction task failed: {0}")]
    ExtractionTask(String),
    /// The pinned manifest did not match the requested selection.
    #[error(
        "the cached build is {cached} but the requested build is {requested}; use matching options or a different output directory"
    )]
    CachedSelectionMismatch {
        /// Human-readable selection recorded in the pinned manifest.
        cached: String,
        /// Requested identifier or build selection.
        requested: String,
    },
}

/// Convenience result type used by all fallible launcher workflows.
pub type Result<T> = std::result::Result<T, Error>;
