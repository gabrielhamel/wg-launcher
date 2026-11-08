//! Progress contract shared by self-update orchestration, adapters, and presentation.

use super::Result;

/// A step in checking and replacing the launcher executable.
#[derive(Clone, Copy)]
pub(crate) enum SelfUpdateStage {
    CheckingRelease,
    FetchingChecksum,
    DownloadingArchive,
    VerifyingArchive,
    OpeningArchive,
    ExtractingExecutable,
    VerifyingExecutable,
    ReplacingExecutable,
}

/// Progress facts; terminal messages and styling belong to presentation.
pub(crate) enum SelfUpdateEvent {
    Stage(SelfUpdateStage),
    ReleaseChecked,
    Transfer {
        stage: SelfUpdateStage,
        total: Option<u64>,
    },
    Advanced(u64),
}

pub(crate) type Observer<'a> = dyn Fn(SelfUpdateEvent) -> Result<()> + 'a;
