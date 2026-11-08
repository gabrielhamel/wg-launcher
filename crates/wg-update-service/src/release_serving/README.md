# Release serving

Release serving makes a prepared game release available to launchers. It tells
them which game content is offered and provides the packages they need to
download.

It answers: “What release is available here, and where can its packages be
obtained?” Preparing a release happens before it is served; installing that
release on someone's machine belongs to the launcher.

## Shared language

| Term | Meaning here |
| --- | --- |
| Publisher | The person or organisation making game content available. |
| Release | A prepared offering of game content and the information describing how to obtain it. |
| Release resource | The metadata, patch chain, or relative package path requested, before checking whether its content exists. |
| Release content | The prepared metadata, patch chain, or package file selected for a request. |
| Release handler | The HTTP adapter that maps a request to release metadata, patch-chain XML, or a safely resolved package response. |
| HTTP response description | The bytes or file to serve and its content type, or a missing-resource result. It does not expose game-release roles to the HTTP adapter. |
| Content type | Whether a served file contains XML, JSON, or binary data. This describes its representation, rather than its role in a game patch. |
| Release folder | The local collection of release descriptions and packages that the service makes available. |
| Product | The game offering a launcher asks about, identified by its publisher and application identity. |
| Application identity | The publisher's name for the game offering being requested. |
| Metadata | The description of an offering, including its client types, languages, and components. |
| Client type | An edition or quality variant of the game, such as SD or HD. |
| Patch chain | The steps offered to bring a game component to its target version. |
| Package | A downloadable piece of a patch. |
| Route prefix | The validated optional path before the service API routes. |
| Public origin | The validated HTTP(S) scheme, hostname, and optional port used in published package URLs. It contains no path or credentials. |
| Service address | The location launchers use to discover the release and request its packages. |
| Resumed download | A download that asks for the remaining portion of a package instead of starting over. |
| Response range | The complete body or remaining bytes selected for a response, including its offset and length. |
| Access log | A record of requests received by the service, useful for understanding what launchers asked for. |

The application prepares release metadata and starts an HTTP release handler.
`infrastructure/http/routing.rs` interprets a path as a `ReleaseResource`
without filesystem access or content-type classification.
`infrastructure/http/release.rs` owns `ReleaseHandler`: it routes requests,
supplies prepared XML or asks the filesystem adapter to resolve a package, and
builds HTTP response descriptions. The filesystem adapter enforces containment
inside the package folder without constructing HTTP responses.

The HTTP adapter lives in `infrastructure/http/`: `server.rs` owns connections and
shutdown, `request.rs` parses incoming messages, and `response.rs` resolves response
ranges and streams bodies. `representation.rs` owns response descriptions and
XML/JSON/binary content types. `mod.rs` serves those descriptions without knowing
whether the requested resource was metadata, a patch chain, or a package.
Those release-specific HTTP decisions stay in `release.rs` and `routing.rs`.

`domain/address.rs` owns validated route prefixes and public origins, API route
construction, and product-host composition. Startup converts string options into
these values before rendering metadata or configuring release routes. Domain
validation describes public-origin constraints without CLI option names; CLI
presentation adds `--public-url` context to the typed origin error at startup.
