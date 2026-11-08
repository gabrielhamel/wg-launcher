# Catalog

The catalog describes the game builds a publisher makes available. It helps
people discover their choices and understand what a selected build would require
before downloading it.

It answers: “Which game can I get, in which edition and language, and how much
is there to download?” [Installation](../installation/README.md) uses that
selection to prepare the game on someone's machine.

## Shared language

| Term | Meaning here |
| --- | --- |
| Publisher | The organisation providing a game and its updates, such as Wargaming or Lesta. |
| Game | The game being selected, such as World of Tanks or World of Warships. |
| Product | A publisher's offering for one game and region, with its own release channel. |
| Catalog | The collection of products available to choose from. |
| Region | The publisher's label for an offering, such as Europe or worldwide. |
| Channel | A stream of releases, such as the regular game or a public test. |
| Component release | One selected component together with its target version and the published patches belonging to it. |
| Resolved release | The selected published build, its component targets, and the packages the publisher offers. It contains no local installation progress. |
| Build selection request | A discovery request that can leave edition or language to publisher defaults and optionally supply a replacement catalog. |
| Build identity | The published application, region, channel, version, edition, language, and payload target describing a build. It stays distinct from the configured product endpoint. |
| Build | A particular published version of a game for a chosen edition and language. |
| Client type | The game's edition or quality variant, such as SD or HD. |
| Language | The language chosen for the game content. |
| Component / part | A named portion of the game, such as its main content or a language pack. Both words refer to the same thing. |
| Patch | One step that brings a component from a starting version to a target version. It may provide the full component or changes to an existing one. |
| Full-install patch | A patch providing a complete component, without requiring an existing component version. WGUS represents its starting version as `0`. |
| Delta patch | A patch changing an existing component from one version to another. |
| Patch source | What a patch starts from: a full component needs no earlier version; a delta needs a specific version. An unspecified saved source cannot establish a chain. |
| Planned patch | One step selected for installation, kept together with the packages needed to apply it. |
| Patch chain | The ordered steps needed to reach a component's target version. |
| Package | A downloadable piece of a patch. One patch can require several packages. |
| Download size | The size of the published packages, before allowing for anything already downloaded. |
| Payload platform | The kind of computer the game content is intended for. It can differ from the computer running the launcher. |

The catalog application resolves update releases from a product, client type,
language, and known component versions. Installation owns local progress and
pending-manifest precedence. WGUS request encoding and redirect targets remain
inside the protocol adapter; redirects preserve the configured product identity.
`conversion.rs` assembles release identity and metadata; `patch_chain_conversion.rs`
normalizes source versions and resolves packages, seeds, and component targets.

Build identity is shared internally by resolved releases and installed records.
Consumer reports project that identity without exposing persistence models. Patch
chain selection visits validated component releases in publisher order, keeping each
component target and its patches together with a shared package lookup, without changing saved release data.
Each component resolves its selected steps to complete planned patches and rejects
missing packages before execution.
It owns version traversal and full-package fallback; package planning resolves
the transport volumes for those selected steps.
