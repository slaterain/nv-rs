# Maintaining the official project

slaterain controls the official main branch and releases. External contributors
use forks and pull requests; no collaborator invitations are needed.

## Accepting changes

Main requires a pull request, code-owner approval, resolved conversations and
the CI job `check` (fmt, tests and clippy on Windows). New pushes dismiss old
approvals. Force pushes and deletion are blocked. CODEOWNERS includes workflows.
The owner has a PR-only ruleset bypass for their own changes (GitHub cannot
accept a self-approval); use it deliberately, after checks pass. No other
contributors receive bypass rights. Changing repository settings remains an
owner responsibility.

Keep forks' CI unprivileged: use pull_request, read-only token permissions,
GitHub-hosted runners and no secrets. Never execute contributed code in a
privileged pull_request_target workflow. Inspect workflow changes carefully.

## Releasing a Windows preview

An interim, labelled preview before the wgpu release (approved by the maintainer). Only the maintainer
decides on a release: agents build and test locally, and tag, upload and post nothing.

1. Merge a checked source commit and compare a play-build run against the
   original game. Record outstanding limitations, not just screenshots.
2. From a clean Windows checkout, run
   `./scripts/package-playtest.ps1 -Version YYYY-MM-DD.N -Game '<your FalloutNV.exe>'`.
   It builds with the lockfile, a static C runtime and no build paths in the programs, stages only
   the list below, runs `scripts/package-check` over it and makes the ZIP and its `.sha256`. The 32-bit
   tools need a toolchain with `i686-pc-windows-msvc` (`-Toolchain32`, default `stable`). `-Game` is required: the
   check that none of the game's bytes are in the programs fails the package. The script refuses a
   release ZIP unless HEAD is on origin (BUILD.txt links to it); `-TestBuild` makes a test package
   instead, marked "test build, not for release" in BUILD.txt and in its name.
3. Test the extracted ZIP from a fresh folder, a path with spaces and another drive: first start,
   Save or New Game, a couple of minutes in the world, no game folder, a wrong exe, tables deleted
   (`docs/PREVIEW_TESTERS.md` is the checklist for other PCs). Verify SHA256 and BUILD.txt.
4. Create a release tag at that exact source commit, which must be public: BUILD.txt points at it, and
   that is how the GPL's source is offered. Create a draft pre-release, attach the ZIP and its
   .sha256 file, and describe verified behavior, known issues, controls, required owned game data and
   reporting instructions. Say which PCs it has run on.
5. Publish after reviewing the draft. Official builds are maintainer-published;
   ordinary pushes and contributor PRs never publish a release automatically.

What the ZIP holds, and why each is allowed:

| File | What | Why it may ship |
| --- | --- | --- |
| `Play.exe` | The launcher (`scripts/launcher`) | Our code, GPL |
| `app\FalloutNV.exe` | `nv-viewer` under that name | Our build. LEGAL.md covers the translated code; `package-check` looks for data of the game's own program in it, and its report goes with the release notes |
| `app\tools\d3d9-capture.exe`, `nvapi-capture.exe`, `dsound-capture.exe` | The table tools (`scripts/`) | Our code, GPL. They read the player's Windows and drivers, on the player's PC |
| `Play with mods.cmd`, `Import my saves.cmd`, `Settings.cmd`, `Settings.ps1` | Launchers and the settings tool | Our text |
| `READ ME.txt`, `mods\READ ME.txt`, `BUILD.txt` | For players; the exact commit | Our text |
| `LICENSE`, `LEGAL.md`, `THIRD-PARTY-NOTICES.txt`, `licenses\` | GPL, the MIT notice, the independence statement, the Rust crates' licences, libvorbis's BSD notice | Required by those licences and LEGAL.md |

Never in it: game files, saves, reports, machine tables (`.tsv`), `.ini`, DLLs, debug files (the
script's type list refuses them), anything from fnv_decomp, or this PC's paths and user name
(`package-check`'s needles). Windows' and the graphics driver's DLLs stay on the player's own PC.

An unsigned exe: SmartScreen warns on a downloaded one ("More info", "Run anyway"), Smart App
Control blocks it with no way through, and an antivirus may hold or remove it, more so as it starts other
programs. The READ ME says so plainly. A code-signing certificate is the only real answer; none is planned yet.

Dependency updates require refreshing distribution/licenses and
THIRD-PARTY-NOTICES.txt before packaging. Keep their upstream provenance and
copyright notices. Packaging a license list alone is insufficient.


## Releasing a Linux or macOS preview

Nobody on the team has a Linux PC or a Mac, so these packages are built by GitHub Actions
(`.github/workflows/preview-packages.yml`, started by hand only) and checked against the game
on the maintainer's Windows PC before anything is published. The archive GitHub built is
published byte for byte: re-packing it on Windows would lose the Unix permissions inside it
(play.sh and the program executable), which NTFS does not keep. The game does NOT run on Linux
or macOS yet: these previews find the game, keep nv-rs's saves apart from the player's, and say
plainly that they cannot start it. Every text that goes out with them says so.

1. Start the workflow on main: Actions > "Preview packages (Linux, macOS)" > Run workflow, with a
   version such as `2026-11-20.1` (or `gh workflow run preview-packages.yml --ref main -f
   version=2026-11-20.1`). CI has no game, so it builds without it: each archive is named
   `nv-rs-preview-<system>-test.tar.gz`, and its BUILD.txt names the run, the branch and the
   commit, and says the program was not checked against the game where it was built and that a
   published package is checked first (step 3). `scripts/package-playtest.sh` refuses a package
   without `--game` unless it is made that way (`--test-build`).
2. Download both artifacts into a scratch folder (not the game's folder):
   `gh run download RUN-ID --repo slaterain/nv-rs`.
3. In PowerShell, from a clean checkout of main, check each archive with your own FalloutNV.exe
   (only read):

   ```powershell
   ./scripts/check-unix-package.ps1 -Archive <download>\nv-rs-preview-linux-x86_64-test\nv-rs-preview-linux-x86_64-test.tar.gz -Game '<game folder>\FalloutNV.exe'
   ./scripts/check-unix-package.ps1 -Archive <download>\nv-rs-preview-macos-arm64-test\nv-rs-preview-macos-arm64-test.tar.gz -Game '<game folder>\FalloutNV.exe'
   ```

   For each it checks the archive against its `.sha256`; extracts it into a temporary folder (removed
   after); checks the program's SHA256 against BUILD.txt; checks BUILD.txt says it was built by a
   GitHub Actions run from `refs/heads/main` and that its source commit is on origin's main (a run
   started on another branch is refused, so its source link always holds); and runs
   `package-check --game --max-run 4096` on the package (no data of the game's program in our
   program). It must end with `check-unix-package: PASSED`. Read the two lines it printed and wrote:
   `N files, M needles: 0 hits` (`package-check` itself now refuses a folder that is missing or holds
   no files, and `--game` with no program in it), and
   `nv-rs-preview-<system>-test\app\nv-viewer: <bytes> bytes, shares ... longest <L>` with L at most
   4096: that line is the proof the program itself was compared with the game. It then writes
   `nv-rs-preview-<system>-test.CHECKED.txt` beside the archive: the run, the commit, the date and
   those lines. Any `REFUSED` line: publish nothing; fix the cause and start again at step 1.
4. Read each BUILD.txt (in the archive): a build without the game lists the licence notices or
   folders still missing (a release package would have been refused for them). If it lists any,
   refresh distribution/licenses and THIRD-PARTY-NOTICES.txt, merge, and start again at step 1.
5. Create a release tag at BUILD.txt's source commit (the same in both packages; the script
   checked it is on main, and it must be public: BUILD.txt and CHECKED.txt point at it, and that is
   how the GPL's source is offered). Create a draft pre-release and attach, as they are, each
   archive, its `.sha256` and its `.CHECKED.txt` (renaming an archive breaks its checksum file).
   The notes say plainly that the game does not run on Linux or macOS yet and what the packages do
   instead (find the game, keep the saves apart, say why they stop), carry both CHECKED.txt texts,
   and give the reporting instructions. Say that nobody on the team has run them.
6. Publish after reviewing the draft. Official builds are maintainer-published; nothing here
   publishes by itself.

`scripts/check-unix-package-test.ps1` tests the check script on made-up packages (no game file): run
it after changing either script.

The first hand-started run after this workflow is merged is its first run with the release
profile from scratch on both runners (the earlier proving runs used the quick play profile).
Record from it, in the pull request that merged the workflow or in the release notes: each job's
build time, whether it stayed inside `timeout-minutes` (240 for now), and both packages' sizes;
then set the timeout to about twice the slower job's time.

## Work queue and AI handoff

Keep a small set of milestone issues. Contributors comment to claim a bounded
task before starting; assignment is not automatic acceptance of a design.
Each handoff identifies branch/commit, changed files, verified evidence, test
results, unresolved questions and the next action. Do not paste entire chats.

The initial private upload history was retained separately because it included
build outputs. The clean public history starts with the reconciled source;
do not publish private research or old build directories to restore that history.
