# wg-update-client

Bring game-asset discovery, downloads, installation, and updates into your Rust app.
The library works on Windows, Linux, and macOS and stays silent unless your app
chooses to display progress. Supported game catalogs publish Windows/x64 payloads.

[Documentation home](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/README.md) · [CLI guide](https://github.com/gabrielhamel/wg-launcher/blob/main/crates/wg-launcher-cli/README.md) · [API reference](https://docs.rs/wg-update-client/1.0.0/wg_update_client/)

**On this page:** [Set up](#set-up) · [Discover builds](#discover-builds) · [Install and update](#install-and-update) · [Progress](#observers-and-async-execution) · [Offline installation](#download-now-extract-later) · [Custom catalogs](#use-your-own-catalog)

## Set up

Add the client and an async runtime to your application:

```toml
[dependencies]
wg-update-client = "1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Each example below is a complete program you can copy into `src/main.rs`.
You can also run the matching example from a repository checkout:

```console
cargo run -p wg-update-client --example inspect --locked
```

The discovery example reads the catalog without downloading game packages.
Installation examples download and write the selected game to the shown path.

## Discover builds

List every EU game variant, then explicitly inspect World of Tanks EU/SD:

```rust,no_run
use wg_update_client::{InspectOptions, ListOptions, inspect, list};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let catalog = list(ListOptions {
        region: Some("eu".into()),
        ..Default::default()
    })
    .await?;

    for build in catalog.builds {
        println!(
            "{} / {} / {}: {}",
            build.game, build.region, build.client_type, build.version
        );
    }
    for warning in catalog.warnings {
        eprintln!("{}: {}", warning.client_type, warning.message);
    }

    let build = inspect(InspectOptions {
        selection: wg_update_client::BuildSelection {
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
```

<details>
<summary>Choose a game, client variant, region, or language</summary>

Use a `BuildSelection` in `InspectOptions`, `InstallOptions`, or
`DownloadAssetsOptions`. For example:

```rust,no_run
use wg_update_client::{BuildSelection, InspectOptions, inspect};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let build = inspect(InspectOptions {
        selection: BuildSelection {
            game: Some("wows".into()),
            region: "ww".into(),
            client_type: Some("high".into()),
            language: Some("FR".into()),
            ..Default::default()
        },
    })
    .await?;
    println!("{}: {} bytes", build.version, build.download_size());
    Ok(())
}
```

`BuildSelection` requires an explicit game. Its other defaults are EU, the
publisher's valid default client variant, and the publisher's default language.
Without a usable client default, the smallest installable variant is selected.
`list` with default options queries all built-in games and regions.
The [catalog guide](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/catalogs.md)
lists game and region names and explains custom catalogs.

</details>

## Install and update

Install into an explicit directory, check its version, and apply an available
update using the saved selection:

```rust,no_run
use wg_update_client::{InstallOptions, StatusOptions, UpdateOptions, install, status, update};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let path = std::path::PathBuf::from("./world-of-tanks");
    let installed = install(InstallOptions {
        selection: wg_update_client::BuildSelection {
            game: Some("wot".into()),
            ..Default::default()
        },
        output: Some(path.clone()),
        ..Default::default()
    })
    .await?;
    println!("Installed {}", installed.plan.manifest.version);

    let current = status(StatusOptions { path: path.clone() }).await?;
    println!(
        "Installed: {} / Latest: {}",
        current.installed.version, current.latest_version
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
```

Run this example with `cargo run -p wg-update-client --example install --locked`.
Later, call `status` or `update` with that same path. Each installation saves just
the selected product, so the original catalog is not needed. Updates keep the
installation directory's name.

Updates use the installed `component_versions` map to request deltas and skip
unchanged components. A publisher may require a full component download when no
applicable delta remains. Completed patches are checkpointed in the pinned manifest;
unfinished deltas retain original sources inside `.wg-launcher/installing` for retry.
The new state and manifest fields are required; older records are not migrated.
See the [storage and update reference](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/reference.md)
for the persisted format and recovery behavior.

## Observers and async execution

Use `_with_observer` when your app needs progress. This example logs the plan and
structured events; replace the logger with your application's event handling:

```rust,no_run
use wg_update_client::{InstallEvent, InstallOptions, install_with_observer};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let installed = install_with_observer(
        InstallOptions {
            selection: wg_update_client::BuildSelection {
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
```

Run it with `cargo run -p wg-update-client --example progress --locked`.

<details>
<summary>Which progress events can my application receive?</summary>

| Workflow | Observer |
| --- | --- |
| `download_with_observer` | `DownloadAssetsEvent`: plan and per-file download progress |
| `extract_with_observer` | `ExtractEvent`: plan, extraction, patching, and cleanup |
| `install_with_observer` / `update_with_observer` | `InstallEvent`: plan and combined download/extraction progress |

The plain functions take only options. Plain and observed calls follow the same
installation path and return the same results and errors; neither prints output.

Observers must implement `Fn(Event) + Send + Sync + 'static`. Events can arrive
concurrently. Keep callbacks quick and forward events to your UI or an event
channel when needed. Callbacks run synchronously, so blocking work delays the
operation. Extraction runs on Tokio's blocking pool; an already-started blocking
task may continue after an async future is dropped. Await it before reusing the
installation directory. An already-current update emits no installation events.

</details>

## Download now, extract later

Stage all packages first, then install from those local files. The extraction
step can run in another process or on another machine after moving the complete
folder:

```rust,no_run
use wg_update_client::{DownloadAssetsOptions, ExtractOptions, download, extract};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    let path = std::path::PathBuf::from("./world-of-tanks");
    let cached = download(DownloadAssetsOptions {
        selection: wg_update_client::BuildSelection {
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
```

Run it with `cargo run -p wg-update-client --example staged_install --locked`.
To stage selected components, set `DownloadAssetsOptions.parts`. Download the
remaining components before extracting; `extract` needs the complete cache.

## Use your own catalog

Supply a product vector and select a game and region, including labels such as `local/KR`.
This example requires WGUS-compatible services at the two example addresses:

```rust,no_run
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
```

Run it with `cargo run -p wg-update-client --example custom_catalog --locked`
after starting your services. The same `BuildSelection` works with install and
download options. For filtering rules, URL fields, and JSON used by the CLI, see
the [custom catalog guide](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/catalogs.md).

## Choose a workflow

| I want to… | Use | Result |
| --- | --- | --- |
| Compare builds | `list(ListOptions)` | Builds and per-variant warnings |
| Preview one build | `inspect(InspectOptions)` | Manifest and package sizes |
| Save packages | `download(DownloadAssetsOptions)` | Cached packages and transfer report |
| Install local packages | `extract(ExtractOptions)` | Installed build and extraction report |
| Download and install | `install(InstallOptions)` | Plan and installation report |
| Check for updates | `status(StatusOptions)` | Installed/latest versions |
| Update in place | `update(UpdateOptions)` | Status and optional installation result |

Every operation is async and returns `wg_update_client::Result<T>`. Match the
public `Error` variants for application-specific recovery. Discovery can return
warnings alongside valid builds; a failed metadata request stops discovery.

**Next:** [API reference](https://docs.rs/wg-update-client/1.0.0/wg_update_client/) · [Installation details](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/reference.md) · [Troubleshooting](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/troubleshooting.md) · [Contributing](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/contributing.md)
