use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Component versions reached locally: starting installed versions, then saved completions.
/// The transparent representation preserves the pinned manifest's version map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ComponentProgress(BTreeMap<String, String>);

impl ComponentProgress {
    pub(super) fn versions(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    pub(super) fn seed_missing(&mut self, installed: &BTreeMap<String, String>) {
        for (part, version) in installed {
            self.0
                .entry(part.clone())
                .or_insert_with(|| version.clone());
        }
    }

    pub(super) fn retain_saved(&mut self, saved: Self) {
        self.0.extend(saved.0);
    }

    pub(super) fn record_completion(&mut self, part: &str, version: &str) {
        self.0.insert(part.to_owned(), version.to_owned());
    }
}
