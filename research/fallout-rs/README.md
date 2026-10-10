# fallout-rs findings for nv-rs

This folder is a research contribution: findings, a map and a method,
written for the nv-rs maintainer and the maintainer's agents to read and
mine. It contains no code from the other project, no game files and no
game text. It does not claim a task.

## What fallout-rs is

fallout-rs is a separate, private Rust rewrite of Fallout 3 and Fallout:
New Vegas (about 430,000 lines in 75 crates, plus a Bevy viewer). It
was written mostly by Claude agents (Anthropic), with a smaller part by
OpenAI Codex, for one user who plays both games. It is not published.

It reads both games' own files at run time and runs them headless: every
record kind of both games decodes and re-encodes byte for byte, every
quest, dialogue line and script of both games runs in a headless session,
and a Bevy viewer draws the world. As a playable game it is behind nv-rs
(see [COMPARISON.md](COMPARISON.md)). As an engine with checks against the
original programs it is further along in some areas, and that is what this
folder offers.

The New Vegas program it studies is the same build nv-rs cites:
`FalloutNV.exe` 1.4.0.525. Every address here applies to nv-rs directly.

## How fallout-rs checks what it claims

1. **Per-game facts tables.** Every rule taken from a program is a row:
   system, rule, address, Rust function, evidence, status. Status is one
   of `oracle-verified` (the real program's function was called and agreed
   with the Rust on every input), `exe-read` (read in the decompiler and
   disassembly, not run), `measured` (counted over the game's data by a
   checker), or `data` (read from the game's records). The New Vegas rows
   are the basis of [nv-facts.tsv](nv-facts.tsv).
2. **The oracle.** A tool starts a readable copy of the game program
   suspended, never lets any of its own code start, and calls single
   functions inside it on random inputs through hand-assembled call
   stubs, then compares each answer with the Rust port. For New Vegas: 43
   rows, 49,159 calls, every mismatch explained; 4,459 setting defaults
   confirmed by running the program's own constructors. The method is in
   [ORACLE.md](ORACLE.md) in enough detail to rebuild it.
3. **Replay against real saves** (Fallout 3). A session is resumed from
   one of the user's own saves, run forward, and compared with a later
   save of the same play: 215 of the 310 recorded changes reproduce.
   AI package choice agrees with what the saves recorded for 98 to 100 %
   of 1,165 to 1,725 actors per save.
4. **Unattended quest runs.** A test player plays quest chains headless
   with fixed seeds (Fallout 3 MQ01 to MQ07 with a labelled test aid; New
   Vegas Back in the Saddle and They Went That-a-Way without it), plus a
   sweep that drives every quest's planned stages (Fallout 3 91.3 %, New
   Vegas 95.7 %) and a determinism check.
5. **Every behaviour change behind a switch.** Each change from one
   reading of a rule to another sits behind an environment switch
   `FO_OLD_<WHAT>=1` that restores the old behaviour, so a checker can be
   run both ways and a regression traced to one change.
6. **About 70 checkers** that each write a report per game; a change
   counts only when a checker on the real game files shows it.

What fallout-rs does not have: a comparison with a recorded frame of the
running game (nv-rs has one), hand-played builds, or a contributor
process. Its New Vegas rules were, until this month, mostly Fallout 3's
rules applied to New Vegas data; the New Vegas program was read only
recently, and several of nv-rs's readings were found right where
fallout-rs was wrong (listed in [NV_FACTS.md](NV_FACTS.md)).

## How to use this folder

| File | What it is for |
| --- | --- |
| [NV_FACTS.md](NV_FACTS.md) | Start here. The rows where nv-rs's code differs from what FalloutNV.exe does, each with the exact correction; then the rules nv-rs does not have yet; then where nv-rs was right and fallout-rs wrong. |
| [nv-facts.tsv](nv-facts.tsv) | All 98 New Vegas facts fallout-rs holds, deduplicated, with addresses, status, evidence and a column saying where nv-rs implements the same rule and whether it agrees. |
| [COMPARISON.md](COMPARISON.md) | The two projects side by side, by nv-rs's milestones, tasks and topic pages, with who leads where and on what evidence. |
| [ORACLE.md](ORACLE.md) | How to call FalloutNV.exe's own functions on thousands of random inputs and compare them with Rust. Relevant to task M10 (comparison harness). |
| [PORTING.md](PORTING.md) | fallout-rs components that could help nv-rs, ranked, with size, dependencies and what porting would involve under nv-rs's rules. |

Treat every row as a lead to confirm in your own decompiler before a
change, as nv-rs's methodology asks. `oracle-verified` rows have the
strongest evidence; `exe-read` rows are one reader's reading.

## Provenance

- Addresses: `FalloutNV.exe` 1.4.0.525 (the build nv-rs cites), read in
  Ghidra by fallout-rs agents; disassembly checked where a row says so.
- Oracle runs: fallout-rs `fo-oracle fnv`, 2026-10-08 (seed 20261007),
  report rerun by the main session.
- nv-rs code: read on branch `main` at 56a3d10 (2026-10-08). Nothing of
  nv-rs was built or run for this comparison.
- Prepared with Claude (Anthropic) agents, reviewed against both code
  bases by the agent that wrote it. The fallout-rs user can share specific
  crates on request (see [PORTING.md](PORTING.md)).
