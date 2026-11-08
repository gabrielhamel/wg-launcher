//! Identity of a published component transition, shared by planning and recovery.

use super::{Patch, PatchSource, TransportVolume};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl<'a> From<&'a TransportVolume> for PatchIdentity<'a> {
    fn from(volume: &'a TransportVolume) -> Self {
        Self {
            component: &volume.component_id,
            source: &volume.source,
            target: &volume.version_to,
        }
    }
}
