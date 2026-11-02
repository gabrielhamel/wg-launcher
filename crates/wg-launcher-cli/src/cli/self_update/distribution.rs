//! Identify package-manager ownership before replacing an executable.

use std::path::Path;

fn manager() -> Option<&'static str> {
    // Future packagers can mark their builds even when installed at a custom path.
    if let Some(manager) = option_env!("WG_LAUNCHER_PACKAGE_MANAGER") {
        return Some(match manager {
            "brew" | "homebrew" => "Homebrew",
            "winget" => "WinGet",
            "scoop" => "Scoop",
            "nix" => "Nix",
            _ => "the package manager",
        });
    }
    let executable = std::env::current_exe().ok()?.canonicalize().ok()?;
    if let Some(prefix) = executable.parent().and_then(Path::parent) {
        let receipt = prefix.join("INSTALL_RECEIPT.json");
        if std::fs::read(receipt)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|receipt| receipt["homebrew_version"].is_string())
        {
            return Some("Homebrew");
        }
    }
    manager_for_path(&executable)
}

fn manager_for_path(path: &Path) -> Option<&'static str> {
    let parts: Vec<_> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    if parts
        .windows(2)
        .any(|pair| pair[0] == "cellar" && pair[1].starts_with("wg-launcher"))
    {
        Some("Homebrew")
    } else if parts.iter().any(|part| part == "winget") {
        Some("WinGet")
    } else if parts.iter().any(|part| part == "scoop") {
        Some("Scoop")
    } else if parts
        .windows(2)
        .any(|pair| pair[0] == "nix" && pair[1] == "store")
    {
        Some("Nix")
    } else {
        None
    }
}

pub(super) fn unsupported_reason() -> Option<String> {
    manager().map(|manager| {
        format!(
            "Self-update not supported for this installation: it is managed by {manager}. {}",
            instruction(manager)
        )
    })
}

pub(super) fn update_instruction() -> String {
    manager().map_or_else(
        || "Run `wg-launcher self-update` to update the CLI.".into(),
        |manager| {
            format!(
                "Self-update not supported for this {manager} installation. {}",
                instruction(manager)
            )
        },
    )
}

fn instruction(manager: &str) -> &'static str {
    match manager {
        "Homebrew" => "Use `brew upgrade` to update it.",
        "WinGet" => "Use `winget upgrade` to update it.",
        "Scoop" => "Use `scoop update` to update it.",
        "Nix" => "Update it through your Nix configuration.",
        _ => "Use your package manager to update it.",
    }
}
