//! Turn self-update failure facts into terminal diagnostics and recovery guidance.

use crate::self_update::{
    application::SelfUpdateError,
    domain::artifact::ArtifactSelectionError,
    infrastructure::{
        ArchiveError, HttpError, artifact::ArtifactDownloadError, executable::ExecutableError,
        release::ReleaseError,
    },
};

pub(super) fn describe(error: &SelfUpdateError) -> String {
    match error {
        SelfUpdateError::ReleaseCheck(cause) => {
            format!(
                "Cannot check the latest CLI release: {}. Check your connection or release mirror and try again.",
                describe(cause)
            )
        }
        SelfUpdateError::ArtifactSelection(error) => match error {
            ArtifactSelectionError::UnsupportedHost => "Self-update not supported for this platform; install a compatible CLI build manually.",
            ArtifactSelectionError::MissingArtifact => "The latest release has no archive and checksum for this platform.",
        }.into(),
        SelfUpdateError::Archive(error) => match error {
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
        }.into(),
        SelfUpdateError::ArtifactDownload(error) => match error {
            ArtifactDownloadError::InvalidChecksum => {
                "The CLI archive checksum is invalid; the installed executable was preserved."
            }
            ArtifactDownloadError::TooLarge => {
                "CLI archive is too large; the installed executable was preserved."
            }
            ArtifactDownloadError::ChecksumMismatch => {
                "CLI archive checksum mismatch; the installed executable was preserved."
            }
        }.into(),
        SelfUpdateError::Executable(error) => match error {
            ExecutableError::MissingDirectory => "Cannot locate the launcher directory.".into(),
            ExecutableError::Staging { directory, source } => format!(
                "Cannot prepare the CLI update in {}: {source}. Check directory permissions.", directory.display()
            ),
            ExecutableError::Timeout => "The downloaded CLI did not respond; the installed executable was preserved.".into(),
            ExecutableError::VersionMismatch => "The downloaded CLI failed its version check; the installed executable was preserved.".into(),
            ExecutableError::Replacement(source) => format!(
                "Cannot replace the CLI executable: {source}. Check directory permissions."
            ),
        },
        SelfUpdateError::Release(error) => match error {
            ReleaseError::MissingCli => "The release manifest does not contain this CLI.",
            ReleaseError::Unstable => "The latest CLI release is not a stable version.",
            ReleaseError::InvalidRepository => "Invalid CLI repository URL.",
        }.into(),
        SelfUpdateError::Http(error) => match error {
            HttpError::UnsafeUrl => {
                "CLI updates require HTTPS (HTTP is allowed only for loopback mirrors)."
            }
            HttpError::MetadataTooLarge => "CLI update metadata is too large.",
        }.into(),
        SelfUpdateError::Io(_)
        | SelfUpdateError::Request(_)
        | SelfUpdateError::Url(_)
        | SelfUpdateError::Json(_)
        | SelfUpdateError::Version(_)
        | SelfUpdateError::Utf8(_)
        | SelfUpdateError::Zip(_) => error.to_string(),
    }
}
