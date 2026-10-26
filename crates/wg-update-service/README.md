# wg-update-service

File-backed WGUS-compatible update server. It streams package files from disk
and supports resumed downloads without depending on the launcher client or its
end-to-end test harness.

[Server guide](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/update-service.md) · [Documentation home](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/README.md) · [API documentation](https://docs.rs/wg-update-service) · [Testing](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/testing.md)

## Start a release

Prepare `metadata.xml`, `patches_chain.xml`, and a `packages/` directory, then:

```console
cargo run -p wg-update-service --locked -- \
  --input ./release \
  --listen 127.0.0.1:8080
```

The service prints both its origin and the product host to use in a launcher's
custom `products.json`. See the [server guide](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/update-service.md) for
the complete directory tree, XML example, reverse-proxy settings, and launcher
commands.

## Run the tests

From a repository checkout:

```console
cargo test -p wg-update-service --locked
cargo test -p wg-launcher-e2e --locked
```

The service tests cover startup validation, HTTP ranges, and safe paths. Every
test in the independent E2E package starts both real binaries and covers WGUS
discovery, downloads, archive extraction, persisted state, and cleanup.

## Add or reuse a scenario

The [testing guide](https://github.com/gabrielhamel/wg-launcher/blob/main/docs/testing.md) explains the shared resources,
scenario coverage, and how to add a failure or recovery case. Keep reusable
payloads and protocol fixtures in the standalone `e2e/` package so the production
server and client packages do not own or compile test infrastructure.
