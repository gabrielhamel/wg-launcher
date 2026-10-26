# wg-launcher CLI

Install and update game assets with a few terminal commands.
The CLI works on Windows, Linux, and macOS; supported game payloads target
Windows/x64. Player login and launching the game are outside its scope.

[Documentation home](../../docs/README.md) · [Rust guide](../wg-update-client/README.md) · [Custom catalogs](../../docs/catalogs.md) · [Help with errors](../../docs/troubleshooting.md)

**On this page:** [Install the CLI](#install-the-cli) · [Choose a build](#choose-your-build) · [Install the game](#install-the-game) · [Update](#check-and-apply-updates) · [Offline installation](#download-now-install-later) · [Commands](#command-overview)

## Install the CLI

You'll need Rust and Cargo. Install the terminal application with:

```console
cargo install wg-launcher-cli --version 1.0.0 --locked
wg-launcher --help
```

No additional archive or patching utilities are required.
For a checkout build, follow the [source installation guide](../../docs/contributing.md#build-from-source).

## Choose your build

First, compare builds. Then inspect a game and region before downloading:

```console
wg-launcher list --region eu
wg-launcher inspect --game wot --region eu
wg-launcher inspect --game wows --region ww
```

`list` shows versions, download sizes, and languages. A `*` marks the default
language. `inspect` shows the selected components and patch sizes without changing
files or downloading game packages.

<details>
<summary>Show HD, language, and region examples</summary>

Choose HD with French assets:

```console
wg-launcher inspect --game wot --region eu --client-type hd --language FR
```

Compare Lesta RU builds:

```console
wg-launcher list --region ru
```

Query every built-in region:

```console
wg-launcher list
```

`list` queries every built-in game and region unless `--game` or `--region`
filters it. Built-in game names are `wot`, `wows`, `wowp`, and `wotb`.
World of Warships, World of Warplanes, and World of Tanks Blitz use the `ww`
catalog region. World of Tanks also provides `eu`, `na`, `asia`, `cn`, `ct`,
`ru`, and `pt-ru`. Custom labels work with a
[custom catalog](../../docs/catalogs.md).

</details>

## Install the game

Choose a destination and start the download and installation:

```console
wg-launcher install --game wot --region eu --output ./world-of-tanks
```

The command performs the operation directly and shows progress. `--game` is
required. With `--game wot`, the region defaults to EU. When `--client-type` is
omitted, the publisher's valid default is used. If no usable default is advertised,
the launcher resolves the available variants and selects the smallest compressed
installation. An explicit `--client-type` always takes precedence.

<details>
<summary>Customize the installation</summary>

Install HD with French assets:

```console
wg-launcher install --game wot --region eu --client-type hd --language FR --output ./world-of-tanks
```

Install another game using its published default client variant:

```console
wg-launcher install --game wowp --region ww --output ./world-of-warplanes
```

Limit simultaneous package downloads:

```console
wg-launcher install --game wot --region eu --output ./world-of-tanks --concurrency 2
```

Download concurrency defaults to `3`. Omitting `--output` creates an automatically
named directory containing the game, region, channel, initial version, payload target,
client type, and language. Updates keep that directory name.

</details>

<details>
<summary>Watch an installation</summary>

![Installation progress](docs/assets/install.gif)

</details>

## Check and apply updates

Check the installed and available versions:

```console
wg-launcher status --path ./world-of-tanks
```

Apply an update when one is available:

```console
wg-launcher update --path ./world-of-tanks
```

Both commands use the saved product, region, client type, and language. You do not
need to supply the build options or a custom catalog again. Updates follow the
publisher's installation chain, which may include more than only changed files.

<details>
<summary>Run from inside the game directory</summary>

```console
cd world-of-tanks
wg-launcher status
wg-launcher update
```

`status` and `update` use the current directory when `--path` is omitted.

</details>

## Download now, install later

Save the packages first, then extract them locally:

```console
wg-launcher download --game wot --region eu --output ./world-of-tanks
wg-launcher extract ./world-of-tanks
```

You can move the whole folder to another machine between these steps. Extraction
uses the downloaded manifest and works without contacting the update service.
Running `wg-launcher extract` inside the destination selects the current directory.

<details>
<summary>Download selected components first</summary>

Use the component names shown by `inspect`:

```console
wg-launcher download --game wot --region eu --output ./world-of-tanks --part client --part locale
```

Extraction needs the complete package set. Download everything still needed with
the same selection and destination, then extract:

```console
wg-launcher download --game wot --region eu --output ./world-of-tanks
wg-launcher extract ./world-of-tanks
```

</details>

## Use your own update service

Point the command at a JSON product array and select a game and region:

```console
wg-launcher list --product products.json
wg-launcher install --product products.json --game local --region KR --output ./game-kr
```

The [custom catalog guide](../../docs/catalogs.md) includes a two-region example,
field explanations, and matching Rust code. Game and region matching ignore case.
Listing defaults to every configured selection. Build commands require `--game`;
the built-in region defaults to `eu` for WoT and `ww` for the other games.

## Resume or move a game

Retry an interrupted command with the same destination and selection. Complete
packages are reused, and partial files resume when the service supports ranges.
Some successfully extracted packages may need to be downloaded again on a retry.

Keep `.wg-launcher` inside the game folder when moving it. That directory contains
the saved selection and any unfinished download. Successful installation clears
its package cache and temporary work while keeping the installed identity.

If something fails, use the [troubleshooting guide](../../docs/troubleshooting.md).

## Command overview

| Command | Use it to… | Files changed? |
| --- | --- | --- |
| `list` | Compare builds, sizes, and languages | No |
| `inspect` | Preview one build and its components | No |
| `install` | Download and install together | Yes |
| `status` | Compare the installed and available versions | No |
| `update` | Update an installation in place | Yes |
| `download` | Save packages for later installation | Yes |
| `extract` | Install from a complete local cache | Yes |

Use `wg-launcher <command> --help` to explore a command's options.

**Next:** [Custom catalogs](../../docs/catalogs.md) · [Troubleshooting](../../docs/troubleshooting.md) · [Rust examples](../wg-update-client/README.md)
