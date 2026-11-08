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
| Completed volume | A package brought to its final cache filename, including a complete partial file promoted without a network request. |
| Reused volume | A package already present at its final filename with the expected size. |
| New bytes retained | Bytes added by successful transfer attempts, excluding resumed prefixes and traffic from failed attempts. |
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
| Resolved update target | The chosen release and local component progress kept together with whether they came from live discovery or unfinished work. It supplies the status comparison and then hands its pinned manifest to execution. |
| Update target | The build chosen for comparison with the installed game. Its source is either the live catalog or the pinned manifest for unfinished work. A pinned target need not be the publisher's latest release. |
| Prefetch budget | The limit on packages queued ahead of the patch currently being applied. A new patch may start below the limit even if finishing it exceeds the limit. |
| Finalization | Record the completed build and remove temporary work and, when requested, cached packages after all patches succeed. |
| Locked installation | Exclusive access to one installation folder. Download adopts compatible pinned work; execution resumes only an exactly matching release and restores its checkpoints. |
| Installation session | One execution owning exclusive installation access, its layout, saved progress, and completed delta count. Its lock stays with blocking workers until they finish. |
| Package retention | Whether successfully applied packages are released after each saved checkpoint and removed at final cleanup, or kept for reuse. |
| Installation completion | Confirm every selected component has reached its target before creating the installed build identity. |
| Execution plan | The remaining ordered patches and their resolved volumes, selected after the session restores checkpoints under its lock. The pipeline owns it until execution consumes the pipeline and receives the remaining patches. Its summary supplies the public installation plan. |
| Installation pipeline | Coordinates package downloads and patch application, including the lock, saved progress, and final cleanup. |
| Delta filename | A publisher filename whose extension, version segment, and hexadecimal checksum segment identify a delta and its target game package. Recognition checks naming syntax, not checksum correctness; unrecognized names remain replacement files. |
| Delta | Changes that need an existing game file, rather than another complete copy of that file. |
| Patch identity | The catalog-owned component, source, and target identifying a published transition. Installation reuses it to preserve the right original files for retry. |
| Patch workspace | Temporary files and preserved originals for the patch being applied. They remain available after failure until that patch is saved as complete. |
| Patch contents | The files decoded from a patch: complete replacement files and delta files that change existing content. Replacements are installed before deltas are applied. |
| Local patch | A planned patch bound to safe package paths in the installation cache. Each planned volume supplies a download source already checked against its archive declaration. The same paths are used for verification, archive reading, and package release; binding paths alone does not verify their contents. |
| Patch completion | Apply a patch successfully, save its checkpoint, then release its recovery files and, when cleanup is enabled, its downloaded packages. |
| Selection mismatch | The product, region, edition, or language fields that make cached work incompatible with a request. Omitted edition and language accept the pinned choice. |
| Release differences | The published fields that differ between requested and saved work. The same comparison decides whether work can resume and explains why it cannot. |
| Pinned release mismatch | Saved work and a requested release differ in their build identity, product, component targets, patch chains, or other published details. The error names the differing fields; matching selection alone does not make the releases interchangeable. |
| Checkpoint | Saved progress after a patch succeeds, so a retry can skip completed work. |
| Resume / retry | Continue unfinished work, reusing completed steps and downloads where possible. |
| Cleanup | Remove packages and temporary work after success, while retaining the installed game's identity. |

## Responsibilities

| Module | Owns |
| --- | --- |
| Catalog `domain/release/patch_identity.rs` | Shared component transition identity; installation owns only its recovery-file encoding. |
| `domain/installed_build.rs` | Installed identity, selection consistency, and comparison with a target build. |
| `domain/pinned_manifest.rs`, `component_progress.rs` | Pinned release, starting versions, saved checkpoints, and checked completion. |
| `domain/selection.rs`, `release_compatibility.rs` | Selection compatibility and diagnostics beside their types; release comparison returns typed differences for resume decisions and diagnostics. |
| `application/` | Public workflows, options, reports, and observer contracts. |
| `application/resolution.rs`, `update_target.rs` | Pinned-cache reuse and resolved update targets owning pending precedence, live fallback, starting-version seeding, and status projection. |
| `application/installed.rs`, `status_report.rs`, `plan.rs` | Validated state loading and consumer status/plan projections. |
| `application/access.rs` | Locked installation access, adopting matching downloads, exact-release resume, and record writes requiring the lock owner. |
| `application/pipeline/session.rs` | Execution state holding locked access, authoritative checkpoints, and execution totals. |
| `application/pipeline/mod.rs`, `execution_plan.rs` | Resolve remaining work under the session lock, own its consumer summary, and hand patches to execution once. |
| `application/pipeline/extraction.rs`, `streaming.rs` | Local execution or overlapping downloads and patch application. |
| `application/pipeline/patch.rs`, `finalization.rs` | Coordinate package execution, checkpoint/release ordering, installed-state writes, and successful cleanup. |
| `application/pipeline/prefetch.rs`, `prefetch_budget.rs` | Download coordination, cancellation, and queued-byte policy. |
| `application/transfer.rs` | Public transfer options, observer events, and reports, shared by workflows and implemented by infrastructure. |
| `infrastructure/transfer/` | `downloader.rs` owns concurrency, retries, and totals; `local.rs` and `attempt.rs` own cache preparation and HTTP attempts. |
| `infrastructure/packages/` | Split-volume reading, archive decoding, replacement/delta classification, and application. |
| `infrastructure/filesystem/` | Layout, locks, persistence through consistent layout-based record operations, safe paths, local patch bindings in `cache.rs`, patch workspaces, and replacement. `cleanup.rs` owns file counting and successful staging/cache removal; application finalization owns their order. |

## Invariants

- All installation-owned files live beneath `.wg-launcher/`. Moving the complete
  folder preserves installed identity and resumable work. The folder name retains
  its initial build identity after updates.
- Status and update validate installed identity before discovery. Status compares
  with the live catalog; update prefers a pending manifest and retains its selection.
- A session acquires the mutation lock and restores authoritative saved checkpoints.
  The pipeline then resolves and owns the execution plan under that lock. Install
  and extract publish the summary of that same plan before progress. Package-only downloads retain their component filter
  and pin the complete manifest.
- Starting versions seed missing checkpoints; saved completions take precedence.
  A validated component view requires a target before planning or installation completion.
  Completion requires every selected component to reach its target before state
  is written.
- Streaming overlaps later downloads with ordered patch execution. The prefetch
  budget limits bytes queued ahead of the applying patch; one complete patch may
  cross the threshold. Download and extraction use the same resolved steps.
- Planned and local patches keep their patch descriptions and bound volumes private, exposing read-only views or consuming access to volumes.
- Planned patches bind their validated transport volumes to safe local paths once. Staged
  extraction verifies declared patch sizes; archive reading retains filename ordering
  for split volumes. Execution and package release consume those same paths.
- Application record writes go through locked installation access. Patch execution and finalization require that lock owner, rather than paths alone.
- Each patch is applied, checkpointed, then its recovery files and eligible packages
  are released. Failures retain uncheckpointed packages and original delta sources.
  Package retention controls both early release and final cache cleanup.
- Full content is decoded before each live file replacement. Delta patches install
  replacements before applying source-dependent changes. Recognized delta filenames
  select an encoding and game-package target; their checksum segments are syntax,
  not verified identity. Inner game-owned `.pkg` files remain packed.
- Blocking workers retain the session lock if their awaiting future is dropped.
  Extraction failure cancels pending downloads before exclusive access is released.
  Background worker failures retain their technical causes. Prefetch readiness
  announces the applying position before awaiting its packages; cancellation waits for the
  producer to stop before the lock is released.
- Finalization validates completion, counts files, writes installed identity, and
  removes temporary work and eligible cache data. `files_before_finalization`
  includes existing game files and launcher metadata, not just newly extracted files.
- Persistence retains the existing JSON layout: the release is flat, component
  progress uses `base_versions`, and active patch identity is an array. Records
  load/save values; domain and application code own compatibility and resume rules.

Package execution explicitly distinguishes full, versioned, and unspecified patch sources, preserving decoded-file classification for unspecified sources. It owns full-component versus delta strategies through
`infrastructure/packages/patch_application.rs`. The pipeline calls that entry point
before persisting checkpoints and releasing recovery sources and packages.
