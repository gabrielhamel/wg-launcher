# Agent guide for `wg-launcher`

This file is for coding agents working on this repository. It records the
product philosophy and architectural decisions that are easy to lose when
looking only at the current code. It is not a replacement for `README.md` or
user-facing CLI documentation.

## Product intent

`wg-update-client` is the reusable Rust implementation of Wargaming game asset
launcher workflows. Its immediate responsibility is to discover, download,
install, inspect the status of, and update game assets published through WGUS
or a compatible service such as Lesta's update service.

The project does not currently authenticate players or launch the game.
Do not broaden its scope into account management, login, news, advertising,
or game process management unless the user explicitly asks for that work.

The project must support Windows, Linux, and macOS as a launcher. The game
payload currently published by the supported catalogs is a Windows/x64
payload; do not confuse the launcher's host OS with the payload platform.

## Architectural philosophy

### Workspace package boundaries

The root is a virtual Cargo workspace with two launcher packages:

- `crates/wg-update-client`: library-only package `wg-update-client`, imported as `wg_update_client`.
- `crates/wg-launcher-cli`: binary package `wg-launcher-cli`, executable `wg-launcher`.

`crates/wg-update-service` is an additional published production package. Its
`wg-update-service` binary serves a file-backed WGUS-compatible release and does
not depend on the client or launcher. The unpublished `e2e/` workspace package
owns all synthetic payloads, archive builders, process harnesses, and test-only
HTTP resources. It depends on none of the production Rust packages and exercises
only their executable and HTTP boundaries. The two launcher packages are workspace
defaults; `--workspace` includes the three production packages and E2E package.

Use `wg_update_client` in Rust and `-p wg-update-client` for library-specific
commands. The CLI package is `wg-launcher-cli`, its binary is `wg-launcher`, and
installation-owned files live under `.wg-launcher/`.

Keep library behavior and its dependencies in the library package. CLI-only
crates such as `clap`, `comfy-table`, and `indicatif` belong exclusively to the
CLI package. The CLI may access only the library's public API; the library must
never depend on the CLI. Each package owns its README, source, and license;
CLI assets live under `crates/wg-launcher-cli/docs/assets`.

The two launcher packages are workspace defaults. Use `--workspace` for full validation and
`-p wg-update-client` for library-only documentation. Install from source with
`cargo install --path crates/wg-launcher-cli --locked`. The release workflow
publishes the service, client, and CLI explicitly; keep the shared package version
and the CLI's versioned path dependency synchronized before releasing.

### The library owns behavior; the CLI owns presentation

The crate must remain usable by a future GUI without invoking or parsing the
CLI. High-level workflows live in the library and are exported from `crates/wg-update-client/src/lib.rs`:

- `list(ListOptions)`
- `inspect(InspectOptions)`
- `download(DownloadAssetsOptions)`
- `extract(ExtractOptions)`
- `install(InstallOptions)`
- `status(StatusOptions)`
- `update(UpdateOptions)`

The CLI in `crates/wg-launcher-cli/src/cli/` should only:

- parse human input;
- translate arguments into the library option types;
- render tables, spinners, progress bars, summaries, and errors;
- call the high-level library functions.

`crates/wg-launcher-cli/src/main.rs` is only the executable bootstrap. Keep it limited to runtime
setup and `cli::run()`; command definitions and rendering belong in
`crates/wg-launcher-cli/src/cli/` and `crates/wg-launcher-cli/src/progress/`.

Do not move terminal formatting, `indicatif` objects, prompts, or process exit
behavior into the library. Conversely, do not reproduce catalog, networking,
download, extraction, storage, retry, or update logic in the CLI.

The observer/event types are the UI contract. Extend those contracts when a
GUI needs more information instead of coupling the core to a specific UI.

### Keep library use simple and progress optional

Product decision: the consumer decides whether to receive progress. The primary
`download`, `extract`, `install`, and `update` functions take only options and
return their result without requiring a callback. Their `_with_observer`
counterparts opt in to the existing structured events. Neither form prints
terminal output. The CLI opts in to observers for its progress display.

Keep both forms on one implementation path so defaults, reports, errors,
locking, cleanup, and resumability behave identically. Apply this convention to
lower-level downloader and extractor APIs too. Do not require dummy callbacks,
UI types, or presentation settings for the ordinary library workflow. Prefer
simple defaults and a small number of concepts before adding API machinery.

### Keep consumer documentation task-focused

The root README is a short getting-started page with CLI and Rust examples.
Do not put source maps, architecture decisions, test internals, or release
procedures there. `docs/README.md` is the navigation hub; package guides follow
user workflows with clickable navigation and expandable examples. Put detailed
installation/API behavior in `docs/reference.md`, developer/source/release
information in `docs/contributing.md`, and fixture details in `docs/testing.md`.

Document consumer-visible behavior and defaults alongside affected workflows.
Describe the current API and storage contracts directly. The library README is
included as its rustdoc landing page via `include_str!`. Runnable programs in
`crates/wg-update-client/examples/` match those snippets. Keep both synchronized,
check examples with `cargo check -p wg-update-client --examples --locked`, and do
not execute game-installing examples during routine validation.

Keep examples and API references synchronized with implementation. Public APIs
must remain documented, rustdoc links must resolve, and examples must compile.
For library API or documentation changes, also run:

```console
RUSTDOCFLAGS="-D warnings" cargo doc -p wg-update-client --lib --no-deps --all-features --locked
cargo check -p wg-update-client --examples --locked
```

### Keep the CLI direct and deliberately small

The supported user workflows are:

- `list`
- `inspect`
- `download`
- `extract`
- `install`
- `status`
- `update`

`install` remains the convenient integrated pipeline. `download` and `extract`
also expose its package and installation phases independently for users who
need explicit control, offline transfer, or staged operation. `inspect`
resolves a selection without changing files and requires the game to be selected.

`extract` is local-only: it takes an optional positional installation
directory, defaults to the current directory, and reads the pinned manifest
from `.wg-launcher/download`. It must not expose region, client type, or language options
or query the live catalog to decide what cached packages mean.

`inspect` should expose enough patch detail to make component downloads
understandable: part, source version, target version, package count, and
compressed size. `download` with no `--part` selects the complete installation;
one or more `--part` flags intentionally filter that set.

The CLI also intentionally has no JSON output, `--dry-run`, `--yes`, or
confirmation step. Running `install` or `update` performs the operation
directly. Do not reintroduce these commands or flags as incidental helpers;
doing so is a product decision that requires an explicit request.

Current defaults are part of the UX:

- `list` queries all known games and regions unless `--game` or `--region` is provided;
- `inspect`, `download`, and `install` require an explicit game;
- built-in region defaults are `eu` for `wot` and `ww` for other games;
- an omitted client type selects a valid publisher default, then falls back to the
  installable variant with the smallest compressed asset total;
- an omitted language means the publishing region's default language;
- `status` and `update` use the current directory unless `--path` is provided.

### Prefer portable in-process implementations

The executable must work across Windows, Linux, and macOS without requiring
users to install platform utilities. Do not shell out to `7z`, `xdelta`,
`rsync`, `curl`, or similar programs for core behavior. The current Rust
dependencies perform HTTP, archive, VCDIFF, and librsync work in process.

Adding a Rust dependency is acceptable when justified, but prefer a focused,
maintained, cross-platform crate and keep the public architecture independent
of that crate.

## Installation and storage invariants

### An installation directory is standalone

All CLI- and library-owned installation files must live inside
`<installation>/.wg-launcher/`:

- package volumes and the pinned manifest live in `.wg-launcher/download`;
- temporary extraction work lives in `.wg-launcher/installing`;
- installed build identity lives in `.wg-launcher/state.json`;
- `.wg-launcher/wg-launcher.lock` prevents concurrent mutation.

This includes all future metadata, logs, caches, locks, temporary files, and
replacement backups. Only game payload files belong outside `.wg-launcher`.
Reserve this directory against archive writes. Preserve it during successful
cleanup; remove only cache and staging contents.

Do not add a dependency on user-home cache or data directories. A user must be
able to move the complete folder to another machine or location and retain its
installed identity and resumable state.

### Default installation names preserve initial identity

The default directory includes game, region, channel, the initially downloaded game
version, payload platform, architecture, client type, and language. For
example:

`wot-eu-production-2.4.1.5493-windows-x64-sd-en`

An update must target that same directory instead of creating or renaming it
for the newer version. The folder name records its initial download identity;
`.wg-launcher/state.json` records its current installed version.

### Installation is resumable and storage-conscious

The installation pipeline is intentionally integrated:

1. resolve and pin the manifest;
2. download a patch's package volumes into `.wg-launcher/download`;
3. begin downloading the next patch when possible;
4. extract or apply the current patch;
5. delete that patch's package volumes only after successful extraction;
6. finalize the installation and write `.wg-launcher/state.json`.

This overlaps network and extraction work while limiting peak disk usage.
Do not change it back into “download everything, then extract everything.”

The downloaded `.wgpkg`/`.dspkg` volumes are transport archives. Extract them
into the game tree, but do not recursively unpack `.pkg` files found inside:
those are game-owned source assets and must remain in the format expected by
the game.

On interruption:

- complete cached files should be reused;
- partial downloads should resume when possible;
- completed patches advance the pinned manifest's `base_versions` before package removal;
- temporary archives may be rebuilt, but original delta sources under
  `.wg-launcher/installing/active/sources` must survive failure until their patch is checkpointed;
- retries skip checkpointed patches instead of downloading or applying them again.

Never delete cached package volumes before their extraction succeeds. Clean the
remaining download cache after a successful installation by default.

Use the per-installation `fs2` lock. Concurrent launchers must not mutate the
same installation.

## Status and update model

`.wg-launcher/state.json` is launcher state stored inside the game directory. It
records exactly one complete product object, plus the game ID, region, channel,
installed version, client type, language, payload platform, architecture, metadata
version, and a required `component_versions` map. The input catalog array is not stored.

`status` loads that record and queries the saved product endpoint using the
recorded selection.
`update` reuses the recorded selection and installation path. A pending pinned
manifest takes precedence during retry so an interrupted update remains
internally consistent.

Updates send installed component versions through WGUS `<part>_current_version`
parameters and skip unchanged components. New components start at `0`; a publisher
may require a full component chain when its old version has no remaining delta.
Pinned manifests require target `component_versions` and resumable `base_versions`.
Checkpoints advance after successful patches, before removing their packages; the
final installed state is written only after all patches succeed. Original delta
sources survive partial patch failure. These fields are required, with no legacy
record migration.

## Catalog and protocol rules

Catalog entries live in `crates/wg-update-client/src/catalog.rs`. A product selection
is a game/region pair that binds a publisher host, application ID, and channel.
When adding or changing one:

1. add the game or region to the relevant built-in catalog, display, and parsing;
2. use the exact live host and application ID;
3. extend relevant E2E catalog coverage;
4. run the real `list --region <region>` command;
5. confirm every expected client type produces a version row, not merely that
   metadata returns HTTP 200.

Supported catalogs include World of Tanks, World of Warships, World of
Warplanes, World of Tanks Blitz, and Lesta RU. The non-WoT Wargaming catalogs
use region `ww`. Lesta RU
uses:

- host: `lstus-ru.lesta.ru`
- application ID: `MT.RU.PRODUCTION`
- CLI region: `ru` (alias `russia`)

Lesta's install chain omits `version_from` on patches. `crates/wg-update-client/src/service/mod.rs`
normalizes that compatible representation: the first patch for a component
starts at `0`, and a later patch starts at the previous patch's `version_to`.
Preserve the explicit value when a publisher provides one. This normalization
is required both for listing installable sizes and for ordering installation
patches.

WGUS is a live external protocol. Treat fixtures as necessary but not
sufficient when touching catalog discovery, redirects, languages, patch-chain
parsing, or web seeds. Live verification should avoid downloading packages;
`list --region ...` is normally enough.

## Terminal experience

`list` should be pleasant for a human terminal:

- show a spinner while network discovery is running;
- render a readable table of game, region, channel, client type, version, target,
  compressed size, and languages;
- mark the default language;
- report unavailable variants as warnings without inventing data.

Installation progress uses `indicatif::MultiProgress`. Keep progress bars
stable while download, extraction, and package-release events interleave.
Avoid replacing the display with streams of `start`, `progress`, and `done`
lines or repeatedly clearing/recreating bars, which causes terminal flicker.

The library emits structured progress events; visual choices belong only to
the CLI.

## File responsibilities

- `crates/wg-update-client/src/catalog.rs`: games, regions, and publisher product endpoints.
- `crates/wg-update-client/src/catalog/product.rs`: owned product configuration and
  HTTP(S) URL validation, including ports and path prefixes.
- `crates/wg-update-client/src/catalog/selection.rs`: custom catalog validation
  and game/region selection shared by listing, build resolution, and cache reuse.
- `crates/wg-launcher-cli/src/main.rs`: minimal async executable bootstrap.
- `crates/wg-update-client/src/service/mod.rs`: WGUS-compatible HTTP/XML protocol and manifest building.
- `crates/wg-update-client/src/model.rs`: protocol-independent public models and stable output naming.
- `crates/wg-update-client/src/download.rs`: resumable, concurrent package transfer.
- `crates/wg-update-client/src/install/`: low-level installation. `mod.rs` exposes the extractor
  and coordinates the pipeline; `chain.rs` orders patches; `prefetch.rs` queues downloads;
  `extraction.rs` applies ordered patches; `archive.rs` reads transport volumes;
  `delta.rs` applies deltas; `storage.rs` handles locks, state, and cleanup;
  `paths.rs` validates paths; `retry.rs` pins component checkpoints and preserves delta
  sources; `types.rs` defines options, reports, and events.
- `crates/wg-update-client/src/launcher/`: high-level workflows, with options and reports
  beside each workflow; `manifest.rs` owns shared plans and pinned manifest persistence.
- `crates/wg-update-client/src/service/wire.rs`: private XML representations of WGUS responses.
- `crates/wg-launcher-cli/src/cli/`: `mod.rs` dispatches commands; `args.rs` parses
  arguments; `output.rs` renders summaries and tables; `observers.rs` adapts events.
- `crates/wg-launcher-cli/src/progress/`: shared download/extraction bars in `mod.rs`,
  discovery spinners in `discovery.rs`, and terminal styles in `styles.rs`.
- `crates/wg-update-client/src/lib.rs`: the public library contract.
- `crates/wg-update-service/src/main.rs`: file-service CLI arguments and process lifetime.
- `crates/wg-update-service/src/server.rs`: file-backed HTTP serving, ranges, and safe path mapping.
- `e2e/tests/scenarios/`: independent black-box workflow and service-process scenarios.
- `e2e/tests/support/`: process helpers, synthetic archives, and test-only HTTP server.

Respect these boundaries. If a change seems to require crossing them, first
consider whether a model, option, report, or event should be extended instead.

## Keep generated code organized by default

The organization established by this repository's module split is the standard
for all new and changed code. Maintain it as you work; do not wait for a separate
cleanup request.

- Give each file a clear, focused responsibility. Split files when they accumulate
  distinct responsibilities or become difficult to read and navigate.
- Group related modules in directories with a small `mod.rs` entry point. Keep
  orchestration readable and move implementation details into named modules.
- Keep workflow-specific options, events, and reports beside their workflow;
  share types only when multiple modules actually need them.
- Separate CLI argument parsing, command dispatch, output rendering, and progress
  adapters. Keep protocol wire representations separate from public models.
- Group black-box E2E tests by category in `e2e/tests/scenarios/`, with shared
  process helpers and fixtures in `e2e/tests/support/`.
- Prefer descriptive names and simple boundaries. Avoid arbitrary line limits,
  one-file-per-item fragmentation, and abstractions added only to split a file.
- Update the source map and affected documentation when the structure changes.
  For organizational changes, preserve behavior and public APIs and run the
  checks required below.

## Working rules for agents

- Inspect `git status` before editing. Preserve unrelated user changes.
- Prefer extending the high-level library contract before changing CLI code.
- Keep persistence changes within the requested scope. Additional storage paths
  or transformations of stored records require an explicit user request.
- Do not silently change defaults, directory naming, cleanup timing, or cache
  identity; these affect resumability and existing installations.
- Errors must explain the actionable condition without exposing an internal
  implementation stage as a user workflow.
- For filesystem and archive code, protect against path traversal and partial
  replacement of existing game files.
- For concurrency changes, preserve the installation lock and cancellation of
  unnecessary pending work after extraction failure.

### End-to-end tests and update-service resources

The E2E guide is `docs/testing.md`; keep it synchronized with `e2e/tests/e2e.rs`
and its category modules in `e2e/tests/scenarios/`.

- Keep reusable synthetic payloads, XML/product templates, archive builders, and
  release-directory writers in `e2e/`. Do not add test
  infrastructure or dev-dependencies back to production packages.
- Keep process scenarios and process-control helpers in `e2e/tests/`. Every E2E
  scenario launches both real executables; it must not call production Rust APIs
  or implement a substitute HTTP server. Use the service access log for request
  assertions and restart it on the same address when republishing fixture data.
  Assert observable exit status, diagnostic messages, HTTP requests, installed
  payload bytes, persisted identity, and cache/staging cleanup as appropriate.
- Use tiny real transport archives and an ephemeral loopback HTTP server. Isolate
  each installation in a temporary directory, bound child process execution,
  capture output for diagnostics, and clean up child processes and server threads.
  Do not use publisher services or multi-gigabyte downloads in this suite.
- Author shared fixtures independently of launcher models and behavior. Fixture
  helpers may panic on setup errors, but production service startup and request
  handling must return actionable errors. Stream package files from disk instead
  of buffering a release in memory, preserve byte-range resumption, and reject
  unsafe paths.
- All installation E2E workflows start from custom-product discovery through the
  public CLI `--product` option. Obtain pinned manifests through real downloads;
  do not seed hand-authored manifests or installed state to bypass discovery.
  Recovery tests may modify downloaded volumes to model partial/corrupt files.
  Pending-update tests stage packages through `download` before retrying `update`.
- Cover integrated/staged installation, filtered downloads, cache reuse, seeded
  partial-file resumption, corrupt-archive retry, pending updates, locking,
  missing-state errors, moving installations, and rejection of another product's
  cache. Check that pinned retries avoid catalog queries even after the service
  publishes another version. Delta-update coverage uses source-dependent synthetic
  librsync patches. Do not claim actual mid-transfer process termination or
  whole-install transactional rollback.
- Use the public `Product` configuration for custom service URLs in tests rather
  than adding a test-only endpoint switch. Cover the dedicated
  `tests/scenarios/service/` path from launcher CLI parsing through file-backed
  WGUS discovery, download, extraction, state, and cleanup. Keep separate
  service-binary tests for CLI startup and HTTP behavior.

Run the focused suite with:

```console
cargo test -p wg-launcher-e2e --locked
```

The normal workspace tests include it, including the existing Windows, Linux,
and macOS CI matrix. A local pass confirms only the host platform; do not claim
the other platforms passed without checking their CI results.

### Custom products

- Built-in catalogs remain the default. `BuildSelection.products` and
  `ListOptions.products` accept an optional replacement product array. Game and
  region selectors are strings and filter either catalog, including custom labels.
  Listing defaults to all configured selections; build selection requires a game.
  Reject empty catalogs, duplicate game/region pairs (ignoring case), and missing
  matches. Different games may share a region. Do not fall back to built-ins when
  a custom catalog was supplied.
- `Product` is owned and serializable. Its `game` is a stable filter and directory
  identifier; older records without it default to `wot`. Its `host` is an absolute HTTP(S) base URL
  with optional port/path prefix. Keep validation in the library and file parsing
  in the CLI; `--product <file.json>` accepts a JSON array and combines with
  `--game` and `--region` on list/inspect/download/install. A singleton object is not accepted.
- Pin exactly the selected product in manifests and installed state for every
  installation, built-in or custom. Both fields are required `Product` values,
  never arrays; status/update must use the saved snapshot.
  Catalog order or changes to unselected regions must not invalidate that cache.
  Status and update use that configuration without requiring the input JSON file.
  Extraction remains local-only. Each manifest and installed-state record
  requires a complete product object.
- Preserve the configured product through protocol redirects. Cache reuse must
  reject a different selected product even if its region/client/language match.
- Report/state/manifest region labels are strings so custom labels are supported;
  `Region` remains the enum for selecting built-in catalogs. Keep directory naming,
  cleanup, pending-manifest precedence, and payload-platform defaults intact.

Before handing off a code change, run the checks appropriate to its scope. The
normal full validation is:

```console
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --locked
git diff --check
```

After every code-editing operation or coherent editing batch, the minimum
mandatory validation is:

```console
cargo fmt --all
cargo check --workspace --locked
```

Fix formatting and compilation errors immediately before continuing. Do not
treat `cargo test`, Clippy, or an earlier successful build as a substitute for
this post-edit check, and do not report a code operation as complete until both
commands pass.

For catalog or protocol changes, also run the relevant live list command, for
example:

```console
cargo run --quiet --locked -- list --region ru
```

Do not use an actual multi-gigabyte installation as a routine test. E2E tests
should cover observable workflows using tiny fixtures; live listing should cover
remote catalog compatibility.
