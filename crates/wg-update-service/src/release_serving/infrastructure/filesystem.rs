use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

const METADATA_FILE: &str = "metadata.xml";
const PATCH_CHAIN_FILE: &str = "patches_chain.xml";
const PACKAGES_DIRECTORY: &str = "packages";

pub(crate) struct ReleaseFiles {
    pub(crate) packages: PathBuf,
    pub(crate) metadata: Vec<u8>,
    pub(crate) patch_chain: Vec<u8>,
}

pub(crate) fn load_release(root: &Path) -> io::Result<ReleaseFiles> {
    let input = root.canonicalize().map_err(|error| {
        invalid_input(format!(
            "cannot open input directory '{}': {error}",
            root.display()
        ))
    })?;
    if !input.is_dir() {
        return Err(invalid_input(format!(
            "input '{}' is not a directory",
            input.display()
        )));
    }
    let packages = input.join(PACKAGES_DIRECTORY);
    if !packages.is_dir() {
        return Err(invalid_input(format!(
            "missing package directory '{}'",
            packages.display()
        )));
    }
    let metadata = read_required(&input.join(METADATA_FILE))?;
    let patch_chain = read_required(&input.join(PATCH_CHAIN_FILE))?;
    Ok(ReleaseFiles {
        packages,
        metadata,
        patch_chain,
    })
}

/// Resolve a package relative to the release's package folder, preserving containment.
pub(crate) fn resolve_package(packages: &Path, relative: &Path) -> Option<PathBuf> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let file = packages.join(relative).canonicalize().ok()?;
    if !file.starts_with(packages) || !file.is_file() {
        return None;
    }
    Some(file)
}

fn read_required(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|error| {
        invalid_input(format!(
            "cannot read required file '{}': {error}",
            path.display()
        ))
    })
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
