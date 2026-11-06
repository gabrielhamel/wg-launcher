use super::super::infrastructure::distribution::manager;

pub(crate) fn unsupported_reason(stderr: bool) -> Option<String> {
    manager().map(|manager| managed_reason(manager, stderr))
}

pub(crate) fn managed_reason(manager: &str, stderr: bool) -> String {
    format!(
        "Self-update not supported for this installation: it is managed by {manager}. {}",
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
            format!(
                "Self-update not supported for this {manager} installation. {}",
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
