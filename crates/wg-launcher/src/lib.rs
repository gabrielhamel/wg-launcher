//! Discover, download, install, and update World of Tanks assets through WGUS
//! and compatible services, including Lesta RU.
//!
//! The library owns the workflows and emits typed progress events without terminal
//! formatting. It runs on Windows, Linux, and macOS; catalog payloads currently
//! target Windows/x64. Player authentication and launching the game are outside
//! its scope.
//!
//! # Package boundary
//!
//! Product decision: this is a library-only package. Terminal parsing, tables,
//! and progress rendering live in the separate `wg-launcher-cli` package, which
//! depends on this public API. Library consumers do not depend on those terminal
//! components. The Rust import remains `wg_launcher`; a local path dependency
//! must point to `crates/wg-launcher` inside the checkout rather than its workspace
//! root. The CLI still installs the `wg-launcher` executable.
//!
//! # Start with the high-level workflows
//!
//! | Workflow | Input | Successful result |
//! | --- | --- | --- |
//! | [`list()`] | [`ListOptions`] | [`ListReport`]: builds and per-variant warnings |
//! | [`inspect()`] | [`InspectOptions`] | [`PatchManifest`]: live selection, no file changes |
//! | [`download()`] | [`DownloadAssetsOptions`] | [`DownloadAssetsOutcome`]: cached packages |
//! | [`extract()`] | [`ExtractOptions`] | [`ExtractOutcome`]: local cache installed |
//! | [`install()`] | [`InstallOptions`] | [`InstallOutcome`]: integrated pipeline |
//! | [`status()`] | [`StatusOptions`] | [`StatusReport`]: recorded/live version comparison |
//! | [`update()`] | [`UpdateOptions`] | [`UpdateOutcome`]: optional in-place installation |
//!
//! Selection defaults are EU, `sd`, and the publisher's default language.
//! [`ListOptions::default()`] queries all known regions. Path-based library
//! options require an explicit path; pass `".".into()` for the current directory.
//!
//! Add the crate and a Tokio runtime to your application:
//!
//! ```toml
//! [dependencies]
//! wg-launcher = "1.2"
//! tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
//! ```
//!
//! Inspect a build without downloading packages:
//!
//! ```no_run
//! use wg_launcher::{BuildSelection, InspectOptions, Region, inspect};
//!
//! #[tokio::main]
//! async fn main() -> wg_launcher::Result<()> {
//!     let manifest = inspect(InspectOptions {
//!         selection: BuildSelection {
//!             region: Region::Eu,
//!             client_type: "hd".into(),
//!             language: Some("FR".into()),
//!         },
//!     }).await?;
//!     println!("{}: {} compressed bytes", manifest.version, manifest.download_size());
//!     Ok(())
//! }
//! ```
//!
//! Discover variants and check an existing installation:
//!
//! ```no_run
//! use wg_launcher::{ListOptions, Region, StatusOptions, list, status};
//!
//! # async fn discover_and_check() -> wg_launcher::Result<()> {
//! let catalog = list(ListOptions { region: Some(Region::Eu) }).await?;
//! for warning in catalog.warnings {
//!     eprintln!("{}: {}", warning.client_type, warning.message);
//! }
//! for build in catalog.builds {
//!     println!("{}: {}", build.client_type, build.version);
//! }
//! let installed = status(StatusOptions { path: "./world-of-tanks".into() }).await?;
//! println!("Update available: {}", installed.update_available);
//! # Ok(())
//! # }
//! ```
//!
//! # Optional progress
//!
//! **Product decision:** the library is silent by default. Consumers that only
//! need the result should not have to create a callback or configure a UI.
//! The short workflow names take only options; opt in to typed events with
//! [`download_with_observer`], [`extract_with_observer`], [`install_with_observer`],
//! or [`update_with_observer`]. Both forms share the same operation, defaults,
//! result types, and failure behavior. The CLI uses the observer forms to render
//! progress. Neither form prints from the library.
//!
//! ```no_run
//! use wg_launcher::{InstallOptions, install};
//!
//! # async fn install_quietly() -> wg_launcher::Result<()> {
//! let outcome = install(InstallOptions {
//!     output: Some("./world-of-tanks".into()),
//!     ..Default::default()
//! }).await?;
//! println!("Installed {}", outcome.plan.manifest.version);
//! # Ok(())
//! # }
//! ```
//!
//! Install with typed progress notifications:
//!
//! ```no_run
//! use wg_launcher::{InstallEvent, InstallOptions, install_with_observer};
//!
//! #[tokio::main]
//! async fn main() -> wg_launcher::Result<()> {
//!     let outcome = install_with_observer(
//!         InstallOptions {
//!             output: Some("./world-of-tanks".into()),
//!             ..Default::default()
//!         },
//!         |event| match event {
//!             InstallEvent::Planned(plan) => println!("Destination: {}", plan.root.display()),
//!             InstallEvent::Progress(progress) => println!("{progress:?}"),
//!         },
//!     ).await?;
//!     println!("Installed version {}", outcome.plan.manifest.version);
//!     Ok(())
//! }
//! ```
//!
//! # Staged and offline installation
//!
//! [`download()`] pins the complete manifest and saves transport volumes in
//! `.wg-launcher/.download`. An empty [`DownloadAssetsOptions::parts`] selects all required
//! packages. A filter downloads only matching components; complete the package
//! set before extracting. Move the entire installation directory to transfer
//! the cache to another machine.
//!
//! [`extract()`] reads the pinned manifest locally and never queries the catalog:
//!
//! ```no_run
//! use wg_launcher::{DownloadAssetsOptions, ExtractOptions, download, extract};
//!
//! # async fn staged() -> wg_launcher::Result<()> {
//! let root = std::path::PathBuf::from("./world-of-tanks");
//! download(DownloadAssetsOptions {
//!     output: Some(root.clone()),
//!     ..Default::default()
//! }).await?;
//! // This second step can run later, offline, after moving the whole directory.
//! extract(ExtractOptions { path: root }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Observers and async execution
//!
//! Async I/O requires a Tokio runtime. Extraction runs on its blocking pool.
//! When you opt in, observer callbacks implement `Fn(Event) + Send + Sync + 'static`; use owned
//! state or an `Arc` when sharing state with the caller. Download and extraction
//! callbacks may run concurrently on different workers. Keep callbacks short,
//! avoid panics, and forward owned events to a UI or channel when appropriate.
//! There is no total ordering across concurrent files or phases. Retried transfers
//! can emit repeated starts and reset their byte counters.
//!
//! Observer workflow events wrap a plan followed by progress. [`InstallationEvent`]
//! combines [`DownloadEvent`], [`ExtractionEvent`], and package-release events.
//! An already-current observed update emits no installation events. Observer notifications
//! are informational; handle the returned [`Result`] to determine success.
//!
//! Blocking extraction already started may continue if its async future is
//! dropped. Await the operation before reusing its installation directory;
//! dropping a future is not a transactional cancellation mechanism.
//!
//! # Storage, retries, and updates
//!
//! All CLI- and library-owned installation files live under `.wg-launcher/`:
//!
//! | Path within installation | Purpose |
//! | --- | --- |
//! | `.wg-launcher/.download/.wg-manifest.json` | Pinned [`PatchManifest`] |
//! | `.wg-launcher/.download/` | Complete and partial transport volumes |
//! | `.wg-launcher/.installing/` | Temporary archive and delta work |
//! | `.wg-launcher/.wg-launcher.json` | Serialized [`InstalledBuild`] identity |
//! | `.wg-launcher/.wg-launcher.lock` | Extraction and integrated installation mutation lock |
//!
//! All files created by the CLI or library for an installation, including future
//! metadata, logs, caches, locks, temporary outputs, and replacement backups, belong
//! under `<installation>/.wg-launcher/`. Game payload files remain in the game tree.
//! The `.wg-launcher` directory is reserved and archive entries cannot write into it.
//! Successful cleanup removes only its cache and temporary work; identity and lock
//! remain so the installation stays portable and usable by `status` and `update`.
//!
//! The exported storage path constants include `.wg-launcher/`; join them directly
//! to the installation root. [`DOWNLOAD_MANIFEST_FILE`] is a cache filename.
//!
//! [`install()`] overlaps later downloads with ordered patch extraction. A patch's
//! packages are released only after successful extraction. Successful high-level
//! installation/extraction removes cache and staging data. Transport `.wgpkg` and
//! `.dspkg` archives are unpacked; inner game-owned `.pkg` files remain intact.
//!
//! Retry with the same explicit destination and selection. A matching pinned
//! manifest takes precedence over a newer catalog build. Complete files are reused
//! by size, and partial transfers resume when supported. Content hashes are not
//! checked. Temporary work can be rebuilt; previously released packages may need
//! to be downloaded again. Errors can leave partially changed game files; the
//! operation does not roll back the entire installation. Package-only downloads
//! and lower-level transfers do not acquire the extraction lock, so applications
//! must serialize competing access to a destination.
//!
//! [`status()`] uses the recorded selection to query the live catalog. [`update()`]
//! keeps the installation path, prefers pending pinned state for retries, and
//! returns `installation: None` when the compared version is already installed:
//!
//! ```no_run
//! use wg_launcher::{DownloadOptions, UpdateOptions, update};
//!
//! # async fn update_existing() -> wg_launcher::Result<()> {
//! let result = update(UpdateOptions {
//!     path: "./world-of-tanks".into(),
//!     download: DownloadOptions::default(),
//! }).await?;
//! if let Some(installation) = result.installation {
//!     println!("Installed {}", installation.plan.manifest.version);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Updates follow the publisher's installation chain. Individual component versions
//! are not persisted, so updates are not fully minimal component-by-component deltas.
//! The default folder name records its initial build; the installed-build record
//! identifies its current version. A version comparison does not verify game files.
//!
//! # Lower-level APIs and errors
//!
//! [`WgusClient`] exposes live metadata and manifest resolution. [`AssetDownloader`]
//! transfers resolved volumes. [`installation_assets()`] orders the installation
//! assets, and [`PackageExtractor`] extracts or streams a supplied manifest with
//! configurable [`ExtractionOptions`]. These building blocks do not provide the
//! entire high-level manifest-persistence lifecycle by themselves. Their primary
//! methods also omit progress callbacks; opt in with
//! [`AssetDownloader::download_with_observer`], [`PackageExtractor::extract_with_observer`],
//! [`PackageExtractor::extract_with_options_and_observer`], or
//! [`PackageExtractor::download_and_extract_with_observer`].
//!
//! # API migration
//!
//! Callers of the earlier callback-taking workflow names should rename those
//! calls to the corresponding `_with_observer` function, preserving their callback.
//! To stop receiving events, remove the callback and keep the short name.
//! The same convention applies to lower-level methods; the configured extraction
//! variant is named `extract_with_options_and_observer`.
//!
//! Fallible operations return [`Result<T>`] with a structured [`Error`]. Each
//! operation documents its failure conditions. Discovery can return per-variant
//! [`ListWarning`] values alongside successful builds; metadata discovery failures
//! still stop the operation.

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

mod catalog;
mod download;
mod error;
mod install;
mod launcher;
mod model;
mod service;

pub use catalog::{Product, Region};
pub use download::{AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport};
pub use error::{Error, Result};
pub use install::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, ExtractionEvent, ExtractionOptions,
    ExtractionReport, INSTALL_MARKER_FILE, InstallationEvent, InstallationReport, InstalledBuild,
    PackageExtractor, STAGING_DIRECTORY, installation_assets,
};
pub use launcher::{
    BuildSelection, BuildSummary, DownloadAssetsEvent, DownloadAssetsOptions,
    DownloadAssetsOutcome, ExtractEvent, ExtractOptions, ExtractOutcome, InspectOptions,
    InstallEvent, InstallOptions, InstallOutcome, InstallPlan, ListOptions, ListReport,
    ListWarning, StatusOptions, StatusReport, UpdateOptions, UpdateOutcome, download,
    download_with_observer, extract, extract_with_observer, inspect, install,
    install_with_observer, list, status, update, update_with_observer,
};
pub use model::{
    Asset, ClientPart, ClientType, Executable, GameMetadata, Patch, PatchFile, PatchManifest,
    WebSeed,
};
pub use service::{PROTOCOL_VERSION, WgusClient};
