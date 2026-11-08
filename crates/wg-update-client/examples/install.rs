use wg_update_client::{
    BuildSelection, InstallOptions, StatusOptions, UpdateOptions, install, status, update,
};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let path = std::path::PathBuf::from("./world-of-tanks");
    let installed = install(InstallOptions {
        selection: BuildSelection {
            game: Some("wot".into()),
            ..Default::default()
        },
        output: Some(path.clone()),
        ..Default::default()
    })
    .await?;
    println!("Installed {}", installed.plan.build.version);

    let current = status(StatusOptions { path: path.clone() }).await?;
    println!(
        "Installed: {} / Target: {}",
        current.installed.version, current.target.version
    );

    if current.update_available {
        update(UpdateOptions {
            path,
            download: Default::default(),
        })
        .await?;
        println!("Update complete");
    }
    Ok(())
}
