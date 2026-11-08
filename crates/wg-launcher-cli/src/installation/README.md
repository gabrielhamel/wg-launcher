# Installation in the launcher

This part of the launcher helps people download, install, and update a game. It
shows what was selected, where the game belongs, how work is progressing, and
what completed or needs attention.

It uses the [shared installation language](../../../wg-update-client/src/installation/README.md).
Its responsibility is to make that work understandable to the person using the
launcher.

## Shared language

| Term | Meaning here |
| --- | --- |
| Destination | The installation folder where the game will be prepared or updated. |
| Plan | A summary of the chosen build, destination, and packages before work begins. |
| Download | Save game packages locally for use now or later. |
| Part filter | Choose which components to download. Preparing the complete game still needs all required packages. |
| Extract | Prepare the game from packages already downloaded, without needing the publisher to be reachable. |
| Install | Download and prepare the chosen game in one operation. |
| Status | Show the installed version and whether an update is available. |
| Update | Bring the game in the existing installation folder up to date. |
| Progress | Information about ongoing downloads and the preparation of game files. |
| Installation progress | One shared terminal surface coordinating sibling download and extraction displays through the full operation. |
| Progress state | Shared, synchronized access to an optional terminal display, including initialization warnings when a display cannot be created. |
| Progress observer | A presentation adapter that receives workflow events and keeps the same progress display through success or failure. |
| Download accounting | The package positions, activity transitions, and completion facts used to update the terminal display. It contains no terminal widgets. |
| Download activity | Whether a package is queued, transferring, or complete; terminal bars display this state. |
| Cache hit | A package already present locally that can be reused without downloading it again. |
| Summary | The result of completed work, including downloads, reused packages, and cleanup. |

Command handlers live in `presentation/commands.rs`. They translate arguments,
call the client library, and render results; the library owns the workflow rules.
`presentation/observers.rs` owns separate download, extraction, and installation
adapters. Each adapter shares its progress state with the event callback and
handles success or failure. `presentation/progress_state.rs` owns shared optional state, synchronized access, and initialization warnings; workflow adapters retain event mapping and finish behavior.

`presentation/progress/` keeps download and extraction rendering separate while
sharing one terminal display. `progress/installation.rs` owns that surface and
coordinates both renderers; extraction does not depend on a download display.
`progress/download_accounting.rs` owns package positions and queued, transferring,
or completed activity. It translates download events into facts for the renderer,
including byte increments and activity transitions. The existing positive-increment
accounting is preserved when a retry resets a package position.
`progress/download.rs` owns stable bars and styles; it reads completion and activity
from the accounting model. Both remain protected by the display's shared lock so
concurrent events update the same package entry consistently.
