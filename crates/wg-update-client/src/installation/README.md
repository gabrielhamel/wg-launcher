# Installation

Installation looks after a game folder: downloading the chosen content,
preparing it for use, checking its version, and bringing it up to date. It also
keeps enough information to continue unfinished work and reuse downloads.

It answers: “What is installed here, and what remains to get it to the chosen
version?” The [catalog](../catalog/README.md) describes what the publisher offers;
installation manages the copy on a person's machine.

## Shared language

| Term | Meaning here |
| --- | --- |
| Installation | One local copy of a game, together with the information needed to maintain it. |
| Installation folder | The folder containing that game and its saved installation information. Moving the complete folder keeps them together. |
| Selection | The chosen game, region, channel, client type, and language. |
| Installed build | The game version recorded after all required installation work succeeds. |
| Component version | The recorded version of one part of the game. Different parts can change independently. |
| Download | Bring packages onto the machine, without yet applying them to the game. |
| Download cache | Packages kept locally so they can be used or reused for installation. |
| Pinned manifest | The resolved release saved for local work, together with completed component checkpoints. Its saved JSON keeps release details and checkpoints in the same record. |
| Pinned selection | The saved choice of build and packages for unfinished work. A retry continues that choice even if the publisher releases something newer. |
| Extraction | Use downloaded packages to put game content in place and apply any changes they contain. |
| Install | Download and prepare the selected game in one operation. |
| Status | A comparison of the recorded installation with what the publisher currently offers. |
| Update | Bring an existing installation to the required build while keeping its folder and selection. |
| Delta | Changes that need an existing game file, rather than another complete copy of that file. |
| Checkpoint | Saved progress after a patch succeeds, so a retry can skip completed work. |
| Resume / retry | Continue unfinished work, reusing completed steps and downloads where possible. |
| Cleanup | Remove packages and temporary work after success, while retaining the installed game's identity. |
