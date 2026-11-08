//! Read only the executable into a fixed staging path; never unpack archive paths.

use crate::self_update::domain::artifact::ArchiveFormat;

type Result<T> = std::result::Result<T, ArchiveError>;

use crate::self_update::progress::{Observer, SelfUpdateEvent, SelfUpdateStage};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("archive executable is a symbolic link")]
    SymbolicLink,
    #[error("unsafe archive executable entry")]
    UnsafeEntry,
    #[error("archive contains no launcher executable")]
    MissingExecutable,
    #[error("archive contains duplicate launcher executables")]
    DuplicateExecutable,
    #[error("invalid executable size")]
    InvalidSize,
}

const LIMIT: u64 = 256 * 1024 * 1024;

pub(crate) fn extract(
    archive: &Path,
    directory: &Path,
    format: ArchiveFormat,
    observer: &Observer<'_>,
) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "wg-launcher.exe"
    } else {
        "wg-launcher"
    };
    let output = directory.join(name);
    let mut found = false;
    match format {
        ArchiveFormat::Zip => {
            let mut archive = zip::ZipArchive::new(File::open(archive)?)?;
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let Some(path) = entry.enclosed_name() else {
                    continue;
                };
                if path.file_name().is_some_and(|part| part == name) && entry.is_file() {
                    if entry
                        .unix_mode()
                        .is_some_and(|mode| mode & 0o170000 == 0o120000)
                    {
                        return Err(ArchiveError::SymbolicLink);
                    }
                    observer(SelfUpdateEvent::Transfer {
                        stage: SelfUpdateStage::ExtractingExecutable,
                        total: Some(entry.size()),
                    })?;
                    write_binary(&mut entry, &output, &mut found, observer)?;
                }
            }
        }
        ArchiveFormat::TarGzip => {
            let decoded = flate2::read::GzDecoder::new(File::open(archive)?).take(LIMIT * 2);
            let mut archive = tar::Archive::new(decoded);
            for entry in archive.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;
                if path.file_name().is_some_and(|part| part == name) {
                    if !path
                        .components()
                        .all(|part| matches!(part, Component::Normal(_)))
                        || !entry.header().entry_type().is_file()
                    {
                        return Err(ArchiveError::UnsafeEntry);
                    }
                    observer(SelfUpdateEvent::Transfer {
                        stage: SelfUpdateStage::ExtractingExecutable,
                        total: Some(entry.size()),
                    })?;
                    write_binary(&mut entry, &output, &mut found, observer)?;
                }
            }
        }
    }
    if !found {
        return Err(ArchiveError::MissingExecutable);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(output)
}

fn write_binary(
    reader: &mut impl Read,
    output: &Path,
    found: &mut bool,
    observer: &Observer<'_>,
) -> Result<()> {
    if *found {
        return Err(ArchiveError::DuplicateExecutable);
    }
    let mut file = File::create(output)?;
    let mut reader = reader.take(LIMIT + 1);
    let mut buffer = [0; 64 * 1024];
    let mut size = 0;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])?;
        size += count as u64;
        observer(SelfUpdateEvent::Advanced(count as u64))?;
    }
    if size == 0 || size > LIMIT {
        return Err(ArchiveError::InvalidSize);
    }
    file.flush()?;
    file.sync_all()?;
    *found = true;
    Ok(())
}
