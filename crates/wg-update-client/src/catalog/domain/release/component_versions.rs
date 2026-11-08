//! Versions known for named game components, independent of local checkpoint policy.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Missing components start at `0`; recorded versions retain publisher spelling.
/// Transparent serialization preserves the existing JSON object representation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ComponentVersions(BTreeMap<String, String>);

impl ComponentVersions {
    pub(crate) fn get(&self, component: &str) -> Option<&str> {
        self.0.get(component).map(String::as_str)
    }

    pub(crate) fn version_or_zero(&self, component: &str) -> &str {
        self.get(component).unwrap_or("0")
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter()
    }

    pub(crate) fn insert(&mut self, component: String, version: String) {
        self.0.insert(component, version);
    }
}

impl FromIterator<(String, String)> for ComponentVersions {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(versions: T) -> Self {
        Self(versions.into_iter().collect())
    }
}
