# Run an update service

[Documentation home](README.md) · [Custom catalogs](catalogs.md) · [Testing](testing.md)

`wg-update-service` publishes a release stored in ordinary files. It implements
the WGUS-compatible metadata and patch-chain routes used by `wg-update-client`,
streams package volumes without loading them into memory, and supports resumed
downloads through HTTP byte ranges.

## Prepare a release directory

Create this layout:

```text
release/
├── metadata.xml
├── patches_chain.xml
└── packages/
    ├── client.wgpkg
    ├── locale.wgpkg
    └── subdirectory/
        └── optional-volume.wgpkg
```

`metadata.xml` and `patches_chain.xml` are required UTF-8 WGUS documents.
`packages/` is required and may contain nested directories. The service exposes
files below it at both `/<relative-path>` and `/packages/<relative-path>` so it
can serve existing WGUS seed URLs as well as explicit package URLs.

Package `<name>` values, declared sizes, client parts, versions, languages, and
the application ID come from these XML files. The service deliberately does not
invent or rewrite that release data. Package names and sizes therefore need to
match the files you publish.

Use `__BASE_URL__` wherever the public origin is needed in either XML document.
It is replaced once at startup. A minimal patch-chain seed looks like this:

```xml
<protocol>
  <version_name>1.0</version_name>
  <patches_chain>
    <patch>
      <part>client</part>
      <version_from>0</version_from>
      <version_to>1.0</version_to>
      <files>
        <file><name>client.wgpkg</name><size>12345</size></file>
      </files>
    </patch>
  </patches_chain>
  <web_seeds>
    <url name="primary" threads="4">__BASE_URL__/packages/</url>
  </web_seeds>
</protocol>
```

The repository's complete synthetic examples are
[`metadata.xml`](../crates/wg-update-service/fixtures/metadata.xml) and
[`patches_chain.xml`](../crates/wg-update-service/fixtures/patches_chain.xml).
The test archive builder creates matching small `.wgpkg` volumes; production
volumes remain the publisher's ordinary transport archives.

## Start the server

From a checkout:

```console
cargo run -p wg-update-service --locked -- \
  --input ./release \
  --listen 127.0.0.1:8080
```

Or install and run the binary:

```console
cargo install --path crates/wg-update-service --locked
wg-update-service --input ./release --listen 0.0.0.0:8080
```

The default API prefix is `updates`, producing these routes:

```text
GET /updates/api/v1/metadata/
GET /updates/api/v1/patches_chain/
GET /packages/<relative package path>
```

Use `--prefix custom/path` to change the API prefix or `--prefix ""` to mount
the API directly at `/api/v1/`. Query parameters are accepted on the two WGUS
routes; their values are interpreted by the release XML rather than by separate
per-product files.

When listening behind a proxy, container port mapping, DNS name, or TLS
terminator, pass the externally reachable origin:

```console
wg-update-service \
  --input ./release \
  --listen 0.0.0.0:8080 \
  --public-url https://downloads.example.com
```

The built-in server speaks HTTP. An HTTPS `--public-url` means a reverse proxy
terminates TLS and forwards requests to the listen address.

For request auditing, append a tab-separated access log containing path, query,
and range fields:

```console
wg-update-service --input ./release --access-log ./access.log
```

The log is optional and append-only, so it can be retained across a supervised
restart or normal log rotation.

## Connect the launcher

Save a JSON array as `products.json`. The `host` is the printed product host:

```json
[
  {
    "region": "local",
    "name": "My local game",
    "host": "http://127.0.0.1:8080/updates",
    "game_id": "LOCAL.WOT",
    "channel": "testing"
  }
]
```

The `game_id` must match the application ID accepted by the XML release. Then
exercise the same public workflow used for built-in publishers:

```console
wg-launcher list --product ./products.json
wg-launcher inspect --product ./products.json --region local
wg-launcher install --product ./products.json --region local --output ./game
wg-launcher status --path ./game
```

The selected product snapshot is saved in `.wg-launcher/state.json`, so later
`status` and `update` commands do not need `products.json`.

## Publish another release

Prepare a complete new directory before exposing it. Replace the directory (or
change a symlink used by `--input`) and restart the service. Metadata and the
patch chain are snapshotted at startup; package files are opened per request.
Keeping the old process and files available while existing downloads finish
avoids mixing release metadata with replaced package bytes.

The server validates the required directory skeleton and text encoding before
it listens. The launcher additionally validates the WGUS XML, declared package
sizes, archive contents, and installation state through its normal workflows.

**Next:** [configure more regions](catalogs.md) · [run the cross-flow tests](testing.md)
