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
| Resolved selection | The known product, client type, and language of an installation, with no unanswered discovery choices. |
| Installed build | The game version recorded after all required installation work succeeds. |
| Component progress | The versions reached locally, starting with installed versions and advancing after successful patches. Saved progress takes precedence when retrying; it remains stored as `base_versions` in the pinned manifest. |
| Component version | The recorded version of one part of the game. Different parts can change independently. Starting versions seed missing checkpoints; completed patch checkpoints take precedence. |
| Download | Bring packages onto the machine, without yet applying them to the game. |
| Download cache | Packages kept locally so they can be used or reused for installation. |
| Pinned manifest | The resolved release saved for local work, together with completed component checkpoints. Its saved JSON keeps release details and checkpoints in the same record. |
| Pinned selection | The saved choice of build and packages for unfinished work. A retry continues that choice even if the publisher releases something newer. |
| Extraction | Use downloaded packages to put game content in place and apply any changes they contain. |
| Install | Download and prepare the selected game in one operation. |
| Status | A comparison of the recorded installation with what the publisher currently offers. |
| Update | Bring an existing installation to the required build while keeping its folder and selection. |
| Update target | The build chosen for comparison with the installed game. Its source is either the live catalog or the pinned manifest for unfinished work. A pinned target need not be the publisher's latest release. |
| Installation pipeline | Coordinates package downloads and patch application, including the lock, saved progress, and final cleanup. |
| Delta | Changes that need an existing game file, rather than another complete copy of that file. |
| Patch workspace | Temporary files and preserved originals for the patch being applied. They remain available after failure until that patch is saved as complete. |
| Patch contents | The files decoded from a patch: complete replacement files and delta files that change existing content. Replacements are installed before deltas are applied. |
| Patch completion | Apply a patch successfully, save its checkpoint, then release its recovery files and, when cleanup is enabled, its downloaded packages. |
| Checkpoint | Saved progress after a patch succeeds, so a retry can skip completed work. |
| Resume / retry | Continue unfinished work, reusing completed steps and downloads where possible. |
| Cleanup | Remove packages and temporary work after success, while retaining the installed game's identity. |

Saved installed identity is checked before status or update discovery. An invalid
product or a region that differs from its saved product is reported as invalid
installation data, including when unfinished work is pinned locally.
