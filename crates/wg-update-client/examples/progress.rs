use wg_update_client::{BuildSelection, InstallEvent, InstallOptions, install_with_observer};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let installed = install_with_observer(
        InstallOptions {
            selection: BuildSelection {
                game: Some("wot".into()),
                ..Default::default()
            },
            output: Some("./world-of-tanks".into()),
            ..Default::default()
        },
        |event| match event {
            InstallEvent::Planned(plan) => {
                println!(
                    "Installing {} into {}",
                    plan.manifest.version,
                    plan.root.display()
                );
            }
            InstallEvent::Progress(progress) => println!("{progress:?}"),
        },
    )
    .await?;

    println!("Finished installing {}", installed.plan.manifest.version);
    Ok(())
}
