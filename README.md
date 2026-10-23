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

Inspect the default EU/SD selection without changing files, or override its
selection options:

```console
wg-launcher inspect
wg-launcher inspect --region eu --client-type sd --language EN
```

The inspection includes the available parts and a patch-chain table with each
source and target version, package count, and compressed download size.

The pipeline can be run as separate resumable stages:

```console
wg-launcher download --region eu --client-type sd --language EN
cd wot-eu-production-2.4.1.5493-windows-x64-sd-en
wg-launcher extract
# Or, from elsewhere:
wg-launcher extract wot-eu-production-2.4.1.5493-windows-x64-sd-en
```

Use `--output /path/to/game` with `download` to choose its destination.
`download` also accepts repeatable `--part <name>` filters. `extract` takes an
optional downloaded game directory and otherwise uses the current directory.
It reads the pinned manifest from `.download`, so it has no region, client type,
or language arguments. Extraction requires the complete package set for that
manifest.

Without any `--part`, `download` retrieves every package required by the
selected build. Use `--part` only when intentionally staging specific
components.

Running `install` starts the download and installation immediately:

```console
wg-launcher install --region eu --client-type sd --language EN
```

The default folder identifies the initially downloaded build, for example:

```text
wot-eu-production-2.4.1.5493-windows-x64-sd-en
```

Later updates modify that same folder; they do not rename it when the installed
version changes.

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

An installation folder is standalone and carries the launcher state needed to
resume or update it:

- `.download/` contains downloaded package volumes and the pinned manifest;
- `.installing/` contains temporary extraction work;
- `.wg-launcher.json` records the installed build;
- `.wg-launcher.lock` prevents concurrent mutation of the same folder.

Completed `download` commands keep `.download`. A successful `extract`,
`install`, or `update` removes `.download` and `.installing`; interrupted work
is retained or safely reconstructed so the command can be retried. Moving the
whole installation folder preserves its installed-version metadata and any
resumable package cache without depending on user-home storage.

During `install` and `update`, downloading continues in parallel with ordered
extraction. Downloads pause once at least 8 GiB of later patches is queued,
then resume as extraction advances. A patch may exceed this threshold because
all of its volumes are needed for extraction. The current patch is excluded
from that buffer; its volumes are deleted only after successful extraction.

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
