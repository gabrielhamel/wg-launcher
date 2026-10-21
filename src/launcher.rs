use crate::{
    Asset, AssetDownloader, DOWNLOAD_MANIFEST_FILE, DownloadOptions, InstallationEvent,
    InstallationReport, InstalledBuild, PackageExtractor, PatchManifest, Region, Result,
    WgusClient,
    install::{LEGACY_DOWNLOAD_CACHE_DIRECTORY, LEGACY_STAGING_DIRECTORY, installation_storage},
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
    let compressed_bytes = assets.iter().map(|asset| asset.size).sum();
    let cache = installation_storage(&root)?.cache;
    let plan = InstallPlan {
        manifest,
        root,
        cache,
        assets,
        compressed_bytes,
    };
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
    let storage = installation_storage(&path)?;
    migrate_legacy_launcher_data(&path, &storage).await?;
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
    migrate_legacy_launcher_data(&path, &storage).await?;
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
        migrate_legacy_launcher_data(&root, &storage).await?;
        if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
            validate_cached_selection(&manifest, selection)?;
            return Ok((manifest, root));
        }
        return Ok((resolve(client, selection).await?, root));
    }

    let live = resolve(client, selection).await?;
    let root = live.default_output_directory();
    let storage = installation_storage(&root)?;
    migrate_legacy_launcher_data(&root, &storage).await?;
    let legacy_root = live.legacy_output_directory();
    migrate_legacy_launcher_data(&legacy_root, &storage).await?;
    if let Some(manifest) = load_cache_manifest(&storage.cache).await? {
        validate_cached_selection(&manifest, selection)?;
        return Ok((manifest, root));
    }
    Ok((live, root))
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

async fn migrate_legacy_launcher_data(
    legacy_root: &Path,
    storage: &crate::install::InstallationStorage,
) -> Result<()> {
    let legacy_cache = legacy_root.join(LEGACY_DOWNLOAD_CACHE_DIRECTORY);
    let has_legacy_data = legacy_cache.exists()
        || legacy_root.join(LEGACY_STAGING_DIRECTORY).exists()
        || legacy_root.join(".wg-launcher.json").exists()
        || legacy_root.join(".wg-launcher.lock").exists();
    if !has_legacy_data {
        return Ok(());
    }
    let legacy_root = legacy_root.to_path_buf();
    let storage = storage.clone();
    tokio::task::spawn_blocking(move || migrate_legacy_launcher_data_sync(&legacy_root, &storage))
        .await
        .map_err(|error| crate::Error::ExtractionTask(error.to_string()))?
}

fn migrate_legacy_launcher_data_sync(
    legacy_root: &Path,
    storage: &crate::install::InstallationStorage,
) -> Result<()> {
    std::fs::create_dir_all(&storage.state)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&storage.lock)?;
    match fs2::FileExt::try_lock_exclusive(&lock) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            return Err(crate::Error::InstallationLocked(legacy_root.to_path_buf()));
        }
        Err(error) => return Err(error.into()),
    }

    let legacy_cache = legacy_root.join(LEGACY_DOWNLOAD_CACHE_DIRECTORY);
    if legacy_cache.exists() {
        if let Some(parent) = storage.cache.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let moved_whole_directory =
            !storage.cache.exists() && std::fs::rename(&legacy_cache, &storage.cache).is_ok();
        if !moved_whole_directory {
            move_directory_contents(&legacy_cache, &storage.cache)?;
            std::fs::remove_dir_all(&legacy_cache)?;
        }
    }

    let legacy_staging = legacy_root.join(LEGACY_STAGING_DIRECTORY);
    if legacy_staging.exists() {
        std::fs::remove_dir_all(legacy_staging)?;
    }
    let legacy_marker = legacy_root.join(".wg-launcher.json");
    if legacy_marker.exists() {
        if let Some(parent) = storage.marker.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&legacy_marker, &storage.marker)
            .or_else(|_| std::fs::copy(&legacy_marker, &storage.marker).map(|_| ()))?;
        if legacy_marker.exists() {
            std::fs::remove_file(legacy_marker)?;
        }
    }
    drop(lock);
    let legacy_lock = legacy_root.join(".wg-launcher.lock");
    if legacy_lock.exists() {
        std::fs::remove_file(legacy_lock)?;
    }
    if legacy_root.is_dir() && std::fs::read_dir(legacy_root)?.next().is_none() {
        std::fs::remove_dir(legacy_root)?;
    }
    Ok(())
}

fn move_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            move_directory_contents(&source_path, &destination_path)?;
            std::fs::remove_dir_all(source_path)?;
        } else {
            let temporary = destination_path.with_extension("wg-launcher-migrating");
            std::fs::copy(&source_path, &temporary)?;
            if destination_path.exists() {
                std::fs::remove_file(&destination_path)?;
            }
            std::fs::rename(temporary, destination_path)?;
            std::fs::remove_file(source_path)?;
        }
    }
    Ok(())
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
    fn migrates_legacy_launcher_files_out_of_the_game_directory() {
        let root = test_directory("legacy-storage");
        let legacy_cache = root.join(LEGACY_DOWNLOAD_CACHE_DIRECTORY);
        std::fs::create_dir_all(&legacy_cache).unwrap();
        std::fs::write(legacy_cache.join("asset.wgpkg.part"), b"partial").unwrap();
        std::fs::create_dir_all(root.join(LEGACY_STAGING_DIRECTORY)).unwrap();
        std::fs::write(root.join(".wg-launcher.json"), b"installed").unwrap();
        std::fs::write(root.join(".wg-launcher.lock"), b"").unwrap();
        let storage = installation_storage(&root).unwrap();

        migrate_legacy_launcher_data_sync(&root, &storage).unwrap();

        assert_eq!(
            std::fs::read(storage.cache.join("asset.wgpkg.part")).unwrap(),
            b"partial"
        );
        assert_eq!(std::fs::read(&storage.marker).unwrap(), b"installed");
        assert!(!root.exists());

        let cache_root = storage.cache.parent().unwrap().to_path_buf();
        if storage.state.exists() {
            std::fs::remove_dir_all(&storage.state).unwrap();
        }
        if cache_root.exists() {
            std::fs::remove_dir_all(cache_root).unwrap();
        }
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
}
