//! Turn self-update failure facts into terminal diagnostics and recovery guidance.

use crate::self_update::{
    application::SelfUpdateError,
    domain::artifact::ArtifactSelectionError,
    infrastructure::{
        ArchiveError, HttpError, artifact::CandidatePreparationError, executable::ExecutableError,
        release::ReleaseDiscoveryError,
    },
};

pub(super) fn describe(error: &SelfUpdateError) -> String {
    match error {
        SelfUpdateError::ReleaseCheck(cause) => format!(
            "Cannot check the latest CLI release: {}. Check your connection or release mirror and try again.",
            describe_release(cause)
        ),
        SelfUpdateError::ArtifactSelection(error) => match error {
            ArtifactSelectionError::UnsupportedHost => "Self-update not supported for this platform; install a compatible CLI build manually.",
            ArtifactSelectionError::MissingArtifact => "The latest release has no archive and checksum for this platform.",
        }.into(),
        SelfUpdateError::CandidatePreparation(error) => describe_preparation(error),
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
            ExecutableError::Io(_) => error.to_string(),
        },
        SelfUpdateError::Progress(error) => error.to_string(),
    }
}

fn describe_release(error: &ReleaseDiscoveryError) -> String {
    match error {
        ReleaseDiscoveryError::MissingCli => {
            "The release manifest does not contain this CLI.".into()
        }
        ReleaseDiscoveryError::Unstable => "The latest CLI release is not a stable version.".into(),
        ReleaseDiscoveryError::InvalidRepository => "Invalid CLI repository URL.".into(),
        ReleaseDiscoveryError::Http(error) => describe_http(error),
        ReleaseDiscoveryError::Url(_)
        | ReleaseDiscoveryError::Json(_)
        | ReleaseDiscoveryError::Version(_) => error.to_string(),
    }
}

fn describe_preparation(error: &CandidatePreparationError) -> String {
    match error {
        CandidatePreparationError::InvalidChecksum => {
            "The CLI archive checksum is invalid; the installed executable was preserved.".into()
        }
        CandidatePreparationError::TooLarge => {
            "CLI archive is too large; the installed executable was preserved.".into()
        }
        CandidatePreparationError::ChecksumMismatch => {
            "CLI archive checksum mismatch; the installed executable was preserved.".into()
        }
        CandidatePreparationError::Http(error) => describe_http(error),
        CandidatePreparationError::Archive(error) => describe_archive(error),
        CandidatePreparationError::Request(_)
        | CandidatePreparationError::Url(_)
        | CandidatePreparationError::Utf8(_)
        | CandidatePreparationError::Io(_) => error.to_string(),
    }
}

fn describe_archive(error: &ArchiveError) -> String {
    match error {
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
        ArchiveError::Io(_) | ArchiveError::Zip(_) => return error.to_string(),
    }
    .into()
}

fn describe_http(error: &HttpError) -> String {
    match error {
        HttpError::UnsafeUrl => {
            "CLI updates require HTTPS (HTTP is allowed only for loopback mirrors).".into()
        }
        HttpError::MetadataTooLarge => "CLI update metadata is too large.".into(),
        HttpError::Request(_) => error.to_string(),
    }
}
