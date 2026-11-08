//! Interpret the supported launcher version formats before executable replacement.

use semver::Version;

pub(crate) struct CandidateVersion(Version);

impl CandidateVersion {
    pub(crate) fn parse(stdout: &str) -> Option<Self> {
        let mut lines = stdout.lines();
        let first = lines.next()?;
        let reported = if first == "wg-launcher" {
            lines.next()?.strip_prefix("Installed:")?.trim()
        } else {
            first
                .strip_prefix("wg-launcher ")?
                .split_whitespace()
                .next()?
        };
        Version::parse(reported).ok().map(Self)
    }

    /// Executable verification requires the exact release, including build metadata.
    pub(crate) fn matches(&self, expected: &Version) -> bool {
        &self.0 == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_output_formats_and_requires_the_exact_release() {
        let expected = Version::parse("1.4.0-beta.1+build.2").unwrap();
        for output in [
            "wg-launcher 1.4.0-beta.1+build.2\n",
            "wg-launcher\r\nInstalled:    1.4.0-beta.1+build.2\r\nLatest:       unavailable\r\n",
        ] {
            let version = CandidateVersion::parse(output).unwrap();
            assert!(version.matches(&expected));
            assert!(!version.matches(&Version::parse("1.4.0-beta.1+build.3").unwrap()));
            assert!(!version.matches(&Version::parse("1.4.0").unwrap()));
        }
    }

    #[test]
    fn rejects_missing_malformed_or_unrelated_versions() {
        for output in [
            "",
            "other-program 1.4.0",
            "wg-launcher",
            "wg-launcher nonsense",
            "wg-launcher\nLatest: 1.4.0",
            "wg-launcher\nInstalled: 1.4",
        ] {
            assert!(CandidateVersion::parse(output).is_none(), "{output:?}");
        }
    }
}
