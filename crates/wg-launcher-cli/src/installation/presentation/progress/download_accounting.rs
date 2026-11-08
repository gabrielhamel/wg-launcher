//! Download positions and lifecycle transitions, independent of terminal widgets.

use std::collections::HashMap;
use wg_update_client::DownloadEvent;

pub(super) struct DownloadAccounting {
    expected_files: usize,
    files: HashMap<String, FileTransfer>,
}

#[derive(Default)]
struct FileTransfer {
    position: u64,
    activity: FileActivity,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum FileActivity {
    #[default]
    Queued,
    Transferring,
    Completed,
}

/// Facts needed to update a volume's existing bar after one event.
pub(super) struct DownloadUpdate {
    pub(super) name: String,
    pub(super) size: u64,
    pub(super) position: u64,
    pub(super) activity: FileActivity,
    pub(super) became_active: bool,
    pub(super) advanced_bytes: u64,
}

impl DownloadAccounting {
    pub(super) fn new(expected_files: usize) -> Self {
        Self {
            expected_files,
            files: HashMap::new(),
        }
    }

    pub(super) fn register(&mut self, name: &str) {
        self.files.entry(name.to_owned()).or_default();
    }

    pub(super) fn observe(&mut self, event: DownloadEvent) -> DownloadUpdate {
        let (name, size, position, activity) = match event {
            DownloadEvent::Started {
                name,
                size,
                resumed_at,
            } => (name, size, resumed_at, FileActivity::Transferring),
            DownloadEvent::Progress {
                name,
                size,
                downloaded,
            } => (name, size, downloaded, FileActivity::Transferring),
            DownloadEvent::Completed { name, size } | DownloadEvent::Skipped { name, size } => {
                (name, size, size, FileActivity::Completed)
            }
        };
        let file = self.files.entry(name.clone()).or_default();
        let became_active = activity == FileActivity::Transferring && file.activity != activity;
        // Preserve positive-increment accounting when a retry resets a volume's position.
        let advanced_bytes = position.saturating_sub(file.position);
        file.position = position;
        file.activity = activity;
        DownloadUpdate {
            name,
            size,
            position,
            activity,
            became_active,
            advanced_bytes,
        }
    }

    pub(super) fn all_complete(&self) -> bool {
        self.files.len() == self.expected_files
            && self
                .files
                .values()
                .all(|file| file.activity == FileActivity::Completed)
    }

    pub(super) fn is_transferring(&self) -> bool {
        self.files
            .values()
            .any(|file| file.activity == FileActivity::Transferring)
    }

    pub(super) fn is_complete(&self, name: &str) -> bool {
        self.files
            .get(name)
            .is_some_and(|file| file.activity == FileActivity::Completed)
    }
}
