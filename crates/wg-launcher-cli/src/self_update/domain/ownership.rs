use std::path::Path;

pub(crate) fn manager_for_path(path: &Path) -> Option<&'static str> {
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
