use super::error::{PackageError as Error, Result};
use super::volumes::MultiVolumeReader;
use crate::catalog::Patch;
use crate::installation::ExtractionEvent;
use crate::installation::infrastructure::filesystem::{
    patch_workspace::PatchWorkspace,
    paths::{safe_archive_path, safe_relative_path},
};
use std::{
    fs,
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Decode a delta patch into its workspace without changing live game files.
pub(crate) fn decode_patch_contents(
    patch: &Patch,
    cache: &Path,
    workspace: &PatchWorkspace,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<()> {
    decode_archive(patch, cache, &workspace.archive(), observer, |_, _| Ok(()))
}

pub(super) fn decode_archive(
    patch: &Patch,
    cache: &Path,
    destination: &Path,
    observer: &Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
    mut decoded_entry: impl FnMut(&Path, &Path) -> Result<()>,
) -> Result<()> {
    let mut volumes = patch
        .volumes
        .iter()
        .map(|volume| safe_relative_path(&volume.name).map(|relative| cache.join(relative)))
        .collect::<std::result::Result<Vec<_>, crate::installation::StorageError>>()?;
    volumes.sort();
    if volumes.is_empty() {
        return Err(crate::catalog::ReleaseError::MissingFullPatch {
            part: patch.component_id.clone(),
        }
        .into());
    }

    let reader = MultiVolumeReader::open(&volumes)?;
    let length = reader.length;
    let mut archive = sevenz_rust::SevenZReader::new(reader, length, Default::default())
        .map_err(Error::ArchiveExtraction)?;
    let total = archive
        .archive()
        .files
        .iter()
        .filter(|entry| !entry.is_directory())
        .count();
    fs::create_dir_all(destination)?;
    let mut index = 0;
    let mut entry_failure = None;
    let result = archive.for_each_entries(|entry, reader| {
        // The decoder callback requires its own error type. Keep our typed
        // cause separately rather than flattening storage failures to text.
        let mut extract_entry = || -> Result<bool> {
            let relative = safe_archive_path(entry.name())?;
            let target = destination.join(&relative);
            if entry.is_directory() {
                let result = sevenz_rust::default_entry_extract_fn(entry, reader, &target)?;
                decoded_entry(&relative, &target)?;
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
            decoded_entry(&relative, &target)?;
            notify(progress.extracted);
            Ok(result)
        };
        extract_entry().map_err(|error| {
            entry_failure = Some(error);
            sevenz_rust::Error::other("archive entry processing failed")
        })
    });
    if let Some(error) = entry_failure {
        return Err(error);
    }
    result.map_err(Error::ArchiveExtraction)
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
