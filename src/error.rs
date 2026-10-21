use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("WGUS returned invalid XML: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("invalid cached manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the operating system did not provide user cache and data directories")]
    UserDirectoriesUnavailable,
    #[error("unknown client type '{requested}' (available: {available})")]
    UnknownClientType {
        requested: String,
        available: String,
    },
    #[error("unsupported language '{requested}' (available: {available})")]
    UnsupportedLanguage {
        requested: String,
        available: String,
    },
    #[error("WGUS did not publish a patch chain for {0}")]
    MissingPatchChain(String),
    #[error("WGUS did not publish a web seed for {0}")]
    MissingWebSeed(String),
    #[error("WGUS redirect limit exceeded for {0}")]
    RedirectLimit(String),
    #[error("unsafe asset path returned by WGUS: {0}")]
    UnsafeAssetPath(String),
    #[error("downloaded file has the wrong size: {path:?} (expected {expected}, got {actual})")]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("invalid response while resuming {path:?}: HTTP {status}")]
    ResumeRejected { path: PathBuf, status: u16 },
    #[error("download failed after {attempts} attempts: {url}: {source}")]
    DownloadRetries {
        attempts: usize,
        url: url::Url,
        #[source]
        source: Box<Error>,
    },
    #[error("no assets match the selected parts: {0}")]
    EmptySelection(String),
    #[error(
        "cached asset is missing or incomplete: {path:?} (expected {expected} bytes, got {actual})"
    )]
    IncompleteCache {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("cannot extract {part}: no full-install patch starts at version 0")]
    MissingFullPatch { part: String },
    #[error("manifest asset is missing for patch volume: {0}")]
    MissingPatchAsset(String),
    #[error("cyclic patch chain for {part} at version {version}")]
    BrokenPatchChain { part: String, version: String },
    #[error("package archive extraction failed: {0}")]
    ArchiveExtraction(String),
    #[error("failed to apply {format} delta {path:?}: {detail}")]
    DeltaDecode {
        format: &'static str,
        path: PathBuf,
        detail: String,
    },
    #[error("invalid delta path in package: {0}")]
    InvalidDeltaPath(String),
    #[error("delta target does not exist: {0:?}")]
    MissingDeltaTarget(PathBuf),
    #[error("delta decoder produced no output: {0:?}")]
    EmptyDeltaOutput(PathBuf),
    #[error("another installation is already running for {0:?}")]
    InstallationLocked(PathBuf),
    #[error("no wg-launcher installation record was found for {0:?}")]
    InstallationNotFound(PathBuf),
    #[error("background extraction task failed: {0}")]
    ExtractionTask(String),
    #[error(
        "the cached build is {cached} but the requested build is {requested}; use matching options or a different output directory"
    )]
    CachedSelectionMismatch { cached: String, requested: String },
}

pub type Result<T> = std::result::Result<T, Error>;
