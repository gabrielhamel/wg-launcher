# wg-launcher end-to-end tests

This unpublished workspace package owns the black-box tests for the complete
launcher and update-service system. It has no Rust dependency on any production
package. Every test starts the real `wg-update-service` executable, invokes the
real `wg-launcher` executable, and communicates only through CLI, HTTP,
access-log, and filesystem contracts.

```console
cargo test -p wg-launcher-e2e --locked
```

Run only the scenarios that cross both real processes:

```console
cargo test -p wg-launcher-e2e --locked scenarios::service
```

The harness builds either executable when it is missing from `target/debug`.
Synthetic payloads, archive construction, real-process lifecycle management, and
process helpers live under `tests/support/`; workflow cases live under
`tests/scenarios/`.

[Testing guide](../docs/testing.md) · [Server guide](../docs/update-service.md)
