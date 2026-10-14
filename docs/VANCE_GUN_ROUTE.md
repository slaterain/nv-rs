# Primm route: Vance's Gun

Issue #13's second Primm route follows *Vance's Gun* (`000E2891`, quest
script `PrimmVanceGunScript`, `000E28A9`) from the empty museum case to the
Wins' safe being unlocked. The quest script documents `VanceGunCase` as 0
(case not found), 1 (case found), 2 (hacked file found), and 3 (quest
complete); `SammyPauline` is 0 (they have the gun), 1 (Pauline wants the
player to take it), and 2 (safe unlocked).

## Quest script path

These records were read from the installed `FalloutNV.esm` with `nvinspect`:

| Step | Record/script | Result |
| --- | --- | --- |
| Inspect the empty display case | Placed activator `000E288A`, script `PrimmMuseumPlaqueScript` `000E288F` | Shows the plaque message, sets `VanceGunCase` to 1, and adds `PrimmSlimGun` topic. |
| Ask Slim what happened | Science success INFO `000E287F` in DIAL `000E2875` (requires Science >= 50) | Sets `VanceGunCase` to 2, adds `VanceGunNote1`, shows the West Side marker, and awards 50 XP. |
| Ask Pauline about the theft and plan | INFO `000BACC4` (`I know the two of you stole Vance's gun down in Primm.`), then INFO `000E32E9` (`What's your plan?`) | Reveals that the pair plan to rob casinos using the gun. `PaulinePlan` (`000E32E7`) has no top-level flag; INFO `000BACC4` has neither `TCLT` nor `NAME`, and no parsed script adds the topic. Acceptance therefore forces `player.AddTopic 000E32E7` to reach this actual quest dialogue. The viewer's player-topic behavior follows the traced menu code (`FalloutNV.exe` 1.4.0.525, `00619030` adds top-level topics and `00952830` handles `AddTopic`). |
| Convince Pauline | Success INFO `000E32EA` in DIAL `000E32E6` (requires Speech >= 55; the choice is tagged `[Speech 55]`) | Sets `SammyPauline` to 1 and awards 55 XP. |
| Ask Sammy for the gun | INFO `000E32EB` in DIAL `000E32E5` | Sets ownership on `SammySafeREF` (`000E32F2`), unlocks it, sets `VanceGunCase` to 3, and sets `SammyPauline` to 2. `SammyPaulineConvinced` (`000E32E5`) is non-top-level; INFO `000E32EA` has no `NAME` link, so acceptance forces `player.AddTopic 000E32E5`. |

The final INFO's script reaches the quest script's explicitly named complete
state. The route stops at that point; it does not claim the weapon has been
transferred from the unlocked safe into the player's inventory.

## Acceptance route

Run the opt-in route with:

```powershell
powershell -File scripts\acceptance.ps1 -Routes vance -Data "<Fallout New Vegas\Data>"
```

The viewer's `--use 000E288A` test input presses E on the placed museum case
and runs its real `OnActivate` script. The `--say` inputs select the topics
through Slim's, Pauline's, and Sammy's real dialogue INFO records. The
acceptance route explicitly adds Pauline's otherwise-unavailable plan topic
with `player.AddTopic 000E32E7`; the topic is not top-level and no parsed
FalloutNV.esm script or the theft INFO adds it. This route therefore records
that topic as forced rather than claiming the whole path is naturally
available. It also forces Sammy's non-top-level `SammyPaulineConvinced`
topic with `player.AddTopic 000E32E5`, because INFO `000E32EA` does not add
it. No quest stage or quest variable is forced. The setup sets Science
to 50 and Speech to 55, and uses console `MoveTo` and `StartConversation`
lines to move the player and start each conversation.

**Status:** the complete acceptance route passed on the Linux viewer built
from current `main` (`56a3d102c79cd6a5bc34160febfeb8af87bd763b`). The
cross-cell move into Wins Residence needs five game seconds before
`PaulineWinsREF.StartConversation`; without that gap the new scene has not yet
spawned its talkers. The final INFO unlocked Sammy's safe, set `VanceGunCase`
to 3 and `SammyPauline` to 2, and awarded the expected 50 and 55 XP. The
successful Speech check was explicitly set to 55; Pauline's exit choice was
selected by its visible `go` text. It has not been compared side by side with
the original game. Logs and screenshots remain private under
`target/task13-validation/vance/`.

## Current handoff (2026-10-08)

- Branch: `codex/issue-13-vances-gun`, based on current `main`
  `56a3d102c79cd6a5bc34160febfeb8af87bd763b` (after PR #26).
- Changed files: `scripts/acceptance.ps1`, `docs/MILESTONES.md`, and this
  route/handoff. No process remains active.
- Verified on the current-main Linux viewer: the run exited 0, reached all
  eight acceptance markers, unlocked the safe, and had no panic. Pauline was
  loaded before her conversation started. One unrelated background warning
  remains for persistent Freeside actor HadrianREF (`0011F9C1`). The
  PowerShell wrapper could not be run here because `pwsh` is not installed;
  the exact viewer arguments in its route were run directly.
  `git diff --check` passes.
- Still open: compare the route with the original game; PR #29 remains open
  with changes requested, and this route is in draft PR #50. Next: human and
  maintainer review on #50, then keep issue #13 open until both routes merge.
