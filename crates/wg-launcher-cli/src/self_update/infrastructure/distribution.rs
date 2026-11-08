//! Identify package-manager ownership before replacing an executable.

use super::super::domain::ownership::{ExecutableOwnership, PackageManager, manager_for_path};
use std::path::Path;

pub(crate) fn ownership() -> ExecutableOwnership {
    // Future packagers can mark their builds even when installed at a custom path.
    if let Some(manager) = option_env!("WG_LAUNCHER_PACKAGE_MANAGER") {
        return ExecutableOwnership::Managed(PackageManager::from_marker(manager));
    }
    let Some(executable) = std::env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok())
    else {
        return ExecutableOwnership::Unresolved;
    };
    if let Some(prefix) = executable.parent().and_then(Path::parent) {
        let receipt = prefix.join("INSTALL_RECEIPT.json");
        if std::fs::read(receipt)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|receipt| receipt["homebrew_version"].is_string())
        {
            return ExecutableOwnership::Managed(PackageManager::Homebrew);
        }
    }
    manager_for_path(&executable).map_or(ExecutableOwnership::Direct, ExecutableOwnership::Managed)
}
