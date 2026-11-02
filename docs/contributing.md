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

## Workspace layout

```text
Cargo.toml                    Workspace and shared package metadata
Cargo.lock                    Shared dependency lockfile
docs/                         User guides and contributor reference
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
      progress/               Shared progress bars, discovery spinner, styles
    docs/assets/              CLI demonstration assets
  wg-update-service/          File-backed server binary
    fixtures/                 Example WGUS XML, product catalogs, and payload files
    src/main.rs               Server CLI bootstrap and arguments
    src/server.rs             File-backed HTTP serving and byte ranges
e2e/                          Independent black-box system test package
  tests/scenarios/            Workflow and server-process scenarios
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
and PowerShell installers. Build the service from source using the
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
to publish only `wg-update-client`. The CLI and service are never published to
crates.io by this workflow. Prerelease tags create GitHub prereleases and skip
crates.io publication. If publication
fails, check whether the client was published before retrying: crates.io versions
cannot be overwritten. Publishing uses the supplied token directly; it does not
write registry credentials to disk.

Rerun the appropriate installer to upgrade the CLI. No standalone updater is
installed; a future built-in updater must be named `wg-launcher self-update`.
The existing `update` command continues to update game assets.

[Back to documentation](README.md)
