use crate::{
    Asset, AssetDownloader, DownloadEvent, DownloadOptions, DownloadReport, Error, Patch,
    PatchManifest, Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub const DOWNLOAD_CACHE_DIRECTORY: &str = ".download";
pub const DOWNLOAD_MANIFEST_FILE: &str = ".wg-manifest.json";
pub const STAGING_DIRECTORY: &str = ".installing";
pub const INSTALL_MARKER_FILE: &str = ".wg-launcher.json";
const INSTALL_LOCK_FILE: &str = ".wg-launcher.lock";

pub fn installation_assets(manifest: &PatchManifest) -> Result<Vec<Asset>> {
    let patches = ordered_patches(manifest)?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    Ok(assets_for_patches(manifest, &patches)?
        .into_iter()
        .flatten()
        .collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ExtractionEvent {
    CheckingCache {
        assets: usize,
    },
    ExtractingPatch {
        part: String,
        version_from: String,
        version_to: String,
        index: usize,
        total: usize,
    },
    /// Progress through a file in the current patch's transport archive.
    ExtractingFile {
        path: String,
        index: usize,
        total: usize,
        extracted: u64,
        size: u64,
    },
    ApplyingDelta {
        path: String,
    },
    InstallingFile {
        path: String,
    },
    Finalizing,
    CleaningCache,
    Completed {
        files: usize,
        deltas: usize,
        cache_removed: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExtractionReport {
    pub installed_files: usize,
    pub applied_deltas: usize,
    pub cache_removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledBuild {
    pub game_id: String,
    pub region: crate::Region,
    pub channel: String,
    pub version: String,
    pub client_type: String,
    pub language: String,
    pub platform: String,
    pub architecture: Option<String>,
    pub metadata_version: String,
}

impl From<&PatchManifest> for InstalledBuild {
    fn from(manifest: &PatchManifest) -> Self {
        Self {
            game_id: manifest.game_id.clone(),
            region: manifest.region,
            channel: manifest.channel.clone(),
            version: manifest.version.clone(),
            client_type: manifest.client_type.clone(),
            language: manifest.language.clone(),
            platform: manifest.platform.clone(),
            architecture: manifest.architecture.clone(),
            metadata_version: manifest.metadata_version.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", content = "detail", rename_all = "snake_case")]
pub enum InstallationEvent {
    Download(DownloadEvent),
    Extraction(ExtractionEvent),
    ReleasedPackages {
        part: String,
        files: usize,
        bytes: u64,
    },
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct InstallationReport {
    pub download: DownloadReport,
    pub extraction: ExtractionReport,
    pub released_package_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtractionOptions {
    pub clean_download_cache: bool,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            clean_download_cache: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InstallationStorage {
    pub cache: PathBuf,
    pub work: PathBuf,
    pub marker: PathBuf,
    pub lock: PathBuf,
}

pub(crate) fn installation_storage(install_root: &Path) -> Result<InstallationStorage> {
    Ok(InstallationStorage {
        cache: install_root.join(DOWNLOAD_CACHE_DIRECTORY),
        work: install_root.join(STAGING_DIRECTORY),
        marker: install_root.join(INSTALL_MARKER_FILE),
        lock: install_root.join(INSTALL_LOCK_FILE),
    })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PackageExtractor;

impl PackageExtractor {
    pub const fn new() -> Self {
        Self
    }

    pub async fn extract<F>(
        &self,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        self.extract_with_options(
            manifest,
            install_root,
            ExtractionOptions::default(),
            observer,
        )
        .await
    }

    pub async fn extract_with_options<F>(
        &self,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
        options: ExtractionOptions,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        let install_root = install_root.as_ref().to_path_buf();
        let storage = installation_storage(&install_root)?;
        let _lock = InstallationLock::acquire(&storage, &install_root)?;
        let observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = Arc::new(observer);
        tokio::task::spawn_blocking(move || {
            extract_sync(&manifest, &install_root, &storage, options, observer)
        })
        .await
        .map_err(|error| Error::ExtractionTask(error.to_string()))?
    }

    pub async fn download_and_extract<F>(
        &self,
        downloader: AssetDownloader,
        manifest: PatchManifest,
        install_root: impl AsRef<Path>,
        download_options: DownloadOptions,
        extraction_options: ExtractionOptions,
        observer: F,
    ) -> Result<InstallationReport>
    where
        F: Fn(InstallationEvent) + Send + Sync + 'static,
    {
        let install_root = install_root.as_ref().to_path_buf();
        let storage = installation_storage(&install_root)?;
        let _lock = InstallationLock::acquire(&storage, &install_root)?;
        let cache = storage.cache.clone();
        let patches = ordered_patches(&manifest)?
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let assets = assets_for_patches(&manifest, &patches)?;
        let first_assets = assets
            .first()
            .cloned()
            .ok_or_else(|| Error::MissingPatchChain(manifest.game_id.clone()))?;
        let observer: Arc<dyn Fn(InstallationEvent) + Send + Sync> = Arc::new(observer);
        let extraction_observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = {
            let observer = Arc::clone(&observer);
            Arc::new(move |event| observer(InstallationEvent::Extraction(event)))
        };

        let game = install_root.clone();
        let work = storage.work.clone();
        let prepare_root = install_root.clone();
        let prepare_storage = storage.clone();
        tokio::task::spawn_blocking(move || prepare_installation(&prepare_root, &prepare_storage))
            .await
            .map_err(|error| Error::ExtractionTask(error.to_string()))??;

        let total = patches.len();
        let mut report = InstallationReport::default();
        let mut pending = Some(spawn_patch_download(
            downloader.clone(),
            first_assets,
            cache.clone(),
            download_options.clone(),
            Arc::clone(&observer),
        ));

        for (index, patch) in patches.into_iter().enumerate() {
            let downloaded = pending
                .take()
                .expect("one pending patch download")
                .await
                .map_err(|error| Error::ExtractionTask(error.to_string()))??;
            merge_download_report(&mut report.download, downloaded);

            if index + 1 < assets.len() {
                pending = Some(spawn_patch_download(
                    downloader.clone(),
                    assets[index + 1].clone(),
                    cache.clone(),
                    download_options.clone(),
                    Arc::clone(&observer),
                ));
            }

            let cache_for_patch = cache.clone();
            let game_for_patch = game.clone();
            let work_for_patch = work.clone();
            let patch_observer = Arc::clone(&extraction_observer);
            let extraction = tokio::task::spawn_blocking(move || {
                let paths = InstallationPaths {
                    cache: &cache_for_patch,
                    game: &game_for_patch,
                    work: &work_for_patch,
                };
                let deltas = extract_patch_sync(&patch, paths, index, total, &patch_observer)?;
                let released = if extraction_options.clean_download_cache {
                    remove_patch_volumes(&cache_for_patch, &patch)?
                } else {
                    0
                };
                Ok::<_, Error>((patch, deltas, released))
            });
            let result = match extraction.await {
                Ok(result) => result,
                Err(error) => {
                    if let Some(download) = pending.take() {
                        download.abort();
                    }
                    return Err(Error::ExtractionTask(error.to_string()));
                }
            };

            let (patch, deltas, released) = match result {
                Ok(result) => result,
                Err(error) => {
                    if let Some(download) = pending.take() {
                        download.abort();
                    }
                    return Err(error);
                }
            };
            report.extraction.applied_deltas += deltas;
            report.released_package_bytes += released;
            if released > 0 {
                observer(InstallationEvent::ReleasedPackages {
                    part: patch.part,
                    files: patch.files.len(),
                    bytes: released,
                });
            }
        }

        let finalize_root = install_root.clone();
        let finalize_manifest = manifest.clone();
        let finalize_storage = storage.clone();
        let finalize_observer = Arc::clone(&extraction_observer);
        report.extraction = tokio::task::spawn_blocking(move || {
            finalize_installation(
                &finalize_manifest,
                &finalize_root,
                &finalize_storage,
                extraction_options,
                report.extraction.applied_deltas,
                finalize_observer,
            )
        })
        .await
        .map_err(|error| Error::ExtractionTask(error.to_string()))??;
        Ok(report)
    }
}

fn assets_for_patches(manifest: &PatchManifest, patches: &[Patch]) -> Result<Vec<Vec<Asset>>> {
    let by_name = manifest
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset))
        .collect::<HashMap<_, _>>();
    patches
        .iter()
        .map(|patch| {
            patch
                .files
                .iter()
                .map(|file| {
                    by_name
                        .get(file.name.as_str())
                        .copied()
                        .cloned()
                        .ok_or_else(|| Error::MissingPatchAsset(file.name.clone()))
                })
                .collect()
        })
        .collect()
}

fn spawn_patch_download(
    downloader: AssetDownloader,
    assets: Vec<Asset>,
    cache: PathBuf,
    options: DownloadOptions,
    observer: Arc<dyn Fn(InstallationEvent) + Send + Sync>,
) -> tokio::task::JoinHandle<Result<DownloadReport>> {
    tokio::spawn(async move {
        downloader
            .download(assets, cache, options, move |event| {
                observer(InstallationEvent::Download(event));
            })
            .await
    })
}

fn merge_download_report(total: &mut DownloadReport, patch: DownloadReport) {
    total.downloaded_files += patch.downloaded_files;
    total.skipped_files += patch.skipped_files;
    total.downloaded_bytes += patch.downloaded_bytes;
}

fn prepare_installation(install_root: &Path, storage: &InstallationStorage) -> Result<()> {
    fs::create_dir_all(install_root)?;
    if storage.work.exists() {
        fs::remove_dir_all(&storage.work)?;
    }
    fs::create_dir_all(&storage.work)?;
    Ok(())
}

struct InstallationLock {
    file: File,
}

impl InstallationLock {
    fn acquire(storage: &InstallationStorage, install_root: &Path) -> Result<Self> {
        fs::create_dir_all(install_root)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&storage.lock)?;
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Self { file }),
            Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                Err(Error::InstallationLocked(install_root.to_path_buf()))
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for InstallationLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

#[derive(Clone, Copy)]
struct InstallationPaths<'a> {
    cache: &'a Path,
    game: &'a Path,
    work: &'a Path,
}

fn extract_patch_sync(
    patch: &Patch,
    paths: InstallationPaths<'_>,
    index: usize,
    total: usize,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<usize> {
    observer(ExtractionEvent::ExtractingPatch {
        part: patch.part.clone(),
        version_from: patch.version_from.clone().unwrap_or_else(|| "?".to_owned()),
        version_to: patch.version_to.clone(),
        index: index + 1,
        total,
    });
    if patch.version_from.as_deref() == Some("0") {
        extract_archive(patch, paths.cache, paths.game, observer)?;
        return Ok(0);
    }

    let patch_dir = paths.work.join(format!("patch-{index}"));
    fs::create_dir_all(&patch_dir)?;
    extract_archive(patch, paths.cache, &patch_dir, observer)?;
    let mut report = ExtractionReport::default();
    apply_patch_directory(&patch_dir, paths.game, observer, &mut report)?;
    fs::remove_dir_all(&patch_dir)?;
    Ok(report.applied_deltas)
}

fn remove_patch_volumes(cache: &Path, patch: &Patch) -> Result<u64> {
    let mut removed = 0;
    for file in &patch.files {
        let path = cache.join(safe_relative_path(&file.name)?);
        removed += fs::metadata(&path)?.len();
        fs::remove_file(&path)?;
        remove_empty_parents(path.parent(), cache)?;
    }
    Ok(removed)
}

fn remove_empty_parents(mut directory: Option<&Path>, cache: &Path) -> Result<()> {
    while let Some(path) = directory {
        if path == cache || !path.starts_with(cache) {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => directory = path.parent(),
            Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn finalize_installation(
    manifest: &PatchManifest,
    install_root: &Path,
    storage: &InstallationStorage,
    options: ExtractionOptions,
    applied_deltas: usize,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let mut report = ExtractionReport {
        installed_files: walk_files(install_root)?.len(),
        applied_deltas,
        cache_removed: false,
    };
    observer(ExtractionEvent::Finalizing);
    write_marker(manifest, &storage.marker)?;
    if storage.work.exists() {
        fs::remove_dir_all(&storage.work)?;
    }
    if options.clean_download_cache {
        observer(ExtractionEvent::CleaningCache);
        if storage.cache.exists() {
            fs::remove_dir_all(&storage.cache)?;
        }
        report.cache_removed = true;
    }
    observer(ExtractionEvent::Completed {
        files: report.installed_files,
        deltas: report.applied_deltas,
        cache_removed: report.cache_removed,
    });
    Ok(report)
}

fn extract_sync(
    manifest: &PatchManifest,
    install_root: &Path,
    storage: &InstallationStorage,
    options: ExtractionOptions,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let cache = &storage.cache;
    observer(ExtractionEvent::CheckingCache {
        assets: manifest.assets.len(),
    });
    let patches = ordered_patches(manifest)?;
    verify_cache(&patches, cache)?;
    prepare_installation(install_root, storage)?;

    let total = patches.len();
    let mut report = ExtractionReport::default();
    let paths = InstallationPaths {
        cache,
        game: install_root,
        work: &storage.work,
    };
    for (index, patch) in patches.into_iter().enumerate() {
        report.applied_deltas += extract_patch_sync(patch, paths, index, total, &observer)?;
        if options.clean_download_cache {
            remove_patch_volumes(cache, patch)?;
        }
    }
    finalize_installation(
        manifest,
        install_root,
        storage,
        options,
        report.applied_deltas,
        observer,
    )
}

fn verify_cache(patches: &[&Patch], cache: &Path) -> Result<()> {
    for file in patches.iter().flat_map(|patch| &patch.files) {
        let path = cache.join(safe_relative_path(&file.name)?);
        let actual = fs::metadata(&path)
            .map(|metadata| {
                if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        if actual != file.size {
            return Err(Error::IncompleteCache {
                path,
                expected: file.size,
                actual,
            });
        }
    }
    Ok(())
}

fn ordered_patches(manifest: &PatchManifest) -> Result<Vec<&Patch>> {
    let mut ordered = Vec::new();
    for part in &manifest.parts {
        let part_patches = manifest
            .patches
            .iter()
            .filter(|patch| &patch.part == part)
            .collect::<Vec<_>>();
        let full = part_patches
            .iter()
            .copied()
            .find(|patch| patch.version_from.as_deref() == Some("0"))
            .ok_or_else(|| Error::MissingFullPatch { part: part.clone() })?;
        ordered.push(full);
        let by_from = part_patches
            .into_iter()
            .filter_map(|patch| patch.version_from.as_deref().map(|from| (from, patch)))
            .collect::<HashMap<_, _>>();
        let mut version = full.version_to.as_str();
        let mut seen = HashSet::new();
        while let Some(next) = by_from.get(version).copied() {
            if !seen.insert(version.to_owned()) {
                return Err(Error::BrokenPatchChain {
                    part: part.clone(),
                    version: version.to_owned(),
                });
            }
            ordered.push(next);
            version = &next.version_to;
        }
    }
    Ok(ordered)
}

fn extract_archive(
    patch: &Patch,
    cache: &Path,
    destination: &Path,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<()> {
    let mut volumes = patch
        .files
        .iter()
        .map(|file| safe_relative_path(&file.name).map(|relative| cache.join(relative)))
        .collect::<Result<Vec<_>>>()?;
    volumes.sort();
    if volumes.is_empty() {
        return Err(Error::MissingFullPatch {
            part: patch.part.clone(),
        });
    }

    let reader = MultiVolumeReader::open(&volumes)?;
    let length = reader.length;
    let mut archive = sevenz_rust::SevenZReader::new(reader, length, Default::default())
        .map_err(|error| Error::ArchiveExtraction(error.to_string()))?;
    let total = archive
        .archive()
        .files
        .iter()
        .filter(|entry| !entry.is_directory())
        .count();
    fs::create_dir_all(destination)?;
    let mut index = 0;
    archive
        .for_each_entries(|entry, reader| {
            let relative = safe_archive_path(entry.name())
                .map_err(|error| sevenz_rust::Error::other(error.to_string()))?;
            let target = destination.join(&relative);
            if entry.is_directory() {
                return sevenz_rust::default_entry_extract_fn(entry, reader, &target);
            }
            index += 1;
            let notify = |extracted| {
                observer(ExtractionEvent::ExtractingFile {
                    path: relative.to_string_lossy().into_owned(),
                    index,
                    total,
                    extracted,
                    size: entry.size(),
                });
            };
            notify(0);
            let mut progress = ExtractionReader {
                reader,
                notify: &notify,
                extracted: 0,
                last_update: Instant::now(),
            };
            let result = sevenz_rust::default_entry_extract_fn(entry, &mut progress, &target)?;
            notify(progress.extracted);
            Ok(result)
        })
        .map_err(|error| Error::ArchiveExtraction(error.to_string()))
}

struct ExtractionReader<'a> {
    reader: &'a mut dyn Read,
    notify: &'a dyn Fn(u64),
    extracted: u64,
    last_update: Instant,
}

impl Read for ExtractionReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.reader.read(buffer)?;
        self.extracted += count as u64;
        if self.last_update.elapsed() >= Duration::from_millis(100) {
            (self.notify)(self.extracted);
            self.last_update = Instant::now();
        }
        Ok(count)
    }
}

fn apply_patch_directory(
    patch_dir: &Path,
    game: &Path,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    report: &mut ExtractionReport,
) -> Result<()> {
    let files = walk_files(patch_dir)?;
    for source in files.iter().filter(|path| delta_target(path).is_none()) {
        let relative = source
            .strip_prefix(patch_dir)
            .map_err(|_| Error::UnsafeAssetPath(source.display().to_string()))?;
        let target = game.join(relative);
        observer(ExtractionEvent::InstallingFile {
            path: relative.to_string_lossy().into_owned(),
        });
        replace_file(source, &target)?;
    }
    for delta in files.iter().filter(|path| delta_target(path).is_some()) {
        let relative = delta
            .strip_prefix(patch_dir)
            .map_err(|_| Error::UnsafeAssetPath(delta.display().to_string()))?;
        let target_relative = delta_target(relative)
            .ok_or_else(|| Error::InvalidDeltaPath(relative.display().to_string()))?;
        let target = game.join(&target_relative);
        if !target.is_file() {
            return Err(Error::MissingDeltaTarget(target));
        }
        observer(ExtractionEvent::ApplyingDelta {
            path: target_relative.to_string_lossy().into_owned(),
        });
        apply_delta(&target, delta)?;
        report.applied_deltas += 1;
    }
    Ok(())
}

fn apply_delta(target: &Path, delta: &Path) -> Result<()> {
    let temporary = target.with_extension("wg-launcher-new");
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    match delta.extension().and_then(OsStr::to_str) {
        Some(extension) if extension.eq_ignore_ascii_case("xdiff") => {
            apply_vcdiff(target, delta, &temporary)?
        }
        Some(extension) if extension.eq_ignore_ascii_case("rdiff") => {
            apply_rsync_delta(target, delta, &temporary)?
        }
        _ => return Err(Error::InvalidDeltaPath(delta.display().to_string())),
    }
    if !fs::metadata(&temporary).is_ok_and(|metadata| metadata.len() > 0) {
        return Err(Error::EmptyDeltaOutput(temporary));
    }
    replace_file(&temporary, target)
}

fn apply_vcdiff(target: &Path, delta: &Path, output: &Path) -> Result<()> {
    let source_file = File::open(target)?;
    let source = unsafe { memmap2::MmapOptions::new().map(&source_file)? };
    let mut source = &source[..];
    let patch = BufReader::new(File::open(delta)?);
    let mut writer = BufWriter::new(File::create(output)?);

    oxidelta::compress::decoder::DeltaDecoder::new(patch)
        .decode_to(&mut source, &mut writer)
        .map_err(|error| Error::DeltaDecode {
            format: "VCDIFF",
            path: delta.to_path_buf(),
            detail: error.to_string(),
        })?;
    writer.flush()?;
    Ok(())
}

fn apply_rsync_delta(target: &Path, delta: &Path, output: &Path) -> Result<()> {
    let source_file = File::open(target)?;
    let delta_file = File::open(delta)?;
    let source = unsafe { memmap2::MmapOptions::new().map(&source_file)? };
    let patch = unsafe { memmap2::MmapOptions::new().map(&delta_file)? };
    let mut writer = BufWriter::new(File::create(output)?);
    fast_rsync::apply(&source, &patch, &mut writer).map_err(|error| Error::DeltaDecode {
        format: "librsync",
        path: delta.to_path_buf(),
        detail: error.to_string(),
    })?;
    writer.flush()?;
    Ok(())
}

fn write_marker(manifest: &PatchManifest, marker: &Path) -> Result<()> {
    let marker_data = InstalledBuild::from(manifest);
    let bytes = serde_json::to_vec_pretty(&marker_data)
        .map_err(|error| Error::ExtractionTask(error.to_string()))?;
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker, bytes)?;
    Ok(())
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup = target.with_extension("wg-launcher-old");
    if target.exists() {
        fs::rename(target, &backup)?;
    }
    match fs::rename(source, target) {
        Ok(()) => {
            if backup.exists() {
                fs::remove_file(backup)?;
            }
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = fs::rename(&backup, target);
            }
            Err(error.into())
        }
    }
}

fn delta_target(path: &Path) -> Option<PathBuf> {
    let value = path.to_string_lossy();
    let extension = path.extension()?.to_str()?;
    if !extension.eq_ignore_ascii_case("xdiff") && !extension.eq_ignore_ascii_case("rdiff") {
        return None;
    }
    let without_extension = value.rsplit_once('.')?.0;
    let (without_checksum, checksum) = without_extension.rsplit_once('.')?;
    if checksum.is_empty()
        || !checksum
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }
    let marker = ".pkg.";
    let position = without_checksum.rfind(marker)?;
    let version = &without_checksum[position + marker.len()..];
    if version.is_empty()
        || !version
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return None;
    }
    Some(PathBuf::from(&without_checksum[..position + 4]))
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file() {
                output.push(entry.path());
            }
        }
    }
    output.sort();
    Ok(output)
}

fn safe_relative_path(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::UnsafeAssetPath(name.to_owned()));
    }
    Ok(path.to_path_buf())
}

fn safe_archive_path(name: &str) -> Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    safe_relative_path(&normalized)
}

#[derive(Debug)]
struct MultiVolumeReader {
    files: Vec<File>,
    starts: Vec<u64>,
    lengths: Vec<u64>,
    position: u64,
    length: u64,
}

impl MultiVolumeReader {
    fn open(paths: &[PathBuf]) -> Result<Self> {
        let mut files = Vec::with_capacity(paths.len());
        let mut starts = Vec::with_capacity(paths.len());
        let mut lengths = Vec::with_capacity(paths.len());
        let mut length = 0_u64;
        for path in paths {
            let file = File::open(path)?;
            let file_length = file.metadata()?.len();
            starts.push(length);
            lengths.push(file_length);
            length = length.checked_add(file_length).ok_or_else(|| {
                Error::ArchiveExtraction("archive volume sizes overflow u64".to_owned())
            })?;
            files.push(file);
        }
        Ok(Self {
            files,
            starts,
            lengths,
            position: 0,
            length,
        })
    }

    fn volume_at(&self, position: u64) -> Option<usize> {
        self.starts
            .iter()
            .zip(&self.lengths)
            .position(|(start, length)| position >= *start && position < *start + *length)
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() || self.position >= self.length {
            return Ok(0);
        }
        let Some(index) = self.volume_at(self.position) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "archive volume contains an invalid empty gap",
            ));
        };
        let local_position = self.position - self.starts[index];
        self.files[index].seek(SeekFrom::Start(local_position))?;
        let remaining = (self.lengths[index] - local_position) as usize;
        let read_length = buffer.len().min(remaining);
        let count = self.files[index].read(&mut buffer[..read_length])?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let next = match position {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::End(offset) => i128::from(self.length) + i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
        };
        if !(0..=i128::from(self.length)).contains(&next) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot seek outside the split archive",
            ));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PatchFile, Region};
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

    fn patch(from: &str, to: &str) -> Patch {
        Patch {
            part: "client".to_owned(),
            version_from: Some(from.to_owned()),
            version_to: to.to_owned(),
            files: vec![],
            torrent_hash: None,
            torrent_urls: vec![],
        }
    }

    fn manifest(patches: Vec<Patch>) -> PatchManifest {
        PatchManifest {
            region: Region::Eu,
            channel: "production".to_owned(),
            host: "example.test".to_owned(),
            game_id: "WOT.EU.PRODUCTION".to_owned(),
            platform: "windows".to_owned(),
            architecture: Some("x64".to_owned()),
            client_type: "sd".to_owned(),
            language: "EN".to_owned(),
            metadata_version: "1".to_owned(),
            version: "1.0".to_owned(),
            available_languages: vec!["EN".to_owned()],
            publisher_ids: vec![],
            parts: vec!["client".to_owned()],
            patches,
            web_seeds: vec![],
            assets: vec![],
        }
    }

    #[test]
    fn recognizes_wargaming_delta_names() {
        assert_eq!(
            delta_target(Path::new(
                "res/packages/scripts.pkg.2.4.1.24080.3AFC1F30.xdiff"
            )),
            Some(PathBuf::from("res/packages/scripts.pkg"))
        );
        assert_eq!(
            delta_target(Path::new("res/packages/gui.pkg.2.4.1.24080.ABC123.rdiff")),
            Some(PathBuf::from("res/packages/gui.pkg"))
        );
        assert_eq!(delta_target(Path::new("res/packages/gui.pkg")), None);
    }

    #[test]
    fn extraction_cleans_the_download_cache_by_default() {
        assert!(ExtractionOptions::default().clean_download_cache);
    }

    #[test]
    fn installed_record_can_be_read_for_future_update_checks() {
        let root = test_directory("installed-record");
        fs::create_dir_all(&root).unwrap();
        let marker = root.join(INSTALL_MARKER_FILE);
        let manifest = manifest(vec![]);

        write_marker(&manifest, &marker).unwrap();
        let installed: InstalledBuild =
            serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();

        assert_eq!(installed.version, manifest.version);
        assert_eq!(installed.region, manifest.region);
        assert_eq!(installed.client_type, manifest.client_type);
        assert_eq!(installed.language, manifest.language);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn orders_an_unordered_patch_chain_and_ignores_unreachable_links() {
        let manifest = manifest(vec![
            patch("3", "4"),
            patch("99", "100"),
            patch("0", "2"),
            patch("2", "3"),
        ]);

        let versions = ordered_patches(&manifest)
            .unwrap()
            .into_iter()
            .map(|patch| patch.version_to.as_str())
            .collect::<Vec<_>>();

        assert_eq!(versions, ["2", "3", "4"]);
    }

    #[test]
    fn rejects_a_cyclic_patch_chain() {
        let manifest = manifest(vec![patch("0", "1"), patch("1", "2"), patch("2", "1")]);

        assert!(matches!(
            ordered_patches(&manifest),
            Err(Error::BrokenPatchChain { .. })
        ));
    }

    #[test]
    fn releases_only_the_successful_patch_volumes() {
        let cache = test_directory("release-test");
        let volume_directory = cache.join("build");
        fs::create_dir_all(&volume_directory).unwrap();
        fs::write(volume_directory.join("client.wgpkg.001"), [1_u8, 2, 3]).unwrap();
        fs::write(volume_directory.join("client.wgpkg.002"), [4_u8, 5]).unwrap();
        fs::write(cache.join(DOWNLOAD_MANIFEST_FILE), b"manifest").unwrap();
        let patch = Patch {
            part: "client".to_owned(),
            version_from: Some("0".to_owned()),
            version_to: "1".to_owned(),
            files: vec![
                PatchFile {
                    name: "build/client.wgpkg.001".to_owned(),
                    size: 3,
                    unpacked_size: None,
                    diffs_size: None,
                },
                PatchFile {
                    name: "build/client.wgpkg.002".to_owned(),
                    size: 2,
                    unpacked_size: None,
                    diffs_size: None,
                },
            ],
            torrent_hash: None,
            torrent_urls: vec![],
        };

        assert_eq!(remove_patch_volumes(&cache, &patch).unwrap(), 5);
        assert!(!volume_directory.exists());
        assert!(cache.join(DOWNLOAD_MANIFEST_FILE).is_file());

        fs::remove_dir_all(cache).unwrap();
    }

    #[test]
    fn installation_lock_rejects_a_concurrent_installer_and_allows_a_retry() {
        let root = test_directory("installation-lock");
        let storage = installation_storage(&root).unwrap();
        let first = InstallationLock::acquire(&storage, &root).unwrap();

        let error = InstallationLock::acquire(&storage, &root)
            .err()
            .expect("a second installer must not acquire the lock");
        assert!(
            matches!(&error, Error::InstallationLocked(path) if path == &root),
            "expected installation contention, got {error:?}"
        );

        drop(first);
        assert!(InstallationLock::acquire(&storage, &root).is_ok());
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn streaming_install_downloads_extracts_and_releases_the_patch() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };

        let root = test_directory("streaming-test");
        fs::create_dir_all(&root).unwrap();
        let payload = root.join("installed.bin");
        let archive = root.join("source.7z");
        let installed = b"small real 7z payload".to_vec();
        fs::write(&payload, &installed).unwrap();
        sevenz_rust::compress_to_path(&payload, &archive).unwrap();
        fs::remove_file(&payload).unwrap();
        let body = fs::read(&archive).unwrap();
        fs::remove_file(&archive).unwrap();
        let interrupted_staging = root.join(STAGING_DIRECTORY);
        fs::create_dir_all(&interrupted_staging).unwrap();
        fs::write(interrupted_staging.join("partial-file"), b"interrupted").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response_body = body.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            )
            .unwrap();
            stream.write_all(&response_body).unwrap();
        });

        let name = "build/client.wgpkg";
        let file = PatchFile {
            name: name.to_owned(),
            size: body.len() as u64,
            unpacked_size: Some(body.len() as u64),
            diffs_size: None,
        };
        let patch = Patch {
            part: "client".to_owned(),
            version_from: Some("0".to_owned()),
            version_to: "1".to_owned(),
            files: vec![file.clone()],
            torrent_hash: None,
            torrent_urls: vec![],
        };
        let mut manifest = manifest(vec![patch]);
        manifest.assets = vec![Asset {
            part: "client".to_owned(),
            version_from: Some("0".to_owned()),
            version_to: "1".to_owned(),
            name: name.to_owned(),
            url: format!("http://{address}/{name}").parse().unwrap(),
            size: body.len() as u64,
            unpacked_size: file.unpacked_size,
            diffs_size: None,
        }];
        let extractor = PackageExtractor::new();
        let storage = installation_storage(&root).unwrap();

        let report = extractor
            .download_and_extract(
                AssetDownloader::new().unwrap(),
                manifest,
                &root,
                DownloadOptions {
                    concurrency: 1,
                    attempts: 1,
                },
                ExtractionOptions::default(),
                |_| {},
            )
            .await
            .unwrap();

        server.join().unwrap();
        assert_eq!(fs::read(root.join("installed.bin")).unwrap(), installed);
        assert!(storage.cache.starts_with(&root));
        assert!(storage.marker.is_file());
        assert!(!storage.cache.exists());
        assert!(!root.join(DOWNLOAD_CACHE_DIRECTORY).exists());
        assert!(!root.join(STAGING_DIRECTORY).exists());
        assert_eq!(report.released_package_bytes, file.size);
        assert!(report.extraction.cache_removed);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn extraction_reader_reports_intermediate_bytes_and_throttles_updates() {
        let mut source = &b"abcdef"[..];
        let positions = std::cell::RefCell::new(Vec::new());
        let notify = |position| positions.borrow_mut().push(position);
        let mut reader = ExtractionReader {
            reader: &mut source,
            notify: &notify,
            extracted: 0,
            last_update: Instant::now() - Duration::from_secs(1),
        };
        let mut buffer = [0; 2];
        assert_eq!(reader.read(&mut buffer).unwrap(), 2);
        assert_eq!(buffer, *b"ab");
        assert_eq!(*positions.borrow(), [2]);
        // A fresh interval prevents an event for every small decoder read.
        reader.last_update = Instant::now();
        assert_eq!(reader.read(&mut buffer).unwrap(), 2);
        assert_eq!(reader.extracted, 4);
        assert_eq!(*positions.borrow(), [2]);
    }

    #[test]
    fn extracts_a_split_7z_archive_without_concatenating_it() {
        let root = test_directory("split-archive");
        let source = root.join("source");
        let cache = root.join("cache");
        let destination = root.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&cache).unwrap();
        fs::write(
            source.join("payload.bin"),
            b"a payload spanning archive volumes",
        )
        .unwrap();
        let archive = root.join("payload.7z");
        sevenz_rust::compress_to_path(source.join("payload.bin"), &archive).unwrap();
        let bytes = fs::read(&archive).unwrap();
        let split = bytes.len() / 2;
        fs::write(cache.join("payload.wgpkg.001"), &bytes[..split]).unwrap();
        fs::write(cache.join("payload.wgpkg.002"), &bytes[split..]).unwrap();
        let patch = Patch {
            part: "client".to_owned(),
            version_from: Some("0".to_owned()),
            version_to: "1".to_owned(),
            files: vec![
                PatchFile {
                    name: "payload.wgpkg.002".to_owned(),
                    size: (bytes.len() - split) as u64,
                    unpacked_size: None,
                    diffs_size: None,
                },
                PatchFile {
                    name: "payload.wgpkg.001".to_owned(),
                    size: split as u64,
                    unpacked_size: None,
                    diffs_size: None,
                },
            ],
            torrent_hash: None,
            torrent_urls: vec![],
        };

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> =
            Arc::new(move |event| captured.lock().unwrap().push(event));
        extract_archive(&patch, &cache, &destination, &observer).unwrap();
        let events = events.lock().unwrap();
        let progress = events
            .iter()
            .filter_map(|event| match event {
                ExtractionEvent::ExtractingFile {
                    path,
                    index,
                    total,
                    extracted,
                    size,
                } => Some((path.as_str(), *index, *total, *extracted, *size)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let size = b"a payload spanning archive volumes".len() as u64;
        assert_eq!(progress.first(), Some(&("payload.bin", 1, 1, 0, size)));
        assert_eq!(progress.last(), Some(&("payload.bin", 1, 1, size, size)));

        assert_eq!(
            fs::read(destination.join("payload.bin")).unwrap(),
            b"a payload spanning archive volumes"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn applies_vcdiff_deltas_in_process() {
        let root = test_directory("vcdiff");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.pkg");
        let delta = root.join("target.xdiff");
        fs::write(&target, [1_u8, 2, 4, 4, 7, 6, 7]).unwrap();
        fs::write(
            &delta,
            [
                214_u8, 195, 196, 0, 0, 0, 13, 7, 0, 7, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8,
            ],
        )
        .unwrap();

        apply_delta(&target, &delta).unwrap();

        assert_eq!(fs::read(&target).unwrap(), [1_u8, 2, 3, 4, 5, 6, 7]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn applies_xdelta_application_header_and_checksummed_windows() {
        let root = test_directory("xdelta-extensions");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.pkg");
        let delta = root.join("target.xdiff");
        fs::write(&target, b"worldhello").unwrap();
        let patch = [
            0xd6, 0xc3, 0xc4, 0, 4, 5, b'1', b'/', b'/', b'1', b'/', 5, 5, 5, 12, 5, 0, 0, 2, 1, 6,
            44, 2, 21, 19, 5, 0, 5, 5, 0, 12, 5, 0, 0, 2, 1, 6, 166, 2, 41, 19, 5, 0,
        ];
        fs::write(&delta, patch).unwrap();
        apply_delta(&target, &delta).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"helloworld");

        fs::write(&target, b"worldhello").unwrap();
        let mut corrupt = patch;
        corrupt[39] ^= 1;
        fs::write(&delta, corrupt).unwrap();
        let error = apply_delta(&target, &delta).unwrap_err();
        assert!(matches!(error, Error::DeltaDecode { .. }));
        assert!(error.to_string().contains("checksum mismatch"));
        assert_eq!(fs::read(&target).unwrap(), b"worldhello");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_vcdiff_returns_error_without_replacing_source() {
        let root = test_directory("vcdiff-invalid");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.pkg");
        let delta = root.join("target.xdiff");
        fs::write(&target, b"original").unwrap();
        fs::write(&delta, [0xd6, 0xc3, 0xc4, 0, 0, 0xff]).unwrap();
        assert!(matches!(
            apply_delta(&target, &delta),
            Err(Error::DeltaDecode { .. })
        ));
        assert_eq!(fs::read(&target).unwrap(), b"original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn applies_librsync_deltas_in_process() {
        let root = test_directory("librsync");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.pkg");
        let delta = root.join("target.rdiff");
        let before = b"the original package data";
        let after = b"the updated package data with more bytes";
        fs::write(&target, before).unwrap();
        let signature = fast_rsync::Signature::calculate(
            before,
            fast_rsync::SignatureOptions {
                block_size: 8,
                crypto_hash_size: 16,
            },
        );
        let indexed = signature.index();
        let mut delta_bytes = Vec::new();
        fast_rsync::diff(&indexed, after, &mut delta_bytes).unwrap();
        fs::write(&delta, delta_bytes).unwrap();

        apply_delta(&target, &delta).unwrap();

        assert_eq!(fs::read(&target).unwrap(), after);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_archive_path_traversal_on_every_platform() {
        for path in ["../outside", "folder/../../outside", "folder\\..\\outside"] {
            assert!(safe_archive_path(path).is_err(), "accepted {path}");
        }
    }
}
