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
| Release content | The metadata, patch chain, or file selected for a request, or the fact that none exists. HTTP headers are chosen separately. |
| Content kind | Whether a served file contains XML, JSON, or binary data. This describes its representation, rather than its role in a game patch. |
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

The application separates route interpretation from content lookup.
`application/routing.rs` interprets a path as a `ReleaseResource` without filesystem
access. `application/content.rs` supplies the prepared XML or asks the filesystem
adapter to resolve a package, producing `ReleaseContent` without HTTP request or
response models. The HTTP adapter parses requests, translates the selected content
into MIME types and status codes, and handles ranges, headers, and streaming.
The filesystem adapter resolves relative package paths and enforces containment
inside the package folder, without constructing HTTP responses.

The HTTP adapter lives in `infrastructure/http/`: `server.rs` owns connections and
shutdown, `request.rs` parses incoming messages, and `response.rs` resolves response
ranges and streams bodies. `mod.rs` maps release content to those responses.

`domain/address.rs` owns validated route prefixes and public origins, API route
construction, and product-host composition. Startup converts string options into
these values before rendering metadata or configuring release routes.
