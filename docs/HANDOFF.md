# Handoff: session of 2026-10-08

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[CONTRIBUTING.md](../CONTRIBUTING.md), [MILESTONES.md](MILESTONES.md) and
[TASKS.md](TASKS.md) first. This session stopped at 93% of the weekly
usage (the stop rule is 95%) before the maintainer's next request could
start.

## State

- `main` has everything merged through PRs (ruleset "Maintainer-reviewed
  main": squash merges, CI `check (core)` and `check (viewer)`, merges with
  `gh pr merge --admin`).
- Play copy (`Desktop\nv-rs-play`): build 29 (main with #51 and #52).
- Landed this session:
  - #51 (`claude/physics-clock`): bodies froze in the air above ~156
    frames a second; the Havok clock now gets the game frame timer's frames
    (`00aa4ee0`, `physics::havok::FrameTimer`, `clutter::HavokFrame`).
    Contact sounds were flat 2D sounds from anywhere loaded (the
    "clattering"): now at the contact point, only within the sound's reach
    (`00837550`, `0082eca0`). docs/PHYSICS.md, "Frozen bodies and far-off
    clatter".
  - #52 (`claude/b14-helpup`): Doc's chair exit no longer waits for his
    seated idle to finish (`00921e80`, `00498f80`). B25 re-checked.

## Unfinished: the maintainer's two reports

- **Doc's help-up is still late (B14).** Doc now rises at his `evp`
  (end of line `00104BFA`, 64.4 s in a `--new-game --no-movies` run) but
  the player's stand-up starts at ~57 s (`VCG01PlayerSection2` after line
  `00104BF9`); his bend-down comes ~7 s after the player is up. His
  standing package (`00107238`) only needs `GetStage VCG01 >= 40`, so the
  game must re-evaluate his packages earlier than the viewer does. Next:
  trace the code that runs a `SayTo` line's end-of-line script and whether
  it (or stage 40 being set) forces a package evaluation; or record the
  original game from the face menu closing to the help-up. Notes and
  dumps: docs/OPENING.md "Help-up timing", `%USERPROFILE%\nv-re\work\b14-helpup`.
- **NPCs sinking to the waist (B25): not reproduced.** 200 s of the vms16
  route with `NV_GROUND_LOG=1` (`%USERPROFILE%\nv-re\work\b25`): nobody with
  a controller deeper than 10.7 under the land; deeper rows are people far
  from the camera at navmesh height (far rule), who start that deep when
  handed back to their controller and rise slowly. Ask the maintainer for
  an F12 report on a sunk NPC; compare far-rule heights with the game.

## Next, in order (the maintainer's request of 2026-10-08)

1. The character proxy's untraced parts: Havok's convex-against-triangle
   collision agents instead of the generic queries,
   `applySurfaceInteractions` (pushing dynamic bodies), the speed-fraction
   divisor (308, approximate). docs/PHYSICS.md "The character proxy".
2. The rest of B1: PR 8 (ragdoll constraints; reading notes in
   docs/PHYSICS.md and `%USERPROFILE%\nv-re\work\b1-solver`) and PR 9
   (continuous collision: a tumbleweed blown fast tunnels through the land,
   00178A82, seen in #51's run).
3. Then merge and publish a play build, as before. The maintainer asked
   to avoid reaching 100% usage.

## How the work was run

As in the previous handoff: one worktree per task from `origin/main`
(`%USERPROFILE%\nv-re\work\wt-<name>`), agents briefed with absolute
paths, the shared Ghidra server (`start-nvrs.ps1`, port 8089), checks with
`lead-check.ps1`, publish with `publish-play.ps1` called in-process. Disk:
build caches had filled C: to 98%; the merged branches' `target-*` and
`viewer\target` folders were deleted (171 GB free after).
Viewer note: `--wait N` without `--screenshot` doesn't exit.
