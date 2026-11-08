use super::patch::{PatchProgress, apply_and_checkpoint_patch};
use super::{InstallationPipeline, finalization::finalize_installation};
use crate::Result;
use crate::installation::infrastructure::filesystem::InstallationLock;
use crate::installation::infrastructure::filesystem::{
    cache::verify_cache,
    records::save_manifest,
    storage::{InstallationLayout, prepare_installation},
};
use crate::installation::{ExtractionEvent, ExtractionOptions, ExtractionReport, PinnedManifest};
use std::{path::Path, sync::Arc};

pub(super) fn extract_sync(
    manifest: &PinnedManifest,
    storage: &InstallationLayout,
    options: ExtractionOptions,
    observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync>,
) -> Result<ExtractionReport> {
    let cache = &storage.cache;
    observer(ExtractionEvent::CheckingCache {
        assets: manifest.release.assets.len(),
    });
    let mut manifest = super::resume_manifest(manifest.clone(), storage)?;
    let patches = manifest
        .ordered_patches()?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    verify_cache(&patches.iter().collect::<Vec<_>>(), cache)?;
    save_manifest(&manifest, storage)?;
    prepare_installation(storage)?;

    let total = patches.len();
    let mut report = ExtractionReport::default();
    for (index, patch) in patches.into_iter().enumerate() {
        let completion = apply_and_checkpoint_patch(
            &mut manifest,
            &patch,
            storage,
            PatchProgress { index, total },
            options,
            &observer,
        )?;
        report.applied_deltas += completion.applied_deltas;
    }
    finalize_installation(&manifest, storage, options, report.applied_deltas, observer)
}

impl InstallationPipeline {
    /// Creates a stateless installation coordinator.
    pub const fn new() -> Self {
        Self
    }

    /// Extracts an already downloaded manifest with default cache cleanup.
    ///
    /// Equivalent to [`Self::extract_with_options_and_observer`] with [`ExtractionOptions::default`].
    /// Reads volumes from `install_root/.wg-launcher/download` without network access and writes
    /// the installed-build record on success.
    ///
    /// # Errors
    ///
    /// Returns the lock, cache, patch-chain, filesystem, archive, delta, or worker
    /// errors described by [`Self::extract_with_options_and_observer`].
    pub async fn extract_with_observer<F>(
        &self,
        manifest: PinnedManifest,
        install_root: impl AsRef<Path>,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        self.extract_with_options_and_observer(
            manifest,
            install_root,
            ExtractionOptions::default(),
            observer,
        )
        .await
    }

    /// Extracts local packages with an explicit cache-retention policy.
    ///
    /// Acquires the installation lock, verifies required volume sizes, applies the
    /// remaining patches, and records the installed build. Pins the supplied manifest
    /// and preserves completed checkpoints and original delta sources on retry.
    /// Callbacks run on the blocking extraction worker; see the
    /// [observer contract](crate#observers-and-async-execution).
    ///
    /// # Errors
    ///
    /// Returns an error for lock contention, incomplete cache, invalid patch chains or
    /// paths, filesystem failures, invalid archives, failed deltas, or worker failure.
    /// Cache verification uses sizes rather than content hashes.
    pub async fn extract_with_options_and_observer<F>(
        &self,
        manifest: PinnedManifest,
        install_root: impl AsRef<Path>,
        options: ExtractionOptions,
        observer: F,
    ) -> Result<ExtractionReport>
    where
        F: Fn(ExtractionEvent) + Send + Sync + 'static,
    {
        let install_root = install_root.as_ref().to_path_buf();
        let storage = InstallationLayout::for_root(&install_root);
        let _lock = InstallationLock::acquire(&storage)?;
        let observer: Arc<dyn Fn(ExtractionEvent) + Send + Sync> = Arc::new(observer);
        tokio::task::spawn_blocking(move || extract_sync(&manifest, &storage, options, observer))
            .await
            .map_err(crate::installation::PipelineError::WorkerFailed)?
    }
}
