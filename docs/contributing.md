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
also include the published service package and unpublished E2E package. Run focused checks with
`-p wg-update-client`, `-p wg-launcher-cli`, `-p wg-update-service`, or
`-p wg-launcher-e2e`.

## Release packages

Keep the shared workspace version and the CLI's versioned `wg-update-client`
dependency synchronized. The release workflow publishes the independent update
service, then the update client, then the CLI. The E2E package remains unpublished.

[Back to documentation](README.md)
