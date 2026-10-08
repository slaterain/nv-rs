# Handoff: session of 2026-10-08

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[CONTRIBUTING.md](../CONTRIBUTING.md), [MILESTONES.md](MILESTONES.md) and
[TASKS.md](TASKS.md) first. This session went past the 95% usage stop
rule at the maintainer's request (to finish the physics follow-up) and
stopped before 100%.

## State

- `main` has everything merged through PRs (ruleset "Maintainer-reviewed
  main": squash merges, CI `check (core)` and `check (viewer)`, merges with
  `gh pr merge --admin`).
- Play copy (`Desktop\nv-rs-play`): build 30 (main with #58).
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

## Done after that (the maintainer's request of 2026-10-08)

#58 (combining #54–#57, each described there; build 30): the proxy's
linear cast is Havok's GSK cast (`00daf8e0`); the proxy pushes bodies
(`applySurfaceInteractions` `00cacf80`) and `fSpeedPct`'s divisor is the
player's run speed; ragdolls run on the game's constraint solver
(`00d6f460`, `00d8d030`); debris gets simplified time of impact against the
world (`00cfb570`, `00d0e210`). Checked: root 1758 and viewer 169 tests,
clippy, fmt, the three acceptance routes; live runs per PR.

## Next, in order

1. Physics leftovers (labelled; docs/PHYSICS.md, TASKS.md B1): the GJK
   (`00daad40`) and penetration depth (`00daa7e0`) under the agents;
   moving platforms (`00c6ca30`); a pushed body overlapping the player's
   hull (no keep-out traced); ragdoll motors and the runtime data's
   constructor; full TOI (`00d100a0`) for moving/critical bodies and the
   agents' event times; the tumbleweed rolling off the viewer's loaded
   terrain (y −4096 near x −64960): what the game does with a body leaving
   its loaded grid.
2. B14's help-up timing and B25 (above).
3. The rest of TASKS.md (B2 grab on the new solver, B7, B8, …).
## How the work was run

As in the previous handoff: one worktree per task from `origin/main`
(`%USERPROFILE%\nv-re\work\wt-<name>`), agents briefed with absolute
paths, the shared Ghidra server (`start-nvrs.ps1`, port 8089), checks with
`lead-check.ps1`, publish with `publish-play.ps1` called in-process. Disk:
build caches had filled C: to 98%; the merged branches' `target-*` and
`viewer\target` folders were deleted (171 GB free after).
Viewer note: `--wait N` without `--screenshot` doesn't exit.
