# Contributing to nv-rs

nv-rs is maintained by **slaterain**. Many people now work on it at once,
most of them with AI agents. This page says how to do that so that your
work can actually be merged. Read it, then give it and
[AGENTS.md](AGENTS.md) to your agent.

The short version:

1. Work from the latest `main`, in a fork, on a topic branch.
2. Claim a task on GitHub before you start ([TASKS.md](docs/TASKS.md),
   issues titled `[task] …`).
3. Trace every behaviour in the original game. No invented behaviour.
4. Open a small pull request against `main` that passes the checks and
   the acceptance routes.

## Already working in a fork? Start here

On 2026-10-07 `main` moved forward by several hundred commits: everything
that had been on the maintainer's `claude/overnight-integration` branch
(NPC AI, combat, physics, dialogue, the Pip-Boy, Chazm's and the Dead
Money contributor's work, and more; see
[docs/MILESTONES.md](docs/MILESTONES.md)). If you forked before that, do
this before anything else:

1. **Update your fork.** On GitHub press **Sync fork** on your fork's
   `main`, or:

   ```sh
   git remote add upstream https://github.com/slaterain/nv-rs.git   # once
   git fetch upstream
   git switch main
   git reset --hard upstream/main      # your fork's main = upstream main
   git push --force-with-lease origin main
   ```

   Keep your own work on topic branches, never on your fork's `main`.
2. **Delete the copied `claude/*` and `codex/*` branches** in your fork.
   They are the maintainer's branches, all of them are now in `main`, and
   pull requests built on them will be closed.
3. **Check that your work isn't already in `main`.** Many systems landed
   at once: search the code and [docs/TASKS.md](docs/TASKS.md) ("Landed")
   before continuing.
4. **Claim it.** Find the matching `[task]` issue, or open one, and
   comment `Claiming this` with your branch name, even if you started
   before claiming. If someone else already claimed it, say what you have
   on the issue and agree who continues; don't open a competing PR.
5. **Rebase your branch on the new `main`**
   (`git rebase upstream/main`), resolve conflicts in favour of traced
   behaviour, and run the checks below.
6. **Open a pull request against `slaterain/nv-rs` `main`**, one system
   per pull request (a draft is fine while it's unfinished). Big work goes
   in as a stack of smaller pull requests, each reviewable on its own.

Work that changes how the game behaves without tracing it (tuned
constants, "feels better" camera or AI changes, procedural effects the
game doesn't have) can't be merged, however good it looks. If your fork
has some, keep it behind `NV_GUESSES=1` (`world::guesses`) or leave it
out, and say in the pull request which parts are traced.

## Claiming a task

Open work is listed in [docs/TASKS.md](docs/TASKS.md) and as GitHub
issues titled `[task] …` (major systems, open to everyone). The
maintainer's list in TASKS.md (items `B1`, `B2`, …) is not open for
claiming; report new bugs as issues instead.

1. Pick an issue nobody has claimed (no assignee, no claim comment).
2. Comment `Claiming this` with the branch name you'll use. The
   maintainer assigns you. First claim wins; don't start on a claimed
   task.
3. One task per person at a time (two if they're small). Big tasks say
   how to split them; claim one part at a time.
4. Post a short progress comment at least once a day. A claim with no
   comment or pull request for 48 hours can be released.
5. Link the pull request to the issue (`Closes #N`). If you give up, say
   so on the issue so someone else can take it.

Found a new bug? Open an issue for it; don't fix it inside an unrelated
pull request. Something big missing that isn't listed? Open an issue
proposing it as a `[task]`.

## Areas

Who owns what. Ask the owner on GitHub before changing their area in a
large way; small fixes anywhere are welcome with a test.

| Area | Owner | Notes |
| --- | --- | --- |
| Opening (VCG01), Doc Mitchell's house, character creation, look-IK | maintainer | Face editor and opening still open. |
| Goodsprings routes: Back in the Saddle (VCG02), Ghost Town Gunfight (VMS16) | maintainer | Acceptance routes; see docs/GOODSPRINGS_ROUTE.md. |
| NPC AI: packages, pathing, combat, animation, dialogue behaviour | maintainer | PACKAGES, PATHING, NPC_COMBAT, ANIMATION, DIALOGUE. |
| Player: movement, actions, sneaking, aiming, furniture, physics | maintainer | MOVEMENT, PLAYER_ACTIONS, FURNITURE, PHYSICS. |
| Weapon effects and impacts | maintainer | WEAPON_EFFECTS. |
| Pip-Boy (except Repair and weapon mods), start menu, radio, local map | maintainer | PIPBOY. |
| Terminals and hacking | Chazm | |
| Item scripts, Repair, weapon mods | Chazm | The Pip-Boy's Repair and Mod pages are Chazm's. |
| Companions, Caravan, casinos (base game) | Chazm | Dead Money's versions overlap: coordinate. |
| Performance (M4) | Chazm | Measure before and after; no behaviour changes. |
| Dead Money (DLC01), crafting and the recipe menu | Dead Money contributor | docs/HANDOFF_DEAD_MONEY.md. |
| macOS / Metal port | open | A fork has a start (`abusager13`); claim it on GitHub. |

Everything else is open through the `[task]` issues.

## What every pull request must show

1. **Checks.** From the root: `cargo test --workspace`,
   `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`.
   In `viewer/`: `cargo test`, `cargo clippy --all-targets`,
   `cargo fmt --all -- --check`, `cargo build --release`. GitHub runs the
   same checks (`check (core)`, `check (viewer)`) and they must pass.
   `cargo test` includes a repository hygiene check that rejects game or
   binary files, absolute user-profile paths and Ghidra's automatic names
   (`FUN_…`, `DAT_…`): write the address instead.
2. **Acceptance routes.**
   `powershell -File scripts\acceptance.ps1 -Data "<your Fallout New Vegas\Data>"`
   replays Doc Mitchell's walk, Back in the Saddle and Ghost Town Gunfight
   in the release viewer. All three must pass, or the pull request says
   which one failed and shows it fails the same way on `main`. The
   gunfight (and, rarely, Back in the Saddle) depends on timing and
   chance: if one fails once, run it again and report both. Paste the
   result table.
3. **Provenance.** Every behaviour comes from the original game:
   translated from `FalloutNV.exe` 1.4.0.525 with the address in a
   comment ([ADR-0003](docs/adr/0003-decompiled-code-in-the-project.md)),
   named from the Xbox prototype symbols where they help
   ([ADR-0002](docs/adr/0002-xbox-prototype-symbols.md)), read from the
   game's data, or recorded from the original game. No stand-ins, no
   "close enough" constants. Anything you couldn't trace is listed in the
   pull request and must not change the base-game routes.
4. **Evidence of play.** Say what you checked by playing (keys, mouse or
   pad) and what you only checked by script (`--run`, `--say`, `--use`,
   `--key-at`) or unit test. Keep logs and screenshots private, or check
   them for usernames and local paths before attaching.
5. **Tests.** A regression test for each behaviour fix, with inputs built
   from scratch by the `testdata` crate. Tests never need game files.
6. **Docs.** The system's topic page in `docs/` says what is done, what is
   traced and what is missing. Add a short line to
   [docs/MILESTONES.md](docs/MILESTONES.md).
7. **AI assistance.** Say which tools helped and with what. You remain
   responsible for the change: review the code and the claims, and run
   the checks yourself.

Never commit game files, extracted assets, executable bytes, decompiler
databases or exports, recordings or logs.

## How the code is laid out

Game rules belong in the core crates (`world`, `physics`, `preview`,
`cellview`, `script`, `ui`); the viewer (`viewer/`, Bevy 0.16, its own
workspace) only draws, plays sound and reads input. Menus use the game's
own XML through `ui`. Core crates use no external libraries. Keep the
player/camera, aim/head, action-input and per-view boundaries in
[docs/VR.md](docs/VR.md). The method is in
[docs/METHODOLOGY.md](docs/METHODOLOGY.md), the research tools in
[`research/`](research/) and the day-to-day steps in
[docs/RESEARCH_WORKFLOW.md](docs/RESEARCH_WORKFLOW.md).

## How pull requests are merged

`main` only changes through pull requests. Each needs the two CI checks
and a review from the maintainer (CODEOWNERS), and is squash-merged. Keep
your branch rebased on `main` (at least daily while it's open, and right
before asking for review). When two traced versions of the same thing
disagree, say so in the pull request; the maintainer decides.

The maintainer has the final say on priorities, design and what is
merged ([docs/GOVERNANCE.md](docs/GOVERNANCE.md)).

## Handoffs

If you stop partway, leave the next person (or agent) a short note in the
system's topic page: what works, what's traced, what's left, and one next
action. Update the milestone tracker with evidence, not just claims.
