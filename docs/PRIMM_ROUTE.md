# Primm deputy and sheriff route

Issue #13 adds an acceptance route for *My Kind of Town* through Deputy
Beagle's rescue and the Primm Slim sheriff outcome. The route drives quest
stages through the quest's own dialogue and scripts; it does not set a quest
stage or variable directly. It remains **PARTIAL** by the project's play
standard: the acceptance script uses console `MoveTo` commands and
`StartConversation` in place of walking and activating with keys or a pad. It
relocates Beagle to the player before his sheriff dialogue because the
viewer's current NPC navigation can strand him after his leave package. The
quest dialogue and result scripts still run normally. The route has not been
compared side by side with the original game.

## Quest evidence

These records were read from `FalloutNV.esm` with `nvinspect`:

- Nash's INFO `0015A78D` sets `nVPrimmDeputyConv.DeputyHostage` and starts
  stage 20.
- Beagle's release INFO `000BACD5` runs stages 20 and 25, stops combat, and
  calls `SetRestrained 0`. His `PrimmDeputyLeaveBison` package (`000DA117`)
  targets marker `000CD9B1`; its end/change actions update `BeagleCaptured`
  and `BeagleFollow` and clear his captive state.
- Nash's INFO `00162C19` offers the optional Slim objective.
- Primm Slim's Science INFO `000EC078` sets `BeagleCaptured` to 4 and
  `PrimmSlimSheriffState` to 1, disables the dead sheriff references, sets
  stage 130, and awards 30 XP. The stage script completes the remaining
  objectives and awards 300 XP.

The cross-place path used by Beagle is the game's navmesh-info search. Its
edge expansion (`FalloutNV.exe 006b8490`) includes the `NVCI` third-list door
edges and adds 409600 to a locked-door edge; the surrounding search is
`006b8c50` and `006b9180`. The implementation now resolves those door edges
from paired `XTEL` records in `crates/world/src/ai/navinfo.rs`, and
`door_toward` takes the first door on the resulting route. Generated test data
covers a multi-door chain and a locked door changing the chosen route.

## Acceptance route

Run the opt-in route with:

```powershell
powershell -File scripts\acceptance.ps1 -Routes primm -Data "<Fallout New Vegas\Data>"
```

It uses `player.ModAV Health 5000` and `player.SetAV Science 30` as test
setup; moves the player to Beagle, Nash (`PrimmJohnsonNashRef`, `000E2882`),
Slim (`PrimmSlimREF`, `000E288C`) and the exit marker; moves Beagle to the
player before the sheriff conversation; starts conversations with
`StartConversation`; and picks offered topics with `--say`. No quest stage or
quest variable is forced. The route's expected results are the Slim sheriff
message and `XP +300` / `XP +30`.

The route is opt-in because it takes several minutes; the default acceptance
set remains `doc,vcg02,vms16`. The latest acceptance run passed end to end.
Beagle's sheriff conversation selected INFO `000DA106`, which set
`BeagleCaptured` to 3. Nash then offered “What about Primm Slim? Could he be
sheriff?”, and selecting it ran INFO `00162C19` and displayed the optional
objective. The route selected Slim's Science 30 response; INFO `000EC078`
succeeded, the sheriff message appeared, and the viewer reported `XP +300`
and `XP +30`. Selecting the direct Nash topic avoids opening the unrelated
general-topic list.

The route remains PARTIAL: player movement to Beagle, Nash, Slim, and the exit
marker, Beagle's relocation to the player, and starting each conversation are
still forced by console commands. It was exercised in the Linux viewer but has
not been compared side by side with the original game. NPC obstacle replanning
remains unfinished shared AI behavior; this run does not establish a Linux-only
navigation defect.
