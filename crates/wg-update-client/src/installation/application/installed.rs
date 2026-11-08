//! Load and validate the saved identity shared by status and update.

pub(super) async fn load_installed_build(
    root: &std::path::Path,
) -> crate::Result<crate::installation::InstalledBuild> {
    let installed =
        crate::installation::infrastructure::filesystem::records::load_installed_build(root)
            .await?
            .ok_or_else(|| InstalledBuildError::InstallationNotFound(root.to_path_buf()))?;
    installed.validate_consistency()?;
    Ok(installed)
}

/// Missing installed identity for status and update workflows.
#[derive(Debug, thiserror::Error)]
pub enum InstalledBuildError {
    /// The installation root had no installed-build record.
    #[error("no wg-launcher installation record was found for {0:?}")]
    InstallationNotFound(std::path::PathBuf),
}
