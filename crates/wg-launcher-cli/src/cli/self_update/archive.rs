//! Read only the executable into a fixed staging path; never unpack archive paths.

use super::Result;
use crate::progress::SelfUpdateProgress;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const LIMIT: u64 = 256 * 1024 * 1024;

pub(super) fn extract(
    archive: &Path,
    directory: &Path,
    zip: bool,
    progress: &SelfUpdateProgress,
) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "wg-launcher.exe"
    } else {
        "wg-launcher"
    };
    let output = directory.join(name);
    let mut found = false;
    if zip {
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
                    return Err(
                        "The CLI archive contains a symbolic link instead of an executable.".into(),
                    );
                }
                progress.transfer("Extracting the CLI executable", Some(entry.size()))?;
                write_binary(&mut entry, &output, &mut found, progress)?;
            }
        }
    } else {
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
                    return Err("The CLI archive has an unsafe executable entry.".into());
                }
                progress.transfer("Extracting the CLI executable", Some(entry.size()))?;
                write_binary(&mut entry, &output, &mut found, progress)?;
            }
        }
    }
    if !found {
        return Err("The CLI archive does not contain the launcher executable.".into());
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
    progress: &SelfUpdateProgress,
) -> Result<()> {
    if *found {
        return Err("The CLI archive contains duplicate launcher executables.".into());
    }
    let mut file = File::create(output)?;
    let size = progress.copy(reader.take(LIMIT + 1), &mut file)?;
    if size == 0 || size > LIMIT {
        return Err("The CLI executable has an invalid size.".into());
    }
    file.flush()?;
    file.sync_all()?;
    *found = true;
    Ok(())
}
