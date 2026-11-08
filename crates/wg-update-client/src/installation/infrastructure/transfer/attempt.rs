//! Execute one volume attempt after deciding how its local cache can be reused.

use super::error::{Result, TransferError as Error};
use super::{
    DownloadEvent, VolumeDownloader,
    downloader::VolumeOutcome,
    local::{self, DownloadPreparation, VolumePaths},
};
use crate::catalog::TransportVolume;
use futures_util::StreamExt;
use reqwest::{StatusCode, header};
use std::{path::Path, sync::Arc};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
};

impl VolumeDownloader {
    pub(super) async fn download_once(
        &self,
        asset: &TransportVolume,
        output: &Path,
        observer: Arc<dyn Fn(DownloadEvent) + Send + Sync>,
    ) -> Result<VolumeOutcome> {
        match local::prepare(asset, output).await? {
            DownloadPreparation::ReuseComplete => {
                observer(DownloadEvent::Skipped {
                    name: asset.name.clone(),
                    size: asset.size,
                });
                Ok(VolumeOutcome::Reused)
            }
            DownloadPreparation::PromotePartial(paths) => {
                paths.promote().await?;
                observer(DownloadEvent::Completed {
                    name: asset.name.clone(),
                    size: asset.size,
                });
                Ok(VolumeOutcome::Completed {
                    new_bytes_retained: 0,
                })
            }
            DownloadPreparation::TransferFrom { paths, offset } => {
                self.transfer_from(asset, paths, offset, &observer).await
            }
        }
    }

    async fn transfer_from(
        &self,
        asset: &TransportVolume,
        paths: VolumePaths,
        mut resumed_at: u64,
        observer: &Arc<dyn Fn(DownloadEvent) + Send + Sync>,
    ) -> Result<VolumeOutcome> {
        let mut request = self.http.get(asset.url.clone());
        if resumed_at > 0 {
            request = request.header(header::RANGE, format!("bytes={resumed_at}-"));
        }
        let response = request.send().await?.error_for_status()?;
        let append = resumed_at > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
        if resumed_at > 0 && !append && response.status() != StatusCode::OK {
            return Err(Error::ResumeRejected {
                path: paths.partial,
                status: response.status().as_u16(),
            });
        }
        if !append {
            resumed_at = 0;
        }
        observer(DownloadEvent::Started {
            name: asset.name.clone(),
            size: asset.size,
            resumed_at,
        });
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&paths.partial)
            .await?;
        let mut writer = BufWriter::new(file);
        let mut downloaded = resumed_at;
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk?;
            writer.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            observer(DownloadEvent::Progress {
                name: asset.name.clone(),
                downloaded,
                size: asset.size,
            });
        }
        writer.flush().await?;
        drop(writer);
        paths.verify_and_promote(asset.size).await?;
        observer(DownloadEvent::Completed {
            name: asset.name.clone(),
            size: asset.size,
        });
        Ok(VolumeOutcome::Completed {
            new_bytes_retained: asset.size - resumed_at,
        })
    }
}
