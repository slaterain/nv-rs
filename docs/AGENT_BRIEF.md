# Brief for contributors' agents

Give this file to each agent you run on nv-rs. It is the whole briefing;
the agent should read the linked files before writing code.

## What nv-rs is

A 1:1 reimplementation of Fallout: New Vegas in Rust/Bevy, matching
FalloutNV.exe 1.4.0.525. "1:1" means behaviour is traced from the
original executable (Ghidra, with the Xbox 360 prototype's symbols for
names) or recorded from the original game. Nothing is guessed.

## Setup

- Repository: `slaterain/nv-rs`. Branch from `claude/overnight-integration`
  (not `main`, until the maintainer merges it). Open pull requests against
  `claude/overnight-integration`.
- Read, in order: `AGENTS.md`, `CONTRIBUTING.md`, `docs/TASKS.md`,
  then the docs for your task's area.
- You need your own legal copy of the game (Data folder) to run the
  viewer and the acceptance routes. Never commit anything from it.

## Pick a task

Only the **major systems** in `docs/TASKS.md` (M1–M12, also GitHub issues
titled `[task] …`) are open. Splittable ones suit several agents:

- M1 Primm quest routes: one quest per agent.
- M2 Main quest Goodsprings → Novac: one leg per agent.
- M3 Base-game quest coverage matrix: one agent.
- M4 Weapon classes: one class per agent (energy, launchers/explosives,
  thrown/mines, melee/unarmed specials, scopes).
- M5 Creature AI: one family per agent.
- M6 World map, fast travel, the whole Mojave.
- M7 Original `.fos` saves (research first).
- M8 Mods and plugins.
- M9 Other DLCs: one DLC per agent.
- M10 Comparison harness with the original game.
- M11 Factions, crime, karma, disguises.
- M12 VR.

**Not open:** the maintainer's list in `docs/TASKS.md` (physics, dialogue,
animation, opening, combat effects, the bugs from playtests), and the
areas owned in `CONTRIBUTING.md` (Chazm: terminals, item scripts,
repair and weapon mods, companions, Caravan, casinos, performance; the
Dead Money contributor: Dead Money, crafting). If your task needs a
change there, keep it minimal and say so in the pull request.

## Rules

1. **Claim first.** Comment `Claiming this` and your branch name on the
   GitHub issue (or on a sub-part of it, e.g. "M4: energy weapons") before
   starting. First claim wins; don't start on a claimed task. Post
   progress once a day; a claim idle for 48 hours can be released.
2. **One task per agent, one branch and one git worktree per agent.**
   Never edit another agent's folder or the shared checkout.
3. **Trace, don't guess.** Find the behaviour in FalloutNV.exe, translate
   it, and comment `// Translated from <address> (decompiled, FalloutNV.exe
   1.4.0.525)`. Anything you couldn't trace is labelled `[G]` and kept
   behind `world::guesses` (off unless `NV_GUESSES=1`); it must not change
   the base game.
4. **Tests for every change.** Fail without it, pass with it.
5. **Checks before every pull request.** Root: `cargo test --workspace`,
   `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`.
   In `viewer/`: `cargo test`, `cargo clippy --all-targets`,
   `cargo fmt --all -- --check`, `cargo build --release`. Then
   `powershell -File scripts\acceptance.ps1 -Data "<your Data folder>"`
   (Doc's walk, Back in the Saddle, Ghost Town Gunfight); paste the
   result table. The gunfight is partly random: rerun once if it fails.
6. **Verify by playing.** Run the release viewer and look (screenshots).
   In the pull request, say what was verified by playing and what only by
   tests.
7. **Small pull requests.** One finished sub-task each, reviewable in one
   sitting. Huge mixed pull requests are sent back to be split.
8. **Rebase daily** on `claude/overnight-integration`.
9. **Never commit** game files or extracted assets, executable bytes,
   decompiler databases or exports, recordings, or logs.
10. **Docs.** Update the topic page in `docs/` for your area (what's done,
    traced, missing) and add a line to `docs/MILESTONES.md`.
