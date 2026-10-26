# Troubleshooting

[Documentation home](README.md) · [CLI guide](../crates/wg-launcher-cli/README.md)

Choose the symptom below. Retry commands with the same game directory and build
selection so the client can reuse completed work.

<details>
<summary>A download or installation was interrupted</summary>

Run the same command again:

```console
wg-launcher install --game wot --region eu --output ./world-of-tanks
```

For custom services, include the same product file and region. Complete packages
are reused, and partial files resume when supported by the service. Temporary
extraction work can be rebuilt. Some packages already released after successful
extraction may need to be downloaded again.

</details>

<details>
<summary>Extraction reports missing or incomplete packages</summary>

A filtered download may not contain every required component. Complete the cache
using the same selection, then extract:

```console
wg-launcher download --game wot --region eu --output ./world-of-tanks
wg-launcher extract ./world-of-tanks
```

For a custom catalog, add the same `--product` and `--region` options to the
`download` command. Extraction takes the game directory and remains local-only.

</details>

<details>
<summary>Status cannot find an installation</summary>

Point it at the directory where installation completed:

```console
wg-launcher status --path ./world-of-tanks
```

A successful install or extraction writes the installed identity. Downloading
packages alone does not create one. Keep the `.wg-launcher` directory with the
game when moving it. For other state errors, include the reported message when
[opening an issue](https://github.com/gabrielhamel/wg-launcher/issues).

</details>

<details>
<summary>The selected product does not match the cached build</summary>

Use the same region, client type, language, and product configuration used to
start the download. The client rejects a different product rather than mixing
packages. If you intend to install another product, choose another output path.
Reordering a custom product array or changing an unselected entry is fine.

</details>

<details>
<summary>Another installation is already running</summary>

Wait for the other operation using that directory to finish, then retry. Use a
separate destination if you need two independent installations. The lock protects
the same game folder from competing extraction or update operations.

</details>

<details>
<summary>A custom region is unknown, or the catalog is rejected</summary>

Check that the file is a non-empty JSON array, every entry has all five required
fields, and region labels are unique ignoring case. Select a configured region:

```console
wg-launcher list --product products.json
wg-launcher inspect --product products.json --region KR
```

A catalog containing only `KR` needs `--region KR` for build selection because
the default remains EU. The [catalog guide](catalogs.md) has a complete example.

</details>

<details>
<summary>A client type, language, or remote service is unavailable</summary>

Check the region's available builds first:

```console
wg-launcher list --region eu
wg-launcher inspect --game wot --region eu
```

Use a listed client type and language. For custom services, check the configured
base URL and application ID and that the server is running. The service must
provide WGUS-compatible metadata, patch chains, and package URLs.

</details>

Need more detail? [Installation reference](reference.md) · [Open an issue](https://github.com/gabrielhamel/wg-launcher/issues)
