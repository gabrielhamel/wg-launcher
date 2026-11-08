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
| Patch chain normalization | Read publisher steps in order, infer omitted sources from the known component versions, and advance those versions to each step's target. |
| Publisher | The organisation providing a game and its updates, such as Wargaming or Lesta. |
| Application ID | The publisher identifier used in service requests, such as `WOT.EU.PRODUCTION`; distinct from the game label `wot`. |
| Service base URL | The configured HTTP(S) address of a product, including its optional port and path prefix. |
| Resolved hostname | The hostname reached after publisher redirects; it does not replace the configured product identity. |
| Game | The game being selected, such as World of Tanks or World of Warships. |
| Product | A publisher's offering for one game and region, with its own release channel. |
| Catalog | The collection of products available to choose from. |
| Region | The publisher's label for an offering, such as Europe or worldwide. |
| Channel | A stream of releases, such as the regular game or a public test. |
| Component release | A view associating one selected component with its target, published patches, and transport volumes. Planning requires a target; completion checks whether that target was reached. |
| Resolved release | The selected published build, its component targets, and the packages the publisher offers. It contains no local installation progress. |
| Build selection request | A discovery request that can leave edition or language to publisher defaults and optionally supply a replacement catalog. |
| Build identity | The published application, region, channel, version, edition, language, and payload target describing a build. It stays distinct from the configured product endpoint. |
| Build | A particular published version of a game for a chosen edition and language. |
| Client type | The game's edition or quality variant, such as SD or HD. |
| Language | The language chosen for the game content. |
| Component / part | A named portion of the game, such as its main content or a language pack. Both words refer to the same thing. |
| Patch | One step that brings a component from a starting version to a target version. It may provide the full component or changes to an existing one. |
| Full-install patch | A patch providing a complete component, without requiring an existing component version. WGUS represents its starting version as `0`. It can also replace a component during an update. |
| Delta patch | A patch changing an existing component from one version to another. |
| Patch source | What a patch starts from: a full component needs no earlier version; a delta needs a specific version. An unspecified saved source cannot establish a chain. |
| Planned patch | One step selected for installation, kept together with the packages needed to apply it. |
| Patch chain | The ordered steps needed to reach a component's target version. |
| Component versions | The known version of each named component. A component with no recorded version starts at `0`; installation separately owns checkpoint precedence. |
| Patch volume | A downloadable archive piece declared by a patch, before its HTTP address is resolved. |
| Transport volume | One downloadable file, with a resolved HTTP URL. Several volumes can form one patch archive; these are distinct from game-owned assets. |
| Package | A downloadable piece of a patch. One patch can require several packages. |
| Published volume bytes | The compressed size of all volumes offered in a resolved release, including unused patch alternatives. |
| Required volumes | The volumes needed for the patch chain selected from starting component versions; cached bytes are not subtracted. |
| Payload platform | The kind of computer the game content is intended for. It can differ from the computer running the launcher. |

## Responsibilities

| Module | Owns |
| --- | --- |
| `domain/product.rs`, `builtin.rs`, and selection modules | Product validation, built-in offerings, and selection rules. |
| `domain/build_identity.rs` | Published application, version, edition, language, and payload target. |
| `domain/release/resolved_release.rs` | Published build data, separate from local progress. |
| `domain/release/component_release.rs` | Component targets, associated patch steps and transport volumes, and target-reached checks. |
| `domain/release/patch_chain.rs` | Version traversal, cycle detection, and full-package fallback. |
| `domain/release/package_plan.rs` | Resolve each component’s selected steps to transport volumes using its volume index. |
| `application/` | Listing, inspection, release resolution, consumer reports, and suggested directory names. |
| `infrastructure/wgus/` | HTTP/XML, query encoding, redirects, language normalization, and response conversion. Patch conversion names absent-chain handling, seed parsing, transport URL resolution, torrent parsing, and component target collection separately. |

## Invariants

- A resolved release contains publisher data; installation owns checkpoints and
  pending-work precedence. Installation calls catalog entry points without
  constructing a WGUS client.
- Component views visit selections in publisher order. Planning rejects missing
  targets and missing volumes for selected steps, while leaving unused publisher
  alternatives alone. The saved release keeps its flat collections.
- Explicit client variants take precedence. Otherwise resolution chooses a valid
  publisher default, then the smallest installable variant. Equal sizes retain
  publisher order; total failure retains the first resolution or planning error.
- Redirects preserve the configured product. Internal `application_id` and
  `resolved_hostname` names distinguish publisher identity from game labels and
  service addresses. Public fields and saved keys remain `game_id` and `host`.
- WGUS chain normalization preserves explicit sources and infers omitted sources
  from the last known component version, initially `0` when unknown. The final
  versions become component targets.
- Published volume bytes include all offered volumes. Required volumes describe
  the selected chain from supplied starting versions; neither subtracts cached bytes.
- Consumer reports expose build and patch information without exposing protocol,
  transport execution, or persistence models.

Internal release and patch models use component identifiers. Protocol translation,
consumer reports, and persistence retain the established `part` and `parts` names.
