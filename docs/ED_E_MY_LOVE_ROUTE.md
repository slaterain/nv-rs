# ED-E My Love: Primm route

Quest `vDialogueEDE` (`001572E8`), quest script `vDialogueEDESCRIPT`
(`00157F1F`), ED-E script `EDEScript` (`00151D03`). This route is **PARTIAL**
and stays opt-in (`pwsh -File scripts/acceptance.ps1 -Routes ede -Data <Data>`).
It has not been compared side by side with Fallout: New Vegas.

## Final route evidence

On 2026-10-21, the release viewer completed the route on macOS arm64 in 257
seconds (`ede` acceptance route). The route code was from this rebased PR; the
macOS rendering changes from PR #108 were applied temporarily to run it on this
host. The Mac output device was muted during this run, so audio behavior was
not verified. The route's required INFOs, handover and return
notice matched; the log records `Quest completed: ED-E My Love` and `XP +100`.
The screenshot is `ede.png` in the acceptance output directory.

| Checkpoint | Viewer log evidence |
| --- | --- |
| Both log packages played; their result scripts advanced `iLogsPlayed` from 0 to 2 | INFO `00157F17`, `00158829` |
| ED-E greeting and both radio messages ran | INFO `001579D7`, `001579CE`, `0015FF67`, `0015FF65` |
| Holotape OnAdd branch and Hidden Valley discovery ran | adding `HVMissionDisc01`; proximity discovery at `HiddenValleyMarkerREF` |
| April's Followers handover ran | INFO `00160411`; quest script performed the upgrade swap |
| ED-E returned through the Pip-Boy Menumode step | `ED-E has returned to Primm.` and objective 60 completion |
| Completion package and reward ran | `EDEQuestCompleteDialogue End action`, `Quest completed: ED-E My Love`, `XP +100` |

The route does not set `iPlayRadio` or `iEDEOut` directly. It does force
prerequisites and inputs: ED-E's hire/day state, two stage-10 dispatches, the
calendar advances, cleanup of the finished log package, holotape addition,
teleports and conversation starts. The holotape OnAdd, marker proximity,
handover, upgrade, Pip-Boy return, completion and reward scripts execute.
The XP log is present, but the STATS meter was not visually checked.

## Translation evidence

**Type-1 package location.** A `PLD2` kind-1 location names a cell. The game
gets that cell through `PackageLocation::GetLocCell` (`0067f3e0`) and checks
the actor against the package location in `00676390`. The viewer now matches
the actor's current cell in `crates/world/src/ai.rs`; a generated type-15
package fixture tests matching and non-matching cells.

**Pip-Boy Menumode.** The game's Pip-Boy menu classes are listed at `00702680`;
`MenuMode`'s class predicate is at `0059c380`, and quest block dispatch is at
`005c4240` (FalloutNV.exe 1.4.0.525). `Pipboy::top_class` now contributes the
open menu class to `run_scripts`' existing menu list, which dispatches it once
with the other menu classes. Tests cover inclusion, duplicate suppression,
and invalid/absent classes.

**Map discovery.** The game's proximity loop at `00779070` checks the player's
3D distance against the marker radius, then sets visible and travel bits via
`0044de40` and `0044de80`. `ShowMap` at `005c8620` can set visibility alone
or both bits; `GetMapMarkerVisible` at `005daac0` reads the 0/1/2 state through
`005a51e0`. The Hidden Valley marker (`000CDA74`) has `XRDS = 10,000`. In the
viewer, `world::map::discover` records a discovered marker, which the map
state exposes as both visible and travelable; the acceptance teleport enters
the proximity path instead of setting either flag directly.

## Remaining gaps

- The passing route uses forced hire/day prerequisites and stage dispatches;
  natural log triggers and waiting five in-game days were not verified.
- Package cleanup after log two is forced. The original game's live cleanup
  behavior and radio package selection have not been compared.
- Adding the holotape and teleporting to Hidden Valley do not test looting the
  body or walking the route.
- No side-by-side original-game comparison was made, and the XP meter was not
  visually checked.
