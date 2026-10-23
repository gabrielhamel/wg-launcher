# Agent guide for `wg-launcher`

This file is for coding agents working on this repository. It records the
product philosophy and architectural decisions that are easy to lose when
looking only at the current code. It is not a replacement for `README.md` or
user-facing CLI documentation.

## Product intent

`wg-launcher` is a reusable Rust implementation of the World of Tanks asset
launcher workflow. Its immediate responsibility is to discover, download,
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

The root is a virtual Cargo workspace with two packages:

- `crates/wg-launcher`: library-only package `wg-launcher`, imported as `wg_launcher`.
- `crates/wg-launcher-cli`: binary package `wg-launcher-cli`, executable `wg-launcher`.

Keep library behavior and its dependencies in the library package. CLI-only
crates such as `clap`, `comfy-table`, and `indicatif` belong exclusively to the
CLI package. The CLI may access only the library's public API; the library must
never depend on the CLI. Each package owns its README, source, and license;
CLI assets live under `crates/wg-launcher-cli/docs/assets`.

Both packages are workspace defaults. Use `--workspace` for full validation and
`-p wg-launcher` for library-only documentation. Install from source with
`cargo install --path crates/wg-launcher-cli --locked`. Release both packages
with `cargo publish --workspace --locked`; keep the shared package version and
the CLI's versioned path dependency synchronized before releasing.

### The library owns behavior; the CLI owns presentation

The crate must remain usable by a future GUI without invoking or parsing the
CLI. High-level workflows live in the library and are exported from `crates/wg-launcher/src/lib.rs`:

- `list(ListOptions)`
- `inspect(InspectOptions)`
- `download(DownloadAssetsOptions)`
- `extract(ExtractOptions)`
- `install(InstallOptions)`
- `status(StatusOptions)`
- `update(UpdateOptions)`

The CLI in `crates/wg-launcher-cli/src/cli.rs` should only:

- parse human input;
- translate arguments into the library option types;
- render tables, spinners, progress bars, summaries, and errors;
- call the high-level library functions.

`crates/wg-launcher-cli/src/main.rs` is only the executable bootstrap. Keep it limited to runtime
setup and `cli::run()`; command definitions and rendering belong in
`crates/wg-launcher-cli/src/cli.rs` and `crates/wg-launcher-cli/src/progress.rs`.

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

### Record product decisions in consumer documentation

Every new or changed product decision must leave a durable explanation in the
relevant library rustdoc or CLI documentation in the same change. Describe the
chosen behavior, its reason, defaults, consumer-visible consequences, and any
migration callers need. `AGENTS.md` records agent guidance; it is not a substitute
for documentation that library or CLI consumers can read.

Keep examples and API references synchronized with implementation. Public APIs
must remain documented, rustdoc links must resolve, and examples must compile.
For library API or documentation changes, also run:

```console
RUSTDOCFLAGS="-D warnings" cargo doc -p wg-launcher --lib --no-deps --all-features --locked
cargo test -p wg-launcher --doc --all-features --locked
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
resolves a selection without changing files and shares `install`'s default
EU/SD selection when invoked without arguments.

`extract` is local-only: it takes an optional positional installation
directory, defaults to the current directory, and reads the pinned manifest
from `.download`. It must not expose region, client type, or language options
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

- `list` queries all known regions unless `--region` is provided;
- `install` defaults to region `eu` and client type `sd`;
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

All launcher-owned state for an installation lives alongside its game files:

- package volumes and the pinned manifest live in `.download`;
- temporary extraction work lives in `.installing`;
- installed build identity lives in `.wg-launcher.json`;
- `.wg-launcher.lock` prevents concurrent mutation.

Do not add a dependency on user-home cache or data directories. A user must be
able to move the complete folder to another machine or location and retain its
installed identity and resumable state.

### Default installation names preserve initial identity

The default directory includes region, channel, the initially downloaded game
version, payload platform, architecture, client type, and language. For
example:

`wot-eu-production-2.4.1.5493-windows-x64-sd-en`

An update must target that same directory instead of creating or renaming it
for the newer version. The folder name records its initial download identity;
`.wg-launcher.json` records its current installed version.

### Installation is resumable and storage-conscious

The installation pipeline is intentionally integrated:

1. resolve and pin the manifest;
2. download a patch's package volumes into `.download`;
3. begin downloading the next patch when possible;
4. extract or apply the current patch;
5. delete that patch's package volumes only after successful extraction;
6. finalize the installation and write `.wg-launcher.json`.

This overlaps network and extraction work while limiting peak disk usage.
Do not change it back into “download everything, then extract everything.”

The downloaded `.wgpkg`/`.dspkg` volumes are transport archives. Extract them
into the game tree, but do not recursively unpack `.pkg` files found inside:
those are game-owned source assets and must remain in the format expected by
the game.

On interruption:

- complete cached files should be reused;
- partial downloads should resume when possible;
- temporary work may be safely rebuilt;
- a package already removed after successful extraction may need to be
  downloaded again if rebuilding requires it.

That last case is lost bandwidth/work, not loss of user game data. Never
delete cached package volumes before their extraction succeeds. Clean the
remaining download cache after a successful installation by default.

Use the per-installation `fs2` lock. Concurrent launchers must not mutate the
same installation.

## Status and update model

`.wg-launcher.json` is launcher state stored inside the game directory. It
records the game ID, region, channel, installed version, client type, language,
payload platform, architecture, and metadata version.

`status` loads that record and queries the matching live catalog selection.
`update` reuses the recorded selection and installation path. A pending pinned
manifest takes precedence during retry so an interrupted update remains
internally consistent.

The current update implementation follows the service's published installation
chain. It does not persist per-component versions and therefore does not yet
implement a fully minimal component-by-component delta update. Do not claim
otherwise. Adding that capability requires a deliberate state-schema and
migration design.

## Catalog and protocol rules

Catalog entries live in `crates/wg-launcher/src/catalog.rs`. A region is more than a display
label: it binds a publisher host, application ID, and channel. When adding or
changing one:

1. add it to `Region`, `Region::ALL`, display, and parsing;
2. use the exact live host and application ID;
3. add a catalog unit test;
4. run the real `list --region <region>` command;
5. confirm every expected client type produces a version row, not merely that
   metadata returns HTTP 200.

Supported catalogs currently include Wargaming regions and Lesta RU. Lesta RU
uses:

- host: `lstus-ru.lesta.ru`
- application ID: `MT.RU.PRODUCTION`
- CLI region: `ru` (alias `russia`)

Lesta's install chain omits `version_from` on patches. `crates/wg-launcher/src/service.rs`
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
- render a readable table of region, channel, client type, version, target,
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

- `crates/wg-launcher/src/catalog.rs`: regions and publisher product endpoints.
- `crates/wg-launcher-cli/src/main.rs`: minimal async executable bootstrap.
- `crates/wg-launcher/src/service.rs`: WGUS-compatible HTTP/XML protocol and manifest building.
- `crates/wg-launcher/src/model.rs`: protocol-independent public models and stable output naming.
- `crates/wg-launcher/src/download.rs`: resumable, concurrent package transfer.
- `crates/wg-launcher/src/install.rs`: patch ordering, extraction, deltas, local state, locks,
  cleanup, and the streaming install pipeline.
- `crates/wg-launcher/src/launcher.rs`: high-level list/inspect/download/extract/install/status/update workflows.
- `crates/wg-launcher-cli/src/cli.rs`: argument parsing and human-readable command output.
- `crates/wg-launcher-cli/src/progress.rs`: terminal-only progress rendering.
- `crates/wg-launcher/src/lib.rs`: the public library contract.

Respect these boundaries. If a change seems to require crossing them, first
consider whether a model, option, report, or event should be extended instead.

## Working rules for agents

- Inspect `git status` before editing. Preserve unrelated user changes.
- Prefer extending the high-level library contract before changing CLI code.
- Keep serialized installation state backward compatible or provide migration.
- Do not silently change defaults, directory naming, cleanup timing, or cache
  identity; these affect resumability and existing installations.
- Errors must explain the actionable condition without exposing an internal
  implementation stage as a user workflow.
- For filesystem and archive code, protect against path traversal and partial
  replacement of existing game files.
- For concurrency changes, preserve the installation lock and cancellation of
  unnecessary pending work after extraction failure.

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

Do not use an actual multi-gigabyte installation as a routine test. Unit tests
should cover state transitions and parsers; live listing should cover remote
catalog compatibility.
