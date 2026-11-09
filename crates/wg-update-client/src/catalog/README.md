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
| Component release | A validated view associating one selected component with its required target and published patches. Planning and completion use the same target. |
| Patch transitions | Published patch steps grouped by their typed source: a full component or a specific starting version. A required step must have exactly one successor; unused alternatives do not affect planning. |
| Volume binding | Match a selected archive declaration to one download source by filename, component, source version, and target version, then confirm its compressed and optional archive sizes agree. Ambiguous sources are rejected. |
| Planned volume | One selected transport volume whose identity and archive sizes have been checked against its patch declaration. |
| Release contents | The selected components, their target versions, published patches, and download sources kept together as one immutable set. Planning checks the chain actually needed. |
| Resolved release | The selected published build, its component targets, and the packages the publisher offers. It contains no local installation progress. |
| Discovery endpoint | The current HTTP(S) service address used for discovery. Redirects change this address without changing the configured product. |
| Patch-chain request context | The publisher application ID, metadata revision, and original language spellings needed to encode requests after XML translation. |
| Discovered product | One configured product together with its current discovery endpoint, normalized metadata, and patch-chain request context, reused when choosing and resolving variants. A publisher redirect discovers fresh metadata at the redirected endpoint while retaining the configured product identity. |
| Selected product request | A validated product choice, with edition and language still allowed to use publisher defaults. Discovery and cache matching use this same choice. |
| Build selection request | A discovery request that can leave edition or language to publisher defaults and optionally supply a replacement catalog. |
| Build identity | The published application, region, channel, version, edition, language, and payload target describing a build. It stays distinct from the configured product endpoint. |
| Build | A particular published version of a game for a chosen edition and language. |
| Client variant / client type | The game's edition or quality variant, such as SD or HD. |
| Language | The language chosen for the game content. |
| Component definition | A component advertised by a client variant, including its publisher flags. |
| Component / part | A named portion of the game, such as its main content or a language pack. Both words refer to the same thing. |
| Patch identity | The component, source, and target that identify one published transition. Planning uses it to match volumes; installation reuses it to retain the correct recovery sources. |
| Normalized patch chain | Publisher patches with explicit starting versions and selected component targets, before their HTTP download sources are resolved. |
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
| `domain/product.rs`, `builtin.rs`, and selection modules | Product validation, built-in offerings, and resolving a build selection request to a selected product request. |
| `domain/build_identity.rs` | Published application, version, edition, language, and payload target. |
| `domain/release/patch_identity.rs` | Shared transition identity for patch/volume matching and installation recovery. |
| `domain/release/resolved_release.rs` | Published build data and immutable release contents, separate from local progress. |
| `domain/release/component_release.rs` | Component-facing target validation, completion checks, and ordered patch selection; delegates chain traversal to a focused helper. |
| `domain/release/patch_chain.rs` | Version traversal, required-transition ambiguity rejection, cycle detection, and full-package fallback. |
| `domain/release/package_plan.rs`, `volume_index.rs` | Assemble selected patch plans and bind their declarations through one release-wide volume index. |
| `application/` | Listing, release resolution, and consumer build/inspection reports. Installation composes the inspect workflow and supplies its suggested folder. |
| `infrastructure/wgus/` | `discovery.rs` owns discovered metadata and redirect-aware variant resolution; `mod.rs` owns HTTP client setup and metadata transport. `endpoint.rs` owns transient discovery addresses and redirects; `request.rs` owns the protocol request context and query encoding. HTTP/XML, language normalization, and response conversion. `patch_chain_normalization.rs` returns patches and targets without resolving download sources; `patch_chain_conversion.rs` parses web seeds and binds normalized patches to HTTP volumes. |

## Invariants

- A resolved release contains publisher data; installation owns checkpoints and
  pending-work precedence. Installation calls catalog entry points without
  constructing a WGUS client.
- Component views visit selections in publisher order. Planning rejects missing
  targets and missing volumes for selected steps, while leaving unused publisher
  alternatives alone. Each planned volume is one transport description validated
  against its archive declaration; local cache binding does not depend on parallel
  lists. Planned patch bindings cannot be independently changed by callers. Release contents can only be replaced together; consumers receive read-only views. The saved release keeps its flat collections. Duplicate names cannot
  silently overwrite another source.
- Discovery reuses one metadata response per product across variant selection and
  resolution. Redirect hops fetch their own metadata; there is no persistent or
  global metadata cache.
- Explicit client variants take precedence. Otherwise resolution chooses a valid
  publisher default, then the smallest installable variant. Equal sizes retain
  publisher order; total failure retains the first resolution or planning error.
- Inspect, download, and install select and validate the product once before cache
  matching or discovery. Edition and language defaults remain unresolved until needed.
- Redirects preserve the configured product. Internal `application_id` and
  `resolved_hostname` names distinguish publisher identity from game labels and
  service addresses. Public fields and saved keys remain `game_id` and `host`.
- Patch identity is a borrowed component/source/target view shared by patches and transport volumes. It does not change their serialized fields or installation recovery encoding.
- WGUS chain normalization preserves explicit sources and infers omitted sources
  from the last known component version, initially `0` when unknown. The final
  versions become component targets.
- Published volume bytes include all offered volumes. Required volumes describe
  the selected chain from supplied starting versions; neither subtracts cached bytes.
- Selection errors retain available identifiers in publisher order; display formatting joins them only when rendering a diagnostic.
- Errors classify their failures beside the owning behavior; the crate root defines
  the shared public categories and preserves diagnostic sources.
- Consumer reports expose build and patch information without exposing protocol,
  transport execution, or persistence models.

Internal release and patch models use component identifiers. Protocol translation,
consumer reports, and persistence retain the established `part` and `parts` names.
