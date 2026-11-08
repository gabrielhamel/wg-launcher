//! Identity of the component transition whose original files are retained for retry.

use crate::catalog::{Patch, PatchSource};

pub(crate) struct PatchIdentity<'a> {
    pub(crate) component: &'a str,
    pub(crate) source: &'a PatchSource,
    pub(crate) target: &'a str,
}

impl<'a> From<&'a Patch> for PatchIdentity<'a> {
    fn from(patch: &'a Patch) -> Self {
        Self {
            component: &patch.component_id,
            source: &patch.source,
            target: &patch.version_to,
        }
    }
}
