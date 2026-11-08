# Installation and API reference

[Documentation home](README.md) · [Rust guide](../crates/wg-update-client/README.md)

Use this page when you need the details behind the everyday workflows.

[Storage](#installation-storage-and-lifecycle) · [Public API](#public-api) · [Errors](#errors)

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

Use the cache path reported by `InstallPlan.cache` when staging or moving an
installation. Storage-path constants and persisted record types are internal.

The default directory name preserves the initial game, selection, and version. Updates
keep that directory and write the current identity to `.wg-launcher/state.json`.
The internal installed-state record stores the complete selected product (including its game label),
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
High-level download, extraction, and integrated installation workflows acquire the
per-installation mutation lock. Competing mutations fail with a lock diagnostic.
File reuse and transfer validation use advertised sizes rather than content hashes. An extraction failure
can leave partially changed game files; there is no whole-install rollback.

Full transport archives decode each file under `.wg-launcher/installing/active/archive`
before replacing its game-tree destination. A decoding failure or termination while
writing that staged file leaves the existing destination intact. Staging is reused
within the patch and cleaned after its checkpoint; peak disk usage includes one
additional decoded file. This is per-file protection, not whole-patch rollback.

Transport `.wgpkg` and `.dspkg` archives are extracted into the game tree.
Game-owned `.pkg` files inside them remain intact.

`status` checks the live component update chain matching the installed identity.
Its `StatusReport.update_download_bytes` estimates the compressed size of applicable
packages, excluding unchanged components. The CLI shows this as `Update size`
when an update is available. This is the complete patch transfer size; cached
packages and partial downloads may reduce the actual download. Status does not
transfer packages, modify files, or use a pending pinned manifest.
`StatusReport.target.version` names the compared build and `target.source` identifies
`UpdateTargetSource::LiveCatalog` or `UpdateTargetSource::PinnedManifest`.
Status always uses the live catalog; an update retry can use a pinned target even
when the publisher now offers a different release. A no-work CLI update against a
pinned target reports `Already at pinned target` rather than claiming the live
catalog is current.

`update`
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

## Public API

Use `list`, `inspect`, `download`, `extract`, `install`, `status`, and `update`.
Download, extraction, installation, and update also provide `_with_observer`
variants for structured progress. `DownloadOptions` controls transfer concurrency
and retries; ordinary calls do not require an observer.

Download summaries distinguish completed volumes, reused volumes, and newly
retained bytes. Completing a volume includes promoting a complete partial file
without an HTTP request. Reuse means a final cache file already has the expected
size. Newly retained bytes count successful attempts only, excluding resumed
prefixes and traffic from failed attempts. The public `DownloadReport` fields
remain `downloaded_files`, `skipped_files`, and `downloaded_bytes`, respectively.

Delta application reports preservation of the original file, the start of
application, and throttled cumulative reconstructed bytes through
`ExtractionEvent::PreparingDelta`, `ApplyingDelta`, and `DeltaProgress`.
The CLI displays reconstructed bytes, throughput, and elapsed time. The final
output size is not known in advance, so delta progress has no percentage or ETA.
A final byte notification follows successful output flushing; patch checkpointing
and installation completion remain separate events.

`inspect` returns `InspectReport`: selected `BuildInfo`, available languages,
component identifiers, patch summaries, package count, compressed bytes, and a
suggested installation directory. Each `PatchSummary` describes its component,
source and target versions, package count, and compressed size. Transport URLs,
torrents, protocol metadata, and recovery checkpoints stay internal.

Workflow plans contain `build: BuildInfo`, the installation root and cache path,
compressed bytes, and `packages: Vec<PackageSummary>`. Each package exposes only
its name and size so applications can initialize progress displays. Progress
events use those package names. Status exposes the installed `BuildInfo` rather
than the persisted installation record. Reports are informational snapshots;
mutating them does not change an installation.

Custom catalogs still use `Product` and `BuildSelection`. Saved endpoint
configuration is managed by the library, so status and update need no catalog file.

## Errors

Workflows and `Product::validate()` return `wg_update_client::Result<T>`.
`Error::kind()` returns a non-exhaustive `ErrorKind` for application recovery:

```rust,no_run
use wg_update_client::{ErrorKind, StatusOptions, status};

#[tokio::main]
async fn main() -> wg_update_client::Result<()> {
    match status(StatusOptions { path: "./game".into() }).await {
        Ok(report) => println!("Installed: {}", report.installed.version),
        Err(error) => match error.kind() {
            ErrorKind::InstallationNotFound => println!("Install the game first"),
            ErrorKind::Network => println!("Try again when the service is reachable"),
            _ => return Err(error),
        },
    }
    Ok(())
}
```

Other categories distinguish missing downloads, incomplete or incompatible
caches, lock contention, invalid selections, unsafe paths, invalid publisher
responses or packages, filesystem errors, invalid stored data, and worker
failures. Classification follows nested causes, including exhausted transfer
retries. Use the display message and `std::error::Error::source()` for detailed
diagnostics; implementation-specific error types are internal.

`ExtractionReport.files_before_finalization` counts regular files beneath the
installation root before state writes and cleanup. It includes pre-existing game
files, launcher metadata, and any cached packages still present. It does not
measure how many files this operation installed. CLI completion messages explicitly
include launcher metadata in this count. `ExtractionEvent::Completed.files` uses
the same count.

**API migration (breaking):** rename `ExtractionReport.installed_files` to
`files_before_finalization`, including the field in serialized reports. Its value
is unchanged. Replace `StatusReport.latest_version` with
`StatusReport.target.version`; use `target.source` to distinguish live discovery
from a pinned retry target. Replace `inspect` manifest access such as
`manifest.version` with `report.build.version`, and `download_size()` with
`report.compressed_bytes`. Use patch summary counts/sizes instead of raw patch
files. Replace `outcome.plan.manifest` with `outcome.plan.build` and
`plan.assets` with `plan.packages`; installed status identity no longer contains
the saved product or component checkpoints. Match `error.kind()` instead of
nested owner errors. Direct protocol, transfer, and extractor APIs and path
constants are no longer exported. Persisted JSON, installation paths, defaults,
and workflow behavior are unchanged.

Discovery reports per-variant failures in `ListReport.warnings`; a metadata
request failure stops discovery. Successful transfers and partial files may remain
after a failed operation, allowing another attempt with the same selection.

Patch planning requires a target version for every selected component. If multiple
published patches start at a version needed by the selected chain, the operation
fails with an ambiguous patch-chain diagnostic instead of choosing by response
order. Ambiguity in unused alternatives does not prevent installation.

[Back to documentation](README.md)
