mod error;
pub use error::StorageError;
pub(crate) mod paths;
pub(crate) mod records;
pub(crate) mod retry;
pub(crate) mod storage;
pub use storage::{
    DOWNLOAD_CACHE_DIRECTORY, DOWNLOAD_MANIFEST_FILE, INSTALL_MARKER_FILE, STAGING_DIRECTORY,
};
pub(crate) use storage::{InstallationLock, installation_storage};
