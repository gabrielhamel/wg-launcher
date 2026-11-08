//! Read split transport volumes as one seekable archive.

use super::error::{PackageError as Error, Result};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

#[derive(Debug)]
pub(super) struct MultiVolumeReader {
    files: Vec<File>,
    starts: Vec<u64>,
    lengths: Vec<u64>,
    position: u64,
    pub(super) length: u64,
}

impl MultiVolumeReader {
    pub(super) fn open(paths: &[PathBuf]) -> Result<Self> {
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
                Error::ArchiveExtraction(sevenz_rust::Error::other(
                    "archive volume sizes overflow u64",
                ))
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
