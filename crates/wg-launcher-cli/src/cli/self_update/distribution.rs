//! Identify package-manager ownership before replacing an executable.

use std::path::Path;

fn manager() -> Option<&'static str> {
    // Future packagers can mark their builds even when installed at a custom path.
    if let Some(manager) = option_env!("WG_LAUNCHER_PACKAGE_MANAGER") {
        return Some(match manager {
            "brew" | "homebrew" => "Homebrew",
            "winget" => "WinGet",
            "apt" | "deb" => "APT",
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

pub(super) fn unsupported_reason(stderr: bool) -> Option<String> {
    manager().map(|manager| {
        format!(
            "Self-update not supported for this installation: it is managed by {manager}. {}",
            instruction(manager, stderr)
        )
    })
}

pub(super) fn update_instruction(stderr: bool) -> String {
    manager().map_or_else(
        || {
            format!(
                "Run {} to update the CLI.",
                styled_command("wg-launcher self-update", stderr)
            )
        },
        |manager| {
            format!(
                "Self-update not supported for this {manager} installation. {}",
                instruction(manager, stderr)
            )
        },
    )
}

fn styled_command(text: &str, stderr: bool) -> String {
    let command = super::super::output::command(text);
    if stderr {
        command.for_stderr().to_string()
    } else {
        command.to_string()
    }
}

fn instruction(manager: &str, stderr: bool) -> String {
    let command = |text| styled_command(text, stderr);
    match manager {
        "Homebrew" => format!("Use {} to update it.", command("brew upgrade")),
        "WinGet" => format!("Use {} to update it.", command("winget upgrade")),
        "APT" => format!(
            "Use {} and {} to update it.",
            command("sudo apt update"),
            command("sudo apt install --only-upgrade wg-launcher")
        ),
        "Scoop" => format!("Use {} to update it.", command("scoop update")),
        "Nix" => "Update it through your Nix configuration.".into(),
        _ => "Use your package manager to update it.".into(),
    }
}
