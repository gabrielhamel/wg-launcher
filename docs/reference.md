# Installation and API reference

[Documentation home](README.md) · [Rust guide](../crates/wg-update-client/README.md)

Use this page when you need the details behind the everyday workflows.

[Storage](#installation-storage-and-lifecycle) · [Low-level APIs](#low-level-apis) · [Errors](#errors)

## Installation storage and lifecycle

All launcher state lives in `.wg-launcher` inside the installation directory.
Its contents use ordinary names because the parent already groups launcher-owned
files. `state.json` describes the installed build; `download/manifest.json` pins
the pending download selection. The installation layout is:

| Path | Purpose |
| --- | --- |
| `.wg-launcher/download/` | Transport package volumes and the pinned `manifest.json` |
| `.wg-launcher/installing/` | Temporary extraction and patch work |
| `.wg-launcher/state.json` | Installed build identity |
| `.wg-launcher/wg-launcher.lock` | Per-installation mutation lock |

All files created by the CLI or library for an installation, including future
metadata, logs, caches, locks, temporary outputs, and replacement backups, belong
under `<installation>/.wg-launcher/`. Game payload files remain in the game tree.
The `.wg-launcher` directory is reserved and archive entries cannot write into it.
Successful cleanup removes only its cache and temporary work; identity and lock
remain so the installation stays portable and usable by `status` and `update`.

The exported `DOWNLOAD_CACHE_DIRECTORY`, `STAGING_DIRECTORY`, and
`INSTALL_MARKER_FILE` constants are paths relative to the installation root and
include the `.wg-launcher/` prefix. Callers should join them directly to the
installation root. `DOWNLOAD_MANIFEST_FILE` is a filename within the cache.

The default directory name preserves the initial game, selection, and version. Updates
keep that directory and write the current identity to `.wg-launcher/state.json`.
`InstalledBuild` records the complete selected product (including its game label),
game ID, region, channel,
version, client type, language, payload platform, architecture, metadata version,
and a required `component_versions` map from component IDs to installed versions.
Only that product is saved, never the input catalog array.

The integrated pipeline pins a manifest, downloads each patch's volumes,
overlaps later downloads with ordered extraction, and releases a patch's
volumes only after extraction succeeds. Downloading pauses once at least
8 GiB of later patches is queued. A single patch can exceed that threshold
because its complete volume set is needed; the current patch is excluded
from the buffer.

Complete cached volumes are reused and partial downloads resume when possible.
Each successful patch advances `base_versions` in the pinned manifest before its
volumes are released. Retries skip those patches. An unfinished delta patch retains
its original source files beneath `.wg-launcher/installing/active/sources` so a
partially applied patch can be retried safely; preserve this directory on failure. Successful high-level
installation and extraction remove the remaining cache and temporary work.
The lower-level `ExtractionOptions` allows callers to control cache cleanup.
Extraction and the integrated pipeline acquire the installation lock. Package-only
downloads and direct `AssetDownloader` calls do not; embedding applications must
serialize competing access to the same destination. File reuse and transfer
validation use advertised sizes rather than content hashes. An extraction failure
can leave partially changed game files; there is no whole-install rollback.

Transport `.wgpkg` and `.dspkg` archives are extracted into the game tree.
Game-owned `.pkg` files inside them remain intact.

`status` checks the live selection matching the installed identity. `update`
reuses that selection and path, with a pending pinned manifest taking precedence
during retries. It sends saved component versions to WGUS, requests the applicable
delta chains, and skips unchanged components. New components start at version `0`.
If the publisher no longer provides a delta from an installed version but publishes
a full chain, that component is installed from the full chain instead. An incomplete
chain fails before package downloads. Build and component versions are compared
independently; a changed component can require an update under the same build label.

Installed state requires `component_versions`. Pinned manifests require
`base_versions` and target `component_versions`; there is no migration of older
records. Installations created with the older format require a fresh installation.
The final installed state changes only after all selected patches succeed.
Lower-level `PackageExtractor` methods also pin and checkpoint manifests under the
installation lock; retries with the original manifest reuse matching checkpoints.

## Low-level APIs

Prefer the high-level workflows for complete installation management. Use
[`WgusClient`](https://docs.rs/wg-update-client/1.0.0/wg_update_client/struct.WgusClient.html)
for direct metadata and manifest access. Its region-based methods select built-in
catalogs; `metadata_for_product` and `manifest_for_product` take a `&Product`.

`AssetDownloader`, `PackageExtractor`, and `installation_assets` let applications
compose transfer and extraction themselves. Their ordinary methods need no
observer; `_with_observer` variants provide structured progress.

## Errors

Fallible operations return `wg_update_client::Result<T>` with a structured `Error`.
Match error variants when your application needs to distinguish conditions.
Discovery reports per-variant failures in `ListReport.warnings`; a metadata
request failure stops discovery. Successful transfers and partial files may remain
after a failed operation, allowing another attempt with the same selection.

[Back to documentation](README.md)
