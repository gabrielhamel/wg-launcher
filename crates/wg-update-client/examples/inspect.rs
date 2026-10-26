use wg_update_client::{BuildSelection, InspectOptions, ListOptions, inspect, list};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let catalog = list(ListOptions {
        region: Some("eu".into()),
        ..Default::default()
    })
    .await?;

    for build in catalog.builds {
        println!(
            "{} / {}: {}",
            build.region, build.client_type, build.version
        );
    }
    for warning in catalog.warnings {
        eprintln!("{}: {}", warning.client_type, warning.message);
    }

    let build = inspect(InspectOptions {
        selection: BuildSelection {
            game: Some("wot".into()),
            ..Default::default()
        },
    })
    .await?;
    println!("Version: {}", build.version);
    println!("Download: {} bytes", build.download_size());
    println!("Components: {}", build.parts.join(", "));
    Ok(())
}
