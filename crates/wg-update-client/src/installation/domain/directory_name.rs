use crate::catalog::{BuildIdentity, Product};
use std::path::PathBuf;

/// Returns a relative directory name identifying this build selection.
///
/// The name includes game, region, channel, version, payload platform, architecture,
/// client type, and language. Missing architecture becomes `unknown-arch`.
/// Values are lowercased and sanitized. Updates keep the initial directory name.
/// No directory is created.
pub(crate) fn initial_directory_name(product: &Product, identity: &BuildIdentity) -> PathBuf {
    let architecture = identity.architecture.as_deref().unwrap_or("unknown-arch");
    PathBuf::from(format!(
        "{}-{}-{}-{}-{}-{}-{}-{}",
        slug(&product.game()),
        slug(&identity.region),
        slug(&identity.channel),
        slug(&identity.version),
        slug(&identity.platform),
        slug(architecture),
        slug(&identity.client_type),
        slug(&identity.language),
    ))
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
