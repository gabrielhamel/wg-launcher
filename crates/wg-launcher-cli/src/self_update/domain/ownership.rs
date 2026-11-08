use std::path::Path;

/// Ownership discovered for the running executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutableOwnership {
    Direct,
    Managed(PackageManager),
    /// The executable could not be located or its path could not be resolved.
    Unresolved,
}

/// The owner responsible for updating a managed launcher installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageManager {
    Homebrew,
    WinGet,
    Apt,
    Scoop,
    Nix,
    Other,
}

impl PackageManager {
    pub(crate) fn from_marker(marker: &str) -> Self {
        match marker {
            "brew" | "homebrew" => Self::Homebrew,
            "winget" => Self::WinGet,
            "apt" | "deb" => Self::Apt,
            "scoop" => Self::Scoop,
            "nix" => Self::Nix,
            _ => Self::Other,
        }
    }
}

pub(crate) fn manager_for_path(path: &Path) -> Option<PackageManager> {
    let parts: Vec<_> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    if parts
        .windows(2)
        .any(|pair| pair[0] == "cellar" && pair[1].starts_with("wg-launcher"))
    {
        Some(PackageManager::Homebrew)
    } else if parts.iter().any(|part| part == "winget") {
        Some(PackageManager::WinGet)
    } else if parts.iter().any(|part| part == "scoop") {
        Some(PackageManager::Scoop)
    } else if parts
        .windows(2)
        .any(|pair| pair[0] == "nix" && pair[1] == "store")
    {
        Some(PackageManager::Nix)
    } else {
        None
    }
}
