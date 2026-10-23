# Rust library

[Back to the README](../README.md)

`wg_launcher` exposes the launcher's workflows independently of the CLI. A GUI
or another application can use typed options, results, and events without
invoking a subprocess or parsing terminal output.

## Inspect a build

Add this checkout and Tokio to your application's `Cargo.toml`, adjusting the
path to your local checkout:

```toml
[dependencies]
wg-launcher = { path = "../wg-launcher" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust,no_run
use wg_launcher::{BuildSelection, InspectOptions, Region, inspect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = inspect(InspectOptions {
        selection: BuildSelection {
            region: Region::Eu,
            client_type: "hd".to_owned(),
            language: Some("FR".to_owned()),
        },
    })
    .await?;

    println!("{} requires {} bytes", manifest.version, manifest.download_size());
    Ok(())
}
```

`BuildSelection::default()` selects EU/SD and leaves the language unset so the
publisher's default is used.

## High-level workflows

All functions below are asynchronous and exported from the crate root.

| Function | Options | Result |
| --- | --- | --- |
| `list` | `ListOptions` | `ListReport`, including builds and unavailable-variant warnings |
| `inspect` | `InspectOptions` | Resolved `PatchManifest`, without filesystem changes |
| `download` | `DownloadAssetsOptions` | `DownloadAssetsOutcome`, including the plan and transfer report |
| `extract` | `ExtractOptions` | `ExtractOutcome`, including the plan and extraction report |
| `install` | `InstallOptions` | `InstallOutcome`, including the plan and combined installation report |
| `status` | `StatusOptions` | `StatusReport`, including installed/latest versions and update availability |
| `update` | `UpdateOptions` | `UpdateOutcome`, including status and an optional installation result |

`download`, `extract`, `install`, and `update` also take an observer callback.
`ListOptions::default()` queries every region. `ExtractOptions`, `StatusOptions`,
and `UpdateOptions` take an explicit installation path; the CLI supplies the
current-directory default.

For a staged download, `DownloadAssetsOptions.parts` filters transferred
components while retaining the complete pinned manifest. An empty list selects
all required packages. `extract` requires the complete cache and resolves its
meaning from the pinned manifest, without querying the live catalog.

## Installation and progress events

```rust,no_run
use wg_launcher::{InstallEvent, InstallOptions, install};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = install(
        InstallOptions {
            output: Some("./world-of-tanks".into()),
            ..InstallOptions::default()
        },
        |event| match event {
            InstallEvent::Planned(plan) => {
                println!("Installing into {}", plan.root.display());
            }
            InstallEvent::Progress(progress) => {
                println!("{progress:?}");
            }
        },
    )
    .await?;

    println!("Installed {} files", outcome.report.extraction.installed_files);
    Ok(())
}
```

Observers implement `Fn(Event) + Send + Sync + 'static`. They may receive events
from concurrent download and extraction work. Keep callbacks short and forward
events to your application's UI or event channel as needed.

| Workflow | Observer event |
| --- | --- |
| `download` | `DownloadAssetsEvent`: the plan, then `DownloadEvent` progress |
| `extract` | `ExtractEvent`: the plan, then `ExtractionEvent` progress |
| `install`, `update` | `InstallEvent`: the plan, then `InstallationEvent` progress |

`DownloadEvent` covers starts, byte progress, completion, and reuse of cached
files. `ExtractionEvent` covers cache checks, patch/file extraction, delta
application, file installation, finalization, cleanup, and completion.
`InstallationEvent` combines download and extraction events with package-release
notifications. An already-current `update` returns no installation result.

The library owns discovery, transfer, extraction, retries, locking, and update
behavior. Terminal bars and formatting belong to the CLI; embedding applications
provide their own presentation.

## Installation storage and lifecycle

All launcher state lives inside the installation directory:

| Path | Purpose |
| --- | --- |
| `.download/` | Transport package volumes and the pinned `.wg-manifest.json` |
| `.installing/` | Temporary extraction and patch work |
| `.wg-launcher.json` | Installed build identity |
| `.wg-launcher.lock` | Per-installation mutation lock |

The default directory name preserves the initial selection and version. Updates
keep that directory and write the current identity to `.wg-launcher.json`.
`InstalledBuild` records the game ID, region, channel, version, client type,
language, payload platform, architecture, and metadata version.

The integrated pipeline pins a manifest, downloads each patch's volumes,
overlaps later downloads with ordered extraction, and releases a patch's
volumes only after extraction succeeds. Downloading pauses once at least
8 GiB of later patches is queued. A single patch can exceed that threshold
because its complete volume set is needed; the current patch is excluded
from the buffer.

Complete cached volumes are reused and partial downloads resume when possible.
Temporary extraction work can be rebuilt. A retry may need to redownload
volumes already released after successful extraction. Successful high-level
installation and extraction remove the remaining cache and temporary work.
The lower-level `ExtractionOptions` allows callers to control cache cleanup.

Transport `.wgpkg` and `.dspkg` archives are extracted into the game tree.
Game-owned `.pkg` files inside them remain intact.

`status` checks the live selection matching the installed identity. `update`
reuses that selection and path, with a pending pinned manifest taking precedence
during retries. Updates follow the published installation chain; per-component
versions are not persisted, so fully minimal component-by-component delta
updates are not implemented.

## Lower-level protocol access

Use `WgusClient` when you need direct metadata or manifest access:

```rust,no_run
use wg_launcher::{Region, WgusClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WgusClient::new()?;
    let manifest = client.manifest(Region::Eu, "hd", Some("FR")).await?;
    println!("{} requires {} bytes", manifest.version, manifest.download_size());
    Ok(())
}
```

`AssetDownloader`, `PackageExtractor`, and `installation_assets` are also
exported for callers that need to compose lower-level operations. Prefer the
high-level workflows when you want the complete installation lifecycle.

Generate the API reference from the checkout with `cargo doc --no-deps --open`.
