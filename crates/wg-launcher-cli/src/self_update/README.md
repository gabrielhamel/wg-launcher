# Launcher self-update

Self-update keeps the launcher itself current. It checks for a newer launcher
release, tells people how to obtain it, and can replace a launcher that was
installed directly.

It answers: “Is my launcher current, and who should update it?” Game content and
game versions belong to [installation](../installation/README.md).

## Shared language

| Term | Meaning here |
| --- | --- |
| Launcher | The tool used to discover, download, install, and update game content. |
| Launcher release | A published version of that tool, separate from any game release. |
| Release artifact | The launcher archive selected for the host platform, together with its published checksum file and archive format. |
| Current version | The version of the launcher being used now. |
| Latest version | The newest launcher release offered by the release source being checked. |
| Update notice | A message explaining that a newer launcher is available and how to obtain it. |
| Self-update event | A progress fact from checking, downloading, verifying, or replacing the launcher. Terminal code decides how to display it. |
| Release discovery failure | A failure reading or interpreting the launcher release manifest, retaining its HTTP or parsing cause. |
| Candidate preparation failure | A failure downloading, checking, or extracting a launcher candidate, retaining the responsible adapter error. |
| Self-update failure | A typed reason an update could not finish, retaining its technical cause and rendered as recovery guidance at the terminal. |
| Self-update outcome | Whether the launcher was updated, was already current, or must be updated through its package manager. |
| Self-update | Replace the launcher with a verified newer release. |
| Executable ownership | Whether the launcher has no detected package manager, is managed by a named package manager, or could not have its executable path resolved. Unresolved ownership retains the existing direct-update fallback. |
| Direct installation | A launcher copy installed without a package manager taking responsibility for it. |
| Package manager | The owner responsible for updating a managed launcher. Detection returns explicit executable ownership; terminal presentation chooses its name and update instructions. |
| Managed installation | A launcher copy maintained by a package manager, such as Homebrew or APT. Updates go through that manager. |
| Downloaded archive | The downloaded launcher archive kept together with the digest computed while writing it, ready for checksum verification. |
| Candidate preparation | Download the selected archive, verify its published checksum, and extract the launcher for executable verification. |
| Candidate | The downloaded launcher being checked before it can replace the current one. |
| Extracted candidate | A launcher extracted from a checksum-verified archive, awaiting its executable version check. |
| Candidate version | The semantic version read from a supported launcher version-output format. Verification requires the exact expected version, including prerelease and build metadata. |
| Verified candidate | An extracted launcher that passed the expected-version check and can be supplied to executable replacement. |
| Verification | Checks that the downloaded launcher matches the published release and reports the expected version. |
| Host target | Explicit operating system, processor architecture, and build environment used to select a compatible launcher archive. |
| Host platform | The kind of computer running the launcher, which determines which launcher download is suitable. |

The application supplies the current version and host target to domain rules.
Release comparison and artifact selection do not discover those facts themselves.

Each adapter returns its own concrete error: `ReleaseDiscoveryError`,
`ArtifactError`, `ArchiveError`, `HttpError`, or `ExecutableError`.
Infrastructure never returns the application error. Application-owned
`CandidatePreparationError` composes artifact, archive, and progress failures.
The application composes
discovery, candidate preparation, executable, selection, and progress failures
in `SelfUpdateError`; it does not recursively wrap itself. Presentation matches
these typed causes for recovery instructions without runtime downcasts. Package-manager ownership is resolved
by the application once per interaction and supplied to instruction rendering;
instruction rendering never performs filesystem discovery. Archive format remains
explicit from release selection through extraction.

`application/candidate_preparation.rs` owns the preparation sequence and its
progress stages. The artifact adapter retrieves checksums, downloads archives,
and checks their digests; the archive adapter extracts the executable.
`prepare_candidate` names the complete artifact preparation operation. It returns
an `ExtractedCandidate`; executable verification consumes it and returns a
`VerifiedCandidate`. The executable adapter runs the candidate with a timeout;
`domain/candidate.rs` parses the supported `--version` output formats and compares the reported semantic
version with the expected release. Replacement accepts only that
verified value. The staging directory remains beside the current executable until
the operation finishes.

The context's `progress.rs` owns shared stages, events, and the observer contract.
Application orchestration, infrastructure adapters, and terminal presentation all
use that contract without importing progress definitions or errors from the application.
Observer failures use `std::io::Error`, preserving fallible rendering and cancellation.
