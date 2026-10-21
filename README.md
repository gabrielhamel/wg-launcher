# wg-launcher

A Rust library and CLI for discovering, downloading, and installing World of
Tanks assets directly from the Wargaming Update Service (WGUS).

It does **not** authenticate a player or launch the game yet.

## Usage

```console
wg-launcher list
wg-launcher list --region eu
```

Use `list` to compare the available regions, editions, versions, platforms,
architectures, download sizes, and languages in a terminal-friendly table.

Running `install` starts the download and installation immediately:

```console
wg-launcher install --region eu --client-type sd --language EN
```

From an installed game directory, check or apply updates without arguments:

```console
wg-launcher status
wg-launcher update
```

From any other directory, identify the game directory explicitly:

```console
wg-launcher status --path /path/to/world-of-tanks
wg-launcher update --path /path/to/world-of-tanks
```

Launcher files never live in the game directory. Package downloads use the
operating system's per-user cache directory. Locks, manifests, installed
version metadata, and temporary patch work use its per-user local data
directory. This maps to XDG directories on Linux, `~/Library` on macOS, and
`%LOCALAPPDATA%` on Windows.

## Lower-level protocol example

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
