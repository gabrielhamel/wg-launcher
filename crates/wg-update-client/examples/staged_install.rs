use wg_update_client::{BuildSelection, DownloadAssetsOptions, ExtractOptions, download, extract};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let path = std::path::PathBuf::from("./world-of-tanks");
    let cached = download(DownloadAssetsOptions {
        selection: BuildSelection {
            game: Some("wot".into()),
            ..Default::default()
        },
        output: Some(path.clone()),
        ..Default::default()
    })
    .await?;
    println!("Packages saved in {}", cached.plan.cache.display());

    // This step uses only the downloaded files, so it can run offline.
    let installed = extract(ExtractOptions { path }).await?;
    println!("Installed {}", installed.plan.manifest.version);
    Ok(())
}
