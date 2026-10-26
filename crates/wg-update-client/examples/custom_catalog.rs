use wg_update_client::{BuildSelection, InspectOptions, ListOptions, Product, inspect, list};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let products = vec![
        Product {
            game: "local".into(),
            region: "eu".into(),
            name: "Local EU game".into(),
            host: "http://localhost:8080/updates".parse()?,
            game_id: "LOCAL.EU".into(),
            channel: "testing".into(),
        },
        Product {
            game: "local".into(),
            region: "KR".into(),
            name: "Local KR game".into(),
            host: "http://localhost:8081/updates".parse()?,
            game_id: "LOCAL.KR".into(),
            channel: "testing".into(),
        },
    ];
    let catalog = list(ListOptions {
        products: Some(products.clone()),
        ..Default::default()
    })
    .await?;
    println!("Found {} builds", catalog.builds.len());

    let build = inspect(InspectOptions {
        selection: BuildSelection {
            game: Some("local".into()),
            region: "KR".into(),
            products: Some(products),
            ..Default::default()
        },
    })
    .await?;
    println!("{}: {}", build.region, build.version);
    Ok(())
}
