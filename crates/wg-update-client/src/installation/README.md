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
| Execution plan | The remaining ordered patches and their resolved volumes, selected after the session restores checkpoints under its lock. Its summary supplies the public installation plan. |
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

## Responsibilities

| Module | Owns |
| --- | --- |
| `domain/installed_build.rs` | Installed identity, selection consistency, and comparison with a target build. |
| `domain/pinned_manifest.rs`, `component_progress.rs` | Pinned release, starting versions, saved checkpoints, and checked completion. |
| `domain/selection.rs` | Cache compatibility and structured mismatch facts. |
| `application/` | Public workflows, options, reports, and observer contracts. |
| `application/resolution.rs`, `update_target.rs` | Pinned-cache reuse, pending update precedence, live fallback, and starting-version seeding. |
| `application/installed.rs`, `comparison.rs`, `plan.rs` | Validated state loading and consumer status/plan projections. |
| `application/pipeline/session.rs` | Exclusive installation access, authoritative checkpoints, execution plan, and execution totals. |
| `application/pipeline/execution_plan.rs` | Resolve remaining work once and project its consumer summary. |
| `application/pipeline/extraction.rs`, `streaming.rs` | Local execution or overlapping downloads and patch application. |
| `application/pipeline/patch.rs`, `finalization.rs` | Apply/checkpoint/release ordering, installed-state writes, and successful cleanup. |
| `application/pipeline/prefetch.rs`, `prefetch_budget.rs` | Download coordination, cancellation, and queued-byte policy. |
| `infrastructure/transfer/` | Cache reuse, partial-file promotion, resumable HTTP transfer, and retries. |
| `infrastructure/packages/` | Split-volume reading, archive decoding, replacement/delta classification, and application. |
| `infrastructure/filesystem/` | Layout, locks, persistence, safe paths, patch workspaces, replacement, and cleanup. |

## Invariants

- All installation-owned files live beneath `.wg-launcher/`. Moving the complete
  folder preserves installed identity and resumable work. The folder name retains
  its initial build identity after updates.
- Status and update validate installed identity before discovery. Status compares
  with the live catalog; update prefers a pending manifest and retains its selection.
- A session acquires the mutation lock, restores authoritative saved checkpoints,
  then resolves its execution plan. Install and extract publish the summary of that
  same plan before progress. Package-only downloads retain their component filter
  and pin the complete manifest.
- Starting versions seed missing checkpoints; saved completions take precedence.
  A component view supplies its target to planning and installation completion.
  Completion requires every selected component to reach its target before state
  is written.
- Streaming overlaps later downloads with ordered patch execution. The prefetch
  budget limits bytes queued ahead of the applying patch; one complete patch may
  cross the threshold. Download and extraction use the same resolved steps.
- Each patch is applied, checkpointed, then its recovery files and eligible packages
  are released. Failures retain uncheckpointed packages and original delta sources.
  Package retention controls both early release and final cache cleanup.
- Full content is decoded before each live file replacement. Delta patches install
  replacements before applying source-dependent changes. Recognized delta filenames
  select an encoding and game-package target; their checksum segments are syntax,
  not verified identity. Inner game-owned `.pkg` files remain packed.
- Blocking workers retain the session lock if their awaiting future is dropped.
  Extraction failure cancels pending downloads before exclusive access is released.
  Background worker failures retain their technical causes.
- Finalization validates completion, counts files, writes installed identity, and
  removes temporary work and eligible cache data. `files_before_finalization`
  includes existing game files and launcher metadata, not just newly extracted files.
- Persistence retains the existing JSON layout: the release is flat, component
  progress uses `base_versions`, and active patch identity is an array. Records
  load/save values; domain and application code own compatibility and resume rules.
