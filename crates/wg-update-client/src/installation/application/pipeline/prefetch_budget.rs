//! Byte budget for packages queued ahead of the patch being applied.

use crate::catalog::PlannedPatch;

pub(super) struct PrefetchBudget {
    patch_bytes: Vec<u128>,
    limit: u64,
}

impl PrefetchBudget {
    pub(super) fn new(patches: &[PlannedPatch], limit: u64) -> Self {
        Self {
            patch_bytes: patches
                .iter()
                .map(|planned| {
                    planned
                        .volumes()
                        .iter()
                        .map(|volume| u128::from(volume.size))
                        .sum()
                })
                .collect(),
            limit,
        }
    }

    /// The current patch is always allowed. A later patch may start below the
    /// limit even if its own packages carry the queue above that limit.
    pub(super) fn allows(&self, applying_patch: usize, candidate_patch: usize) -> bool {
        if candidate_patch <= applying_patch {
            return true;
        }
        let queued_bytes: u128 = self.patch_bytes
            [applying_patch.saturating_add(1)..candidate_patch]
            .iter()
            .sum();
        queued_bytes < u128::from(self.limit)
    }
}
