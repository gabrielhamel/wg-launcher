# Catalog in the launcher

This part of the launcher helps people explore available games and choose what
they want. It presents the catalog as readable lists and build details, and
explains when a choice is unavailable.

The meaning of games, builds, and packages comes from the
[shared catalog language](../../../wg-update-client/src/catalog/README.md).
Here, the focus is helping a person make that selection.

## Shared language

| Term | Meaning here |
| --- | --- |
| List | Show the available game builds and their main differences. |
| Inspect | Show the details of one chosen build before downloading anything. |
| Game filter | Limit the displayed choices to one game. |
| Region filter | Limit the displayed choices to one publisher region. |
| Client type | The game's edition or quality variant, such as SD or HD. |
| Default choice | The choice used when the person leaves an option open. For client type, the publisher's valid preference comes first, then the smallest available installation. |
| Default language | The language the publisher selects for that region when no language is requested. |
| Custom catalog | A supplied collection of products used in place of the built-in catalog. |
| Warning | An explanation that a particular choice is unavailable, alongside any choices that could be shown. |
