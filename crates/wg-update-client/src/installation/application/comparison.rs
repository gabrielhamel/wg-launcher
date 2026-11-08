//! Adapt an installation comparison to the consumer status report.

use super::status::{StatusReport, UpdateTarget, UpdateTargetSource};
use crate::{
    Result,
    installation::{InstalledBuild, PinnedManifest},
};
use std::path::PathBuf;

pub(super) fn build_status(
    path: PathBuf,
    installed: InstalledBuild,
    manifest: &PinnedManifest,
    source: UpdateTargetSource,
) -> Result<StatusReport> {
    let comparison = installed.compare_with(manifest)?;
    Ok(StatusReport {
        path,
        installed: crate::catalog::BuildInfo::from_identity(
            &installed.product,
            &installed.identity,
        ),
        target: UpdateTarget {
            version: manifest.release.identity.version.clone(),
            source,
        },
        update_available: comparison.update_available,
        update_download_bytes: comparison.download_bytes,
    })
}
