use super::http::Resource;
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

pub(crate) fn package_resource(packages: &Path, request_path: &str) -> Resource {
    let relative = request_path.trim_start_matches('/');
    let relative = relative.strip_prefix("packages/").unwrap_or(relative);
    let path = Path::new(relative);
    if relative.is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Resource::NotFound;
    }
    let Ok(file) = packages.join(path).canonicalize() else {
        return Resource::NotFound;
    };
    if !file.starts_with(packages) || !file.is_file() {
        return Resource::NotFound;
    }
    Resource::File(file, content_type(relative))
}

fn read_required(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|error| {
        invalid_input(format!(
            "cannot read required file '{}': {error}",
            path.display()
        ))
    })
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".xml") {
        "application/xml"
    } else if path.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
