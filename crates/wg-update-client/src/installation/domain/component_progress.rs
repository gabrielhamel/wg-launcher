use crate::catalog::ComponentVersions;
use serde::{Deserialize, Serialize};

/// Component versions reached locally: starting installed versions, then saved completions.
/// The transparent representation preserves the pinned manifest's version map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ComponentProgress(ComponentVersions);

impl ComponentProgress {
    pub(super) fn versions(&self) -> &ComponentVersions {
        &self.0
    }

    pub(super) fn seed_missing(&mut self, installed: &ComponentVersions) {
        for (part, version) in installed.iter() {
            if self.0.get(part).is_none() {
                self.0.insert(part.clone(), version.clone());
            }
        }
    }

    /// Saved checkpoints override the caller's versions when resuming work.
    pub(super) fn restore_saved_checkpoints(&mut self, saved: Self) {
        for (part, version) in saved.0.iter() {
            self.0.insert(part.clone(), version.clone());
        }
    }

    pub(super) fn record_completion(&mut self, part: &str, version: &str) {
        self.0.insert(part.to_owned(), version.to_owned());
    }
}
