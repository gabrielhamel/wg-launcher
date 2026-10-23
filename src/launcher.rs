use crate::{
    Asset, AssetDownloader, DOWNLOAD_MANIFEST_FILE, DownloadEvent, DownloadOptions, DownloadReport,
    ExtractionEvent, ExtractionReport, InstallationEvent, InstallationReport, InstalledBuild,
    PackageExtractor, PatchManifest, Region, Result, WgusClient, install::installation_storage,
    installation_assets,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSelection {
    pub region: Region,
    pub client_type: String,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ListOptions {
    pub region: Option<Region>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildSummary {
    pub region: Region,
    pub channel: String,
    pub platform: String,
    pub architecture: Option<String>,
    pub client_type: String,
    pub version: String,
    pub metadata_version: String,
    pub default_language: String,
    pub languages: Vec<String>,
    pub parts: Vec<String>,
    pub compressed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListWarning {
    pub region: Region,
    pub client_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListReport {
    pub builds: Vec<BuildSummary>,
    pub warnings: Vec<ListWarning>,
}

pub async fn list(options: ListOptions) -> Result<ListReport> {
    list_with_client(&WgusClient::new()?, options).await
}

/// Selects a build whose resolved WGUS manifest should be inspected.
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    pub selection: BuildSelection,
}

/// Resolves a build without downloading or changing any files.
pub async fn inspect(options: InspectOptions) -> Result<PatchManifest> {
    resolve(&WgusClient::new()?, &options.selection).await
}

/// Configures a package-only download. The complete manifest is pinned inside
/// `<root>/.download` even when `parts` limits the transferred package set.
#[derive(Debug, Clone, Default)]
pub struct DownloadAssetsOptions {
    pub selection: BuildSelection,
    pub output: Option<PathBuf>,
    pub parts: Vec<String>,
    pub download: DownloadOptions,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum DownloadAssetsEvent {
    Planned(Box<InstallPlan>),
    Progress(DownloadEvent),
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadAssetsOutcome {
    pub plan: InstallPlan,
    pub report: DownloadReport,
}

/// Downloads package files into `<root>/.download` without extracting them.
pub async fn download<F>(
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
        .download(
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
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum ExtractEvent {
    Planned(Box<InstallPlan>),
    Progress(ExtractionEvent),
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractOutcome {
    pub plan: InstallPlan,
    pub report: ExtractionReport,
}

/// Extracts the complete package cache into the installation folder. On
/// success the package cache and temporary `.installing` directory are removed.
pub async fn extract<F>(options: ExtractOptions, observer: F) -> Result<ExtractOutcome>
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
        .extract(plan.manifest.clone(), &plan.root, move |event| {
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

#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    pub selection: BuildSelection,
    pub output: Option<PathBuf>,
    pub download: DownloadOptions,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub manifest: PatchManifest,
    pub root: PathBuf,
    pub cache: PathBuf,
    pub assets: Vec<Asset>,
    pub compressed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum InstallEvent {
    Planned(Box<InstallPlan>),
    Progress(InstallationEvent),
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    pub plan: InstallPlan,
    pub report: InstallationReport,
}

#[derive(Debug, Clone)]
pub struct StatusOptions {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub path: PathBuf,
    pub installed: InstalledBuild,
    pub latest_version: String,
    pub update_available: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub path: PathBuf,
    pub download: DownloadOptions,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateOutcome {
    pub status: StatusReport,
    pub installation: Option<InstallOutcome>,
}

pub async fn install<F>(options: InstallOptions, observer: F) -> Result<InstallOutcome>
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
        .download_and_extract(
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

pub async fn update<F>(options: UpdateOptions, observer: F) -> Result<UpdateOutcome>
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
    let installation = install(
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
        let manifest = PatchManifest {
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
        };

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
