//! Turn self-update failure facts into terminal diagnostics and recovery guidance.

use crate::self_update::{
    application::ReleaseCheckError,
    domain::artifact::ArtifactSelectionError,
    infrastructure::{
        ArchiveError, HttpError, artifact::ArtifactDownloadError, executable::ExecutableError,
        release::ReleaseError,
    },
};
use std::error::Error;

pub(super) fn describe(error: &(dyn Error + 'static)) -> String {
    if let Some(ReleaseCheckError(cause)) = error.downcast_ref() {
        return format!(
            "Cannot check the latest CLI release: {}. Check your connection or release mirror and try again.",
            describe(cause.as_ref())
        );
    }
    if let Some(error) = error.downcast_ref::<ArtifactSelectionError>() {
        return match error {
            ArtifactSelectionError::UnsupportedHost => "Self-update not supported for this platform; install a compatible CLI build manually.",
            ArtifactSelectionError::MissingArtifact => "The latest release has no archive and checksum for this platform.",
        }.into();
    }
    if let Some(error) = error.downcast_ref::<ArchiveError>() {
        return match error {
            ArchiveError::SymbolicLink => {
                "The CLI archive contains a symbolic link instead of an executable."
            }
            ArchiveError::UnsafeEntry => "The CLI archive has an unsafe executable entry.",
            ArchiveError::MissingExecutable => {
                "The CLI archive does not contain the launcher executable."
            }
            ArchiveError::DuplicateExecutable => {
                "The CLI archive contains duplicate launcher executables."
            }
            ArchiveError::InvalidSize => "The CLI executable has an invalid size.",
        }
        .into();
    }
    if let Some(error) = error.downcast_ref::<ArtifactDownloadError>() {
        return match error {
            ArtifactDownloadError::InvalidChecksum => {
                "The CLI archive checksum is invalid; the installed executable was preserved."
            }
            ArtifactDownloadError::TooLarge => {
                "CLI archive is too large; the installed executable was preserved."
            }
            ArtifactDownloadError::ChecksumMismatch => {
                "CLI archive checksum mismatch; the installed executable was preserved."
            }
        }
        .into();
    }
    if let Some(error) = error.downcast_ref::<ExecutableError>() {
        return match error {
            ExecutableError::MissingDirectory => "Cannot locate the launcher directory.".into(),
            ExecutableError::Staging { directory, source } => format!(
                "Cannot prepare the CLI update in {}: {source}. Check directory permissions.", directory.display()
            ),
            ExecutableError::Timeout => "The downloaded CLI did not respond; the installed executable was preserved.".into(),
            ExecutableError::VersionMismatch => "The downloaded CLI failed its version check; the installed executable was preserved.".into(),
            ExecutableError::Replacement(source) => format!(
                "Cannot replace the CLI executable: {source}. Check directory permissions."
            ),
        };
    }
    if let Some(error) = error.downcast_ref::<ReleaseError>() {
        return match error {
            ReleaseError::MissingCli => "The release manifest does not contain this CLI.",
            ReleaseError::Unstable => "The latest CLI release is not a stable version.",
            ReleaseError::InvalidRepository => "Invalid CLI repository URL.",
        }
        .into();
    }
    if let Some(error) = error.downcast_ref::<HttpError>() {
        return match error {
            HttpError::UnsafeUrl => {
                "CLI updates require HTTPS (HTTP is allowed only for loopback mirrors)."
            }
            HttpError::MetadataTooLarge => "CLI update metadata is too large.",
        }
        .into();
    }
    error.to_string()
}
