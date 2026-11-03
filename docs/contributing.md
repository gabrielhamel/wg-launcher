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
Use the shared `cli::output::command` style for installation, update, status,
version, and package-manager hints. Match color detection to stdout or stderr;
without color, commands remain plain text. Keep normal Markdown code formatting
in documentation.

## Workspace layout

```text
Cargo.toml                    Workspace and shared package metadata
Cargo.lock                    Shared dependency lockfile
docs/                         User guides and contributor reference
packaging/                    Signed apt packaging and generated WinGet manifests
crates/
  wg-update-client/           Reusable update client, README, and license
    examples/                 Runnable programs matching the library guide
    src/
      lib.rs                  Public API and consumer documentation
      catalog.rs              Regions and publisher endpoints
      catalog/product.rs      Owned products and service URL validation
      catalog/selection.rs    Catalog validation and region selection
      model.rs                Protocol-independent public models
      error.rs                Shared error types
      download.rs             Resumable package transfers
      launcher/               High-level workflows, grouped by operation
      install/                Extraction, patch chains, deltas, storage, prefetch, retry
      service/                WGUS client and private XML wire models
  wg-launcher-cli/            CLI package, README, and license
    src/
      main.rs                 Runtime bootstrap
      cli/                    Dispatch, arguments, output, observer adapters
        output/status.rs      Status colors and update size presentation
        output/cli_update.rs  Boxed CLI release notice and update instructions
        self_update/          CLI release checks, manager detection, verified replacement
      progress/               Shared progress bars, discovery spinner, styles
        self_update.rs        CLI update transfer progress and stage spinners
    docs/assets/              CLI demonstration assets
  wg-update-service/          File-backed server binary
    fixtures/                 Example WGUS XML, product catalogs, and payload files
    src/main.rs               Server CLI bootstrap and arguments
    src/server.rs             File-backed HTTP serving and byte ranges
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
`install/types.rs`. End-to-end scenarios and their process helpers live entirely
under `e2e/tests/`.

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
