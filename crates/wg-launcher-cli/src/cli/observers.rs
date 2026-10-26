use super::output::{human_bytes, print_plan};
use crate::progress::{DownloadProgress, ExtractionProgress};
use wg_update_client::{DownloadAssetsEvent, ExtractEvent, InstallEvent};

pub(super) type ProgressBars = Option<(DownloadProgress, ExtractionProgress)>;

pub(super) fn finish_install_progress(
    progress: &std::sync::Mutex<ProgressBars>,
    report: &wg_update_client::InstallationReport,
) {
    if let Some((download, extraction)) = progress.lock().expect("progress lock").as_ref() {
        download.finish_success();
        extraction.finish_success(&report.extraction);
    }
}

pub(super) fn fail_install_progress(progress: &std::sync::Mutex<ProgressBars>) {
    if let Some((download, extraction)) = progress.lock().expect("progress lock").as_ref() {
        download.finish_error();
        extraction.finish_error();
    }
}

pub(super) fn observe_install(event: InstallEvent, progress: &std::sync::Mutex<ProgressBars>) {
    match event {
        InstallEvent::Planned(plan) => {
            let plan = *plan;
            println!(
                "{} {} {} {}: {} assets, {} -> {}",
                plan.manifest.region,
                plan.manifest.version,
                plan.manifest.client_type,
                plan.manifest.language,
                plan.assets.len(),
                human_bytes(plan.compressed_bytes),
                plan.root.display(),
            );
            println!("  package cache: {}", plan.cache.display());
            match DownloadProgress::new(
                plan.compressed_bytes,
                plan.assets
                    .iter()
                    .map(|asset| (asset.name.as_str(), asset.size)),
            ) {
                Ok(download) => match ExtractionProgress::new(&download) {
                    Ok(extraction) => {
                        *progress.lock().expect("progress lock") = Some((download, extraction));
                    }
                    Err(error) => {
                        eprintln!("warning: terminal progress unavailable: {error}");
                    }
                },
                Err(error) => {
                    eprintln!("warning: terminal progress unavailable: {error}");
                }
            }
        }
        InstallEvent::Progress(event) => {
            let guard = progress.lock().expect("progress lock");
            let Some((download, extraction)) = guard.as_ref() else {
                return;
            };
            match event {
                wg_update_client::InstallationEvent::Download(event) => download.update(event),
                wg_update_client::InstallationEvent::Extraction(event) => extraction.update(event),
                wg_update_client::InstallationEvent::ReleasedPackages => {
                    extraction.finish_patch();
                }
            }
        }
    }
}

pub(super) fn observe_download(
    event: DownloadAssetsEvent,
    progress: &std::sync::Mutex<Option<DownloadProgress>>,
) {
    match event {
        DownloadAssetsEvent::Planned(plan) => {
            print_plan(&plan);
            match DownloadProgress::new(
                plan.compressed_bytes,
                plan.assets
                    .iter()
                    .map(|asset| (asset.name.as_str(), asset.size)),
            ) {
                Ok(download) => *progress.lock().expect("progress lock") = Some(download),
                Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
            }
        }
        DownloadAssetsEvent::Progress(event) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.update(event);
            }
        }
    }
}

pub(super) fn observe_extract(
    event: ExtractEvent,
    progress: &std::sync::Mutex<Option<ExtractionProgress>>,
) {
    match event {
        ExtractEvent::Planned(plan) => {
            print_plan(&plan);
            match ExtractionProgress::standalone() {
                Ok(extraction) => *progress.lock().expect("progress lock") = Some(extraction),
                Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
            }
        }
        ExtractEvent::Progress(event) => {
            if let Some(progress) = progress.lock().expect("progress lock").as_ref() {
                progress.update(event);
            }
        }
    }
}
