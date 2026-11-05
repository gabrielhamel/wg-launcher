pub(super) mod ownership;

use super::CURRENT;
use reqwest::Url;
use semver::Version;

pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) base: Url,
    pub(crate) artifacts: Vec<String>,
}

impl Release {
    pub(crate) fn is_newer(&self) -> bool {
        Version::parse(CURRENT).is_ok_and(|current| self.version.cmp_precedence(&current).is_gt())
    }
}

pub(crate) fn candidate_version(stdout: &str) -> Option<&str> {
    let mut lines = stdout.lines();
    let first = lines.next().unwrap_or_default();
    // Released CLIs use the original one-line format; newer builds use status rows.
    if first == "wg-launcher" {
        lines
            .next()
            .and_then(|line| line.strip_prefix("Installed:"))
            .map(str::trim)
    } else {
        first
            .strip_prefix("wg-launcher ")
            .and_then(|line| line.split_whitespace().next())
    }
}
