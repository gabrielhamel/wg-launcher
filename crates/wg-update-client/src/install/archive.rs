use super::{
    ExtractionEvent,
    paths::{safe_archive_path, safe_relative_path},
};
use crate::{Error, Patch, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) fn extract_archive(
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
