# Documentation

[Project home](../README.md) · [CLI](../crates/wg-launcher-cli/README.md) · [Rust](../crates/wg-update-client/README.md) · [API](https://docs.rs/wg-update-client/1.0.0/wg_update_client/)

Choose the path that matches what you want to build or install.

## Start here

| I want to… | Guide | What you'll learn |
| --- | --- | --- |
| Install a game from the terminal | [CLI walkthrough](../crates/wg-launcher-cli/README.md) | Preview, install, check, and update |
| Add updates to a Rust app | [Rust walkthrough](../crates/wg-update-client/README.md) | Complete programs and typed results |
| Connect my own update service | [Custom catalogs](catalogs.md) | Product arrays, multiple regions, and selection |
| Host my own update service | [Update service](update-service.md) | Release files, server CLI, routes, and deployment |
| Fix an interrupted operation | [Troubleshooting](troubleshooting.md) | Resume, complete a cache, and retry |

<details>
<summary>I'm using the terminal. What's the quickest starting point?</summary>

```console
cargo install wg-launcher-cli --version 1.0.0 --locked
wg-launcher inspect --game wot --region eu
```

This previews a build without downloading game packages.
[Continue to installation →](../crates/wg-launcher-cli/README.md#install-the-game)

</details>

<details>
<summary>I'm building an app. Which example should I open?</summary>

| Example | Purpose |
| --- | --- |
| [inspect.rs](../crates/wg-update-client/examples/inspect.rs) | Discover variants and inspect a build |
| [install.rs](../crates/wg-update-client/examples/install.rs) | Install, check status, and update |
| [progress.rs](../crates/wg-update-client/examples/progress.rs) | Receive installation events |
| [staged_install.rs](../crates/wg-update-client/examples/staged_install.rs) | Download and extract separately |
| [custom_catalog.rs](../crates/wg-update-client/examples/custom_catalog.rs) | Select a region from custom products |

From a source checkout, run an example with:

```console
cargo run -p wg-update-client --example inspect --locked
```

The installation examples download a game into the path shown in their source.
The custom-catalog example needs your services running at its configured URLs.

[Read the Rust walkthrough →](../crates/wg-update-client/README.md)

</details>

## Look something up

- [Command overview](../crates/wg-launcher-cli/README.md#command-overview)
- [Built-in regions and custom product fields](catalogs.md)
- [File-backed update service](update-service.md)
- [Installation behavior and low-level APIs](reference.md)
- [Generated Rust API reference](https://docs.rs/wg-update-client/1.0.0/wg_update_client/)

## Work on the project

[Build and validate](contributing.md) · [E2E tests and fixtures](testing.md) · [Agent guide](../AGENTS.md)
