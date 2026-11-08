# Contributing

[Documentation home](README.md) · [Testing guide](testing.md) · [Agent guide](../AGENTS.md)

## Build from source

```console
git clone https://github.com/gabrielhamel/wg-launcher.git
cd wg-launcher
cargo install --path crates/wg-launcher-cli --locked
```

To run a command from the checkout, use `cargo run --locked -- --help`.

## Check a change

```console
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --locked
cargo check -p wg-update-client --examples --locked
```

For library documentation and examples:

```console
RUSTDOCFLAGS="-D warnings" cargo doc -p wg-update-client --lib --no-deps --all-features --locked
cargo check -p wg-update-client --examples --locked
```

The library guide is also the rustdoc landing page. Matching programs under `crates/wg-update-client/examples/` are checked for
compilation and can be run explicitly from the checkout. Validation does not
install a game. Keep those programs and their guide snippets synchronized.

## Terminal output

CLI command hints use bold cyan with no literal backticks in terminal output.
Use the shared `terminal::command` style for installation, update, status,
version, and package-manager hints. Match color detection to stdout or stderr;
without color, commands remain plain text. Keep normal Markdown code formatting
in documentation.

## Context guides

The workspace [context map](../CONTEXT_MAP.md) displays one map per production crate.

Each context has a short introduction and a glossary of its shared language:

| Area | Guides |
| --- | --- |
| Available games and builds | [Catalog](../crates/wg-update-client/src/catalog/README.md) · [Catalog in the launcher](../crates/wg-launcher-cli/src/catalog/README.md) |
| Local game copies | [Installation](../crates/wg-update-client/src/installation/README.md) · [Installation in the launcher](../crates/wg-launcher-cli/src/installation/README.md) |
| Launcher releases | [Self-update](../crates/wg-launcher-cli/src/self_update/README.md) |
| Providing game releases | [Release serving](../crates/wg-update-service/src/release_serving/README.md) |

## Context map generation

Edit the CML sources under [context-maps](context-maps) when context
boundaries or integration contracts change. Diagram rendering is automated;
discovering boundaries and choosing DDD patterns still requires review.

The [Context maps workflow](../.github/workflows/context-maps.yml) validates all
models and renders SVGs only on relevant pushes to `main` or manual runs on
`main`. It does not run on pull requests. After rendering,
`github-actions[bot]` commits only changed SVGs. It also removes diagrams
whose CML model was removed. An unchanged rendering produces no commit.

The commit job uses the built-in `GITHUB_TOKEN` with `contents: write`; no extra
secret is needed. Repository rules must allow that bot to push to `main`.
A concurrent update to `main` can reject the push; rerun the workflow against
the latest `main` in that case. The workflow never force-pushes.

To regenerate locally, install Python 3.12+, Java 17+, and Graphviz, then run:

```console
python3 docs/context-maps/generate.py
```

Set `JAVA_HOME` if Java is not on `PATH`; `dot` must be on `PATH`.
The script downloads the pinned Context Mapper CLI 6.12.0 distribution, verifies
its SHA-256, and validates every model before replacing generated SVGs.
It fixes Context Mapper's randomized context orientations in the generated
Graphviz source before rendering, so identical models produce identical SVGs
with the same toolchain.
Tools and intermediate files live in a temporary directory outside the checkout.
Use the workflow's Ubuntu/Java/Graphviz environment for consistent layout;
different Graphviz versions can produce different SVG bytes.

The [Context Mapper CLI](https://github.com/ContextMapper/context-mapper-cli)
and [context map generator](https://contextmapper.org/docs/context-map-generator/)
provide model validation and diagram generation.

## Workspace layout

```text
Cargo.toml                    Workspace and shared package metadata
Cargo.lock                    Shared dependency lockfile
CONTEXT_MAP.md                One context map per production crate
docs/                         User guides and contributor reference
  context-maps/               CML models and the pinned CLI rendering script
    generated/                SVG context maps maintained by GitHub Actions
.github/workflows/context-maps.yml  Main-only context map generation and bot commits
packaging/                    Signed apt packaging and generated WinGet manifests
crates/
  wg-update-client/           Reusable update client, README, and license
    examples/                 Runnable programs matching the library guide
    src/
      lib.rs                  Public API and consumer documentation
      catalog/                Published products and release discovery context
        mod.rs                Context entry points and exported models
        domain/               Built-ins, products, selections, release/patch rules
          release/patch_chain.rs  Ordered patches with their resolved packages
        application/          list, inspect, and live build resolution
          reports.rs          Consumer build identity and inspection/patch summaries
          directory_name.rs   Suggested directory names shared by inspect and installation
        infrastructure/wgus/  WGUS transport and response translation
          mod.rs              HTTP requests, redirects, and release resolution
          conversion.rs       Wire-to-domain metadata/releases and publisher normalization
          wire.rs             XML response representations
      installation/           Local installation lifecycle context
        mod.rs                Context entry points and exported contracts
        domain/               Installed identity, pinned manifests, compatibility, checkpoints
        application/          download, extract, install, status, update
          resolution.rs       Pinned-cache reuse and live-resolution coordination
          plan.rs             Consumer plan and progress package summaries
          pipeline/           InstallationPipeline and shared execution contracts
            extraction.rs     Local extraction and installation finalization
            streaming.rs      Overlapping downloads and ordered patch execution
            patch.rs          Apply a patch, persist its checkpoint, then release packages
        infrastructure/
          transfer/           Resumable HTTP package downloads
          packages/           Transport archives, delta application, throttled output progress
            contents.rs       Classify replacement files and typed deltas before execution
          filesystem/         Installation filesystem adapters
            storage.rs        Installation-relative paths and staging preparation
            lock.rs           Exclusive installation mutation lock
            cache.rs          Package verification and release
            records.rs        One replacement policy for manifests and installed-build persistence
            patch_workspace.rs  Active patch staging and preserved original sources
            replacement.rs    File replacement with rollback on rename failure
      error.rs                Public Error, ErrorKind classification, and Result alias
  wg-launcher-cli/            CLI package, README, and license
    src/
      main.rs                 Runtime bootstrap
      cli/                    Top-level argument parsing and context dispatch
      catalog/                Catalog CLI context
        presentation/         Selection arguments, build/patch tables
          commands.rs         list and inspect command handlers
        infrastructure/       Custom product file loading
      installation/           Game installation CLI context
        presentation/         Arguments, summaries, observer adapters, progress
          commands.rs         download, extract, install, status, update handlers
          progress/           Shared display with separate download/extraction renderers
      self_update/            CLI executable update context
        domain/               Release comparison and ownership/path rules
          artifact.rs         Select the host archive format and checksum pair
        application/          Release checks, executable update workflow, events and outcomes
        infrastructure/       Release HTTP, archives, ownership detection, replacement
          release.rs          Discover published launcher releases
          artifact.rs         Download, verify, and extract the selected artifact
          http.rs             Shared HTTPS, redirect, and bounded-transfer policy
        presentation/         Command handlers, notices, update instructions, event rendering
      terminal/               Shared command styling, byte sizes, spinner/styles
    docs/assets/              CLI demonstration assets
  wg-update-service/          File-backed server binary
    fixtures/                 Example WGUS XML, product catalogs, and payload files
    src/
      main.rs                 Minimal executable bootstrap
      lib.rs                  Public service API
      release_serving/        File-backed release serving context
        domain/               Release URL/prefix rules and template rendering
        application/          FileService startup, routing, and lifetime coordination
        infrastructure/       HTTP/ranges, filesystem mapping, access logs
        presentation/         Service CLI arguments and startup output
e2e/                          Independent black-box system test package
  src/bin/cli_update_fixture.rs  Independent replacement executable for CLI update tests
  tests/scenarios/            Workflow and server-process scenarios
    cli_update.rs             Startup/version checks and executable replacement
    updates/                  Component delta updates and recovery scenarios
  tests/support/              Real-process harness
    fixture/                  Independent full archives and delta fixtures
```

Each directory's `mod.rs` is its entry point. Workflow options, events, and results
live beside their workflow; shared installation events and reports live in
`installation/application/pipeline/types.rs`. End-to-end scenarios and their
process helpers live entirely under `e2e/tests/`.

Organize by bounded context first, with domain, application, and infrastructure
inside each context. `catalog` owns published product and release knowledge,
including WGUS discovery and patch planning. Its `ResolvedRelease` describes
the selected build and published packages without local progress. `installation` owns the local
installation lifecycle, including transfer, package application, storage, and
recovery. Its `PinnedManifest` combines a resolved release with component
checkpoints; only installation advances those checkpoints. The persistence
adapter retains the existing flat JSON fields through Serde flattening.

Patch sources distinguish full content from changes to a specific version, while
retaining the saved `version_from` string or null representation. Planned patches
keep each step with its resolved packages; prefetch delivers the patch with its
transfer result instead of coordinating separate patch and package lists.
The patch coordinator persists each checkpoint before releasing its workspace
and cached volumes. `PatchWorkspace` owns the active patch's staging paths and
original sources, without deleting recovery files on failure.

CLI self-update emits events and returns outcomes; its presentation commands own
terminal output and adapt those events to the stable progress display.

Discovery requests keep optional defaults and catalog filters; a resolved selection
contains the known product, client type, and language used for installation compatibility.
Both streaming installation and staged extraction use the same patch-completion
operation, preserving apply-before-checkpoint and checkpoint-before-package-release ordering.
Suggested directory naming belongs to the catalog application contract so
inspection and installation use the same suggestion without making catalog
depend on installation. Transfer and archive decoding are technical capabilities within the
installation context. Neither is a separate bounded context.

The CLI mirrors catalog and installation presentation and keeps executable
self-updating in its own context. Its game commands delegate business behavior
to the client library; their `presentation/commands.rs` modules coordinate
arguments, library calls, and terminal output rather than duplicating the
library application layer. `terminal/` contains
only shared presentation primitives. The service has one context,
`release_serving`, with its own release rules, startup workflow, adapters, and CLI.
Do not create empty layer directories or additional contexts merely for symmetry.

Each context's `mod.rs` exposes its entry points and models. Installation calls
catalog entry points for live resolution and patch planning; catalog has no
knowledge of installation storage or execution. The crate root exposes consumer workflows and reports while keeping protocol,
transport, persistence, and execution models internal. Internal imports name the owning context or a nearby module rather than
routing through root re-exports. The root `error.rs`
adapts local errors to actionable public categories and preserves their sources.

Within each context, domain code makes decisions without network or filesystem
operations. Application workflows coordinate rules and infrastructure, preserving
overlapping transfer/extraction, checkpoint-before-release ordering, and
cancellation under the installation lock. Infrastructure owns HTTP/XML,
archive/decoder libraries, and filesystem mechanics. WGUS-specific normalization
stays in the catalog adapter.

Errors are defined beside their owning behavior: domain and infrastructure
modules use local `error.rs` files, while small workflow errors live directly in
the workflow file. Domain errors contain no HTTP, XML, JSON, or I/O failures.
Internal operations return their owning error type; application workflows expose
`Error` with an `ErrorKind` and a diagnostic source chain. There are
no repository traits or additional crates solely to enforce these boundaries.

These modules are internal implementation details. Library consumers use the
public exports from `wg_update_client`.

The two launcher packages are workspace defaults. Commands using `--workspace`
also include the service package and unpublished E2E package. Run focused checks with
`-p wg-update-client`, `-p wg-launcher-cli`, `-p wg-update-service`, or
`-p wg-launcher-e2e`.

## Release packages

Set the release version once in `workspace.package.version` in the root
[`Cargo.toml`](../Cargo.toml). All packages inherit it; the CLI uses a local path
dependency on the client. Run `cargo check --workspace` after changing the version
to refresh the generated `Cargo.lock`, and commit both files together.
Documentation links use `latest`, Rust dependency examples use the supported
major version, and source installation instructions use release source archives.

Only `wg-update-client` is published to crates.io. The CLI, service, and E2E
packages have `publish = false`.

[`dist-workspace.toml`](../dist-workspace.toml) configures cargo-dist, pinned to
0.33.1. It distributes `wg-launcher-cli` for Windows x64, Linux x64, and macOS
Apple Silicon and Intel, with ZIP/tar.gz archives, SHA-256 checksums, and shell
and PowerShell installers, plus a Homebrew formula. Build the service from source using the
[service guide](update-service.md#start-the-server).
The generated [release workflow](../.github/workflows/release.yml) runs a release
plan on pull requests and builds and publishes binaries when a version tag is
pushed. Creating a GitHub Release does not trigger this workflow. Pushes to
`main` run the separate checks workflow, without building or publishing a release.
Pull requests also run checks and a release plan, without publishing.

Install the pinned release tool and regenerate CI after configuration changes:

```console
cargo install cargo-dist --version 0.33.1 --locked
dist generate
dist generate --check
dist plan
```

Keep 0.33.1 pinned until a newer version has been checked: 0.34.0 generates
invalid local reusable-workflow references for custom publishing jobs.

Before releasing, run the validation commands above and review the release plan.
Ensure the repository has a `CRATES_IO_TOKEN` Actions secret authorized to publish
`wg-update-client`. Once the release commit is on the remote, push its
matching tag. Replace `<version>` below with `workspace.package.version`
from the root manifest (without a `v` prefix):

```console
git tag <version>
git push origin <version>
```

The workflow builds all four targets, creates a GitHub Release with the archives
and installers, then calls [`publish-crates.yml`](../.github/workflows/publish-crates.yml)
to publish only `wg-update-client`. The parallel
[`publish-homebrew.yml`](../.github/workflows/publish-homebrew.yml) job downloads
the generated `wg-launcher.rb` from that release and pushes it to
[`gabrielhamel/homebrew-tap`](https://github.com/gabrielhamel/homebrew-tap).
The `HOMEBREW_TAP_SSH_KEY` Actions secret is a deploy key with write access only
to that tap; it does not need a personal access token. Keep the matching deploy
key enabled in the tap repository. This custom job replaces cargo-dist's built-in
Homebrew publisher, so dist's warning that the built-in job is disabled is
expected. To retry only Homebrew publishing, run that workflow manually with an
existing release tag that contains `wg-launcher.rb`; it does not create a release
or republish the Rust crate. The CLI and service are never published to crates.io
by this workflow. Prerelease tags create GitHub prereleases and skip
crates.io publication. If publication
fails, check whether the client was published before retrying: crates.io versions
cannot be overwritten. Publishing uses the supplied token directly; it does not
write registry credentials to disk.

### apt and WinGet publishing

[`publish-apt.yml`](../.github/workflows/publish-apt.yml) builds the tagged CLI on
Ubuntu 22.04 with `WG_LAUNCHER_PACKAGE_MANAGER=apt`. The resulting amd64 Debian
package requires glibc 2.35 or newer. [`packaging/apt.py`](../packaging/apt.py)
generates the apt indexes, signs `Release`/`InRelease` using the `APT_SIGNING_KEY`
Actions secret, tests installation through apt, and deploys the repository to
GitHub Pages at `https://gabrielhamel.github.io/wg-launcher/apt`. The public key is
served as `wg-launcher.asc`; never commit the private signing key. The
`github-pages` environment must allow `main` for manual retries and the tag
pattern `[0-9]*.[0-9]*.[0-9]*` for tagged-release deployments. The public key
fingerprint is `EC919478EC4B9D1C8E76EEB2F397D2E90A411990`.

Validated `.deb` files are attached to their GitHub Release. Repository rebuilds
reuse those immutable assets and retain earlier published packages so cached
indexes and explicit-version installs remain usable. Changing a package's bytes
requires a new release version.

[`publish-winget.yml`](../.github/workflows/publish-winget.yml) uses
[`packaging/winget.py`](../packaging/winget.py) to generate the three manifests for
`GabrielHamel.WgLauncher` from a verified Windows release archive. It validates
and installs them using the native WinGet client, checks self-update refusal,
attaches `wg-launcher-winget.zip` to the release, and submits a pull request to
`microsoft/winget-pkgs` using the `WINGET_TOKEN` secret. The token needs only
`public_repo` access. Microsoft validation and review control when a submission
becomes available through `winget install`; uploading manifests alone does not
publish the listing. The first submission is
[PR #468869](https://github.com/microsoft/winget-pkgs/pull/468869); its author must
complete Microsoft's Contributor License Agreement before it can be accepted.

The hosted WinGet client currently recognizes older schema headers than the
community repository. The workflow first validates all three files against the
actual current Microsoft JSON schemas. Native validation may then accept only
the specific schema-header warning HRESULT, rejecting all semantic errors and
other warnings. Installation, checksum verification, and package-manager
ownership are still tested using WinGet itself.

Both workflows run after stable tagged releases and can be retried manually
with an existing release tag. WinGet retries reuse an open or merged submission
for that exact version instead of opening duplicate pull requests. They do not republish the Rust crate. Prereleases
are excluded from these distribution channels.

`wg-launcher self-update` reads the published cargo-dist manifest and upgrades
the running executable in process. No standalone updater is installed. The
existing `update` command continues to update game assets.

Homebrew formulae use the same prebuilt archives as standalone installs.
Homebrew writes an `INSTALL_RECEIPT.json` ownership marker; the CLI recognizes
that receipt and canonical Homebrew Cellar paths, including the existing
published binary's path-based protection.

For other package-manager distributions built from source, build with `WG_LAUNCHER_PACKAGE_MANAGER` set
to `homebrew`, `apt`, `winget`, `scoop`, `nix`, or another manager name. This compile-time
marker disables built-in replacement and points users at their package manager.
The CLI also recognizes canonical Homebrew Cellar, WinGet, Scoop, and Nix store
paths, including executions reached through symlinks. Future package-manager
integrations must preserve an ownership marker, such as Homebrew's receipt or
the compile-time setting, rather than relying only on path detection.

For a trusted release mirror, set the runtime `WG_LAUNCHER_UPDATE_MANIFEST` URL.
It must serve the cargo-dist `dist-manifest.json` structure with the
`wg-launcher-cli` release, version, artifact names, and stable-release flag.
Archives and their `.sha256` files must be in the same URL directory. HTTPS is
required except for loopback HTTP mirrors. Without this setting, the CLI reads
the latest GitHub Release manifest and fetches assets from its pinned release
tag. This does not use the GitHub API or require a token. Startup checks have a
two-second HTTP timeout and store no user cache; `WG_LAUNCHER_NO_UPDATE_CHECK`
skips automatic checks only.

[Back to documentation](README.md)
