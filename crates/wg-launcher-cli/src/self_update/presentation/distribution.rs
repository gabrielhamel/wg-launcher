use super::super::{domain::ownership::PackageManager, infrastructure::distribution::manager};

pub(crate) fn unsupported_reason(stderr: bool) -> Option<String> {
    manager().map(|manager| managed_reason(manager, stderr))
}

pub(crate) fn managed_reason(manager: PackageManager, stderr: bool) -> String {
    let name = manager_name(manager);
    format!(
        "Self-update not supported for this installation: it is managed by {name}. {}",
        instruction(manager, stderr)
    )
}

pub(crate) fn update_instruction(stderr: bool) -> String {
    manager().map_or_else(
        || {
            format!(
                "Run {} to update the CLI.",
                styled_command("wg-launcher self-update", stderr)
            )
        },
        |manager| {
            let name = manager_name(manager);
            format!(
                "Self-update not supported for this {name} installation. {}",
                instruction(manager, stderr)
            )
        },
    )
}

fn styled_command(text: &str, stderr: bool) -> String {
    let command = crate::terminal::command(text);
    if stderr {
        command.for_stderr().to_string()
    } else {
        command.to_string()
    }
}

fn instruction(manager: PackageManager, stderr: bool) -> String {
    let command = |text| styled_command(text, stderr);
    match manager {
        PackageManager::Homebrew => format!("Use {} to update it.", command("brew upgrade")),
        PackageManager::WinGet => format!("Use {} to update it.", command("winget upgrade")),
        PackageManager::Apt => format!(
            "Use {} and {} to update it.",
            command("sudo apt update"),
            command("sudo apt install --only-upgrade wg-launcher")
        ),
        PackageManager::Scoop => format!("Use {} to update it.", command("scoop update")),
        PackageManager::Nix => "Update it through your Nix configuration.".into(),
        PackageManager::Other => "Use your package manager to update it.".into(),
    }
}

fn manager_name(manager: PackageManager) -> &'static str {
    match manager {
        PackageManager::Homebrew => "Homebrew",
        PackageManager::WinGet => "WinGet",
        PackageManager::Apt => "APT",
        PackageManager::Scoop => "Scoop",
        PackageManager::Nix => "Nix",
        PackageManager::Other => "the package manager",
    }
}
