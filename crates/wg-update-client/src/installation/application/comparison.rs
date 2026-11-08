//! Adapt an installation comparison to the consumer status report.

use super::status::{StatusReport, UpdateTarget, UpdateTargetSource};
use crate::{
    Result,
    catalog::{ResolvedRelease, required_volumes},
    installation::InstalledBuild,
};
use std::path::PathBuf;

pub(super) fn build_status(
    path: PathBuf,
    installed: &InstalledBuild,
    target: &ResolvedRelease,
    reached_versions: &std::collections::BTreeMap<String, String>,
    source: UpdateTargetSource,
) -> Result<StatusReport> {
    let update_available = installed.differs_from(target);
    let download_bytes = if update_available {
        required_volumes(target, reached_versions)?
            .iter()
            .map(|asset| asset.size)
            .sum()
    } else {
        0
    };
    Ok(StatusReport {
        path,
        installed: crate::catalog::BuildInfo::from_identity(
            &installed.product,
            &installed.identity,
        ),
        target: UpdateTarget {
            version: target.identity.version.clone(),
            source,
        },
        update_available,
        update_download_bytes: download_bytes,
    })
}
