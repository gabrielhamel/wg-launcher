use crate::{
    Asset, AssetDownloader, DOWNLOAD_MANIFEST_FILE, DownloadEvent, DownloadOptions, DownloadReport,
    ExtractionEvent, ExtractionReport, InstallationEvent, InstallationReport, InstalledBuild,
    PackageExtractor, PatchManifest, Region, Result, WgusClient, install::installation_storage,
    installation_assets,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Selects a catalog build.
///
/// The default is EU, client type `sd`, and the publishing region's default language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSelection {
    /// Publisher catalog region.
    pub region: Region,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Requested language; `None` uses the publisher default.
    pub language: Option<String>,
}

impl Default for BuildSelection {
    fn default() -> Self {
        Self {
            region: Region::Eu,
            client_type: "sd".to_owned(),
            language: None,
        }
    }
}

/// Configures discovery. By default every supported region is queried.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListOptions {
    /// Restrict discovery to one region, or query all regions with `None`.
    pub region: Option<Region>,
}

/// One installable client variant discovered by [`list`].
#[derive(Debug, Clone, Serialize)]
pub struct BuildSummary {
    /// Publisher catalog region.
    pub region: Region,
    /// Publisher release channel.
    pub channel: String,
    /// Game payload platform, independent of the launcher host OS.
    pub platform: String,
    /// Game payload architecture, or `None` when not published.
    pub architecture: Option<String>,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Publisher game build version.
    pub version: String,
    /// Publisher metadata revision, distinct from the game version.
    pub metadata_version: String,
    /// Publisher default language used for size resolution.
    pub default_language: String,
    /// Advertised normalized language identifiers.
    pub languages: Vec<String>,
    /// Required component identifiers.
    pub parts: Vec<String>,
    /// Compressed byte total for the ordered installation assets in the default language.
    pub compressed_bytes: u64,
}

/// A client variant that could not be resolved or ordered during discovery.
#[derive(Debug, Clone, Serialize)]
pub struct ListWarning {
    /// Publisher catalog region.
    pub region: Region,
    /// Publisher client variant identifier, such as `sd` or `hd`.
    pub client_type: String,
    /// Human-readable reason the variant is unavailable.
    pub message: String,
}

/// Discovered builds and per-variant failures.
#[derive(Debug, Clone, Serialize)]
pub struct ListReport {
    /// Successfully resolved, installable variants.
    pub builds: Vec<BuildSummary>,
    /// Variants omitted because manifest resolution or patch ordering failed.
    pub warnings: Vec<ListWarning>,
}

/// Discovers installable variants using the live catalog, without writing files.
///
/// Variant failures are collected in [`ListReport::warnings`] so other variants
/// can still be returned.
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be created or a region's metadata
/// cannot be fetched or parsed. Such a failure stops discovery.
pub async fn list(options: ListOptions) -> Result<ListReport> {
    list_with_client(&WgusClient::new()?, options).await
}

/// Selects a build whose resolved WGUS manifest should be inspected.
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    /// Build to resolve; defaults to EU/SD with the publisher default language.
    pub selection: BuildSelection,
}

/// Resolves a live build without downloading packages or changing files.
///
/// # Errors
///
/// Returns an error for failed network requests, invalid protocol responses,
/// unsupported client types or languages, or missing patch chains or web seeds.
pub async fn inspect(options: InspectOptions) -> Result<PatchManifest> {
    resolve(&WgusClient::new()?, &options.selection).await
}

/// Configures a package-only download. The complete manifest is pinned inside
/// `<root>/.download` even when `parts` limits the transferred package set.
#[derive(Debug, Clone, Default)]
pub struct DownloadAssetsOptions {
    /// Build to download; defaults to EU/SD and the publisher default language.
    pub selection: BuildSelection,
    /// Installation root; `None` derives a relative path from the resolved build.
    /// An explicit path allows a pinned manifest to be reused before live resolution.
    pub output: Option<PathBuf>,
    /// Exact component identifiers to transfer; empty selects the complete installation.
    /// The complete manifest is pinned even when only some components are transferred.
    pub parts: Vec<String>,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Package-only download observer event.
///
/// A single plan precedes progress.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum DownloadAssetsEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`DownloadEvent`] stream.
    Progress(DownloadEvent),
}

/// Plan and transfer totals from a successful package-only download.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadAssetsOutcome {
    /// Resolved build, destination, and selected assets.
    pub plan: InstallPlan,
    /// Successful transfers and cache hits.
    pub report: DownloadReport,
}

/// Runs [`download_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`download_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`download_with_observer`].
pub async fn download(options: DownloadAssetsOptions) -> Result<DownloadAssetsOutcome> {
    download_with_observer(options, |_| {}).await
}

/// Downloads packages into the installation's `.download` directory without extracting.
///
/// Reuses a matching pinned manifest, complete cached files, and resumable partial
/// files. Does not write an installed-build record or clean the cache. A filtered
/// download is insufficient for extraction until all required packages are present.
/// This package-only workflow does not acquire the extraction installation lock;
/// callers must serialize access to the same destination.
///
/// The observer receives a plan before progress. Callbacks may run concurrently;
/// see the [crate-level observer contract](crate#observers-and-async-execution).
///
/// # Errors
///
/// Returns an error for discovery or cache I/O failures, a pinned selection mismatch,
/// an invalid patch chain, an empty asset selection, unsafe paths, or exhausted
/// transfer retries. Cached and partial files may remain after failure.
pub async fn download_with_observer<F>(
    options: DownloadAssetsOptions,
    observer: F,
) -> Result<DownloadAssetsOutcome>
where
    F: Fn(DownloadAssetsEvent) + Send + Sync + 'static,
{
    let client = WgusClient::new()?;
    let (manifest, root) =
        resolve_installation(&client, &options.selection, options.output).await?;
    let all_assets = installation_assets(&manifest)?;
    let assets = select_download_assets(all_assets, &options.parts);
    if assets.is_empty() {
        return Err(crate::Error::EmptySelection(options.parts.join(", ")));
    }
    let plan = build_plan(manifest, root, assets)?;
    observer(DownloadAssetsEvent::Planned(Box::new(plan.clone())));
    save_cache_manifest(&plan.manifest, &plan.cache).await?;
    let report = AssetDownloader::new()?
        .download_with_observer(
            plan.assets.clone(),
            &plan.cache,
            options.download,
            move |event| {
                observer(DownloadAssetsEvent::Progress(event));
            },
        )
        .await?;
    Ok(DownloadAssetsOutcome { plan, report })
}

fn select_download_assets(assets: Vec<Asset>, parts: &[String]) -> Vec<Asset> {
    if parts.is_empty() {
        return assets;
    }
    assets
        .into_iter()
        .filter(|asset| parts.iter().any(|part| part == &asset.part))
        .collect()
}

/// Configures extraction of the complete pinned package cache.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Installation root containing a complete `.download` cache and pinned manifest.
    /// Use `".".into()` explicitly to select the current directory.
    pub path: PathBuf,
}

/// Local extraction observer event.
///
/// A single plan precedes progress.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum ExtractEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`ExtractionEvent`] stream.
    Progress(ExtractionEvent),
}

/// Plan and results from successful local extraction.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractOutcome {
    /// Pinned build and required cached assets.
    pub plan: InstallPlan,
    /// Installed file count, applied deltas, and cleanup result.
    pub report: ExtractionReport,
}

/// Runs [`extract_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`extract_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`extract_with_observer`].
pub async fn extract(options: ExtractOptions) -> Result<ExtractOutcome> {
    extract_with_observer(options, |_| {}).await
}

/// Installs the complete pinned package cache without contacting the live catalog.
///
/// Reads `.download/.wg-manifest.json`, acquires the installation lock, extracts
/// packages, applies deltas, and writes the installed-build record. Success removes
/// `.download` and `.installing`. Inner game-owned `.pkg` files remain intact.
/// The observer receives the plan before extraction progress.
///
/// # Errors
///
/// Returns an error if the manifest is missing or invalid, required volumes are
/// incomplete, the patch chain is invalid, the installation is locked, or filesystem,
/// archive, delta, or background-task operations fail. Failed extraction may leave
/// partially changed game files and temporary work; it is not a transactional rollback.
pub async fn extract_with_observer<F>(
    options: ExtractOptions,
    observer: F,
) -> Result<ExtractOutcome>
where
    F: Fn(ExtractEvent) + Send + Sync + 'static,
{
    let root = std::path::absolute(options.path)?;
    let cache = installation_storage(&root)?.cache;
    let manifest = load_cache_manifest(&cache)
        .await?
        .ok_or_else(|| crate::Error::DownloadManifestNotFound(cache.clone()))?;
    let assets = installation_assets(&manifest)?;
    let plan = build_plan(manifest, root, assets)?;
    observer(ExtractEvent::Planned(Box::new(plan.clone())));
    let report = PackageExtractor::new()
        .extract_with_observer(plan.manifest.clone(), &plan.root, move |event| {
            observer(ExtractEvent::Progress(event));
        })
        .await?;
    Ok(ExtractOutcome { plan, report })
}

async fn list_with_client(client: &WgusClient, options: ListOptions) -> Result<ListReport> {
    let regions = options
        .region
        .map_or_else(|| Region::ALL.to_vec(), |region| vec![region]);
    let mut builds = Vec::new();
    let mut warnings = Vec::new();

    for region in regions {
        let metadata = client.metadata(region).await?;
        for client_type in &metadata.client_types {
            match client
                .manifest(region, &client_type.id, Some(&metadata.default_language))
                .await
            {
                Ok(manifest) => match installation_assets(&manifest) {
                    Ok(assets) => builds.push(BuildSummary {
                        region,
                        channel: manifest.channel,
                        platform: manifest.platform,
                        architecture: manifest.architecture,
                        client_type: manifest.client_type,
                        version: manifest.version,
                        metadata_version: manifest.metadata_version,
                        default_language: metadata.default_language.clone(),
                        languages: manifest.available_languages,
                        parts: manifest.parts,
                        compressed_bytes: assets.iter().map(|asset| asset.size).sum(),
                    }),
                    Err(error) => warnings.push(ListWarning {
                        region,
                        client_type: client_type.id.clone(),
                        message: error.to_string(),
                    }),
                },
                Err(error) => warnings.push(ListWarning {
                    region,
                    client_type: client_type.id.clone(),
                    message: error.to_string(),
                }),
            }
        }
    }

    Ok(ListReport { builds, warnings })
}

/// Configures the integrated installation pipeline.
///
/// Defaults to EU/SD, the publisher default language, an automatically named
/// directory, and [`DownloadOptions::default`].
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Build selection to resolve or match against the pinned cache.
    pub selection: BuildSelection,
    /// Installation root; `None` derives the name from the live manifest.
    /// Supply the same explicit root when resuming a build after the live version changes.
    pub output: Option<PathBuf>,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Resolved build and filesystem plan emitted before transfer or extraction.
///
/// A plan event is informational; it does not guarantee the operation will succeed.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    /// Complete pinned build manifest, even for a filtered download.
    pub manifest: PatchManifest,
    /// Installation root, which can be relative to the current directory.
    pub root: PathBuf,
    /// Package cache directory beneath the root; relative when the root is relative.
    pub cache: PathBuf,
    /// Selected transport volumes in installation order.
    pub assets: Vec<Asset>,
    /// Total compressed bytes in `assets`, before subtracting cached data.
    pub compressed_bytes: u64,
}

/// Integrated installation or update observer event.
///
/// A single plan precedes progress. An already-current update emits no events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum InstallEvent {
    /// Resolved build and destination, emitted before work begins.
    Planned(Box<InstallPlan>),
    /// Progress from the underlying [`InstallationEvent`] stream.
    Progress(InstallationEvent),
}

/// Plan and results from a successful integrated installation.
#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    /// Resolved build, destination, and ordered assets.
    pub plan: InstallPlan,
    /// Transfer, extraction, and package-release totals.
    pub report: InstallationReport,
}

/// Selects an existing installation for a live version comparison.
#[derive(Debug, Clone)]
pub struct StatusOptions {
    /// Installation root containing the installed-build record; use `".".into()`
    /// explicitly for the current directory.
    pub path: PathBuf,
}

/// Recorded build identity and the version it was compared against.
///
/// [`status`] compares with the live catalog. [`update`] may instead compare with
/// a pending pinned manifest to preserve retry consistency.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    /// Absolute installation root.
    pub path: PathBuf,
    /// Identity loaded before any update.
    pub installed: InstalledBuild,
    /// Live catalog version, or pending pinned version during update retries.
    pub latest_version: String,
    /// Whether recorded and compared versions differ; versions are not ordered
    /// semantically and game files are not scanned for integrity.
    pub update_available: bool,
}

/// Configures an in-place update using the installation's recorded selection.
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    /// Existing installation root; use `".".into()` explicitly for the current directory.
    pub path: PathBuf,
    /// Transfer concurrency and retry policy.
    pub download: DownloadOptions,
}

/// Version comparison and optional installation result.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateOutcome {
    /// Status before updating, compared with the live or pending pinned version.
    pub status: StatusReport,
    /// Installation result, or `None` when the compared version was already installed.
    /// An already-current update emits no installation events.
    pub installation: Option<InstallOutcome>,
}

/// Runs [`install_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`install_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`install_with_observer`].
pub async fn install(options: InstallOptions) -> Result<InstallOutcome> {
    install_with_observer(options, |_| {}).await
}

/// Downloads and installs a build with overlapping transfer and extraction.
///
/// Pins the manifest, acquires the installation lock for the transfer/extraction
/// pipeline, and reuses cached volumes. Later downloads can overlap ordered patch
/// extraction; packages are removed only after their patch succeeds. Success writes
/// [`INSTALL_MARKER_FILE`](crate::INSTALL_MARKER_FILE) and removes cache and staging data.
///
/// The observer receives a plan, then interleaved download and extraction events.
/// See the [observer contract](crate#observers-and-async-execution).
///
/// # Errors
///
/// Returns an error for discovery, pinned selection mismatch, invalid patch chains,
/// lock contention, transfer, filesystem, archive, or delta failures. Pending downloads
/// are stopped after extraction failure. There is no whole-install rollback; retries
/// can rebuild temporary work and may redownload already released packages.
pub async fn install_with_observer<F>(
    options: InstallOptions,
    observer: F,
) -> Result<InstallOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let client = WgusClient::new()?;
    let (manifest, root) =
        resolve_installation(&client, &options.selection, options.output).await?;
    let assets = installation_assets(&manifest)?;
    let plan = build_plan(manifest, root, assets)?;
    observer(InstallEvent::Planned(Box::new(plan.clone())));

    let extractor = PackageExtractor::new();
    save_cache_manifest(&plan.manifest, &plan.cache).await?;
    let report = extractor
        .download_and_extract_with_observer(
            AssetDownloader::new()?,
            plan.manifest.clone(),
            &plan.root,
            options.download,
            Default::default(),
            move |event| observer(InstallEvent::Progress(event)),
        )
        .await?;

    Ok(InstallOutcome { plan, report })
}

/// Loads the installed-build record and compares it with the live catalog.
///
/// Uses the recorded region, client type, and language. Does not modify files,
/// consult pending manifests, or verify installed file integrity.
///
/// # Errors
///
/// Returns an error if the installation record is missing, unreadable, or invalid,
/// or live manifest resolution fails.
pub async fn status(options: StatusOptions) -> Result<StatusReport> {
    let path = std::path::absolute(options.path)?;
    let installed = load_installed_build(&path).await?;
    let latest = WgusClient::new()?
        .manifest(
            installed.region,
            &installed.client_type,
            Some(&installed.language),
        )
        .await?;
    Ok(StatusReport {
        path,
        update_available: installed.version != latest.version,
        latest_version: latest.version,
        installed,
    })
}

/// Runs [`update_with_observer`] without progress notifications.
///
/// Use this entry point when only the final result matters. To receive typed
/// progress events, call [`update_with_observer`] instead. Neither entry point
/// prints terminal output.
///
/// # Errors
///
/// Returns the same errors as [`update_with_observer`].
pub async fn update(options: UpdateOptions) -> Result<UpdateOutcome> {
    update_with_observer(options, |_| {}).await
}

/// Updates an existing installation in place using its recorded build selection.
///
/// A pending pinned manifest takes precedence over the live catalog. If its version
/// (or the live version when no manifest is pinned) equals the installed version,
/// returns without installation or observer events. Otherwise delegates to [`install_with_observer`]
/// using the same directory, preserving the original directory name.
///
/// Updates follow the publisher's installation chain. Per-component installed
/// versions are not stored, so this is not a fully minimal component delta update.
///
/// # Errors
///
/// Returns an error for missing or invalid state, a pending selection mismatch,
/// failed catalog resolution, or any error from [`install_with_observer`].
pub async fn update_with_observer<F>(options: UpdateOptions, observer: F) -> Result<UpdateOutcome>
where
    F: Fn(InstallEvent) + Send + Sync + 'static,
{
    let path = std::path::absolute(options.path)?;
    let storage = installation_storage(&path)?;
    let installed = load_installed_build(&path).await?;
    let status = if let Some(pending) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(
            &pending,
            &BuildSelection {
                region: installed.region,
                client_type: installed.client_type.clone(),
                language: Some(installed.language.clone()),
            },
        )?;
        StatusReport {
            path,
            update_available: installed.version != pending.version,
            latest_version: pending.version,
            installed,
        }
    } else {
        status(StatusOptions { path }).await?
    };
    if !status.update_available {
        return Ok(UpdateOutcome {
            status,
            installation: None,
        });
    }
    let installation = install_with_observer(
        InstallOptions {
            selection: BuildSelection {
                region: status.installed.region,
                client_type: status.installed.client_type.clone(),
                language: Some(status.installed.language.clone()),
            },
            output: Some(status.path.clone()),
            download: options.download,
        },
        observer,
    )
    .await?;
    Ok(UpdateOutcome {
        status,
        installation: Some(installation),
    })
}

async fn load_installed_build(root: &Path) -> Result<InstalledBuild> {
    let marker = installation_storage(root)?.marker;
    match tokio::fs::read(&marker).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(crate::Error::InstallationNotFound(root.to_path_buf()))
        }
        Err(error) => Err(error.into()),
    }
}

async fn resolve_installation(
    client: &WgusClient,
    selection: &BuildSelection,
    requested_root: Option<PathBuf>,
) -> Result<(PatchManifest, PathBuf)> {
    if let Some(root) = requested_root {
        let storage = installation_storage(&root)?;
        if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
            validate_cached_selection(&manifest, selection)?;
            return Ok((manifest, root));
        }
        return Ok((resolve(client, selection).await?, root));
    }

    let live = resolve(client, selection).await?;
    let root = live.default_output_directory();
    let storage = installation_storage(&root)?;
    if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(&manifest, selection)?;
        return Ok((manifest, root));
    }
    Ok((live, root))
}

fn build_plan(manifest: PatchManifest, root: PathBuf, assets: Vec<Asset>) -> Result<InstallPlan> {
    let compressed_bytes = assets.iter().map(|asset| asset.size).sum();
    let cache = installation_storage(&root)?.cache;
    Ok(InstallPlan {
        manifest,
        root,
        cache,
        assets,
        compressed_bytes,
    })
}

async fn resolve(client: &WgusClient, selection: &BuildSelection) -> Result<PatchManifest> {
    client
        .manifest(
            selection.region,
            &selection.client_type,
            selection.language.as_deref(),
        )
        .await
}

async fn load_cache_manifest(cache: &Path) -> Result<Option<PatchManifest>> {
    let path = cache.join(DOWNLOAD_MANIFEST_FILE);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn save_cache_manifest(manifest: &PatchManifest, cache: &Path) -> Result<()> {
    tokio::fs::create_dir_all(cache).await?;
    let mut bytes = serde_json::to_vec_pretty(manifest)?;
    bytes.push(b'\n');
    let target = cache.join(DOWNLOAD_MANIFEST_FILE);
    let temporary = cache.join(format!("{DOWNLOAD_MANIFEST_FILE}.part"));
    tokio::fs::write(&temporary, bytes).await?;
    if cfg!(windows) && target.exists() {
        tokio::fs::remove_file(&target).await?;
    }
    tokio::fs::rename(temporary, target).await?;
    Ok(())
}

fn validate_cached_selection(manifest: &PatchManifest, selection: &BuildSelection) -> Result<()> {
    let language_matches = selection
        .language
        .as_deref()
        .is_none_or(|language| language.eq_ignore_ascii_case(&manifest.language));
    if manifest.region == selection.region
        && manifest
            .client_type
            .eq_ignore_ascii_case(&selection.client_type)
        && language_matches
    {
        return Ok(());
    }
    Err(crate::Error::CachedSelectionMismatch {
        cached: format!(
            "{}/{}/{}",
            manifest.region, manifest.client_type, manifest.language
        ),
        requested: format!(
            "{}/{}/{}",
            selection.region,
            selection.client_type,
            selection.language.as_deref().unwrap_or("region default")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use url::Url;

    fn test_directory(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wg-launcher-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    fn test_manifest() -> PatchManifest {
        PatchManifest {
            region: Region::Eu,
            channel: "production".to_owned(),
            host: "example.invalid".to_owned(),
            game_id: "WOT.EU.PRODUCTION".to_owned(),
            platform: "windows".to_owned(),
            architecture: Some("x64".to_owned()),
            client_type: "sd".to_owned(),
            language: "EN".to_owned(),
            metadata_version: "1".to_owned(),
            version: "1".to_owned(),
            available_languages: vec!["EN".to_owned()],
            publisher_ids: Vec::new(),
            parts: Vec::new(),
            patches: Vec::new(),
            web_seeds: Vec::new(),
            assets: Vec::new(),
        }
    }

    #[tokio::test]
    async fn progress_is_optional_for_staged_and_integrated_installation() {
        use std::sync::{Arc, Mutex};

        for staged in [false, true] {
            let mut installed = Vec::new();
            for observed in [false, true] {
                let root = test_directory("optional-progress");
                let cache = root.join(crate::DOWNLOAD_CACHE_DIRECTORY);
                std::fs::create_dir_all(&cache).unwrap();
                let source = root.join("payload.pkg");
                std::fs::write(&source, b"game-owned payload stays packed").unwrap();
                let package = cache.join("client.wgpkg");
                sevenz_rust::compress_to_path(&source, &package).unwrap();
                std::fs::remove_file(&source).unwrap();
                let size = std::fs::metadata(&package).unwrap().len();
                let mut manifest = test_manifest();
                manifest.parts = vec!["client".into()];
                manifest.patches = vec![crate::Patch {
                    part: "client".into(),
                    version_from: Some("0".into()),
                    version_to: "1".into(),
                    files: vec![crate::PatchFile {
                        name: "client.wgpkg".into(),
                        size,
                        unpacked_size: None,
                        diffs_size: None,
                    }],
                    torrent_hash: None,
                    torrent_urls: vec![],
                }];
                manifest.assets = vec![Asset {
                    part: "client".into(),
                    version_from: Some("0".into()),
                    version_to: "1".into(),
                    name: "client.wgpkg".into(),
                    // Complete cached volumes make this an entirely offline test.
                    url: "http://127.0.0.1:0/client.wgpkg".parse().unwrap(),
                    size,
                    unpacked_size: None,
                    diffs_size: None,
                }];
                save_cache_manifest(&manifest, &cache).await.unwrap();
                let events = Arc::new(Mutex::new(Vec::new()));
                let captured = Arc::clone(&events);
                let (download_report, extraction_report) = if staged {
                    let options = DownloadAssetsOptions {
                        output: Some(root.clone()),
                        ..Default::default()
                    };
                    let downloaded = if observed {
                        download_with_observer(options, move |event| {
                            captured.lock().unwrap().push(format!("{event:?}"));
                        })
                        .await
                    } else {
                        download(options).await
                    }
                    .unwrap();
                    // Downloading alone must preserve the cache and omit the marker.
                    assert!(package.is_file());
                    assert!(!root.join(crate::INSTALL_MARKER_FILE).exists());
                    let options = ExtractOptions { path: root.clone() };
                    let captured = Arc::clone(&events);
                    let extracted = if observed {
                        extract_with_observer(options, move |event| {
                            captured.lock().unwrap().push(format!("{event:?}"));
                        })
                        .await
                    } else {
                        extract(options).await
                    }
                    .unwrap();
                    (downloaded.report, extracted.report)
                } else {
                    let options = InstallOptions {
                        output: Some(root.clone()),
                        ..Default::default()
                    };
                    let result = if observed {
                        install_with_observer(options, move |event| {
                            captured.lock().unwrap().push(format!("{event:?}"));
                        })
                        .await
                    } else {
                        install(options).await
                    }
                    .unwrap();
                    (result.report.download, result.report.extraction)
                };
                assert_eq!(download_report.skipped_files, 1);
                assert_eq!(download_report.downloaded_bytes, 0);
                assert!(extraction_report.cache_removed);
                assert!(!cache.exists());
                assert!(!root.join(crate::STAGING_DIRECTORY).exists());
                let bytes = std::fs::read(root.join("payload.pkg")).unwrap();
                assert_eq!(bytes, b"game-owned payload stays packed");
                let record = load_installed_build(&root).await.unwrap();
                assert_eq!(record, InstalledBuild::from(&manifest));
                installed.push((bytes, record, extraction_report.installed_files));
                let events = events.lock().unwrap();
                if observed {
                    assert!(events.first().unwrap().starts_with("Planned"));
                    assert!(events.iter().any(|event| event.contains("Completed")));
                } else {
                    assert!(events.is_empty());
                }
                std::fs::remove_dir_all(root).unwrap();
            }
            assert_eq!(installed[0], installed[1]);
        }
    }

    #[tokio::test]
    async fn current_update_needs_no_observer_and_observed_update_emits_no_events() {
        let root = test_directory("optional-update-progress");
        let manifest = test_manifest();
        save_cache_manifest(&manifest, &root.join(crate::DOWNLOAD_CACHE_DIRECTORY))
            .await
            .unwrap();
        std::fs::write(
            root.join(crate::INSTALL_MARKER_FILE),
            serde_json::to_vec(&InstalledBuild::from(&manifest)).unwrap(),
        )
        .unwrap();
        let options = UpdateOptions {
            path: root.clone(),
            download: DownloadOptions::default(),
        };
        let quiet = update(options.clone()).await.unwrap();
        let observed = update_with_observer(options, |_| {
            panic!("an already-current update must not emit progress")
        })
        .await
        .unwrap();
        assert!(quiet.installation.is_none());
        assert!(observed.installation.is_none());
        assert_eq!(quiet.status.installed, observed.status.installed);
        assert!(!quiet.status.update_available);
        assert!(!observed.status.update_available);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn missing_cache_error_is_identical_with_or_without_an_observer() {
        let options = ExtractOptions {
            path: test_directory("missing-optional-progress"),
        };
        let quiet = extract(options.clone()).await.unwrap_err();
        let observed = extract_with_observer(options, |_| {
            panic!("a missing manifest must fail before emitting a plan")
        })
        .await
        .unwrap_err();
        assert!(matches!(quiet, crate::Error::DownloadManifestNotFound(_)));
        assert_eq!(quiet.to_string(), observed.to_string());
    }

    #[test]
    fn installation_storage_is_self_contained() {
        let root = test_directory("standalone-storage");
        let storage = installation_storage(&root).unwrap();

        assert_eq!(storage.cache, root.join(".download"));
        assert_eq!(storage.work, root.join(".installing"));
        assert_eq!(storage.marker, root.join(".wg-launcher.json"));
        assert_eq!(storage.lock, root.join(".wg-launcher.lock"));
    }

    #[test]
    fn cached_selection_accepts_an_unspecified_default_language() {
        let manifest = test_manifest();

        assert!(
            validate_cached_selection(
                &manifest,
                &BuildSelection {
                    language: None,
                    ..BuildSelection::default()
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn an_empty_part_selection_downloads_every_asset() {
        let assets = ["locale", "client", "sdcontent"]
            .into_iter()
            .map(|part| Asset {
                part: part.to_owned(),
                version_from: Some("0".to_owned()),
                version_to: "1".to_owned(),
                name: format!("{part}.wgpkg"),
                url: Url::parse(&format!("https://example.invalid/{part}.wgpkg")).unwrap(),
                size: 1,
                unpacked_size: None,
                diffs_size: None,
            })
            .collect::<Vec<_>>();

        let selected = select_download_assets(assets.clone(), &[]);
        assert_eq!(selected.len(), assets.len());

        let selected = select_download_assets(assets, &["locale".to_owned()]);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].part, "locale");
    }
}
