pub(super) mod artifact;
pub(super) mod candidate;
pub(super) mod ownership;

use reqwest::Url;
use semver::Version;

pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) base: Url,
    pub(crate) artifacts: Vec<String>,
}

impl Release {
    pub(crate) fn is_newer_than(&self, current: &Version) -> bool {
        self.version.cmp_precedence(current).is_gt()
    }
}
