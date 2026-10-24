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
edges. Each door edge uses distance between the two info positions × the
current info's `k` × 1, then adds the path cost so far `g`, then adds 409600
for a locked door. The surrounding search is `006b8c50` and `006b9180`. The
implementation resolves door edges from paired `XTEL` records in
`crates/world/src/ai/navinfo.rs`, and `door_toward` takes the first door on the
resulting route. Generated test data covers a multi-door chain and a locked
door changing the chosen route. When a paired door points back to the current
node's parent, the search skips that reverse edge and continues expanding the
other doors; it does not abort the whole search.

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

The route is opt-in because it takes several minutes; the default acceptance
set remains `doc,vcg02,vms16`. On 2026-10-20 the route passed on macOS against
the installed game data: it exited 0 and logged the sheriff result, `XP +300`
and `XP +30`, with no panic. The test used the current PR #29 code and a
temporary overlay of PR #108's Metal renderer changes. A PR #29-only Metal
run had hit the device's resource limit before the sheriff result, so this
pass verifies the quest route with the Mac renderer fix present; it is not a
PR #29-only renderer run. Beagle's sheriff conversation selected INFO
`000DA106`, which set `BeagleCaptured` to 3. Nash then offered “What about
Primm Slim? Could he be sheriff?”, and selecting it ran INFO `00162C19` and
displayed the optional objective. Slim's Science 30 response ran INFO
`000EC078` and completed the quest. Selecting the direct Nash topic avoids
opening the unrelated general-topic list. Nash's exit wording varies by
greeting (observed as “See you later,” “Goodbye,” “Time for me to go,” and
“I need to get going”); the acceptance selector supports these alternatives
and closes the conversation before starting Slim's dialogue. It then selects
Slim's Science 30 response, closes with “Goodbye,” and checks the sheriff
result plus both XP awards. The game `Data` folder is available on this
machine.

The route remains PARTIAL: player movement to Beagle, Nash, Slim, and the exit
marker, Beagle's relocation to the player, and starting each conversation are
still forced by console commands. It was exercised in the Linux viewer but has
also passed on macOS with PR #108's Metal renderer overlay, as recorded in
`docs/PLAYTESTING.md`. Neither run has been compared side by side with the
original game. NPC obstacle replanning remains unfinished shared AI behavior;
this run does not establish a Linux-only navigation defect.
