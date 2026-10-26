# Games, regions, and custom catalogs

[Documentation home](README.md) · [CLI guide](../crates/wg-launcher-cli/README.md) · [Rust guide](../crates/wg-update-client/README.md)

Use the built-in catalog to get started, or provide your own games, services, and
region labels. The same catalog selection is available from the CLI and Rust library.

[Built-in games](#built-in-games-and-regions) · [Create a catalog](#create-a-custom-catalog) · [Select a build](#select-a-game-and-region) · [Rust example](#use-the-catalog-from-rust)

## Built-in games and regions

| Game | Regions | Catalog |
| --- | --- | --- |
| `wot` | `eu`, `na`, `asia`, `cn`, `ct` | World of Tanks |
| `wot` | `ru`, `pt-ru` | Lesta Mir Tankov |
| `wows` | `ww` | World of Warships |
| `wowp` | `ww` | World of Warplanes |
| `wotb` | `ww` | World of Tanks Blitz |

Try a preview:

```console
wg-launcher list --region eu
wg-launcher inspect --game wot --region eu --client-type hd --language FR
wg-launcher inspect --game wows --region ww
```

`list` defaults to every game and region. `inspect`, `download`, and `install`
require a game. When `--client-type` is omitted, the publisher default is used;
if it is missing or invalid, the smallest installable variant is selected. Game
and region filters ignore case.

## Create a custom catalog

Save a JSON array as `products.json`. This example contains EU and a custom `KR`
region. Replace the URLs and application IDs with those of your running services:

```json
[
  {
    "game": "local",
    "region": "eu",
    "name": "Local EU game",
    "host": "http://localhost:8080/updates",
    "game_id": "LOCAL.EU",
    "channel": "testing"
  },
  {
    "game": "local",
    "region": "KR",
    "name": "Local KR game",
    "host": "http://localhost:8081/updates",
    "game_id": "LOCAL.KR",
    "channel": "testing"
  }
]
```

The addresses above are examples. Supplying a catalog does not start a server;
they must point to services that already speak the WGUS-compatible protocol.
To host one from ordinary release files, use the
[`wg-update-service` guide](update-service.md).

<details>
<summary>What goes in each field?</summary>

| Field | What to enter | Example |
| --- | --- | --- |
| `game` | Stable game label selected with `--game` | `local` |
| `region` | Region label selected with `--region` | `KR` |
| `name` | A readable product name | `Local KR game` |
| `host` | Complete HTTP(S) service base URL | `http://localhost:8081/updates` |
| `game_id` | Application ID recognized by that service | `LOCAL.KR` |
| `channel` | Release channel used in reports and directory names | `testing` |

New catalogs should provide all six fields. For compatibility, files created
before multi-game support default a missing `game` field to `wot`. A URL may
include a port and path prefix; a trailing
slash is optional. Credentials, query strings, fragments, and empty identities
are rejected. For the URL above, the client queries
`/updates/api/v1/metadata/` and `/updates/api/v1/patches_chain/`. The service
publishes XML metadata, installation chains, and HTTP package seeds.

</details>

## Select a game and region

List everything in the file, or narrow the results:

```console
wg-launcher list --product products.json
wg-launcher list --product products.json --game local --region KR
```

Inspect before downloading, then install the selected product:

```console
wg-launcher inspect --product products.json --game local --region KR
wg-launcher install --product products.json --game local --region KR --output ./game-kr
```

The file replaces the built-in catalog for that command. Matching ignores case,
so `--region kr` also selects `KR`. Use an array even for a one-product file.

<details>
<summary>How do defaults and validation work?</summary>

- With no `--product`, the command uses the built-in catalogs.
- `list` with a product file queries all entries unless `--game` or `--region` filters it.
- `inspect`, `download`, and `install` require an explicit game.
- Built-in region defaults are `eu` for WoT and `ww` for the other games.
- An empty file array, duplicate game/region pairs, or a missing selection is an
  error. Different games may use the same region label.
- An unavailable custom region does not select a built-in product instead.

If your catalog contains only `KR`, select it explicitly with `--region KR`.

</details>

## Check and update without the catalog file

After installation, the selected product is saved with the game:

```console
wg-launcher status --path ./game-kr
wg-launcher update --path ./game-kr
```

Every installation saves exactly one complete product, including its service URL.
It never saves the whole catalog. You can remove the input file or move the game
folder and continue using status/update.

To resume `install` or `download`, provide the same selected product and output
directory. Reordering the catalog or editing another region does not affect
cached packages. Changing the selected product requires another destination.

## Use the catalog from Rust

Set `ListOptions.products` or `BuildSelection.products` to `Some(Vec<Product>)`.
Use string game and region labels to filter or select an entry; use `None` for
the product vector to select the built-in catalog.

The [complete Rust example](../crates/wg-update-client/README.md#use-your-own-catalog)
creates EU and KR products, lists their builds, and inspects KR. Its
`BuildSelection` also works with `InstallOptions` and `DownloadAssetsOptions`.

**Next:** [CLI walkthrough](../crates/wg-launcher-cli/README.md) · [Rust example](../crates/wg-update-client/README.md#use-your-own-catalog) · [Troubleshooting](troubleshooting.md)
