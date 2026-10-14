# Primm route: ED-E My Love (`vDialogueEDE`)

Issue #13, ED-E My Love. Quest `vDialogueEDE` **001572E8**, quest script
`vDialogueEDESCRIPT` **00157F1F**, ED-E's own script `EDEScript` **00151D03**.
Installed data, viewer release build. Logs and screenshots are private and
never committed.

Quest records and script behavior were checked in the installed data and
FalloutNV.exe 1.4.0.525. The viewer evidence below has not been compared
side-by-side with a run of the original game. "Reached" means the viewer's own
scripts, packages and dialogue did it; steps marked *forced* use console lines
to establish their prerequisites, which are listed here.

## The quest, from the data

The quest is start-game-enabled and allows repeated stages (`DATA` 0x00004B19).
Its variables, by the slot number a `GetQuestVariable` names (`SLSD`/`SCVR` in
SCPT 00157F1F): `iLogsPlayed` 1, `fLastLogDay` 2, `iPlayRadio` 4, `iEdeRadio` 5,
`iEDEOut` 6, `iEDEDays` 7, `iHVUpgrade` 8, `iFolUpgrade` 9, `iEDEDaysPassed` 10,
`iAprilDead` 11, `iCounter` 12, `bEDEExamined` 13, `bGibsonOnce` 14, `iDoOnce`
15, `bCompleteOnce` 17.

| Stage | Flag | Set by | Result |
| --- | --- | --- | --- |
| 10 | — | INFO results that call `SetStage vDialogueEDE 10` (Old Lady Gibson's greeting, INFO 000841E7, is the Primm one) | the stage-10 entry gate must pass: `GetScriptVariable(EDE1Ref 001732D1, 16) == 0`, `GetQuestVariable(vDialogueEDE 001572E8, 10) >= 5`, and `GetQuestVariable(VNPCFollowers 000B16D0, 30) == 1`. Its first passing entry runs the log dispatcher: `fLastLogDay` = the day, objective 1; with `iLogsPlayed == 0`, ED-E gets `EDEDialoguePackage02` (0015882D, topic `EDELog1`) and objective 5 |
| 10 | — | a second `SetStage vDialogueEDE 10` while `iLogsPlayed == 1` | assigns `EDEDialoguePackage03` (0015882E, topic `EDELog2`) and immediately sets stage 20 |
| 20 | — | the stage-10 dispatcher | package dispatch only; the log is not confirmed until INFO 00158829 runs and sets `iLogsPlayed` to 2 |
| 100 | **0x02 FAILS_QUEST** | `GetQuestCompleted == 0` and both `VFSEDEScientistRef.GetDead` and `HVKnightEDERef.GetDead` | the quest fails (its failure notice) |

The logs themselves: INFO 00157F17 (`EDELog1`, sets `iLogsPlayed` to 1) and INFO
00158829 (`EDELog2`, sets it to 2), each with `EDEContinue…` and `EDEEndLog`
follow-ups.

Stage 10's gate is in the `vDialogueEDE` quest data. Slot 10 is
`iEDEDaysPassed`; `iCounter` is slot 12 and is not this five-day condition.
`GetScriptVariable(EDE1Ref, 16)` reads `bSameCell` from ED-E's script
`00151D03`: it is set while the player shares the cell of `rLogBlocker` and
cleared after leaving. The other conditions are `iEDEDaysPassed >= 5` and
`VNPCFollowers.bEDEHired == 1`. The quest script recalculates the day counter
from `GameDaysPassed - fLastLogDay`, and every successful stage-10 entry
records the current day, so another log dispatch needs five days after the
previous stage-10 entry.

There are many dialogue INFO results that call `SetStage vDialogueEDE 10`;
Doctor Henry's top-level scientist topic, DIAL `00144BAD`, INFO `00144BAF`, is
one valid trigger. It only checks that Henry is speaking, then calls
`SetStage`. Old Lady Gibson's greeting INFO `000841E7` is another early
trigger; if ED-E is hired, it also sets `bGibsonOnce`, so the later HELIOS
topic INFO `0014253F` cannot be used as a repeat trigger after that flag is
set.

The game records make the log-two boundary explicit. PACK `0015882E`
(`EDEDialoguePackage03`) is type 15 and its dialogue data points to DIAL
`00158826` (`EDELog2`). Its INFO `00158829` contains the result script that
sets `iLogsPlayed` to 2 and adds the `EDELog2` topic. The stage-10 dispatcher
sets stage 20 immediately after assigning the package, before that INFO can
run; stage 20 therefore means “log-two package queued,” not “both logs heard.”
The INFO's player prompt is `<Log 2 Playback::Begin>`; its two recorded
response fragments are “Download Complete. Begin Recording.” and the Navarro
message. It offers two `<Log Playback::Continue>` follow-up topics (`00158825`
and `00157F13`). The quest variable changes in the INFO's start-result script,
so observing that INFO execute is the useful log-two checkpoint; stage 20
alone is not.

The latest valid release-viewer probe selected Henry's INFO `00144BAF` after
initializing `iLogsPlayed=1`, `iEDEDaysPassed=5`, ED-E's hired/party state and
teammate state before the first frame. The stage-10 gate passed and the quest
reported its stage-20 dispatch. The conversation remained open and the log
contains no execution of INFO `00158829`; this probe does **not** prove that
log two played. It forced the prior-log/day/hire prerequisites and did not
replay log one or wait five days naturally. The actual INFO result remains the
acceptance checkpoint for log two.

The rest of the quest script's GameMode, in order:

* the radio start branch: with `iEDEDaysPassed >= 2`, `iLogsPlayed >= 2`,
  `iPlayRadio == 0` and ED-E hired, the quest sets `iPlayRadio` to 1. If
  `HVKnightEDERef.GetDead` is true, it selects `iEDERadio = 4`; if the knight
  is alive but `GetStage VMS55 > 0`, it selects `iEDERadio = 1`; both call
  `EDE1Ref.EvaluatePackage` and leave `iPlayRadio` at 1. Only the remaining
  branch (knight alive and `VMS55` stage 0) selects `iEDERadio = 2`, calls
  `EvaluatePackage`, and sets `iPlayRadio = 2`. The source comment describes
  that condition as having been to Hidden Valley, but the actual gate is quest
  `VMS55`'s stage, not a location check;
* the note gate: with objective 10 displayed but not complete, the player has
  `HVPatrolNote01` (NOTE `0015A7BA`), and objective 12 is not complete, the
  script completes objective 10, displays and completes 12, displays and
  completes 13, and displays 15;
* the radio-2 state gate: with `iPlayRadio == 2`, objective 13 or 15 displayed,
  `HiddenValleyMarkerREF.GetMapMarkerVisible == 2`, and ED-E hired, the script
  sets `iEDERadio` to 3, evaluates ED-E's package, and sets `iPlayRadio` to 3.
  This selects the *radio-3* dialogue package; it is not playback of radio 2.
  Because this gate runs before the note gate, it can pass on the next
  quest-script tick after the note gate displays objectives 13/15 if the
  marker is already travelable and the radio-start branch selected radio 2;
* the handover: `iEDEOut` 1 → 2 (records `iEDEDays` = the day ED-E was taken) →
  after `GameDaysPassed - iEDEDays >= 3` → 3; the Menumode block on 3 moves the
  upgraded `EDE2Ref`/`EDE3Ref` to `EDEHomeMarker` and sets 4; on 4, while the
  player is **outside** `PrimmNashResidence`, GameMode enables the upgraded
  ED-E, shows the return notice and completes objective 60;
* the upgrade swap (for whichever of `iFolUpgrade`/`iHVUpgrade` is 1): ED-E's
  items move to the new form, the old one is disabled and moved away, the perks
  and teammate flags are cleared, the follower count lowered, `iEDEOut` set to 1
  again;
* **completion**: `bCompleteOnce == 0` and `GetQuestCompleted == 1` and
  `GetStage < 100` → `RewardXP 100` once. This is the success reward, and it is
  gated on the completed flag being set *without* stage 100.

The physical pickup that supplies the required note is MISC `HVMissionDisc01`
(`0015D9DB`), with `HVMissionDisc01Script` (`0015D9D5`) on `OnAdd player`.
That script adds NOTE `HVPatrolNote01`, calls `ShowMap HiddenValleyMarkerREF`
with no second argument, increments `VDialogueHiddenValley.PatrolsFound`, and
if ED-E objective 12 is active, completes 12 and displays 13; it then removes
the holotape item. The other two patrol discs add different notes, so they do
not satisfy this ED-E gate. The one-argument `ShowMap` reveals the marker but
does not set its fast-travel bit. The later radio gate specifically requires
`GetMapMarkerVisible == 2` (visible and travelable), so the note pickup alone
does not satisfy it. FalloutNV.exe 1.4.0.525's marker-update loop at
`00779070` supplies the separate path: for an eligible marker, it compares
the squared 3D player-to-marker distance with the squared marker radius, then
sets the visible bit at `0044de40` and travel bit at `0044de80` when either
is absent. The radius comes from the marker's `XRDS` (helper `00568cb0`), or
`iMapMarkerRevealDistance` when `XRDS` is zero. Installed `FalloutNV.esm`
record `HiddenValleyMarkerREF` (`000CDA74`) has `XRDS = 10,000`; physically
entering that radius naturally makes the script's value 2. Existing
`world::map::discover` mirrors this rule and its 3D-distance boundary has a
regression test in `crates/world/tests/exterior.rs`. The acceptance route now
uses `player.MoveTo HiddenValleyMarkerREF`, a forced teleport that exercises
proximity discovery instead of directly setting the map and travel flags.

The required holotape is physically separate from the marker. Installed data
places MISC `HVMissionDisc01` (`0015D9DB`) in the inventory of `HVPaladinDead02`
(`0015A7B4`); the placed actor `HVPaladinDead02REF` (`0015A7BD`) is in the
interior cell `RepconHQ03` (`000E5A88`, REPCONN Office Top Floor). Looting the
body runs the holotape's `OnAdd player` script and reveals Hidden Valley, but
does not put the player inside the exterior marker's 10,000-unit radius. The
natural radio gate therefore requires a later trip from REPCONN to Hidden
Valley; adding the note or revealing the map entry alone cannot prove it. The
acceptance route now spawns the tape with `player.AddItem HVMissionDisc01 1`
and later teleports the player to the marker. This runs the item's OnAdd and
the viewer's proximity-discovery paths, but does not verify looting the corpse
or walking the route in the world.

The Followers handover is initiated by INFO `00160411`, reached from April
Martimer's “Ok, you can take it for a little while” topic. Its start-result
script clears the old objectives and displays objective 35. Its end-result
script applies the quick fade, sets `iFolUpgrade` to 1, delays the quest for one
second, and disables player controls. The next quest GameMode tick performs
the EDE1→EDE2 item/actor swap and sets `iEDEOut` to 1; the three-day timer and
Menumode steps then run as listed above. The parallel Hidden Valley choice is
INFO `0015A304`; its end-result sets `iHVUpgrade` to 1. These are distinct
script-driven branches, not interchangeable checkpoints.

The radio lines are dialogue packages, not Pip-Boy radio playback. In
FalloutNV.exe 1.4.0.525, the `EvaluatePackage` script command is at `005c95a0`
(function-table entry `011917c0`); its body is not present in the available
decompilation, so the exact package-reselection behavior is inferred from the
quest script and package records. The quest sets `iEDERadio` and calls
`EDE1Ref.EvaluatePackage`. EDE1Ref's package list maps values 1/2/3/4 to
dialogue packages `0015FF68`/`00160417`/`00160418`/`00160419`, whose topics are
`0015FC73`/`0015FF64`/`0015FF63`/`0015FF62`. Each package also requires
`VNPCFollowers.bEDEHired == 1`, the matching `iEDERadio` value, and
`GetInSameCell(PlayerRef) == 1`; its target is the player at 64 units. The
higher-priority default `EDEDialoguePackage` applies while `GetTalkedToPC == 0`
and has no radio topic. Thus a quest-variable transition or `EvaluatePackage`
call alone does not prove a radio line played: ED-E must be in the player's
cell, the hire/talk state must allow the radio package, and that package's
dialogue must start. Radio INFO results reset `iEDERadio` to 0; INFOs
`0015FF65`, `0015FF66`, `0015FF67`, and `0015F8D5` contain those resets.

The completed flag is set by ED-E's own package: **PACK 0016041A
`EDEQuestCompleteDialogue`**, type 15 (Dialogue), the **first** package in
`EDE2Ref` (ACRE 001732D0, base CREA 001694E2) and `EDE3Ref` (ACRE 001732CF, base
CREA 001694E0) base lists. Its condition is `GetQuestVariable(vDialogueEDE, 6)
== 4`, i.e. `iEDEOut == 4`; its End action's script is `set vDialogueEDE.iEDEOut
to 5` then `completequest vDialogueEDE`. Its own dialogue procedure takes it to
`EDEHomeMarker` (REFR 000A23D1) and talks to the player while the player is in
the second location, which the package names as the **cell** `PrimmNashResidence`
(PLD2 kind 1, form 000D70E2).

## How far the route got

The acceptance route (`ede` in `scripts/acceptance.ps1`) now plays both log
INFOs, both radio messages, April's Followers handover, the three-day wait,
the Pip-Boy return step, ED-E's completion package and the 100 XP reward. It
still forces the hire/day prerequisites and stage triggers, package cleanup,
holotape spawn, travel, and conversation starts. The holotape's OnAdd and
Hidden Valley's proximity-discovery scripts run. The player begins outside
`PrimmNashResidence`, satisfying the stage-10 `bSameCell == 0` gate, while
EDE1Ref is moved beside the player. The quest selects its radio branch after
the log-two result and day advance; acceptance does not set `iPlayRadio`:

The verified Linux run invoked the release binary directly at
`viewer/target/release/nv-viewer`; Windows builds use `nv-viewer.exe`. The
route is also listed in `scripts/acceptance.ps1`, whose executable-path
selection remains Windows-specific.

```
viewer/target/release/nv-viewer <Data> WastelandNV --at -67845,3000,8400,180
  --run "StartQuest vDialogueEDE"
  --run "set vDialogueEDE.iLogsPlayed to 0"
  --run "set vDialogueEDE.iEDEDaysPassed to 5"
  --run "set VNPCFollowers.bEDEHired to 1"
  --run "SetObjectiveDisplayed vDialogueEDE 10 1"
  --run "EDE1Ref.Enable"
  --run "EDE1Ref.MoveTo player"
  --run-at 1 "SetStage vDialogueEDE 10"
  --run-at 20 "set vDialogueEDE.iEDEDaysPassed to 5"
  --run-at 21 "SetStage vDialogueEDE 10"
  --run-at 45 "set GameDaysPassed to 10"
  --run-at 46 "EDE1Ref.RemoveScriptPackage EDEDialoguePackage"
  --run-at 60 "player.AddItem HVMissionDisc01 1"
  --run-at 75 "player.MoveTo HiddenValleyMarkerREF"
  --run-at 77 "player.MoveTo EDEHomeMarker"
  --run-at 100 "player.MoveTo VFSEDEScientistRef"
  --run-at 116 "VFSEDEScientistRef.StartConversation player"
  --run-at 145 "set GameDaysPassed to 13"
  --run-at 185 "player.MoveTo EDEHomeMarker"
  --run-at 200 "EDE2Ref.StartConversation player"
  --say-id 001579CE --say "End" --say "End"
  --say "Ok, you can take it for a little while" --say "Log Off"
  --key-at 160 tab --key-at 164 tab --wait 240
  --answer-boxes --box-answers 2 --screenshot <private>
```

| Step | Evidence |
| --- | --- |
| Log one plays through its package | INFO `00157F17`; its result sets `iLogsPlayed=1` |
| Log two plays through its package | INFO `00158829`; its result sets `iLogsPlayed=2` and stage 20 was queued |
| ED-E's greeting and radio packages run | INFO `001579D7` GREETING, Goodbye `001579CE`, and radio INFOs `0015FF67` and `0015FF65` appear |
| Holotape and note branch run | adding `HVMissionDisc01` runs its OnAdd script, completes objective 12 and displays the objective to speak to Knight Lorenzo |
| Hidden Valley is discovered | moving the player to `HiddenValleyMarkerREF` runs proximity discovery and sets the marker's travel state |
| Followers upgrade runs | INFO `00160411` sets `iFolUpgrade=1`; the quest swaps actors and later reports the upgrades complete |
| Return step runs | Pip-Boy Menumode moves `EDE2Ref` to `EDEHomeMarker`; GameMode displays "ED-E has returned to Primm" and completes objective 60 |
| ED-E's completion package runs | the log records `EDE2Ref: package EDEQuestCompleteDialogue End action` |
| the quest completes | `Quest completed: ED-E My Love` |
| the reward branch fires | `XP +100` appears in the viewer log; the STATS total was not visually verified |

Verified in the Linux release viewer (Vulkan backend). The successful run
played log-one INFO `00157F17` and log-two INFO `00158829`, advancing
`iLogsPlayed` from 0 to 1 to 2. After the forced calendar advance and package
cleanup, the quest selected and played radio INFO `0015FF67` from DIAL
`0015FF64`, then radio INFO `0015FF65`. April's handover INFO `00160411`
set `iFolUpgrade=1`; the quest's GameMode script swapped ED-E's actors and
recorded the day. After the forced three-day calendar advance, opening and
closing the Pip-Boy ran the quest's Menumode return block, which moved
`EDE2Ref` to `EDEHomeMarker`. The GameMode return block displayed "ED-E has
returned to Primm", enabled the upgraded actor and completed objective 60.
ED-E's completion package then ran its End action, completed the quest and
logged `XP +100`. The route does not set `iPlayRadio` or `iEDEOut` directly.
Its setup, stage dispatches, package cleanup, holotape spawn, travel and
conversation starts remain forced; the item's OnAdd and Hidden Valley's
proximity-discovery scripts, handover, wait, return, completion and reward
scripts run. The STATS total was not visually checked, and this route has not
been compared side-by-side with the original game.

A separate targeted viewer probe forced `iLogsPlayed = 2`, the day/hire
prerequisites, and EDE1Ref's placement in the player's cell, then ran
`EDE1Ref.StartConversation player` and chose `Log Off`. The viewer executed
INFO `0015FF67` from radio-2 DIAL `0015FF64`, whose result resets
`iEDERadio` to 0. Re-reading the private log with Empryo showed an
EDE1Ref dialogue-package Talk step immediately before that line. The earlier
claim that no package started was a false negative: this type-15 package has
no nonempty Begin action, so the viewer emits no `package … Begin action`
line for it. The INFO belongs to the radio-2 DIAL used by PACK `00160417`, so
the viewer did select and run the actor's radio package after the forced
conversation closed. That close also re-evaluates the actor in the viewer;
this probe therefore does not prove that the quest's own `EvaluatePackage`
call caused the selection, or that the original game plays it identically.
The log/day/hire state, actor placement and initial conversation were forced.
The logs remain private.

A corrected follow-up probe reached the same radio INFO without forcing a
conversation: it started `vDialogueEDE`, set `iLogsPlayed=2`,
`iEDEDaysPassed=5` and `bEDEHired=1` before enabling ED-E, moved EDE1Ref into
the player's cell, handled the startup message boxes, and selected the offered
Goodbye INFO `001579CE` from ED-E's natural GREETING `001579D7` using
`--say-id`. After that conversation closed, EDE1Ref began a second Talk step
and ran radio INFO `0015FF67` from DIAL `0015FF64`; its result reset
`iEDERadio` to 0. The probe did not use `StartConversation`, `AddScriptPackage`
or set `iEDERadio` directly. This verifies the viewer's radio-package path
after a natural greeting under forced quest prerequisites. Since the viewer
also reevaluates actor packages each frame, it does not isolate the quest's
explicit `EvaluatePackage` call as the cause, and it has not been compared in
a live run of the original game. The private probe log is not committed.

The radio/note state gate was first checked separately in a background
release-viewer run, without enabling or moving ED-E and without starting
dialogue. That earlier probe forced `iPlayRadio = 2`, so it tested only the
later state gate. Its fixture was:

```
StartQuest vDialogueEDE
set vDialogueEDE.iLogsPlayed to 2
set vDialogueEDE.iEDEDaysPassed to 5
set VNPCFollowers.bEDEHired to 1
SetObjectiveDisplayed vDialogueEDE 10 1
ShowMap HiddenValleyMarkerREF 1
player.AddNote HVPatrolNote01
```

The first tick saw objective 10 displayed and unfinished, with the note absent
and marker hidden. After the last two lines, the note was present and marker
value was 2; the note gate completed objectives 10/12 and displayed 13/15.
On the following tick the quest's own script advanced `iPlayRadio` 2 → 3 and
`iEDERadio` 0 → 3 with `bEDEHired` still 1. This validates the viewer's
ordered note and radio-2 *state* gates from forced prerequisites; it does not
play any radio conversation. Forcing `iPlayRadio = 2` skips the initial branch
that selects `iEDERadio = 2`; the later gate instead selects `iEDERadio = 3`.
This probe had no enabled EDE1Ref in the player's cell, so no radio package
could satisfy its same-cell condition. That earlier probe forced the note and
marker directly. The current acceptance route leaves `iPlayRadio` at 0,
places EDE1Ref with the player, handles its greeting with `--say-id 001579CE`,
and observes the radio INFO before adding the holotape and teleporting to the
marker. Its final return state remains forced.

The original engine's package path explains the follow-up. In FalloutNV.exe
1.4.0.525, `AddScriptPackage` (`005cc4f0`) installs the new package and
evaluates the actor. A type-15 dialogue package enters the dialogue procedure
(`008e8600`), which saves the running package and makes a conversation package
(`008b19c0`). When the conversation is activated, `005fa330` stores the
dialogue package at DONE and runs its End action; closing the conversation
restores that saved DONE package (`008b1070` → `00913250`). This path does not
remove the script-package override. ED-E's first-player `GREETING` topic
(`DIAL 000000C8`) includes INFO `001579D7`, whose result runs
`enableplayercontrols` and `RemoveScriptPackage EDEDialoguePackage`. The
new-player dialogue query resolves that greeting to INFO `001579D7`, with
Goodbye INFO `001579CE` offered afterwards. The handler (`005cc7c0`) removes
the actor's current script-package override without comparing the named
argument, then reevaluates the actor. That proves how the command works, but
not that the game calls it naturally after the log package finishes. In the
viewer, a post-log player-started conversation offered Goodbye and the log
topics but did not rerun INFO `001579D7`; the finished log package remained
active until acceptance forced `EDE1Ref.RemoveScriptPackage`.
Whether the original game has another cleanup path has not been verified live.

## Fixes (one commit each)

### The dialogue package's second location in a cell

`EDEQuestCompleteDialogue`'s second location (`PLD2`) is kind 1, "in a cell",
naming `PrimmNashResidence`. `at_second_location` (`crates/world/src/ai.rs`)
returned `false` for anything but kind 0, so `dialogue_step` answered
`DialogueStep::Wait` on every update: the package could never reach its Talk
step, so its End action (the `completequest`) never ran and the quest never
completed — the handover and objective 60 were unaffected because they are
quest-script GameMode, not this package. The location kinds are the loader's
(`0067f060`; the cell comparison as `00676390`). The fix compares the actor's
current cell with the package's cell for kind 1. Test: a generated type-15
dialogue package whose second location is kind 1 (`crates/testdata/src/ai.rs`)
with a regression in `crates/world/tests/ai.rs` (matching cell → `Talk`,
different cell → not at the location).

### The quest's Pip-Boy `MenuMode` block

The viewer previously never called `Runner::menu_mode` while its manually
drawn Pip-Boy was open, so the quest's Menumode block that changes
`iEDEOut` 3 → 4 could not run. The Pip-Boy now passes its top menu class to
the runner each open frame. `MenuMode 0` matches any open menu, `MenuMode 1`
matches Pip-Boy classes 1002, 1003, 1023, 1035 and 1061, and other numbers
match that exact class (`0059c380`; block dispatch `005c4240`, FalloutNV.exe
1.4.0.525). The script condition, package condition and quest block dispatch
share that predicate. A generated quest fixture checks `MenuMode 0`, `1` and
an exact class (`crates/testdata/src/lib.rs`, `crates/world/tests/scripting.rs`);
the condition tests cover Pip-Boy and non-Pip-Boy classes
(`crates/world/tests/more_functions.rs`).

### The map-marker travel value

`GetMapMarkerVisible` returns 0 for a hidden marker, 1 for a visible marker,
and 2 when the visible marker can also be fast-travelled to. FalloutNV.exe
1.4.0.525's script-function handler at `005daac0` calls `005a51e0`, which
returns that three-state value. `ShowMap` at `005c8620` sets visibility via
`0044de40`; a nonzero second argument also sets the travel bit via
`0044de80`. The viewer now tracks both states, saves them, and returns 2 only
when the marker is visible and travelable. Tests cover `ShowMap` with and
without its second argument, save/load, and travel eligibility.

## Remaining gaps

1. **The natural log triggers and timing are still forced.** Acceptance sets
   the hire state and `iEDEDaysPassed`, then uses `SetStage vDialogueEDE 10`
   twice to dispatch the two packages and sets `GameDaysPassed` to 10 for the
   radio delay. The actual log INFOs and their result scripts do run. Repairing
   ED-E, hiring it, waiting the five in-game days between logs, and reaching
   each stage trigger through dialogue remain unverified.
2. **Radio playback is in viewer acceptance, but package cleanup is forced
   and the route is not compared with the original.** After log two, the route
   advances `GameDaysPassed` and calls
   `EDE1Ref.RemoveScriptPackage EDEDialoguePackage` so the completed log
   override no longer blocks the lower-priority radio package. Then the quest's
   radio branch selects and runs INFO `0015FF67` from DIAL `0015FF64`. This
   verifies the viewer sequence, but does not establish that this cleanup is
   what the original game does in a live run, or isolate the quest's explicit
   `EvaluatePackage` call from per-frame package reevaluation.
3. **The pickup and trip are still forced.** Acceptance adds
   `HVMissionDisc01` to the player and teleports to `HiddenValleyMarkerREF`.
   This runs the item's OnAdd script and the marker's proximity-discovery path
   (`00779070`, 10,000-unit radius), already modeled by
   `world::map::discover`. It does not test looting the corpse or walking to
   Hidden Valley in the world.
4. **The original-game comparison remains outstanding.** The complete route
   above is verified in the Linux viewer, not in Fallout: New Vegas itself.
   The STATS screen was not used to visually confirm the XP meter after the
   logged `XP +100` event.

## Not compared with the original game

The acceptance route and its complete objective-60 return have not been
compared side-by-side with the original game. Its forced prerequisites are
listed above; the route reaches completion through the quest's own handover,
Menumode/GameMode, dialogue-package and reward scripts.
