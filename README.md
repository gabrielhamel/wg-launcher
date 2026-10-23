# wg-launcher

Discover, download, install, and update World of Tanks assets through Wargaming
and Lesta's update services. Supports Windows, Linux, and macOS as a launcher;
the game payload currently targets Windows/x64.

## Packages

| Package | Purpose | Documentation |
| --- | --- | --- |
| [`wg-launcher`](crates/wg-launcher) | Reusable Rust library, imported as `wg_launcher` | [Library guide](crates/wg-launcher/README.md) |
| [`wg-launcher-cli`](crates/wg-launcher-cli) | Terminal application, installed as `wg-launcher` | [CLI guide](crates/wg-launcher-cli/README.md) |

Product decision: the library and CLI are separate Cargo packages so applications
can use the library without depending on terminal presentation. The CLI consumes
only the library's public API. Library progress is optional: use plain workflows
for the result alone, or their `_with_observer` counterparts for events.

## Install and run the CLI

```console
git clone https://github.com/gabrielhamel/wg-launcher.git
cd wg-launcher
cargo install --path crates/wg-launcher-cli --locked
wg-launcher inspect
```

From a checkout, `cargo run --locked -- --help` also works at the workspace root.
See the [CLI guide](crates/wg-launcher-cli/README.md) for all commands and examples.

## Use the library

```toml
[dependencies]
wg-launcher = "1.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For a local checkout, point the dependency at `crates/wg-launcher`, rather than
the workspace root. See the [library guide](crates/wg-launcher/README.md) for
examples, progress handling, and installation behavior.

## Workspace layout

```text
Cargo.toml                    Workspace and shared package metadata
Cargo.lock                    Shared dependency lockfile
crates/
  wg-launcher/                Library package, README, and license
    src/                      Workflows, protocol, models, storage, transfer, extraction
  wg-launcher-cli/            CLI package, README, and license
    src/                      Bootstrap, argument parsing, terminal progress
    docs/assets/              CLI demonstration assets
```

Both packages are workspace defaults, so root checks cover both. Run focused
checks with `-p wg-launcher` or `-p wg-launcher-cli`.

```console
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --locked
cargo doc -p wg-launcher --lib --no-deps --locked --open
```

## Source and release migration

The former combined package is now a virtual workspace. The library package and
Rust import names are unchanged. Installing from this checkout now requires
`--path crates/wg-launcher-cli`; the installed executable remains `wg-launcher`.
The published CLI package is named `wg-launcher-cli`; `wg-launcher` contains only
the library in releases made from this layout. Older published versions retain
their previous contents.

The release workflow publishes both packages with
`cargo publish --workspace --locked`, allowing Cargo to publish the library
before its CLI dependent.
Before a release, update the shared workspace version and the CLI's versioned
library dependency together. A source-tree reorganization does not itself
publish a release.
