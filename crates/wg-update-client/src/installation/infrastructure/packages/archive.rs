use super::error::{PackageError as Error, Result};
use super::volumes::MultiVolumeReader;
use crate::catalog::Patch;
use crate::installation::ExtractionEvent;
use crate::installation::infrastructure::filesystem::{
    patch_workspace::PatchWorkspace,
    paths::{safe_archive_path, safe_relative_path},
    replacement::replace_file,
};
use std::{
    fs,
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Decode full content safely before replacing each live game file.
pub(crate) fn install_full_patch(
    patch: &Patch,
    cache: &Path,
    game: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<()> {
    fs::create_dir_all(game)?;
    decode_archive(
        patch,
        cache,
        &workspace.archive(),
        observer,
        |relative, output| {
            let target = game.join(relative);
            let result = if output.is_dir() {
                fs::create_dir_all(target).map_err(crate::installation::StorageError::from)
            } else {
                replace_file(output, &target, &workspace.replacement_backup())
            };
            result.map_err(|error| sevenz_rust::Error::other(error.to_string()))
        },
    )
}

/// Decode a delta patch into its workspace without changing live game files.
pub(crate) fn decode_patch_contents(
    patch: &Patch,
    cache: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<()> {
    decode_archive(patch, cache, &workspace.archive(), observer, |_, _| Ok(()))
}

fn decode_archive(
    patch: &Patch,
    cache: &Path,
    destination: &Path,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    mut install: impl FnMut(&Path, &Path) -> std::result::Result<(), sevenz_rust::Error>,
) -> Result<()> {
    let mut volumes = patch
        .files
        .iter()
        .map(|file| safe_relative_path(&file.name).map(|relative| cache.join(relative)))
        .collect::<std::result::Result<Vec<_>, crate::installation::StorageError>>()?;
    volumes.sort();
    if volumes.is_empty() {
        return Err(crate::catalog::ReleaseError::MissingFullPatch {
            part: patch.part.clone(),
        }
        .into());
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
                let result = sevenz_rust::default_entry_extract_fn(entry, reader, &target)?;
                install(&relative, &target)?;
                return Ok(result);
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
            install(&relative, &target)?;
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
