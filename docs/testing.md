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

Use a separate temporary directory for each scenario. Start each service with
`--listen 127.0.0.1:0` and read its bound address from that child's startup
announcement. Do not probe and release a free port before spawning the service,
or treat a connection to an existing listener as proof that the child is ready.
When republishing fixtures, restart on the previously announced address.
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

The dedicated server-crossing category also verifies parallel services receive
distinct ports and preserve their address when restarted:

```console
cargo test -p wg-launcher-e2e --locked scenarios::service
```

Every scenario starts the real `wg-update-service` executable against a release
directory and invokes the real `wg-launcher` executable. Scenarios that republish
a version rewrite the release files and restart the service on the same address.
The optional access log proves request selection, cache reuse, and range
resumption without a test-only endpoint or in-process server.

CLI self-update scenarios use the same real service to host a cargo-dist
manifest, checksums, and replacement archives through the supported release-mirror
setting. They run a copied launcher, never the shared build executable, and
replace it with the independent `cli-update-fixture` executable from `e2e/src/bin`.
Coverage includes startup/help update notices, current/latest version output, offline
checks, skipped checks, equal/older releases, package-manager refusal (including
Homebrew receipts in custom prefixes), checksum
and embedded-version failures, successful replacement, staging cleanup, and
preservation of unrelated files, the shared build executable, and another launcher
copy. The synthetic replacement has its own test version, independent of the
workspace release. These scenarios share the
Windows/Linux/macOS CI matrix; a local run verifies only the host platform.

```console
cargo test -p wg-launcher-e2e --locked scenarios::cli_update
```

The tap's separate workflow installs the published formula on Apple Silicon
macOS, Intel macOS, and Linux x64, then verifies that self-update refuses the
Homebrew-owned executable.

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
and retries after completed and partially applied patches. Those update retry tests induce ordinary extraction failures. The recovery category
also terminates real processes at observed transfer and extraction phases, as
described below. Neither category claims whole-install transactional rollback. The harness builds missing
workspace binaries before launching them. Tests are included in
`cargo test --workspace --locked` and the existing Windows/Linux/macOS CI matrix.
Only macOS is verified when running locally on a Mac.

## Recovery coverage

Run the recovery category with:

```console
cargo test -p wg-launcher-e2e --locked scenarios::recovery
```

The category lives in [`scenarios/recovery/`](../e2e/tests/scenarios/recovery/).
`downloads.rs` covers cache repair; `extraction.rs` covers blocked destinations,
state writes, movement, and cleanup; `delta.rs` covers a malformed delta stream.
`interruption/` groups actual child-process termination tests by phase. Existing
[`updates/delta.rs`](../e2e/tests/scenarios/updates/delta.rs) tests additionally
cover missing delta sources, checkpoints across multi-component updates, and
mixed archives whose replacements must be installed before their deltas. Retry
assertions also check the saved `version_from` fields and active-patch identity
used to retain original sources across executions.

The agreed recovery matrix maps to the following executable cases. Rows describe
coverage, rather than a claim that every possible failure in that area is tested.

| # | Failure or interruption | Observable check |
|---|---|---|
| 1 | Kill during package transfer | A nonempty incomplete `.part` file survives; retry requests its exact remaining range. |
| 2 | Stop service during transfer | Real launcher exits unsuccessfully; restarting the service at the same address permits a ranged retry. |
| 3 | Interrupt concurrent downloads | Each complete volume is reused and each incomplete durable prefix resumes independently. |
| 4 | Required package returns 404 | Other volumes and pinned manifest survive; restoration permits retry without discovery. |
| 5 | Truncated or corrupt cached volume | Wrong-size files are fetched again; invalid headers and damaged compressed streams fail extraction without truncating existing destinations. |
| 6 | Invalid partial volume | Oversized partials restart, complete partials promote without HTTP, and corrupt prefixes fail extraction. |
| 7 | Later archive is corrupt | Earlier component checkpoint and payload survive; failed component retries offline. |
| 8 | Kill during full extraction | Kill both between completed files and while a large file decodes in staging; retries recreate exact bytes and preserve the previous destination during decoding. |
| 9 | Destination is blocked | User directory contents and transport volumes survive; removing the obstruction permits offline retry. |
| 10 | Fail after applying one delta | Original sources survive and a repeated patch produces the expected source-dependent output. |
| 11 | Kill during delta application | First target is changed, last is untouched, installed identity stays old; pinned retry completes all targets. |
| 12 | Invalid delta or missing source | Malformed librsync data preserves the target/source snapshot; repaired cached archive retries. Existing update tests cover missing sources. |
| 13 | One patch completes before the next fails | Checkpointed patch is skipped despite its deleted transport volumes. |
| 14 | Extraction/checkpoint/package-release boundary | Block the checkpoint write after extraction; separately kill after persisted checkpoint while cleanup delays package release. |
| 15 | Final state write fails | All component checkpoints remain, installed state is absent, retry finalizes without downloads. A state-directory obstruction remains untouched. |
| 16 | Cleanup fails or is interrupted | Kill during staging cleanup after committed state; Unix permission failure preserves identity and retries after permissions are restored. |
| 17 | New release during pending installation | Retry uses the old pinned manifest and avoids discovery. |
| 18 | New release during pending update | A newer advertised chain cannot replace the pending delta update; no additional HTTP requests occur. |
| 19 | Move a pending installation | Completed checkpoint and remaining packages work at the new path, offline and without the product JSON file. |
| 20 | Concurrent mutation and lock release | A real download owns the installation lock; a competing CLI is refused; a killed launcher's destination can be retried. |
| 21 | Offline extraction and retry | With the real service stopped, cached full extraction and retry/finalization succeed. |

### How interruption tests synchronize

Download interruption uses a test-owned TCP gate that forwards opaque bytes to
and from the real service. Both package responses pause after 64 KiB; the harness
waits for durable prefixes in both `.part` files and runs a competing CLI before
terminating the launcher or stopping the service. Downloads cannot finish while
the controller is descheduled. On service loss the gate disconnects established
streams and rejects forwarding until the service restarts at its original address;
on retry it opens fully. WGUS XML advertises the gate using ordinary web-seed URLs.
The gate does not parse HTTP, manufacture responses, or implement ranges: those
remain the real service's responsibility. Its threads and sockets are joined and
closed on cleanup, including assertion failures.

The two transfer payloads are 256 KiB each. Extraction and delta fixtures contain
600 small files, with an additional 8 MiB fixture for interruption during full-file
decoding. Those tests poll filesystem evidence with a deadline, kill/reap the child,
and assert the incomplete state; missing the intended phase fails the test.
Leftover staging files extend checkpoint and cleanup windows. No guessed sleep
before termination, production fault switches, or substitute HTTP server are used.
Every child has distinct output logs and is killed/reaped on assertion failure.
The service-loss case retains a 150-second bound for the production network retry
budget; other children have a 20-second deadline.

The original polling-only transfer test passed the PR matrix but raced on the
post-merge macOS run: both volumes finished before a nonempty `.part` file was
observed. The TCP gate removes that scheduling race rather than making payloads
larger or retrying a test that missed its phase.

### Findings and limits

The original 29-test suite passed on macOS before this expansion. New regression
tests exposed three failures: a directory at `state.json` was moved aside during
replacement, package-only downloads ignored the installation lock, and damaged
compressed data truncated an existing game file before extraction failed. The fixes
preserve non-file destinations, acquire the mutation lock around high-level
download cache writes/transfers while rereading pending checkpoints under that lock,
and stage each full-archive file before replacing its game-tree destination.
The low-level downloader still leaves coordination to its caller.

Size-based reuse does not authenticate content. A same-size corrupt package or
partial prefix may pass downloading and then fail archive/delta decoding. Tests
model repair by replacing or removing the affected cached volume. Arbitrary
same-size changes to a delta source are not guaranteed to be detected; these tests
do not introduce a content-hash integrity contract.

The permission-denied cleanup test is Unix-only and requires an unprivileged
account; cross-platform blocked-destination and process-termination tests still
run on Windows. The suite does not simulate actual disk exhaustion, power loss,
`fsync` durability, interruption inside an individual rename, every point in
recursive cache deletion, or whole-install transactional rollback. Process-kill
coverage proves the observed phase boundaries, not all possible instruction-level
crash windows. Platform results must come from the corresponding CI job.

Run another category by passing its module name as a test filter:

```console
cargo test -p wg-launcher-e2e --locked scenarios::updates
```

Each scenario runs the executable in a separate process with an isolated
temporary installation directory, real HTTP transfers, real archive extraction,
and a 20-second process deadline (150 seconds for service-loss retry exhaustion). Output is captured for failure diagnostics.
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
