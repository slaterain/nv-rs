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
edges. It measures the walk to the near teleport point and the walk from the
far teleport point, adds them, multiplies by the current info's `k` and 3,
then adds the path cost and any door penalty. The linked door's navmesh portal
identifies the destination info. Disabled doors and destination infos marked
0x10 are skipped; the request's lock disposition determines whether an
unopenable door is skipped, ignored, or penalized. Minimal-use doors carry the
penalty. The surrounding search is `006b8c50` and `006b9180`.
`door_toward` takes the first door on the route. Generated data covers a
multi-door chain and door selection, including keys and ownership. The
alarmed-guard check uses the world's current-package model; it has not been
compared with the original process state. The cross-place caller uses the
default lock disposition, and same-space `virtual_path` searches still omit
door edges. The parent-edge failure condition is retained.

## Acceptance route

Run the opt-in route with:

```powershell
powershell -File scripts\acceptance.ps1 -Routes primm -Data "<Fallout New Vegas\Data>"
```

It uses `player.ModAV Health 5000` and `player.SetAV Science 30` as test
setup; advances Nash's opening greeting once with a timed Space input so the
dialogue menu does not pause the route; moves the player to Beagle, Nash
(`PrimmJohnsonNashRef`, `000E2882`),
Slim (`PrimmSlimREF`, `000E288C`) and the exit marker; moves Beagle to the
player before the sheriff conversation; starts conversations with
`StartConversation`; and picks offered topics with `--say`. It moves to Nash
at game time 370 seconds, waits one second for the destination cell to load,
then starts the conversation at 371 seconds before Nash leaves for his idle
marker. No quest stage or quest variable is forced. The route's expected
results are the Slim sheriff message and `XP +300` / `XP +30`.

On 2026-10-31 the rebased viewer at `22e8d69b` ran this route on Fedora in a
native Wayland session using installed game data. The first run ended at 500
seconds while Slim's voiced response was still playing and missed all three
expected results. With the route limit extended to 650 seconds, a fresh run
recorded the sheriff result and both XP awards, exited 0 without a panic, and
passed the PowerShell acceptance check in 656 seconds. This used a development
build with reduced optimization for `engine-fallout` and `engine-havok` to
fit the machine's memory, not a release build.

The route is opt-in because it takes several minutes; the default acceptance
set remains `doc,vcg02,vms16`. A Linux viewer run on 2026-10-08 completed the
route with exit code 0 and logged the sheriff result, `XP +300` and `XP +30`,
with no panic. The route also passed on macOS on 2026-10-20. Beagle's sheriff
conversation selected INFO
`000DA106`, which set `BeagleCaptured` to 3. Nash then offered “What about
Primm Slim? Could he be sheriff?”, and selecting it ran INFO `00162C19` and
displayed the optional objective. Slim's Science 30 response ran INFO
`000EC078` and completed the quest. Selecting the direct Nash topic avoids
opening the unrelated general-topic list. Nash's exit wording varies by
greeting (observed as “See you later,” “Goodbye,” “Time for me to go,” and
“I need to get going”); the acceptance selector supports these alternatives
and closes the conversation before starting Slim's dialogue. It then selects
Slim's Science 30 response, closes with “Goodbye,” and checks the sheriff
result plus both XP awards.

The route remains PARTIAL: player movement to Beagle, Nash, Slim, and the exit
marker, Beagle's relocation to the player, and starting each conversation are
still forced by console commands. No run has been compared side by side
with the original game. NPC obstacle replanning remains unfinished shared AI
behavior; these runs do not establish a Linux-only navigation defect.
