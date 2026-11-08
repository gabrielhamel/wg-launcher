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
| Installation layout | The game folder and all launcher-owned paths derived from it, passed together so filesystem work uses one consistent location. |
| Installation folder | The folder containing that game and its saved installation information. Moving the complete folder keeps them together. |
| Selection | The chosen game, region, channel, client type, and language. |
| Resolved selection | The known product, client type, and language of an installation, with no unanswered discovery choices. |
| Installed build | The game version recorded after all required installation work succeeds. |
| Component progress | The versions reached locally, starting with installed versions and advancing after successful patches. Saved progress takes precedence when retrying; it remains stored as `base_versions` in the pinned manifest. |
| Component version | The recorded version of one part of the game. Different parts can change independently. Starting versions seed missing checkpoints; completed patch checkpoints take precedence. |
| Download | Bring packages onto the machine, without yet applying them to the game. |
| Download preparation | Decide whether to reuse a complete volume, promote a complete partial file, or transfer the missing bytes. |
| Transport volume | One downloaded file forming all or part of a patch archive. Split volumes are read together as one archive. |
| Download cache | Packages kept locally so they can be used or reused for installation. |
| Pinned manifest | The resolved release saved for local work, together with completed component checkpoints. Its saved JSON keeps release details and checkpoints in the same record. |
| Pinned selection | The saved choice of build and packages for unfinished work. A retry continues that choice even if the publisher releases something newer. |
| Extraction | Use downloaded packages to put game content in place and apply any changes they contain. |
| Install | Download and prepare the selected game in one operation. |
| Status | A comparison of the recorded installation with what the publisher currently offers. |
| Update | Bring an existing installation to the required build while keeping its folder and selection. |
| Build comparison | Whether the recorded build or component versions differ from the target, independent of the packages needed to reach it. |
| Update target | The build chosen for comparison with the installed game. Its source is either the live catalog or the pinned manifest for unfinished work. A pinned target need not be the publisher's latest release. |
| Prefetch budget | The limit on packages queued ahead of the patch currently being applied. A new patch may start below the limit even if finishing it exceeds the limit. |
| Finalization | Record the completed build and remove temporary work and, when requested, cached packages after all patches succeed. |
| Installation session | One execution owning exclusive installation access, its layout, saved progress, and completed delta count. Its lock stays with blocking workers until they finish. |
| Package retention | Whether successfully applied packages are released after each saved checkpoint and removed at final cleanup, or kept for reuse. |
| Installation completion | Confirm every selected component has reached its target before creating the installed build identity. |
| Installation pipeline | Coordinates package downloads and patch application, including the lock, saved progress, and final cleanup. |
| Delta filename | A publisher filename whose extension, version segment, and hexadecimal checksum segment identify a delta and its target game package. Recognition checks naming syntax, not checksum correctness; unrecognized names remain replacement files. |
| Delta | Changes that need an existing game file, rather than another complete copy of that file. |
| Patch identity | The component, source version, and target version that identify the patch whose original files are preserved for retry. |
| Patch workspace | Temporary files and preserved originals for the patch being applied. They remain available after failure until that patch is saved as complete. |
| Patch contents | The files decoded from a patch: complete replacement files and delta files that change existing content. Replacements are installed before deltas are applied. |
| Patch completion | Apply a patch successfully, save its checkpoint, then release its recovery files and, when cleanup is enabled, its downloaded packages. |
| Selection mismatch | The product, region, edition, or language fields that make cached work incompatible with a request. Omitted edition and language accept the pinned choice. |
| Pinned release mismatch | Saved work and a requested release differ in their build identity, product, component targets, patch chains, or other published details. The error names the differing fields; matching selection alone does not make the releases interchangeable. |
| Checkpoint | Saved progress after a patch succeeds, so a retry can skip completed work. |
| Resume / retry | Continue unfinished work, reusing completed steps and downloads where possible. |
| Cleanup | Remove packages and temporary work after success, while retaining the installed game's identity. |

Saved installed identity is checked before status or update discovery. An invalid
product or a region that differs from its saved product is reported as invalid
installation data, including when unfinished work is pinned locally.

Status and update share validated installed-record loading and report construction.
The installed-build domain compares versions without requiring a pinned manifest.
The application separately estimates package bytes from explicit starting component
versions and builds the public status report. Status uses installed versions;
updates use the pinned manifest's progress, including completed checkpoints. Download
preparation inspects local files before HTTP execution, keeping reuse, promotion,
and resumed transfer explicit without changing retry or cleanup behavior.

The pipeline decides how saved checkpoints resume a manifest and when a completed
manifest becomes installed identity. Filesystem records only load and save those
values. Package application owns full-patch replacement and source-dependent delta execution.
Archive decoding reports each fully decoded entry without deciding where live game
files are installed. Full content is decoded before each live file is replaced;
delta contents stay in the patch workspace until application. Storage failures
retain their technical causes rather than becoming decoder error strings. Split-volume
reading lives separately from archive decoding and replacement policy.

Shared finalization records successful work for both local extraction and streaming
installation. `pipeline/prefetch_budget.rs` owns the byte-budget rule and names the applying
and candidate patch positions. `pipeline/prefetch.rs` owns async download
coordination and cancellation.
Background worker failures retain their technical causes and are distinguished from
a download coordinator ending before every planned patch is ready. Installation
layout describes the root and derived paths without reading or creating files.
Preparation, locking, patch execution, and finalization receive that layout
instead of independent root and storage arguments.

Component progress exposes the versions reached locally, including saved patch
completions. Restoring saved checkpoints explicitly gives those values precedence.
Both asynchronous and synchronous record loaders call the same saved record a pinned
manifest; their names distinguish execution mode rather than implying different records.
Patch workspaces name the patch identity explicitly while preserving its saved array
representation. Package `patch_application.rs` owns replacement-first execution and
source preservation; `delta.rs` reconstructs and replaces an individual file.
Finalization counts files before state writes and cleanup, including pre-existing files
and launcher metadata, reported as `ExtractionReport.files_before_finalization`. The count is unchanged;
it includes launcher metadata and cached packages still present before cleanup.

`application/update_target.rs` owns pending-manifest precedence, selection validation,
live fallback, and starting-version seeding for updates. Status remains live-only.
Shared observer events live in `application/progress.rs`; pipeline reports and
execution options live in `application/pipeline/reports.rs`. Package directory
classification delegates publisher filename recognition to `packages/delta_filename.rs`.

Cache compatibility records the differing selection fields before error formatting;
diagnostics name those fields and show both selections. Component progress uses the
catalog's component-version value while retaining ownership of seeding and saved
checkpoint precedence. Its JSON representation remains a component-to-version map.
Shared update target and source types live beside target resolution; status report
construction remains in `application/comparison.rs`.

`pipeline/session.rs` carries mutable execution state through local extraction and
streaming workers. Both strategies acquire exclusive access through the session, resume saved
checkpoints, prepare storage, apply patches, and finalize through it. Streaming
moves the session between workers rather than cloning the manifest for each patch.
A shared lock guard protects pending downloads until cancellation finishes, even
if patch execution fails. Blocking workers retain exclusive access if their awaiting
future is dropped. `PackageRetention` names the existing release-or-keep policy
inside execution; `ExtractionOptions.clean_download_cache` remains unchanged.
Pinned manifests produce installed identity through `complete_installation`,
which checks all selected component targets before any state write or cleanup.
The internal `VolumeDownloader` transfers transport volumes; public workflow,
report, event, and saved JSON names are unchanged.
