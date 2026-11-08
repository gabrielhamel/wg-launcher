//! Download positions and lifecycle transitions, independent of terminal widgets.

use std::collections::HashMap;
use wg_update_client::DownloadEvent;

pub(super) struct DownloadAccounting {
    expected_files: usize,
    files: HashMap<String, FileTransfer>,
}

#[derive(Default)]
struct FileTransfer {
    attempt_position: u64,
    credited_bytes: u64,
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
    pub(super) completed_bytes_delta: u64,
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
        // Retransmitting an earlier prefix must not count twice toward package completion.
        let credited_bytes = position.min(size).max(file.credited_bytes);
        let completed_bytes_delta = credited_bytes - file.credited_bytes;
        file.credited_bytes = credited_bytes;
        file.attempt_position = position;
        file.activity = activity;
        DownloadUpdate {
            name,
            size,
            position: file.attempt_position,
            activity,
            became_active,
            completed_bytes_delta,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_do_not_credit_the_same_package_bytes_twice() {
        let mut accounting = DownloadAccounting::new(1);
        accounting.register("client.wgpkg");
        let events = [
            DownloadEvent::Started {
                name: "client.wgpkg".into(),
                size: 100,
                resumed_at: 40,
            },
            DownloadEvent::Progress {
                name: "client.wgpkg".into(),
                size: 100,
                downloaded: 80,
            },
            DownloadEvent::Started {
                name: "client.wgpkg".into(),
                size: 100,
                resumed_at: 0,
            },
            DownloadEvent::Progress {
                name: "client.wgpkg".into(),
                size: 100,
                downloaded: 50,
            },
            DownloadEvent::Completed {
                name: "client.wgpkg".into(),
                size: 100,
            },
        ];
        let updates = events.map(|event| accounting.observe(event));
        assert_eq!(
            updates.each_ref().map(|update| update.position),
            [40, 80, 0, 50, 100]
        );
        assert_eq!(
            updates
                .each_ref()
                .map(|update| update.completed_bytes_delta),
            [40, 40, 0, 0, 20]
        );
        assert!(accounting.all_complete());
    }

    #[test]
    fn cache_hits_complete_packages_without_active_transfers_or_double_credit() {
        let mut accounting = DownloadAccounting::new(1);
        accounting.register("client.wgpkg");
        assert!(!accounting.all_complete());
        let skipped = || DownloadEvent::Skipped {
            name: "client.wgpkg".into(),
            size: 100,
        };
        assert_eq!(accounting.observe(skipped()).completed_bytes_delta, 100);
        assert_eq!(accounting.observe(skipped()).completed_bytes_delta, 0);
        assert!(accounting.all_complete());
        assert!(!accounting.is_transferring());
    }
}
