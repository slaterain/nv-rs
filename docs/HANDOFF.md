# Handoff: session of 2026-10-08 (end)

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[CONTRIBUTING.md](../CONTRIBUTING.md), [MILESTONES.md](MILESTONES.md) and
[TASKS.md](TASKS.md) first. This session went past the 95% usage stop
rule at the maintainer's request and stopped at about 96%.

## State

- `main` has everything (PRs only; squash; `gh pr merge --admin` once
  `check (core)` and `check (viewer)` pass).
- Play copy (`Desktop\nv-rs-play`): build 33 (main `4d82854`).
- Landed today: #51 (Havok on the game's frame timer; contact sounds at
  their point), #52/#60 (Doc's chair exit and seated package checks),
  #58 (B1 parts 8–10: proxy push and GSK cast, ragdoll constraints,
  simplified TOI), #64 (help-up timing from the maintainer's recordings,
  SayTo speaker idles, daylight opening, thin bodies vs floors, weather
  log spam), #65 (opening step table; nested stage scripts keep the
  quest's variables).

## Open

1. **The opening after the walk to the tester** (docs/OPENING.md, "The
   whole opening, step by step"): the tag-skill menu needs input the
   automation can't give, so traits, the farewell, Doc's walk out and
   stage 200 weren't run or compared. Drive them (menu keys / `--run`
   stages) and compare with the original.
2. **Help-up trigger in the exe**: implemented from the recording
   (`GameState::evaluate_everyone` when the face menu closes); the
   engine path isn't traced (ruled-out list in OPENING.md).
3. **B15 face menu** not built (auto-accepts).
4. **B25** (NPCs sinking) not reproduced: needs an F12 report.
5. Physics leftovers (docs/PHYSICS.md, TASKS.md B1): GJK and penetration
   depth under the agents, moving platforms, a pushed body overlapping the
   player, ragdoll motors, full TOI, bodies leaving the loaded terrain.
6. Re-adding the same script package (VCG01 stage 7) starting its begin
   action again: untraced.

## Tips

- Use `--walk` for any log about the opening camera (fly mode never
  advances the first-person idles).
- The maintainer's recordings can be aligned with voice files by
  loudness envelopes (WebAudio in the built-in browser, served by a small
  local HTTP server); it pinned the help-up exactly.
- Ghidra server: `read_memory` and `search_instructions` endpoints exist
  (vtable slots, offset writers).
