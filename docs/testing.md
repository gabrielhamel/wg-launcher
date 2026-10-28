# End-to-end testing and reusable service fixtures

Launcher end-to-end tests and their shared resources.

[Documentation home](README.md) · [Contributing](contributing.md)

## Shared resources

Black-box tests, fixtures, and process harnesses live in the standalone
[`e2e/`](../e2e/) workspace package. It has no dependency on the service, client,
or launcher packages. Its fixture builder writes the same `metadata.xml`,
`patches_chain.xml`, and `packages/` layout accepted by the server binary.

## Extending the suite

Keep reusable test resources in `e2e/`, independent of production models. Tests
should exercise the persisted
contract rather than construct their expectations through the implementation
under test. Create pinned manifests through actual CLI `download` operations
instead of writing them from test helpers. Keep process scenarios and helpers in
`e2e/tests/`.

Use a separate temporary directory and ephemeral server port for each scenario.
Assert the relevant user-visible result: process status and diagnostics, exact
payload bytes, installed identity, retained or released cache files, and observed
HTTP requests. Preserve the bounded process execution and server shutdown when
adding failure scenarios.

## Run end-to-end tests

The CLI end-to-end scenarios exercise complete workflows using local fixtures.
Runnable library examples are checked for compilation separately.

```console
cargo test -p wg-launcher-e2e --locked
```

The dedicated server-crossing category is:

```console
cargo test -p wg-launcher-e2e --locked scenarios::service
```

Every scenario starts the real `wg-update-service` executable against a release
directory and invokes the real `wg-launcher` executable. Scenarios that republish
a version rewrite the release files and restart the service on the same address.
The optional access log proves request selection, cache reuse, and range
resumption without a test-only endpoint or in-process server.

Server-only CLI and HTTP contract tests belong to the service package:

```console
cargo test -p wg-update-service --locked
```

These start `wg-update-service`, verify fail-fast input validation, request the
WGUS metadata route, and resume a package from a byte offset.

The entry point is [tests/e2e.rs](../e2e/tests/e2e.rs).
Scenarios are grouped by category in
[tests/scenarios/](../e2e/tests/scenarios/), with shared process helpers and
fixtures in [tests/support/](../e2e/tests/support/).
Synthetic full archives and source-dependent librsync deltas are grouped in
[tests/support/fixture/](../e2e/tests/support/fixture/). Update scenarios are in
[tests/scenarios/updates/](../e2e/tests/scenarios/updates/); they verify component
versions in requests and state, delta-only transfers, unchanged components,
publisher full-chain fallback, empty current responses, incomplete-chain rejection,
and retries after completed and partially applied patches. The retry tests induce
ordinary extraction failures; they do not terminate a process mid-transfer or
claim whole-install transactional rollback. The harness builds missing
workspace binaries before launching them. Tests are included in
`cargo test --workspace --locked` and the existing Windows/Linux/macOS CI matrix.
Only macOS is verified when running locally on a Mac.

Run another category by passing its module name as a test filter:

```console
cargo test -p wg-launcher-e2e --locked scenarios::updates
```

Each scenario runs the executable in a separate process with an isolated
temporary installation directory, real HTTP transfers, real archive extraction,
and a 20-second process deadline. Output is captured for failure diagnostics.
External HTTP(S) is directed to an unavailable local proxy; loopback transfers
bypass it. No publisher traffic or game downloads are needed.

## Scope

The E2E release is synthetic; it does not prove compatibility with every live
publisher response. Catalog/protocol changes still require real publisher listing
checks. The independent harness owns synthetic WGUS documents and archive bytes.
It does not implement an HTTP server or call production Rust APIs.

The root workspace keeps the two launcher packages as defaults, so an unqualified
`cargo run` still selects the launcher. Name the service package explicitly with
`cargo run -p wg-update-service`. Full workspace checks include the three
published production packages and the unpublished E2E package.
