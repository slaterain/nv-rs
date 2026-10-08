# fallout-rs components that could help nv-rs

No fallout-rs code is in this pull request. This page lists the parts that
look most useful to nv-rs, ranked by how much they would move nv-rs's
open milestones and tasks, so the maintainer can ask for the ones worth
having. The fallout-rs user can share specific crates on request.

Under nv-rs's rules (CONTRIBUTING, AGENTS, ADR-0003) any port would have to:
use no external libraries in core crates; carry only traced behaviour,
each rule with its `FalloutNV.exe` 1.4.0.525 address; come as small pull
requests with tests built from `testdata`; and leave out game data. Two
things make that work real rather than a copy:

- Much of fallout-rs's engine was read from **Fallout3.exe**, not the New
  Vegas program. Each rule needs its New Vegas twin. fallout-rs has a map
  of about 720 Fallout 3 functions to their FalloutNV.exe twins (about 110
  settled as same or different, about 490 matched by structure or features
  but not read in detail, 112 without a twin found),
  which can be shared as a table of address pairs.
- fallout-rs is a headless engine with its own world state; nv-rs's
  `world` crate has a different shape. Most items below are worth taking
  as a method and a checker, not as code.

| Rank | Component | What it does | How it is checked | Size and dependencies | Porting under nv-rs's rules |
| --- | --- | --- | --- | --- | --- |
| 1 | Oracle (`fo-oracle`) | Calls FalloutNV.exe's functions in a suspended process on random inputs and compares with Rust (see ORACLE.md). | Its own reports: 49,159 New Vegas calls; function bytes compared with the file; leftover-process check. | About 12,000 lines, most of them the individual checks; the process and stub machinery is a small part. Uses only the standard library and raw Windows calls. | Fits `research/nv-oracle` (its own workspace, no external crates) as a fourth tool next to `nv-call`; each check then targets an nv-rs function. Helps task M10 directly. |
| 2 | Quest sweep and quest graph (`fo-quest-graph`, `fo-quest-sweep`) | For every quest: the stages, what sets each one (scripts, dialogue, triggers), a plan to reach it, and a headless run that reports how far each quest gets and why it stops. | Reports per game; New Vegas 95.7 % of planned steps reached; reruns compared. | About 10,900 lines; depend on the headless engine (world, actor, quest, script crates). | Task M3 (coverage matrix). The planner reads plugin data only, so the graph half is portable as a generator; the run half needs nv-rs's headless world. |
| 3 | Save replay and comparison (`fo-replay-check`, `fo-save-compare`, `resume`) | Resumes a session from a real save, plays forward, and compares the result with a later real save of the same play, change by change. | Fallout 3: 215 of 310 recorded changes reproduced. | About 2,700 lines plus the save reader; depends on the engine. | Task M10's state-level half, once nv-rs's `fos` import can resume a session. Needs two saves of the same play by a contributor. |
| 4 | Save reader for New Vegas change forms (`fo-save`) | Decodes every change record of both games' saves to its exact length, including the New Vegas-only forms and the player record. | 196,228 of 196,228 records in 13 New Vegas saves; byte-identical rewrite. | About 12,900 lines, no external dependencies. | Task M7: compare with `crates/fos`; the layouts in NV-094 to NV-097 are the useful part. |
| 5 | Script decompiler and compiler round trip (`fo-scriptc`) | Decompiles compiled scripts to source and compiles them back; compares the bytes. | New Vegas: 99.80 % of compiled scripts recompile to the same bytes. | About 4,000 lines; uses a compression library. | A test for nv-rs's `script` crate: every script's compiled form against nv-rs's compiler. The check, not the code, is the useful part. |
| 6 | Unattended quest player (`fo-quest-run`) | Plays named quest chains headless with seeds: walks, talks, fights, picks dialogue by the quest graph. | Back in the Saddle and They Went That-a-Way complete without aids; determinism check. | About 3,300 lines; depends on the whole engine. | As an idea for `scripts/acceptance.ps1`: routes that run without the viewer. |
| 7 | Detection (`fo-detect`) | The New Vegas detection sum (NV-043). | Oracle 1,500 of 1,500. | About 2,500 lines; depends on the rules crate only. | Small and direct: the fix list in NV_FACTS.md is the port. |
| 8 | Combat tactics (`fo-tactics`) | The attack planner (19 actions, 27 costs), cover states, grenade window, flee, fighting strength, block. | Self-checked over 1,000 states; several functions oracle-verified on Fallout 3. | About 10,800 lines; read from Fallout3.exe, New Vegas twins partly read. | Task M5. Each function needs its New Vegas twin read first (the twin map helps). |
| 9 | Character controller rules (`fo-controller`) | The game's step, slope and cast rules for the player's body. | Found fallout-rs's own body more permissive than the game's. | About 7,200 lines; uses a physics library. | nv-rs has its own solver; the rule reading (with addresses) is the useful part for `crates/physics`. |
| 10 | Actor shadows and HDR rules (`render/`, `fo-hdr`) | Shadow window, bias and filters; the HDR pipeline as the program runs it. | GPU against a CPU port over many frames. | Several thousand lines; viewer-side. | nv-rs has no shadows yet. Read from Fallout3.exe; fallout-rs could not find the HDR render function in FalloutNV.exe, so this needs new reading. |

## Not suggested

- fallout-rs's menu engine and Pip-Boy pages: nv-rs is further along on
  screen.
- Anything read from the Xbox 360 symbols: fallout-rs did not use them.
- Game data, reports, renders, saves: never shared.
