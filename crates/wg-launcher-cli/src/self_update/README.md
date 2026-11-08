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
| Self-update outcome | Whether the launcher was updated, was already current, or must be updated through its package manager. |
| Self-update | Replace the launcher with a verified newer release. |
| Direct installation | A launcher copy installed without a package manager taking responsibility for it. |
| Package manager | The owner responsible for updating a managed launcher. Detection returns that ownership; terminal presentation chooses its name and update instructions. |
| Managed installation | A launcher copy maintained by a package manager, such as Homebrew or APT. Updates go through that manager. |
| Candidate | The downloaded launcher being checked before it can replace the current one. |
| Verification | Checks that the downloaded launcher matches the published release and reports the expected version. |
| Host platform | The kind of computer running the launcher, which determines which launcher download is suitable. |
