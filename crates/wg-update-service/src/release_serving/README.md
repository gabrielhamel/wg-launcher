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
| Release folder | The local collection of release descriptions and packages that the service makes available. |
| Product | The game offering a launcher asks about, identified by its publisher and application identity. |
| Application identity | The publisher's name for the game offering being requested. |
| Metadata | The description of an offering, including its client types, languages, and components. |
| Client type | An edition or quality variant of the game, such as SD or HD. |
| Patch chain | The steps offered to bring a game component to its target version. |
| Package | A downloadable piece of a patch. |
| Service address | The location launchers use to discover the release and request its packages. |
| Resumed download | A download that asks for the remaining portion of a package instead of starting over. |
| Access log | A record of requests received by the service, useful for understanding what launchers asked for. |

The application release routes select metadata, patch chains, and package responses,
including their content types. The HTTP adapter parses requests and handles ranges,
headers, and streaming.
The filesystem adapter resolves relative package paths and enforces containment
inside the package folder, without constructing HTTP responses.
