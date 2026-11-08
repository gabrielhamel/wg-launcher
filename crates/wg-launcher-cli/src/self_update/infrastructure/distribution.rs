//! Identify package-manager ownership before replacing an executable.

use super::super::domain::ownership::{PackageManager, manager_for_path};
use std::path::Path;

pub(crate) fn manager() -> Option<PackageManager> {
    // Future packagers can mark their builds even when installed at a custom path.
    if let Some(manager) = option_env!("WG_LAUNCHER_PACKAGE_MANAGER") {
        return Some(PackageManager::from_marker(manager));
    }
    let executable = std::env::current_exe().ok()?.canonicalize().ok()?;
    if let Some(prefix) = executable.parent().and_then(Path::parent) {
        let receipt = prefix.join("INSTALL_RECEIPT.json");
        if std::fs::read(receipt)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|receipt| receipt["homebrew_version"].is_string())
        {
            return Some(PackageManager::Homebrew);
        }
    }
    manager_for_path(&executable)
}
