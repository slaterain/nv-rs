# Handoff: session of 2026-10-07 → 08

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[CONTRIBUTING.md](../CONTRIBUTING.md), [MILESTONES.md](MILESTONES.md) and
[TASKS.md](TASKS.md) first. This session stopped because the account's
weekly usage reached its limit; the maintainer wants usage kept for
reviewing and merging contributors' pull requests.

## State

- `main` has everything: work goes through pull requests only (ruleset
  "Maintainer-reviewed main": squash merges, a code-owner review and the
  CI checks `check (core)`, `check (viewer)`; the maintainer can't approve
  their own PR, so merges use `gh pr merge --admin`). A merge commit (used
  for #26 and #34 to keep contributors' history) needs the repository's
  "Allow merge commits" and the ruleset's allowed methods switched on for
  the merge and back off after — ask the maintainer first.
- Play copy (`Desktop\nv-rs-play`): build 26 (main `31ca50c`): Doc seated,
  the menu-background blur, greetings, physics part 1. Build 27 (main with
  the contact solver and B30) was being checked at the stop.
- Landed this session (see TASKS.md for each): Chazm's #31 via #34 (with
  review fixes, all traced), B14 (#40), B22 (#32), B24 (by #34's texture
  cache fix), B29 (#33), B30 (#44), B31 (#38), B32 (#42, not a game bug),
  B33 (#41), B23/B27 (#46), B1 parts 1–7 and 10 (#39, #43, #45), docs #35–#37.
- GitHub: task issues #13–#24. Assigned: #13 and #15 to abusager13 (their
  PRs #29 and #30 have "changes requested" reviews), #16, #19, #20 and #23
  to Chazm. Open: #14, #17, #18, #21, #22, #24.

## Unfinished at the stop

Nothing was running at the stop; every finished branch is merged.

- **B1 PR 8** (ragdoll constraints) and **PR 9** (continuous collision):
  not started. Ragdolls still use their own solver. Reading notes for PR 8
  (constraint atom layouts from the PDB, the Jacobian builder `00d6f460`
  and its helpers `00d6ced0`, `00dcee80`, `00dcefb0`; extend
  `nif::ragdoll` for friction torque and motor axes; move ragdolls onto
  `physics::solver`) are in docs/PHYSICS.md and
  `%USERPROFILE%\nv-re\work\b1-solver`.
- **Character proxy leftovers** (merged in #45, labelled in the code):
  Havok's convex-against-triangle collision agents (generic queries for
  now), `applySurfaceInteractions` (pushing dynamic bodies), the
  speed-fraction divisor (308, approximate), creature shapes from their
  bounds, swimming/flying/climbing, stairs checked by tests only.
- **Contributors' PRs**: #29 (Primm route) and #30 (quest matrix) wait for
  abusager13's changes; review notes are on the PRs.
- **Play copy**: build 27 published; build 28 (main `6d1d2fc`: the
  character proxy and B23/B27) was being checked; publish it if
  `%USERPROFILE%\nv-re\work\lead-check-main3.txt` shows everything
  passing (`publish-play.ps1 -Main %USERPROFILE%\nv-re\work\wt-main-build`).

## Next, in order

1. Finish B1: PR 8 (ragdoll constraints; reading notes in the PR 7 docs
   and `%USERPROFILE%\nv-re\work\b1-solver`), PR 9 (continuous collision),
   and the character proxy's leftovers (Havok's convex-triangle collision
   agents instead of the generic queries, `applySurfaceInteractions`, the
   speed-fraction divisor).
2. B2 (grab) on the new solver.
3. The rest of the maintainer's list in TASKS.md: B7, B8, B9, B15, B17,
   B18, B19, B20, B21, B25 (watch: not seen since #45), B28, the B14 camera snap,
   B31's depth of field.
4. Review contributors' pull requests as they come (CONTRIBUTING.md).

## How the work was run (what worked)

- One worktree and branch per task from `origin/main`
  (`%USERPROFILE%\nv-re\work\wt-<name>`), its own `CARGO_TARGET_DIR`; at
  most 4 agents at once. Agents still write into the main checkout by
  relative paths: brief them to use absolute paths for every write and
  check `git status` there after each agent.
- Ghidra: the shared read-only server (`%USERPROFILE%\nv-re\tools\
  ghidra-mcp\start-nvrs.ps1`, port 8089; docs/RESEARCH_WORKFLOW.md).
  Havok comes only from the exe (7.1.0-r1) and the Xbox PDB: never a Havok
  SDK (the maintainer's 2010 SDK's licence forbids translating it, and
  it's another version). Map: `%USERPROFILE%\nv-re\work\havok-map`.
- Checks before each merge or play build: `%USERPROFILE%\nv-re\work\
  lead-check.ps1 -i <worktree> -Target <cargo dir>` (root and viewer
  tests, clippy `-D warnings`, fmt, release build, acceptance). Acceptance
  starts each route as a new game and answers the game's pop-ups with
  `--answer-boxes --box-answers 2`; vms16 and occasionally vcg02 depend on
  timing (rerun once; A/B against the previous build before blaming a
  merge).
- Publish: `& %USERPROFILE%\nv-re\work\publish-play.ps1 -Main <worktree>
  -Notes @(...)` (call it in-process; `-File` splits the notes).
- `--screenshot` captures only the main 3D camera; for menus capture the
  window (`%USERPROFILE%\nv-re\work\vigor-regress\grab.ps1`).
