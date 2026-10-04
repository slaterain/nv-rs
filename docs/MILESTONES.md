# nv-rs milestones

Updated 2026-10-04. Priority tracker and session handoff.

## Baseline

README and previous research notes report readers, rendering, streaming,
walking, actors, AI, combat, dialogue, quests, inventory, menus, audio,
V.A.T.S. and custom saves. Many corresponding source modules exist.
The first foundations batch built and tested both workspaces and ran the
opening to Doc's instruction to use the vigor tester. Missing cutscene and
menu behavior prevents calling that a faithful opening. Evidence remains in
ENGINE_REFERENCE.md, CLAUDE_REFERENCE.md and %USERPROFILE%\nv-re\findings.
The clean public source history was prepared on 2026-10-03. Earlier private
history is retained separately; do not publish its build outputs.

## Ordered gates

| Milestone | Completion gate | Status |
| --- | --- | --- |
| M1: Opening and persistent world | Retail opening movie and scripted wakeup, animations/movement, Doc Mitchell's creation sequence, exit into Goodsprings, talk/interact, save, restart and reload. Player, NPC, quest, inventory and reference state survives. Compare with the original game. | Active; initial live run and package-action reader complete; choreography and full route pending. |
| M2: Core gameplay loop | Sunny's tutorial and a representative Goodsprings quest branch through their own scripts: movement, weapons/reloads, damage/death, AI, dialogue, loot, trade and progression. Save/reload at intermediate stages. Verify melee and V.A.T.S.; track other weapon classes explicitly. | Partial implementation reported; acceptance pending. |
| M3: Base-game systems and campaign | Coverage matrix for quests, actor/creature types, weapon classes, effects, factions/crime, companions, travel and menus. Representative routes and ultimately campaign completion, with evidence and regression tests for blockers. | Inventory and acceptance routes needed. |
| M4: Stability and performance | Recorded routes and extended play without crashes or lost state. Measure frame times, memory, loading and streaming stalls on target hardware. Publish traces/settings and agreed budgets; remove measured stalls without changing behavior. | Not measured here; instrument earlier when it helps M1/M2. |
| M5: DLC and mods | Official DLC progression and reproducible plugin/archive/loose-file, script and content-extension cases; document interfaces and exclusions. | Load-order infrastructure exists; broad compatibility unverified. |
| M6: VR | Shared simulation with action inputs, independent aim and multiple views. Headset-tested tracking, controllers, menus, combat, comfort and frame budget. | Architecture documented; headset validation pending. |

Preserve save correctness, mod semantics and VR boundaries throughout; order
does not postpone foundational fixes. Set performance budgets from measured
hardware/display requirements. Original save compatibility and native binary
mod compatibility need separate researched scope; custom saves and plugin
reading do not establish them.

## Active work: M1

The published baseline includes the opening package look lock, Doc's queued
chair exit, native Info HUD and tester bounds correction, SPECIAL interface,
and same-cell trigger reset on F9. Live evidence reaches the tester through
stage55/60 and opens SPECIAL; it is not full-route acceptance.
Historical checks and play hashes remain in [OPENING.md](OPENING.md).

Current batch: failed custom save writes preserve the previous save, and
failed F9 destinations preserve the running world. Generated regressions
pass; 913 core and 84 viewer tests, both clippy/format/release checks and an
isolated installed-data forced-save smoke passed on
`codex/m1-opening-overnight`. Evidence and process handoff:
[PERSISTENCE.md](PERSISTENCE.md). No new play build published yet.

The bounded gamedb evaluation silently missed the central look-lock function
in a four-export probe. Full indexing is deferred; source snapshots, hashes,
DB and inventory remain private. Shared query workflow and limitations:
[RESEARCH_WORKFLOW.md](RESEARCH_WORKFLOW.md).

Outstanding M1 gates:
- Exact opening camera transition replay and Doc/player assistance timing.
- Native Doc head/eye tracking; final rotation math remains unresolved
  ([OPENING_LOOK_IK.md](OPENING_LOOK_IK.md)).
- Original face editor instead of auto-accept, and opening movie playback
  ([FACE_CREATION.md](FACE_CREATION.md)).
- In-progress animation save restoration, full character-creation route,
  exit to Goodsprings and save/restart/reload acceptance.
- SPECIAL's remaining visual/input fidelity and progression comparison
  ([VIGOR.md](VIGOR.md)); general idle callbacks remain partial.

Next action: finish persistence PR review/publication, then continue the
native opening gaze/face trace. Do not repeat the established
package-look lock or tester activation fixes.

## Deferred

Cosmetic material/lighting discrepancies, isolated facial polish, sun glare,
distant water and other rendering leftovers stay in the research archives.
Promote only for a milestone blocker or explicit user priority. Small gameplay
fixes needed by the active route count as blockers; never invent a substitute.

## Handoff

After each batch replace the active-work section with the latest outcome,
evidence links, blockers and one next action. Keep this file short; detailed
logs belong in topic references. Completion requires a reproducible acceptance
run, not a count of parsed records or implemented functions.
