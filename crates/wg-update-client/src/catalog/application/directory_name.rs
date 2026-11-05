use super::reports::BuildInfo;
use std::path::PathBuf;

impl BuildInfo {
    /// Returns a relative directory name identifying this build selection.
    ///
    /// The name includes game, region, channel, version, payload platform, architecture,
    /// client type, and language. Missing architecture becomes `unknown-arch`.
    /// Values are lowercased and sanitized. Updates keep the initial directory name.
    /// No directory is created.
    pub(crate) fn default_output_directory(&self) -> PathBuf {
        let architecture = self.architecture.as_deref().unwrap_or("unknown-arch");
        PathBuf::from(format!(
            "{}-{}-{}-{}-{}-{}-{}-{}",
            slug(&self.game),
            slug(&self.region.to_string()),
            slug(&self.channel),
            slug(&self.version),
            slug(&self.platform),
            slug(architecture),
            slug(&self.client_type),
            slug(&self.language),
        ))
    }
}

fn slug(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous_separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '.' {
            output.push(character);
            previous_separator = false;
        } else if !previous_separator && !output.is_empty() {
            output.push('-');
            previous_separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    output
}
