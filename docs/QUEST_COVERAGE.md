# Base-game quest coverage matrix

Every quest (`QUST`) in `FalloutNV.esm`, what its scripts need from the engine, and how far it has been played in the nv-rs viewer. **Generated**; rerun it after changing nv-rs's script functions, the played table or the system gaps, and commit the result:

```
cargo run --release -p nvinspect -- "<Data>\FalloutNV.esm" coverage quests > docs/QUEST_COVERAGE.md
```

The generator is `crates/nvinspect/src/quest_coverage.rs`. Two small tables in `crates/nvinspect/src/quest_played.rs` are kept by hand: how far each quest has been played (status, note, evidence) and the systems nv-rs lacks (with the functions that need them and the evidence). Everything else is read from the data and from nv-rs's own function tables. Quests are named by editor ID; no game text is quoted and nothing decompiled is included.

**436 quests**: 0 PLAYED, 8 PARTIAL, 401 NOT PLAYED, 27 NEVER STARTED.

- Running from a new game (start game enabled): 281.
- Script functions the quests' scripts call: 290; carried out by nv-rs: 272 (18 not). Functions only their conditions ask that nv-rs doesn't carry out: 1.
- Quests with nothing listed against them (no missing function, condition or system): 408 of 436. That says nothing about how well the functions they use work: see the played status.
- Quests whose scripts all parse (so nv-rs can run them): 435 of 436.
- Console commands in the quests' compiled scripts: none (every statement is a keyword or a script function).

## How to read it

* **PLAYED**: reached its end in the viewer, with its stages moved by the game's own scripts, triggers and dialogue (a test character may set the start).
* **PARTIAL**: some of it was played; the note says how far and what was helped along with a console line (`--run "SetStage ..."`).
* **NOT PLAYED**: no recorded play; the row lists what its scripts need that nv-rs lacks.
* **NEVER STARTED**: nothing in the data starts it (it doesn't run from a new game, no script calls `StartQuest` or `SetStage` on it, no perk or casino names it), so the original never plays it either, unless the engine starts it itself [C]. Computed from the data; the played table can override it.
* **[G]** marks a guess, **[C]** something to check against the original game.
* **Starts**: `new game` when the quest runs from the start (`DATA` flag 0x01); *started by* the scripts that call `StartQuest` on it, a perk's quest-stage entry, or a casino whose winnings quest it is [G]; *stages set by* the other scripts that call `SetStage` on it (a stage set on a quest that isn't running starts it: `SetStage` `005c7140` → `0060d510` → `0060c9c0`, traced in docs/GOODSPRINGS_ROUTE.md; most of these run while it already runs). A script is named by where it is: another quest's stage (`VCG01 stage 200`), a quest's dialogue lines (`VDialogueBenny dialogue`), a script with the record it's on (`SomeScript (on NPC_ Someone +2)`, +2: two more records carry it), a package, a terminal or a placed reference. Its own stages, dialogue and quest script don't count: they run only while it runs.
* **Flags**: `DATA` byte 0: start game enabled (0x01), repeated stages (0x08, named in `world::quest`), repeated topics (0x04, the editor's name [G]); other bits in hex.
* **Scripts**: how many compiled scripts belong to the quest: its quest script, its stages' result scripts, its dialogue lines' result scripts, and every other script that starts it, sets one of its objectives or variables, completes, stops or resets it. Read from the compiled scripts (function numbers, and the records their reference lists name), not from their text. A shared script (a town's freeform quest, a generic actor script) counts for every quest it drives.
* **Systems missing**: systems those scripts open or need that nv-rs doesn't have yet, from the hand-kept list below (a function nv-rs answers but whose screen or playback isn't built).
* **Missing functions**: script functions those scripts call that nv-rs doesn't carry out (not in `world::scripting::HANDLED` or `world::script_functions::handled()`), then, marked (condition), ones only the quest's conditions ask (its own, its stages', its targets' and its dialogue lines' `CTDA`): nv-rs answers those with 0. A function counted as carried out may still be incomplete.
* **Can't run**: scripts nv-rs can't run because their source doesn't parse (nv-rs runs scripts from their source text, not the compiled form).
* **Moves the player to**: the worldspaces and interior cells its scripts send the player to (`player.MoveTo`, `MoveToFade`), from the target reference's place. Whether each loads well in the viewer isn't checked here.
* Rows are grouped by status. Within a group, rows are ordered by their main blocker (the one listed against the most quests), so quests waiting on the same system sit together; rows with nothing listed come last.

## Most common blockers

### Systems nv-rs lacks

Kept by hand (`quest_played::SYSTEM_GAPS`): a system and the functions that need it, counted over the quests whose scripts call one of them (or, for a record type listed, the quests such a record names).

| System | Functions (records) | Quests | Some of them | Why, and the evidence |
| --- | --- | --- | --- | --- |
| face editor | `ShowRaceMenu` | 2 | `VCG01`, `VCG04` | the race and face menu isn't built: it accepts itself and the face is kept (docs/TASKS.md B15, viewer/src/scripts.rs) |

### Script functions nv-rs doesn't carry out

By how many quests' scripts call them, then how many quests' conditions only ask them. Carrying out the top ones unblocks the most quests.

| Function | Called by | Asked by conditions of | Some of them |
| --- | --- | --- | --- |
| `AutoDisplayObjectives` | 4 | 0 | `CGTutorial`, `VCG01`, `VCG04`, `VMQ01` |
| `GetAnimAction` | 4 | 0 | `LilysPsychoticBreaks`, `vDialogueLily`, `VMS41`, `VNPCFollowers` |
| `IsPlayerMovingIntoNewSpace` | 4 | 0 | `VEuclidQuest`, `VMQ03a`, `VMS03`, `VMS49` |
| `UseWeapon` | 4 | 0 | `vDialogueGomorrah`, `VMS03`, `VMS21`, `VMS49` |
| `DetonatePlacedExplosives` | 3 | 0 | `RadioNewVegas`, `vHDBattleController`, `VMQ03b` |
| `SetLevel` | 3 | 0 | `PressDemoQuest`, `VCG04`, `VNPCFollowers` |
| `Rotate` | 2 | 0 | `VDialogueCraigBoone`, `VFreeformMcCarran2` |
| `ShowBarberMenu` | 2 | 0 | `HD00RobotsDialog`, `VFreeformFreeside` |
| `AgeRace` | 1 | 0 | `VCG00` |
| `EnableLoadingMenu` | 1 | 0 | `VFreeformTheFort` |
| `ExitGame` | 1 | 0 | `FadeToCreditsTimer` |
| `GetContainerInventoryCount` | 1 | 0 | `vMojaveExpressControl` |
| `GetDisposition` | 0 | 1 | `GenericKids` |
| `PlaceAtReticle` | 1 | 0 | `VEuclidQuest` |
| `ResetXP` | 1 | 0 | `VCG04` |
| `SetPCCanUsePowerArmor` | 1 | 0 | `Generic` |
| `SetScreenSplatterFade` | 1 | 0 | `VCG00` |
| `ShowAllMapMarkers` | 1 | 0 | `Generic` |
| `TriggerScreenSplatter` | 1 | 0 | `VCG00` |

Quests with a script whose source doesn't parse (`nvinspect <Data> scripts` lists why): `VFreeformMcCarran`.

## Matrix

### PARTIAL (8)

| Quest | Form ID | Stages | Objectives | Flags | Starts | Scripts | Systems missing | Missing functions | Can't run | Moves the player to | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `CGTutorial` | 00059C85 | 28 | 0 | start game enabled, 0x10 | new game; stages set by `CG01TeddyBear01SCRIPT (on MISC CG01ToyCar +3)`, `CG01ToyBoxTriggerSCRIPT`, `CG02TargetSCRIPT`, `CGTutorialSCRIPT` +9 more | 43 |  | `AutoDisplayObjectives` |  |  | Seen in passing during the VCG02 runs: its stages moved with Sunny's lessons and its V.A.T.S. hint kept repeating while it stayed at stage 70 (the data's own timer [G]). Not watched on purpose. (Evidence: docs/GOODSPRINGS_ROUTE.md (blockers left, item 5).) |
| `VCG01` | 00104C1C | 38 | 4 | start game enabled, 0x10 | new game; stages set by `GSDocMitchellExitTriggerScript (on ACTI GSDocMitchellExitTrigger)`, `VCG00 stage 100`, `VCG01VigorTesterSCRIPT (on ACTI VCG01VigorTester)`, `VCG01VigorTesterTriggerSCRIPT (on ACTI VCG01VigorTesterTrigger)` +1 more | 183 | face editor | `AutoDisplayObjectives` |  |  | Doc Mitchell's opening. Played by its own scripts and dialogue from a new game to stage 55 (Doc asks for the vigor tester; the face menu accepts itself); from a stage 55 save, the tester trigger, stage 60 and the SPECIAL menu; from `--stage VCG01 110`, Doc walking to his door and talking (acceptance route `doc`); the farewell's follow-up chain to the open door was walked on the real data by a throwaway program. Not played in one run end to end: the face editor, the name entry to the door, the questionnaire on the couch. (Evidence: docs/OPENING.md, docs/PLAYTESTING.md, docs/DIALOGUE.md, scripts/acceptance.ps1 route doc.) |
| `VCG00` | 00102037 | 38 | 0 | repeated topics | stages set by `PACK VCG00BennyTravelToPlayer`, `PACK VCG00JessupMoveTowardPlayer` | 73 |  | `AgeRace`, `TriggerScreenSplatter`, `SetScreenSplatterFade` |  | `GSDocMitchellHouse` | The opening. Nothing in the scripts or the exe's data was found to start it (the two packages that set its stages belong to its own scene); the viewer's `--new-game` sets its stage 0, whose script asks for the intro movie and moves on to Doc Mitchell's quest. The movie plays (`--new-game --movies`: `FNVIntro.bik`, seen in the maintainer's merge check). [C] How the original starts it. (Evidence: docs/OPENING.md (observed baseline), docs/MOVIES.md, docs/CONTRIB_CHAZM.md, docs/CLAUDE_REFERENCE.md.) |
| `VCG02` | 0010A214 | 10 | 9 |  | started by `VFreeformGoodsprings dialogue`; stages set by `GSSunnySmilesScript (on NPC_ GSSunnySmiles)`, `GSTrudyScript (on NPC_ GSTrudy)`, `PACK VCG02SunnySneakCloserToWell`, `PACK VCG02SunnyTravelToWell1` +3 more | 48 |  |  |  |  | Completed once (XP, caps, VCG03 started) with the start, the bottle hits, the gecko kills at the first and third wells, the player's following and the reward request as console lines; Sunny's walks, the patrol trigger, her barks and the stage 30/40 package end actions ran by themselves. Blocker: talking to Sunny can't start it (her greeting comes from this quest, not from VFreeformGoodsprings) [C]. (Evidence: docs/GOODSPRINGS_ROUTE.md, scripts/acceptance.ps1 route vcg02.) |
| `VCG03` | 0015D912 | 2 | 4 |  | stages set by `GSSunnySmilesScript (on NPC_ GSSunnySmiles)`, `GSTrudyScript (on NPC_ GSTrudy)`, `VCG02 dialogue` | 14 |  |  |  |  | Only its start: VCG02's reward sets stage 10 and Sunny sets off on her walk to the campfire. Nothing after that was played. (Evidence: docs/GOODSPRINGS_ROUTE.md.) |
| `VDLCPackQuest` | 0017912D | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  | In the `--new-game` run its four pack notices came up and were accepted; whether the items arrived wasn't recorded. (Evidence: docs/OPENING.md (observed baseline).) |
| `VMS16` | 00104EAE | 10 | 10 |  | stages set by `GSRingoScript (on NPC_ GSRingo)`, `GSSunnySmilesScript (on NPC_ GSSunnySmiles)`, `GoodspringsJoeCobbScript (on NPC_ GSJoeCobb)`, `GoodspringsPowderGangerScript (on NPC_ GSPGHM2 +5)` +1 more | 41 |  |  |  |  | Reached stage 100 with Trudy's help (the gangers come in and die, XP, fame and infamy); the dialogue choices that lead there were replayed as console lines, and the player didn't fight (the townspeople killed the gang). Not played: the dialogue picked in the menu, the branch without help, the player's own fight. (Evidence: docs/GOODSPRINGS_ROUTE.md, scripts/acceptance.ps1 route vms16.) |
| `nVPrimmDeputyConv` | 000DA115 | 10 | 10 | start game enabled, repeated topics, 0x10 | new game; stages set by `PrimmNashDialogue dialogue`, `VFreeformNCRCF dialogue` | 68 |  |  |  |  | Only the Primm deputy route in PR #29, which is still open, is recorded: pending review, so its evidence is not merged and the route is not proof of coverage. That route jumps the player with five `player.MoveTo` console lines and opens every conversation with a console `StartConversation`; the quest's own scripts, its stages and the deputy's dialogue were not driven without them. Nothing else of the quest was tried. (Evidence: PR #29 (open, not merged); docs/GOODSPRINGS_ROUTE.md is the route model.) |

### NOT PLAYED (401)

| Quest | Form ID | Stages | Objectives | Flags | Starts | Scripts | Systems missing | Missing functions | Can't run | Moves the player to | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `VCG04` | 0011649E | 0 | 0 | 0x10 | started by `VCG01 stage 200` | 3 | face editor | `SetLevel`, `AutoDisplayObjectives`, `ResetXP` |  |  |  |
| `VMQ01` | 000842DD | 6 | 12 |  | started by `VCG01 stage 200`; stages set by `TERM VNovacTerminalManny`, `VDialogueNovac dialogue`, `VFreeformBoulderCity dialogue`, `VTopsBennyIntroTriggerSCRIPT (on ACTI VTopsBennyIntroTrigger)` | 29 |  | `AutoDisplayObjectives` |  |  |  |
| `LilysPsychoticBreaks` | 00160269 | 0 | 0 |  | started by `LilyScript (on CREA Lily)` | 2 |  | `GetAnimAction` |  |  |  |
| `VMS41` | 0013F405 | 10 | 11 |  | started by `VDialogueRemnants dialogue`; stages set by `JacobstownCalamityScript (on NPC_ JacobstownCalamity)`, `JacobstownKeeneScript (on CREA JacobstownKeene)`, `JacobstownMarcusScript (on CREA JacobstownMarcus)`, `LilyScript (on CREA Lily)` +3 more | 46 |  | `GetAnimAction` |  |  |  |
| `VNPCFollowers` | 000B16D0 | 0 | 0 | start game enabled, 0x10 | new game | 322 |  | `GetAnimAction`, `SetLevel` |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` +2 more |  |
| `vDialogueLily` | 0013E510 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 69 |  | `GetAnimAction` |  |  |  |
| `VEuclidQuest` | 0016AEFF | 0 | 0 |  | started by `EuclidsCFinderSCRIPT (on WEAP WeapNVEuclidsCFinder)` | 4 |  | `IsPlayerMovingIntoNewSpace`, `PlaceAtReticle` |  |  |  |
| `VMQ03a` | 00131F08 | 6 | 8 |  | started by `VDialogueMrHouse dialogue`, `VFreeformHooverDam dialogue`, `VHDAssassinationConsoleScript`, `VMQTops dialogue`; stages set by `ElDoradoReactorSCRIPT (on TERM ElDoradoWallTerminal)`, `VHDAssassinationDialog dialogue`, `VHDCaptainGrahamSCRIPT (on NPC_ VHDCaptainGraham)`, `VHDColonelMooreSCRIPT (on NPC_ VHDColonelMoore)` +3 more | 81 |  | `IsPlayerMovingIntoNewSpace` |  | `HooverDamIntOliverArea`, `WastelandNV` |  |
| `VMS03` | 000E282D | 3 | 10 | start game enabled, 0x10 | new game; stages set by `FantasticSCRIPT (on NPC_ Fantastic)` | 73 |  | `IsPlayerMovingIntoNewSpace`, `UseWeapon` |  |  |  |
| `VMS49` | 00145F85 | 1 | 19 | repeated topics | started by `VDialogueVeronica dialogue`; stages set by `HVHardinScript (on NPC_ EdgarHardin)`, `NolanMcNamaraSCRIPT (on NPC_ NolanMcNamara)`, `VMS18 dialogue`, `VMS49PaladinScript (on NPC_ VMS49PaladinAAM +3)` | 240 |  | `IsPlayerMovingIntoNewSpace`, `UseWeapon` |  | `HiddenValley02`, `HiddenValleyBunker1` |  |
| `VMS21` | 00110A63 | 6 | 36 |  | stages set by `VMS21BigSalScript (on NPC_ VMS21BigSal)`, `VMS21NeroScript (on NPC_ VMS21Nero)`, `vDialogueGomorrah dialogue`, `vDialogueGomorrahScript (on QUST vDialogueGomorrah)` | 98 |  | `UseWeapon` |  |  |  |
| `vDialogueGomorrah` | 0010C721 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 190 |  | `UseWeapon` |  | `TheStripWorldNew` |  |
| `RadioNewVegas` | 0014DF04 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 134 |  | `DetonatePlacedExplosives` |  |  |  |
| `VMQ03b` | 00131F09 | 7 | 3 |  | started by `VDialogueCaesar dialogue`, `VFreeformTheFort dialogue`, `VHDAssassinationConsoleScript`; stages set by `VHDAssassinationDialog dialogue`, `VHDCatoHostiliusSCRIPT (on NPC_ VHDCatoHostilius)`, `VHDKimballSpeechSCRIPT (on QUST VHDKimballSpeech)`, `VHDKimballVertibirdScript (on ACTI VHDVertibirdKimballEncount01)` +1 more | 36 |  | `DetonatePlacedExplosives` |  |  |  |
| `vHDBattleController` | 00133075 | 3 | 5 |  | started by `VHDHouseBattle stage 10`, `VHDIndependentBattle stage 10`; stages set by `VFreeformHooverDam dialogue`, `VLegateEndFailsafeTrigger (on ACTI VLegateEndFailSafe)`, `VLegateShowdownControlScript (on ACTI VLegateShowdownCodeHolder)`, `VMQ05 stage 100` | 44 |  | `DetonatePlacedExplosives` |  | `WastelandNV` |  |
| `PressDemoQuest` | 0011EBA7 | 1 | 1 |  | started by `VMS30 dialogue`, `vDialogueCasinoCashier dialogue` | 5 |  | `SetLevel` |  | `WastelandNV` |  |
| `VDialogueCraigBoone` | 00096BCD | 0 | 1 | start game enabled, 0x10 | new game | 90 |  | `Rotate` |  | `WastelandNV` |  |
| `VFreeformMcCarran2` | 000E7913 | 0 | 0 | start game enabled, 0x10 | new game | 140 |  | `Rotate` |  |  |  |
| `VFreeformFreeside` | 0010E195 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 577 |  | `ShowBarberMenu` |  | `2ELVBStation`, `WastelandNV` |  |
| `VFreeformTheFort` | 0011F9CF | 0 | 0 | start game enabled, 0x10 | new game | 115 |  | `EnableLoadingMenu` |  | `WastelandNV` |  |
| `FadeToCreditsTimer` | 0017A17C | 0 | 0 |  | started by `NarratorScript (on NPC_ Narrator)` | 2 |  | `ExitGame` |  |  |  |
| `vMojaveExpressControl` | 0015F232 | 0 | 0 | start game enabled, 0x10 | new game | 2 |  | `GetContainerInventoryCount` |  |  |  |
| `GenericKids` | 00049037 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  | `GetDisposition` (condition) |  |  |  |
| `Generic` | 000038B2 | 18 | 0 | start game enabled, repeated stages, 0x10 | new game; started by `perk AnimalFriend`, `perk Explorer`, `perk FortuneFinder`, `perk HereandNow` +4 more | 21 |  | `ShowAllMapMarkers`, `SetPCCanUsePowerArmor` |  |  |  |
| `AchievementQuest` | 0007691F | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `ArcadeRetrieveArmorTimer` | 0017B387 | 0 | 0 |  | started by `VDialogueArcadeGannon dialogue` | 7 |  |  |  |  |  |
| `AudioHolotapes` | 00044935 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `AutoDocTimerQuest` | 00179228 | 0 | 0 |  | started by `VMS35AutoDocScript (on ACTI VMS35AutoDoc)` | 2 |  |  |  |  |  |
| `BeatrixFX` | 0017BA11 | 0 | 0 |  | started by `VFSBeatrixSCRIPT (on NPC_ VFSBeatrixRussell)` | 2 |  |  |  |  |  |
| `BennyToArenaTimer` | 0017A972 | 0 | 0 |  | started by `VDialogueBenny dialogue` | 2 |  |  |  |  |  |
| `CG00` | 0001F388 | 0 | 0 | repeated topics | stages set by `CG00DadSCRIPT`, `CG00SCRIPT` | 3 |  |  |  |  |  |
| `CG02` | 00014E84 | 0 | 0 | repeated topics | stages set by `CG02CakeTriggerScript`, `CG02JonasSCRIPT`, `CG02RadroachSCRIPT (on CREA CG02Radroach)` | 3 |  |  |  |  |  |
| `CampSearchlightAstorBarkQuest` | 00158305 | 0 | 0 |  | started by `CampSearchlightAstorBarkScript (on ACTI CampSearchlightAstorBark)` | 3 |  |  |  |  |  |
| `ControlRumble` | 00070EC9 | 2 | 0 | start game enabled, repeated stages, 0x10 | new game | 7 |  |  |  |  |  |
| `CreatureHits` | 0007E262 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `DLC01GenericTrog` | 00094682 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `DialogueBoSSniffer` | 00164E27 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `DialogueCrimeKarma` | 00075D21 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 0 |  |  |  |  |  |
| `DialogueJeannieMayCrawford` | 0008D74D | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `DialogueLegionSniffer` | 001252C5 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `DialogueMSMini` | 000C8E15 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `DialogueNCRSniffer` | 00126773 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `DialogueNobark` | 0008CD26 | 0 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  |  |  |
| `DialogueOldLadyGibson` | 00084224 | 0 | 0 | start game enabled, 0x10 | new game | 16 |  |  |  |  |  |
| `DisguiseFactionPulseQuest` | 001411A8 | 0 | 0 |  | started by `BrotherhoodFactionOutfitWarningScript (on ARMO ArmorPowerBrotherhoodOfSteelT51B +2)`, `CaesarsLegionFactionOutfitWarningScript (on ARMO ArmorNVCaesar +7)`, `KhanFactionOutfitWarningScript (on ARMO ArmorNVGKSimple +3)`, `NCRFactionOutfitWarningScript (on ARMO ArmorNVBoone02 +16)` +2 more | 17 |  |  |  | `Endgame` |  |
| `Doctors` | 000963D4 | 3 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `FFEDoctorSCRIPT`, `LucySCRIPT`, `NellisArgyllScript (on NPC_ NellisArgyll)` | 19 |  |  |  |  |  |
| `FerociousLoyaltyQuest` | 0016517F | 1 | 0 | 0x10 | started by `perk FerociousLoyalty` | 1 |  |  |  |  |  |
| `FistoFX` | 00173905 | 0 | 0 |  | started by `VFreeformFreeside dialogue` | 3 |  |  |  |  |  |
| `FollowersHireRL3` | 000943EB | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `FortEquipmentConfiscationQuest` | 00164762 | 0 | 0 |  | started by `VFreeformCottonwoodCove dialogue`, `VFreeformTheFort dialogue` | 22 |  |  |  |  |  |
| `FriendOfTheNightQuest` | 0016511C | 1 | 0 | 0x10 | started by `perk FriendOfTheNight` | 1 |  |  |  |  |  |
| `GenericAdult` | 0004ADEE | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericAdultCombat` | 0015ACB5 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericFeralGhoul` | 0007AE95 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericFiend` | 000FABE3 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericFriendlyFire` | 00040C5D | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericFriendlyFireCustomVoice` | 0007C70B | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericIdleChatter` | 000B87B5 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericPlayer` | 00041E7F | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericRobot` | 00015A37 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `GenericSupMutBehemoth` | 0002242A | 0 | 0 | start game enabled | new game | 0 |  |  |  |  |  |
| `GenericSupermutant` | 0001F93C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `HD00JukeboxMusic` | 000C4992 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `ImplantTimerQuest` | 00176ACB | 0 | 0 |  | started by `vDialogueMedicalClinic dialogue` | 14 |  |  |  |  |  |
| `L38TimerQuest` | 0017A66D | 0 | 0 | start game enabled, 0x10 | new game; started by `TERM Lucky38ControlTerminalWarning` | 2 |  |  |  |  |  |
| `Lilysmedicinetimer` | 00160267 | 0 | 0 |  | started by `VNPCFollowers dialogue`, `vDialogueLily dialogue` | 7 |  |  |  |  |  |
| `MeatOfChampions` | 00164AFA | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `NQlvl` | 0002012B | 1 | 0 | start game enabled, repeated stages, 0x10 | new game; stages set by `DeathclawCageDoorGenericSCRIPT`, `RavenRockTrapLaserEmitterSCRIPT (on ACTI RavenRockTrapLaserTripwireEmitter01)`, `SprungBearTrapScript (on ACTI SprungTrapBearTrap)`, `TRAPPitchingMachineScript (on ACTI TrapPitchingMachine01)` +16 more | 45 |  |  |  |  |  |
| `NellisSunshineBoogie` | 0015ECC9 | 2 | 4 |  | stages set by `NellisLoyalScript (on NPC_ NellisLoyal)`, `VFreeformNellis dialogue` | 14 |  |  |  |  |  |
| `P04CompanionFireQuest` | 0017BA12 | 0 | 0 |  | started by `TERM P04CompanionFireTerminalSub` | 2 |  |  |  |  |  |
| `PerkWildWasteland` | 000ED567 | 1 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `PrimmNashDialogue` | 000A23D8 | 0 | 0 | start game enabled, 0x10 | new game | 17 |  |  |  |  |  |
| `REPCONHQFreeform` | 000E7533 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 69 |  |  |  |  |  |
| `RadioBlackMountain` | 000E61A6 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 222 |  |  |  |  |  |
| `RadioRepconQuest` | 000CD18A | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `RadioTenpenny` | 0001E5CA | 0 | 0 | start game enabled, repeated topics | new game | 0 |  |  |  |  |  |
| `RadioVault101PA` | 00071622 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `SantiagoFX` | 0017BA13 | 0 | 0 |  | started by `VFSDebtSantiagoSCRIPT (on NPC_ VFSDebtSantiago)` | 2 |  |  |  |  |  |
| `SecuritronVaultExplodeQuest` | 00165E65 | 0 | 0 |  | started by `SVPowerRegulatorScript (on ACTI SVPowerRegulator)` | 2 |  |  |  |  |  |
| `StripRadioQuest` | 0014E8E3 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `TempMan` | 000CD076 | 0 | 0 | start game enabled, 0x10 | new game | 2 |  |  |  |  |  |
| `TopsRadioQuest` | 0014E8E2 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 9 |  |  |  |  |  |
| `UniqueFriendlyFire` | 0007A028 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 0 |  |  |  |  |  |
| `VCG01Test` | 001055BB | 0 | 0 | start game enabled, 0x10 | new game | 72 |  |  |  |  |  |
| `VCampGolfBarkstrings` | 0010B909 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `VCampGolfHanlon` | 001084AC | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 20 |  |  |  |  |  |
| `VCasinoCompsUltraLuxe` | 00160301 | 0 | 0 | start game enabled, 0x10 | new game; started by `casino UltraLuxeCasinoData (its winnings quest) [G]` | 6 |  |  |  |  |  |
| `VCassCompanion` | 0015DA16 | 16 | 15 |  | stages set by `RoseofSharonCassidyScript (on NPC_ RoseofSharonCassidy)`, `VDialogueCass dialogue`, `VDialogueCrimsonCaravan dialogue`, `VMS18 dialogue` +1 more | 42 |  |  |  | `FreesideVanGraffWarehouse` |  |
| `VCassTimer` | 00161F07 | 0 | 0 | repeated topics | started by `VDialogueCass dialogue`, `VNPCFollowers dialogue` | 16 |  |  |  |  |  |
| `VDeadSea` | 00125E7E | 4 | 2 | start game enabled, 0x10 | new game; stages set by `LegionaryDeadSeaDialogueSCRIPT (on NPC_ LegionaryDeadSea)`, `vDeadSeaNCRDeadscript` | 22 |  |  |  |  |  |
| `VDialogueArcadeGannon` | 0016117F | 0 | 0 | start game enabled, 0x10 | new game | 107 |  |  |  |  |  |
| `VDialogueBenny` | 001194B8 | 0 | 0 | start game enabled, repeated stages, 0x10 | new game | 88 |  |  |  |  |  |
| `VDialogueBetsy` | 0015435E | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueBitterSprings` | 00139A43 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 2 |  |  |  |  |  |
| `VDialogueBlackMountain` | 0015CDA0 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueBrotherhoodOfSteelFaction` | 001535FF | 0 | 0 | start game enabled, 0x10 | new game | 6 |  |  |  |  |  |
| `VDialogueCaesar` | 001227A1 | 3 | 3 | start game enabled, 0x10 | new game | 133 |  |  |  |  |  |
| `VDialogueCass` | 00133FDC | 0 | 0 | start game enabled, 0x10 | new game | 68 |  |  |  |  |  |
| `VDialogueChrisHaversam` | 0008D243 | 0 | 0 | start game enabled, 0x10 | new game | 40 |  |  |  |  |  |
| `VDialogueCliffBriscoe` | 0008D0A4 | 0 | 0 | start game enabled, 0x10 | new game | 15 |  |  |  |  |  |
| `VDialogueCrimsonCaravan` | 000F8B6A | 0 | 0 | start game enabled, 0x10 | new game | 52 |  |  |  |  |  |
| `VDialogueGenericNCRTrooper` | 000FC6F3 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueGoodspringsGeneric` | 00105BD0 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueHiddenValley` | 000E327C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 167 |  |  |  | `HiddenValley01` |  |
| `VDialogueJasonBright` | 0008D241 | 0 | 0 | start game enabled, 0x10 | new game | 16 |  |  |  |  |  |
| `VDialogueLucky38Entrance` | 001132EF | 2 | 0 | start game enabled, 0x10 | new game; stages set by `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `NVCRMrHouseSCRIPT (on CREA NVCRMrHouse)`, `SecuritronVaultExplodeQuestScript (on QUST SecuritronVaultExplodeQuest)`, `TERM Lucky38ControlTerminalWarning2` +3 more | 29 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VDialogueLucky38Penthouse` | 0011A9F1 | 0 | 0 | start game enabled, 0x10 | new game | 7 |  |  |  |  |  |
| `VDialogueMrHouse` | 00116B58 | 3 | 0 | start game enabled, 0x10 | new game; stages set by `NVCRMrHouseSCRIPT (on CREA NVCRMrHouse)` | 173 |  |  |  |  |  |
| `VDialogueNovac` | 0008D750 | 0 | 0 | start game enabled, 0x10 | new game | 40 |  |  |  |  |  |
| `VDialogueRaul` | 000E61A5 | 0 | 0 | start game enabled, 0x10 | new game | 62 |  |  |  |  |  |
| `VDialogueRemnants` | 000E5959 | 0 | 0 | start game enabled, 0x10 | new game | 59 |  |  |  |  |  |
| `VDialogueRex` | 001176B7 | 0 | 0 | start game enabled, 0x10 | new game | 4 |  |  |  |  |  |
| `VDialogueTEST` | 00140E14 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueTops` | 00128588 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 15 |  |  |  |  |  |
| `VDialogueVault21` | 001290C2 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VDialogueVegasNorth` | 000F2429 | 25 | 23 | start game enabled, 0x10 | new game; stages set by `1EWatchFiendDeathScript (on NPC_ 1EWatchFiend4 +3)`, `JulesScript (on NPC_ VOVNVJules)`, `VOVCrandonScript (on NPC_ NorthVegasCrandon)`, `VOVNVAliceHostetlerScript (on NPC_ NorthVegasAliceHostetler)` +4 more | 155 |  |  |  |  |  |
| `VDialogueVegasNorthSupport` | 0017A645 | 0 | 0 |  | started by `VOVNVHostetlerHomeShowdownScript (on ACTI VOVNVHostetlerCodeHolder)` | 2 |  |  |  |  |  |
| `VDialogueVeronica` | 001467B4 | 0 | 0 | start game enabled, 0x10 | new game | 86 |  |  |  | `HiddenValleyBunker1` |  |
| `VDialogueVeronicaTimer` | 0016740D | 0 | 0 |  | started by `VDialogueVeronica dialogue` | 11 |  |  |  |  |  |
| `VDialogueVickyVance` | 000E8D24 | 0 | 0 | start game enabled, 0x10 | new game | 20 |  |  |  |  |  |
| `VDialogueWestsideGeneric` | 0013308B | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDialogueWhiteGloveSociety` | 00157CC3 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VDoctors` | 000CE959 | 4 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `DoctorKempSCRIPT (on NPC_ DoctorKemp)`, `DoctorTemplateSCRIPT (on CREA Sawbones +4)`, `DoctorUsanagiScript (on NPC_ DoctorUsanagi)`, `GSDocMitchellScript (on NPC_ DocMitchell)` +8 more | 46 |  |  |  |  |  |
| `VEFR00LegionMix2CaesarsHire` | 00163429 | 4 | 1 | start game enabled, repeated stages, 0x10 | new game; started by `Patch04Quest00SCRIPT (on QUST Patch04Quest00)`; stages set by `VEFR0xLegionDropboxSCRIPT (on CONT MetalBox05LegionDropbox +9)`, `VMQ02 stage 110` | 10 |  |  |  |  |  |
| `VEFR00NCRMixed2LimitedAccess` | 0015FB7D | 10 | 0 | start game enabled, repeated stages, 0x10 | new game; started by `Patch04Quest00SCRIPT (on QUST Patch04Quest00)` | 2 |  |  |  |  |  |
| `VEFR01LegionGood2CaesarsFavor` | 00163427 | 4 | 1 | start game enabled, repeated stages, 0x10 | new game; started by `Patch04Quest00SCRIPT (on QUST Patch04Quest00)`; stages set by `VEFR0xLegionDropboxSCRIPT (on CONT MetalBox05LegionDropbox +9)`, `VMQ02 stage 110` | 9 |  |  |  |  |  |
| `VEFR01NCRGood2EmergencyRadio` | 0015FB7C | 10 | 1 | start game enabled, repeated stages, 0x10 | new game; stages set by `VEFR01NCRGood2QuestSCRIPT`, `VEFR01NCRGood2SupplyCache (on CONT VEFR01NCRSupplyCacheContainer)` | 40 |  |  |  |  |  |
| `VEFR02LegionBad2CaesarsFoe` | 00163428 | 3 | 0 | start game enabled, 0x10 | new game; started by `Patch04Quest00SCRIPT (on QUST Patch04Quest00)` | 5 |  |  |  |  |  |
| `VEFR02NCRBad2MostWanted` | 0015F4FF | 10 | 0 | start game enabled, repeated stages, 0x10 | new game; started by `Patch04Quest00SCRIPT (on QUST Patch04Quest00)` | 43 |  |  |  |  |  |
| `VERElDorado01` | 0015A8D3 | 0 | 0 | start game enabled, 0x10 | new game | 2 |  |  |  |  |  |
| `VERGoodsprings01` | 00157B4E | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 1 |  |  |  |  |  |
| `VERIvanpah01` | 0015C7C1 | 0 | 0 | start game enabled, 0x10 | new game | 2 |  |  |  |  |  |
| `VERLegionVets` | 0017A836 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VERNovac01` | 0015F958 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VERPrimmDen01` | 00173FAA | 0 | 0 |  | started by `VERT2PrimmDenEnterTriggerScript (on ACTI VERT2PrimmDenEnterTriggerActivator)` | 2 |  |  |  |  |  |
| `VERSewers01` | 001564E0 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 1 |  |  |  |  |  |
| `VERShadows01` | 00168D8E | 0 | 0 | start game enabled, 0x10 | new game | 2 |  |  |  |  |  |
| `VERStalkersSouth01` | 001617A2 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VES01VictorGSMyBodyguard` | 00154151 | 3 | 0 | start game enabled, 0x10 | new game | 12 |  |  |  |  |  |
| `VES01aVictorInNovac` | 0015FFEC | 2 | 0 | start game enabled, 0x10 | new game | 9 |  |  |  |  |  |
| `VES01bVictorInBoulder` | 0015FFED | 2 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  |  |  |
| `VES02DialogueGeckoman` | 0015834F | 5 | 0 | start game enabled, 0x10 | new game; stages set by `VES02GeckomanGSScript (on ACTI VESGSGeckomanTrigger01)` | 8 |  |  |  |  |  |
| `VES03DialogueWastelandHunterGS` | 0015AD38 | 3 | 0 | start game enabled, 0x10 | new game | 9 |  |  |  |  |  |
| `VES04FactReactPrimmArea01PowderGangers` | 0015C847 | 3 | 0 | start game enabled, 0x10 | new game | 7 |  |  |  |  |  |
| `VES05NightkinTumbleweedRancher` | 0015FFEE | 2 | 0 | start game enabled, 0x10 | new game | 13 |  |  |  |  |  |
| `VES08CampGolf` | 00163BE8 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VES09BoulderCity` | 0015C48A | 0 | 0 | start game enabled, 0x10 | new game | 4 |  |  |  |  |  |
| `VES34Vault` | 00159FA0 | 9 | 4 | start game enabled, repeated topics, 0x10 | new game; stages set by `TERM EastPumpStationTerminal`, `TERM V34MasterOverrideTerminal`, `TERM V34ReactorTerminal`, `V34MorganBlakeScript (on NPC_ 1ESharecropperMorganBlake)` | 19 |  |  |  |  |  |
| `VEnding` | 0015BB01 | 0 | 0 |  | started by `VHDHouseBattle stage 100`, `VHDIndependentBattle stage 100`, `VHDLegionBattle stage 100`, `vDialogueTestaclesDebug dialogue` +1 more | 198 |  |  |  | `Endgame` |  |
| `VFSAtomicPimp` | 0010E1DB | 4 | 12 |  | started by `VFreeformFreeside dialogue`; stages set by `VFSJamesGarretSCRIPT (on NPC_ VFSJamesGarret)` | 70 |  |  |  |  |  |
| `VFSDebtCollector` | 0010E1DC | 1 | 10 |  | started by `VFreeformFreeside dialogue`; stages set by `VFSFrancineGarretSCRIPT (on NPC_ VFSFrancineGarret)` | 73 |  |  |  |  |  |
| `VFSHighTimes` | 0010E1DA | 1 | 10 |  | started by `VFreeformFreeside dialogue`; stages set by `VFSJulieFarkasSCRIPT (on NPC_ VFSJulieFarkas)` | 33 |  |  |  |  |  |
| `VFactionSquad` | 0017BA14 | 2 | 0 | start game enabled, repeated stages, 0x10 | new game | 4 |  |  |  |  |  |
| `VFollowerDialogueRose` | 000FED31 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VFreeform188` | 000E5F34 | 0 | 0 | start game enabled, 0x10 | new game | 16 |  |  |  |  |  |
| `VFreeformBlackMountain` | 000E61A4 | 0 | 0 | start game enabled, 0x10 | new game | 44 |  |  |  |  |  |
| `VFreeformBoulderCity` | 0010C6E9 | 0 | 0 | start game enabled, 0x10 | new game | 44 |  |  |  |  |  |
| `VFreeformCampGolf` | 0010431C | 12 | 15 | start game enabled, 0x10 | new game; stages set by `MisfitsSCRIPT (on NPC_ 2CMags +3)`, `SgtMcCredieScript (on NPC_ 2CMcCredie)`, `VRRCJackScript (on NPC_ VRRCJack)` | 62 |  |  |  |  |  |
| `VFreeformCasaMadrid` | 000F8B79 | 0 | 0 | start game enabled, 0x10 | new game | 72 |  |  |  |  |  |
| `VFreeformCottonwoodCove` | 00130B8E | 0 | 0 | start game enabled, 0x10 | new game | 128 |  |  |  |  |  |
| `VFreeformFreeside2` | 00156A93 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 3 |  |  |  |  |  |
| `VFreeformGoodsprings` | 00104C66 | 0 | 0 | start game enabled, 0x10 | new game | 125 |  |  |  |  | Its variables were set by console lines in the Goodsprings routes. Sunny's line from this quest that should start VCG02 isn't picked (see VCG02). (Evidence: docs/GOODSPRINGS_ROUTE.md.) |
| `VFreeformHooverDam` | 001207BA | 0 | 0 | start game enabled, 0x10 | new game | 155 |  |  |  | `HooverDamIntOliverArea` |  |
| `VFreeformMcCarran` | 000E7363 | 0 | 0 | start game enabled, 0x10 | new game | 133 |  |  | `VFreeformMcCarran dialogue` |  |  |
| `VFreeformMcCarran3` | 000F7EAC | 0 | 0 | start game enabled, 0x10 | new game | 68 |  |  |  |  |  |
| `VFreeformNCRCF` | 000CEF3A | 0 | 0 | start game enabled, 0x10 | new game | 45 |  |  |  |  |  |
| `VFreeformNellis` | 000FED44 | 0 | 0 | start game enabled, 0x10 | new game | 221 |  |  |  |  |  |
| `VFreeformNovac` | 00084223 | 0 | 0 | start game enabled, 0x10 | new game | 66 |  |  |  |  |  |
| `VFreeformSSHQ` | 001019A6 | 0 | 0 | start game enabled, 0x10 | new game | 68 |  |  |  |  |  |
| `VFreeformTheStreet01` | 00117AC4 | 7 | 1 | start game enabled, repeated stages, 0x10 | new game | 54 |  |  |  |  |  |
| `VFreeformUnderpass` | 00103FED | 0 | 0 | start game enabled, 0x10 | new game | 2 |  |  |  |  |  |
| `VFreeformVault11` | 000E8875 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VFreeformVault11ChamberSequence` | 00176A78 | 0 | 0 |  | started by `VFreeformVault11KickerScript (on ACTI VFreeformVault11TriggerMarker)` | 9 |  |  |  |  |  |
| `VFreeformVault22` | 000E6A6A | 0 | 0 | start game enabled, 0x10 | new game | 76 |  |  |  | `Vault22a`, `Vault22b`, `Vault22c`, `Vault22d` +1 more |  |
| `VGenericTimer` | 00168A86 | 0 | 0 |  | started by `HVMcNamaraHardinConvoScript (on ACTI HVMcNamaraHardinConvoTrigger)`, `REFR 0016A21F`, `VCG01 dialogue`, `VMS49 dialogue` +2 more | 9 |  |  |  | `HiddenValley02`, `HiddenValleyBunker1` |  |
| `VHDAssassinationDialog` | 0013A243 | 0 | 0 | start game enabled, 0x10 | new game | 57 |  |  |  |  |  |
| `VHDHouseBattle` | 00154234 | 7 | 6 |  | stages set by `MrHouseSCRIPT (on CREA MrHouse)`, `VFreeformHooverDam dialogue`, `VHDEastPPConsoleSCRIPT (on ACTI VHDEastPPConsole)`, `VHDSecuritronVaultSCRIPT (on ACTI VHDSecuritronVaultTRG)` +3 more | 27 |  |  |  | `Lucky38World`, `WastelandNV` |  |
| `VHDIndependentBattle` | 00154233 | 8 | 7 |  | stages set by `VFreeformHooverDam dialogue`, `VHDEastPPConsoleSCRIPT (on ACTI VHDEastPPConsole)`, `VHDSecuritronVaultSCRIPT (on ACTI VHDSecuritronVaultTRG)`, `VHDWestPPConsoleSCRIPT (on ACTI VHDWestPPConsole)` +5 more | 45 |  |  |  | `Lucky38World`, `WastelandNV` |  |
| `VHDKimballSpeech` | 00130C78 | 0 | 0 | repeated topics | started by `VDialogueMrHouse dialogue`, `VFreeformHooverDam dialogue`, `VHDAssassinationConsoleScript`, `VHDAssassinationDialog dialogue` +3 more | 41 |  |  |  | `HooverDamIntOliverArea` |  |
| `VHDLegionBattle` | 00137AB9 | 4 | 7 | start game enabled, 0x10 | new game; stages set by `VFreeformHooverDam dialogue`, `VHDLegionEnterLegatesTentScript (on ACTI VHDLegionEnterLegatesTentTrigger)` | 93 |  |  |  | `HooverDamIntOliverArea`, `WastelandNV` |  |
| `VHDOliverDeathTimer` | 001724CC | 0 | 0 |  | started by `VFreeformHooverDam dialogue` | 2 |  |  |  | `WastelandNV` |  |
| `VHDOliverRetreatTimer` | 00179A8F | 0 | 0 |  | started by `VFreeformHooverDam dialogue` | 3 |  |  |  |  |  |
| `VHiddenValleyDustStorm` | 00161869 | 0 | 0 | start game enabled, 0x10 | new game | 7 |  |  |  |  |  |
| `VMQ02` | 00129D14 | 12 | 35 |  | started by `VFreeformTheFort dialogue`; stages set by `FortArrivalTriggerScript (on ACTI FortArrivalTrigger)`, `FortCaesarScript (on NPC_ FortCaesar)`, `FortLegionaryPraetorianScript (on NPC_ FortBennyGuard +3)`, `FortLegionaryScript (on NPC_ FortLegionaryRecruitAAM2 +10)` +16 more | 135 |  |  |  | `WastelandNV` |  |
| `VMQ03aTimer` | 001446AF | 0 | 0 |  | started by `VHDAssassinationDialog dialogue` | 3 |  |  |  |  |  |
| `VMQ03bTimer` | 0013F373 | 0 | 0 |  | started by `VHDAssassinationDialog dialogue` | 3 |  |  |  | `WastelandNV` |  |
| `VMQ05` | 00136166 | 9 | 18 | start game enabled, 0x10 | new game; stages set by `VFreeformHooverDam dialogue`, `VHDCaptainGrahamSCRIPT (on NPC_ VHDCaptainGraham)`, `VHDColonelMooreSCRIPT (on NPC_ VHDColonelMoore)`, `VHDKimballSpeechSCRIPT (on QUST VHDKimballSpeech)` +3 more | 68 |  |  |  | `HooverDamIntOliverArea` |  |
| `VMQ06` | 0013A40B | 4 | 1 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VMQCaesarFail` | 00167F0F | 2 | 2 | start game enabled, 0x10 | new game; stages set by `VDialogueCaesar dialogue` | 6 |  |  |  |  |  |
| `VMQHouse` | 001429F2 | 9 | 24 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VMQHouse1` | 00147885 | 8 | 9 | start game enabled, 0x10 | new game; stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `SecuritronVaultExplodeQuestScript (on QUST SecuritronVaultExplodeQuest)` +4 more | 57 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` +1 more |  |
| `VMQHouse2` | 00147886 | 3 | 5 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `SecuritronVaultExplodeQuestScript (on QUST SecuritronVaultExplodeQuest)` +4 more | 24 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VMQHouse3` | 00147887 | 4 | 4 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `TERM Lucky38ControlTerminalWarning` +3 more | 19 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VMQHouse4` | 00147888 | 4 | 4 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `TERM Lucky38ControlTerminalWarning` +3 more | 20 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VMQHouse5` | 00147889 | 4 | 3 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `TERM Lucky38ControlTerminalWarning` +3 more | 15 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VMQHouse6` | 0014788A | 6 | 4 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `TERM Lucky38ControlTerminalWarning` +3 more | 24 |  |  |  | `HooverDamIntOliverArea`, `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22` +1 more |  |
| `VMQHouse7` | 001599CD | 2 | 3 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `TERM Lucky38ControlTerminalWarning` +3 more | 12 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` |  |
| `VMQHouse8` | 0014788B | 2 | 1 |  | stages set by `Lucky38SecuritronSCRIPT (on CREA VL38SecuritronMk2 +5)`, `Lucky38VictorElevatorScript (on CREA Lucky38VictorElevator)`, `Lucky38VictorEntranceSCRIPT (on CREA Lucky38VictorEntrance)`, `MrHouseSCRIPT (on CREA MrHouse)` +4 more | 11 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38SuiteFloor22`, `Lucky38World` +1 more |  |
| `VMQHouseFail` | 00165AC2 | 2 | 2 |  | stages set by `NVCRMrHouseSCRIPT (on CREA NVCRMrHouse)`, `TERM Lucky38ControlTerminalWarning3` | 5 |  |  |  |  |  |
| `VMQNCRFail` | 00167F0E | 2 | 6 | start game enabled, 0x10 | new game | 10 |  |  |  | `HooverDamIntOliverArea` |  |
| `VMQTops` | 0011345D | 16 | 13 | start game enabled, 0x10 | new game; stages set by `BennySCRIPT (on NPC_ Benny)`, `PACK VTopsBennyFlee`, `TopsInteriorDoorScript (on DOOR VTopsInteriorDoor)`, `VDialogueBenny dialogue` +10 more | 238 |  |  |  | `WastelandNV` |  |
| `VMQYesMan01` | 00157321 | 5 | 10 |  | stages set by `Lucky38UpgradeSecuritronsSCRIPT (on ACTI UpgradeSecuritronsObject)`, `VGenericTimerSCRIPT (on QUST VGenericTimer)`, `VMQTops dialogue` | 51 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38World` |  |
| `VMQYesMan01a` | 0016A161 | 2 | 4 |  | stages set by `SVMainConsoleScript (on ACTI SVMainConsole)`, `SecuritronVaultExplodeQuestScript (on QUST SecuritronVaultExplodeQuest)`, `VMQTops dialogue` | 13 |  |  |  |  |  |
| `VMQYesMan02` | 00157322 | 1 | 31 |  | stages set by `VMQTops dialogue` | 74 |  |  |  |  |  |
| `VMQYesMan03` | 0015827D | 1 | 7 |  | stages set by `VMQTops dialogue` | 31 |  |  |  | `HooverDamIntOliverArea` |  |
| `VMQYesManFailsafe` | 00165C37 | 2 | 2 | start game enabled, 0x10 | new game; stages set by `VMQTops dialogue` | 18 |  |  |  |  |  |
| `VMQYesManSupport` | 00174B63 | 0 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38World`, `WastelandNV` |  |
| `VMS01` | 00080664 | 10 | 21 |  | started by `VDialogueCliffBriscoe dialogue`; stages set by `ChrisHaversamScript (on NPC_ ChrisHaversam)`, `DialogueNobark dialogue`, `GhoulJasonBrightSCRIPT (on CREA JasonBright2 +1)`, `REPCONEnterScript (on ACTI REPCONEntrance2Trigger +1)` +6 more | 90 |  |  |  |  |  |
| `VMS01Nightkin` | 00080235 | 0 | 0 | start game enabled, 0x10 | new game | 13 |  |  |  |  |  |
| `VMS01REPCONGhouls` | 000810E2 | 1 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VMS02` | 0008D0E3 | 13 | 10 |  | started by `VFreeformNCRCF dialogue`; stages set by `NCRCFChavezScript (on NPC_ NCRCFChavez)`, `NCRCFEddieDeathScript (on NPC_ NCRCFEddie)`, `PrimmNashDialogue dialogue`, `PrimmSergeantLeeScript (on NPC_ PrimmSergeantLee)` +2 more | 44 |  |  |  |  |  |
| `VMS04` | 000E37E7 | 7 | 6 |  | started by `vFreeformSloan dialogue`; stages set by `PrimmCorporalHayesSCRIPT (on NPC_ PrimmCorporalHayes)`, `QJChemsScript (on MISC QJChems)`, `QJMelissaScript (on NPC_ QJMelissa)`, `QJSuitCaseScript (on CONT QJSuitCase)` +1 more | 22 |  |  |  |  |  |
| `VMS05` | 000EDB37 | 11 | 10 |  | started by `VDialogueCrimsonCaravan dialogue`; stages set by `AliceMcLaffertyScript (on NPC_ AliceMcLafferty)`, `ThomasHildernScript (on NPC_ ThomasHildern)`, `VFreeformVault22 dialogue` | 39 |  |  |  |  |  |
| `VMS06` | 000EFB3B | 7 | 7 |  | stages set by `KeelyScript (on NPC_ Keely)`, `TERM V22DownloadMainframe`, `ThomasHildernScript (on NPC_ ThomasHildern)`, `VFreeformVault22 dialogue` | 29 |  |  |  |  |  |
| `VMS07` | 000E6A83 | 0 | 0 | start game enabled, 0x10 | new game | 54 |  |  |  |  |  |
| `VMS08` | 000E790F | 1 | 17 | start game enabled, 0x10 | new game; stages set by `VMcCarranCurtisSCRIPT (on NPC_ CaptainCurtis)`, `VMcCarranHsuSCRIPT (on NPC_ ColonelHsu)` | 87 |  |  |  |  |  |
| `VMS08Timer` | 00161898 | 0 | 0 |  | started by `VMS08TrainExitTriggerSCRIPT (on ACTI VMS08TrainExitTrigger)` | 2 |  |  |  |  |  |
| `VMS09` | 000E8F7D | 3 | 3 |  | stages set by `VFreeformSSHQ dialogue` | 8 |  |  |  |  |  |
| `VMS09a` | 00105998 | 2 | 1 |  | stages set by `SSHQAllenMarksTriggerScript (on ACTI SSHQAllenMarksTrigger)`, `VFreeformSSHQ dialogue` | 7 |  |  |  |  |  |
| `VMS10` | 000E8FA0 | 11 | 8 | start game enabled, 0x10 | new game | 19 |  |  |  |  |  |
| `VMS11` | 000F0629 | 14 | 11 | start game enabled, 0x10 | new game; stages set by `AndersonScript (on NPC_ WestsideAnderson)`, `VFreeformCasaMadrid dialogue`, `VFreeformMcCarran2 dialogue`, `VFreeformMcCarranBoydSCRIPT (on NPC_ LieutenantBoyd)` +2 more | 51 |  |  |  |  |  |
| `VMS12` | 000F81BD | 12 | 10 |  | started by `VFreeformMcCarran3 dialogue`; stages set by `MajorDhatriScript (on NPC_ 1EMajorDhatri)` | 49 |  |  |  |  |  |
| `VMS13` | 000F8892 | 7 | 6 |  | stages set by `CaptainParkerScript (on NPC_ 1ECaptainParker)`, `VFreeformCasaMadrid dialogue`, `VMS13DermotScript (on NPC_ WestsideDermot)`, `VMS13NoteTriggerScript (on ACTI VMS13NoteTrigger)` +3 more | 26 |  |  |  |  |  |
| `VMS15` | 000FF5DF | 11 | 10 |  | stages set by `NellisArgyllScript (on NPC_ NellisArgyll)`, `NellisJackSCRIPT (on NPC_ NellisJack)`, `NellisLoyalScript (on NPC_ NellisLoyal)`, `NellisPearlScript (on NPC_ NellisPearl)` +3 more | 35 |  |  |  |  |  |
| `VMS16b` | 0015EC5B | 6 | 8 |  | started by `VFreeformGoodsprings dialogue`; stages set by `GoodspringsJoeCobbScript (on NPC_ GSJoeCobb)`, `GoodspringsPowderGangerScript (on NPC_ GSPGHM2 +5)` | 21 |  |  |  |  | The Powder Gangers' side of the gunfight: not tried (the gunfight route ran only Trudy's side). (Evidence: docs/GOODSPRINGS_ROUTE.md.) |
| `VMS18` | 0010D2C6 | 1 | 25 | start game enabled, 0x10 | new game; stages set by `VMS18CarlyleStClairSCRIPT (on NPC_ VMS18CarlyleStClair)`, `VMS18HeckGundersonSCRIPT (on NPC_ VMS18HeckGunderson)`, `VMS18MortimerSCRIPT (on NPC_ VMS18Mortimer)`, `VMS18TedGundersonSCRIPT (on NPC_ VMS18TedGunderson)` | 212 |  |  |  |  |  |
| `VMS18Timer` | 001299AF | 0 | 0 |  | started by `VMS18 dialogue` | 3 |  |  |  |  |  |
| `VMS19` | 0010E196 | 8 | 21 |  | stages set by `VFSLocalVictim02Script (on NPC_ VFSLocalVictim02)`, `VFSLocalVictim03Script (on NPC_ VFSLocalVictim03)`, `VFSLocalVictimScript (on NPC_ VFSLocalVictim01)`, `VFSPacerScript (on NPC_ VFSPacer)` +2 more | 98 |  |  |  |  |  |
| `VMS20` | 0010E908 | 6 | 9 |  | started by `VFreeformBoulderCity dialogue`; stages set by `BCJessupScript (on NPC_ TestGreatKhanHF +8)`, `BCTrooperHostageScript (on NPC_ BCTrooperHostageCF +1)`, `BCTrooperScript (on NPC_ BCLieutenantMonroe +5)` | 41 |  |  |  |  |  |
| `VMS21a` | 00110A65 | 23 | 18 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `VMS21 stage 50`, `VMS21 stage 55`, `VMS21aCarlitosSCRIPT (on NPC_ CarlitosWayne)`, `VMS21aJoanaFriendScript (on NPC_ VMS21aJoanaFriend01 +1)` +3 more | 136 |  |  |  |  |  |
| `VMS21aSupport` | 00127A5B | 0 | 0 |  | started by `VMS21a dialogue`, `vDialogueGomorrah dialogue` | 10 |  |  |  |  |  |
| `VMS22` | 00112577 | 10 | 8 | start game enabled, repeated stages, 0x10 | new game; stages set by `VMS22CamTargetScript (on ACTI VMS22CamTargetTrigger02 +1)`, `VMS22MichaelAngeloScript (on NPC_ VMS22MichaelAngelo)` | 58 |  |  |  |  |  |
| `VMS23` | 00116B41 | 8 | 6 |  | stages set by `VMQNCRFail stage 5`, `VMQNCRFailSCRIPT (on QUST VMQNCRFail)`, `VMS23StartTrigSCRIPT (on ACTI VMS23StartTrig)`, `VStreetDennisCrockerSCRIPT (on NPC_ VStreetDennisCrocker)` +1 more | 23 |  |  |  |  |  |
| `VMS24` | 001176B8 | 6 | 7 |  | started by `VFreeformFreeside dialogue`, `vDialogueTestaclesDebug dialogue`; stages set by `JacobstownCalamityScript (on NPC_ JacobstownCalamity)`, `JacobstownMarcusScript (on CREA JacobstownMarcus)`, `RexSCRIPT (on CREA Rex)`, `VDialogueRemnants dialogue` +6 more | 33 |  |  |  |  |  |
| `VMS25` | 0011F86A | 19 | 20 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `VMS18 dialogue`, `VMS18HeckArrestTriggerSCRIPT (on ACTI VMS18HeckArrestTrigger)`, `VMS18HeckGundersonSCRIPT (on NPC_ VMS18HeckGunderson)`, `VMS18TedGundersonSCRIPT (on NPC_ VMS18TedGunderson)` +2 more | 112 |  |  |  |  |  |
| `VMS26` | 0011F86B | 6 | 6 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 0 |  |  |  |  |  |
| `VMS29` | 0011F95B | 12 | 17 | start game enabled, 0x10 | new game; stages set by `VFSNCRTrooperSentToKingsScript (on NPC_ VFSNCRTrooperSentToKings)`, `VMQNCRFail stage 5`, `VMQNCRFailSCRIPT (on QUST VMQNCRFail)`, `VStreetDennisCrockerSCRIPT (on NPC_ VStreetDennisCrocker)` | 86 |  |  |  |  |  |
| `VMS29a` | 00124123 | 7 | 14 | start game enabled, repeated topics, 0x10 | new game; stages set by `VDialogueCass dialogue`, `VFSGloriaVanGraffScript (on NPC_ VFSGloriaVanGraff)`, `VFSJeanBaptisteCuttingScript (on NPC_ VFSJeanBaptisteCutting)`, `VFSPacerScript (on NPC_ VFSPacer)` +3 more | 151 |  |  |  | `FreesideSilverRush`, `FreesideVanGraffWarehouse`, `FreesideWorld` |  |
| `VMS30` | 001214AA | 20 | 14 | start game enabled, 0x10 | new game; stages set by `VCFHAlexRichardsScript (on NPC_ VCFHAlexRichards)`, `VCFHCarlMayesScript (on NPC_ VCFHCarlMayes)`, `VCFHMajorPolatliScript (on NPC_ VCFHMajorPolatli)`, `VCFHSergeantCooperScript (on NPC_ VCFHSergeantCooper)` +5 more | 140 |  |  |  |  |  |
| `VMS30HealTimer1` | 00152E8A | 0 | 0 |  | started by `VMS30InjuredTrooper01Script (on NPC_ VForlornHopeNCRTrooperInjured01)` | 4 |  |  |  |  |  |
| `VMS30HealTimer2` | 00152E8B | 0 | 0 |  | started by `VMS30InjuredTrooper02Script (on NPC_ VForlornHopeNCRTrooperInjured02)` | 2 |  |  |  |  |  |
| `VMS30HealTimer3` | 00152E8C | 0 | 0 |  | started by `VMS30InjuredTrooper03Script (on NPC_ VForlornHopeNCRTrooperInjured03)` | 2 |  |  |  |  |  |
| `VMS31` | 001214AB | 13 | 6 | start game enabled, 0x10 | new game; stages set by `REFR VLegateTentGuard01MarkerREF`, `REFR VLegateTentGuard02MarkerREF`, `REFR VMS31MedicalSuppliesRef`, `VCFHAlexRichardsScript (on NPC_ VCFHAlexRichards)` +6 more | 54 |  |  |  |  |  |
| `VMS32` | 001268BD | 6 | 4 |  | started by `VFreeformTheFort dialogue`; stages set by `FortHowitzerScript (on ACTI FortHowitzer)`, `FortLuciusScript (on NPC_ FortLucius)`, `HowitzerFiringMechanismScript (on MISC HowitzerFiringMechanism)`, `VFreeformNellis dialogue` +1 more | 14 |  |  |  | `WastelandNV` |  |
| `VMS33` | 001271EA | 7 | 4 |  | started by `VFreeformTheFort dialogue`; stages set by `MartinaGroesbeckScript (on NPC_ MartinaGroesbeck)`, `VMQ02 stage 110`, `VMS33ThugScript (on NPC_ VMS33OmertaThug03 +2)`, `VulpesIncultaNiptonSCRIPT (on NPC_ VLegionaryVulpesInculta)` +2 more | 24 |  |  |  |  |  |
| `VMS34` | 00129D1E | 0 | 0 | start game enabled, 0x10 | new game | 13 |  |  |  | `WastelandNV` |  |
| `VMS35` | 001252D6 | 10 | 9 |  | started by `VDialogueCaesar dialogue`; stages set by `FortCaesarScript (on NPC_ FortCaesar)`, `VFreeformTheFort dialogue`, `VMQ02 stage 110`, `VMS35AutoDocScript (on ACTI VMS35AutoDoc)` +1 more | 26 |  |  |  | `WastelandNV` |  |
| `VMS36` | 00133045 | 5 | 8 | start game enabled, 0x10 | new game; stages set by `SgtMcCredieScript (on NPC_ 2CMcCredie)`, `VBSCaptainGillesSCRIPT (on NPC_ VBSCaptainGilles)`, `VCFHMajorPolatliScript (on NPC_ VCFHMajorPolatli)`, `VMcCarranHsuSCRIPT (on NPC_ ColonelHsu)` | 33 |  |  |  |  |  |
| `VMS37` | 00133046 | 3 | 2 | start game enabled, 0x10 | new game; stages set by `VBSCaptainGillesSCRIPT (on NPC_ VBSCaptainGilles)`, `VMS36 dialogue` | 22 |  |  |  |  |  |
| `VMS38` | 001348DB | 21 | 14 | start game enabled, repeated topics, 0x10 | new game; stages set by `VMS38CazadorEggsPickScript (on ACTI VPitQuestCazadorEggs)`, `VMS38DeathclawEggsPickScript (on ACTI VPitQuestDeathclawEggs)`, `VMS38FireGeckoEggsPickScript (on ACTI VPitQuestFireGeckoEggs)`, `VMS38MantisEggsPickScript (on ACTI VPitQuestMantisEggs)` +4 more | 317 |  |  |  | `OVWestSewers03` |  |
| `VMS38a` | 0013AE5A | 0 | 0 |  | started by `VMS38 stage 10`, `VMS38 stage 110`, `VMS38 stage 30`, `VMS38 stage 50` +2 more | 13 |  |  |  |  |  |
| `VMS38b` | 0015F18A | 0 | 0 |  | started by `TERM VMS38LucyTerminalSubASubC` | 2 |  |  |  |  |  |
| `VMS39` | 001349A7 | 8 | 6 |  | started by `VDialogueCraigBoone dialogue`; stages set by `BooneSCRIPT (on NPC_ CraigBoone)`, `VMS18 dialogue`, `VMS39TimerSCRIPT (on QUST VMS39Timer)`, `VNPCFollowers dialogue` | 27 |  |  |  |  |  |
| `VMS39Timer` | 00135696 | 0 | 0 |  | started by `VDialogueCraigBoone dialogue` | 3 |  |  |  |  |  |
| `VMS40` | 00139B8D | 3 | 6 | start game enabled, 0x10 | new game; stages set by `BlakeSCRIPT (on NPC_ CrimsonCaravanBlake)`, `VBSCaptainGillesSCRIPT (on NPC_ VBSCaptainGilles)`, `VBSLtMarklandSCRIPT (on NPC_ VBSLieutenantMarkland)` | 20 |  |  |  |  |  |
| `VMS40Timer` | 00142AB3 | 0 | 0 |  | started by `VMS40 dialogue` | 2 |  |  |  |  |  |
| `VMS42` | 0013E5AF | 8 | 7 | start game enabled, 0x10 | new game; stages set by `VMS42C4ExplosionScript`, `VMS42MicroclineRockScript (on ACTI MicroclineRock)`, `VMS42QuestFailScript (on NPC_ VVault19PhilipLem +1)`, `VRRCPapaKhanSCRIPT (on NPC_ VRRCPapaKhan)` | 40 |  |  |  | `Vault19b`, `WastelandNV` |  |
| `VMS43` | 0014050C | 4 | 2 | start game enabled, 0x10 | new game; stages set by `AstorDeadScript (on NPC_ CampSearchlightAstor)` | 52 |  |  |  |  |  |
| `VMS43a` | 00144519 | 12 | 0 |  | stages set by `ActivateEdwardsCamp (on ACTI EdwardsCampTrigger)`, `ActivateEdwardsEchoSandbox (on ACTI EdwardsSandboxEchoTrigger)`, `CampSearchlightEdwardsAddsGhoulsScript (on ACTI CampSearchlightEdwardsRecruitGhouls)`, `CampSearchlightEdwardsFindsAstorScript (on ACTI CampSearchlightEdwardsFindsAstor)` +3 more | 31 |  |  |  |  |  |
| `VMS44` | 0014050D | 10 | 5 | start game enabled, repeated stages, 0x10 | new game; stages set by `CampSearchlightLoganSearchFire (on ACTI CampSearchlightLoganFireSearch)`, `CampSearchlightLoganSearchFire01`, `CampSearchlightLoganSearchPolice (on ACTI LoganInPolice)`, `LoganDeadScript (on NPC_ CampSearchlightLogan)` +2 more | 32 |  |  |  |  |  |
| `VMS45` | 00140C3A | 7 | 20 | start game enabled, repeated topics, 0x10 | new game; stages set by `QJMelissaScript (on NPC_ QJMelissa)`, `VFreeformHooverDam dialogue`, `VRRCDianeScript (on NPC_ VRRCDiane)`, `VRRCGreatKhanScript (on NPC_ VGreatKhanSniffer01 +21)` +4 more | 108 |  |  |  |  |  |
| `VMS46` | 00140C3B | 8 | 8 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `DonHostetlerScript (on NPC_ NorthVegasMrHostetler)`, `V03MotorRunnerScript (on NPC_ V03MotorRunner)`, `VRRCDianeScript (on NPC_ VRRCDiane)`, `VRRCJackScript (on NPC_ VRRCJack)` | 48 |  |  |  |  |  |
| `VMS46Timer` | 00146F1D | 0 | 0 |  | started by `VMS46 dialogue` | 6 |  |  |  |  |  |
| `VMS47` | 00140C3C | 4 | 4 | start game enabled, repeated topics, 0x10 | new game; stages set by `VFSJulieFarkasSCRIPT (on NPC_ VFSJulieFarkas)`, `VRRCJerryThePunkScript (on NPC_ VRRCJerrythePunk)` | 10 |  |  |  |  |  |
| `VMS49Timer` | 00163E72 | 0 | 0 |  | started by `VMS49HVExitTriggerSCRIPT (on ACTI VMS49HVExitTrigger)` | 2 |  |  |  |  |  |
| `VMS50` | 00148A34 | 5 | 6 |  | started by `JacobstownMercenaryScript (on NPC_ JacobstownNorton +2)`, `vDialogueJacobstown dialogue`; stages set by `JacobstownMarcusScript (on CREA JacobstownMarcus)` | 21 |  |  |  |  |  |
| `VMS51` | 0014EF5F | 4 | 2 |  | started by `VDialogueCrimsonCaravan dialogue`; stages set by `AliceMcLaffertyScript (on NPC_ AliceMcLafferty)`, `VMS51BottleCapPressScript (on ACTI VMS51BottleCapPress)` | 8 |  |  |  |  |  |
| `VMS52` | 0014F3B1 | 10 | 18 |  | started by `VMSRSGlobalCommBreakdown dialogue`; stages set by `CHFSergeantReyesScript (on NPC_ CHFSergeantReyes)`, `ChiefHanlonScript (on NPC_ 2CHanlon)`, `VCampGolfHanlon dialogue`, `VMS52RadioAlphaScript (on ACTI VMS52RadioAlpha)` +12 more | 43 |  |  |  |  |  |
| `VMS53` | 00151403 | 1 | 4 |  | stages set by `BooneSCRIPT (on NPC_ CraigBoone)`, `VNovacCliffBriscoeScript (on NPC_ CliffBriscoe)`, `VNovacJeannieCrawfordScript (on NPC_ JeannieMayCrawford)`, `VNovacSniperVictimSCRIPT (on NPC_ AliceMcBride +4)` | 19 |  |  |  |  |  |
| `VMS54` | 00157E60 | 6 | 14 |  | started by `VDialogueArcadeGannon dialogue`; stages set by `ArcadeScript (on NPC_ VFSArcadeGannon)`, `JacobstownMarcusScript (on CREA JacobstownMarcus)`, `SLRemnantsBunkerPanelScript (on ACTI SLRemnantsBunkerPanel)`, `VDialogueArcadeGannonScript (on QUST VDialogueArcadeGannon)` +8 more | 44 |  |  |  |  |  |
| `VMS55` | 0015D79D | 11 | 26 | start game enabled, 0x10 | new game; stages set by `HVBunkerExplosionScript (on ACTI HVBunkerExplosionTrigger)`, `HVKnightEDEScript (on NPC_ HVKnightEDE)`, `HVPaladinHitSquadFollowerScript (on NPC_ HVPaladinHitSquadRandom)`, `HVPaladinHitSquadLeaderScript (on NPC_ HVPaladinHitSquad1)` +6 more | 65 |  |  |  | `HiddenValley02` |  |
| `VMS55a` | 001633F2 | 4 | 2 |  | stages set by `HVBunkerExplosionScript (on ACTI HVBunkerExplosionTrigger)`, `HVHardinScript (on NPC_ EdgarHardin)`, `VDialogueHiddenValley dialogue` | 10 |  |  |  |  |  |
| `VMS55b` | 001638F0 | 4 | 2 |  | stages set by `HVBmRadioConsoleScript (on ACTI HVBMRadioConsole)`, `HVBunkerExplosionScript (on ACTI HVBunkerExplosionTrigger)`, `VDialogueHiddenValley dialogue` | 8 |  |  |  |  |  |
| `VMS56` | 0015E480 | 7 | 8 |  | stages set by `CorporalBetsyScript (on NPC_ 1ECorporalBetsy)`, `CorporalSterlingScript (on NPC_ 1ECorporalSterling)`, `DoctorUsanagiScript (on NPC_ DoctorUsanagi)`, `LieutenantGorobetsScript (on NPC_ 1ELieutenantGorobets)` +4 more | 26 |  |  |  |  |  |
| `VMS57` | 00166B87 | 2 | 2 | start game enabled, 0x10 | new game; stages set by `BMTabithaScript (on CREA Tabitha)`, `VFreeformBlackMountain dialogue` | 14 |  |  |  |  |  |
| `VMSRSGlobalCommBreakdown` | 0010D229 | 20 | 24 | start game enabled, 0x10 | new game; stages set by `RSFoxtrotQuestTestSCRIPT` | 20 |  |  |  |  |  |
| `VMSTopsPerformances` | 00125498 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 11 |  |  |  |  |  |
| `VMSTopsTalentPool` | 00116504 | 11 | 11 | start game enabled, 0x10 | new game; stages set by `VDialogueTops dialogue`, `VMSLonesomeDrifterSCRIPT (on NPC_ VMSTopsLonesomeDrifter)`, `VMSTommyToriniSCRIPT (on NPC_ VMSTopsTommyTorini)`, `VMSTopsBillyKnightSCRIPT (on NPC_ VMSTopsBillyKnight)` +2 more | 53 |  |  |  |  |  |
| `VNellisGenericBoomerBanter` | 001057D6 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `VNellisInfestation` | 001083DE | 4 | 6 |  | stages set by `NellisRaquelSCRIPT (on NPC_ NellisRaquel)`, `VFreeformNellis dialogue`, `VMS15Script (on QUST VMS15)` | 17 |  |  |  |  |  |
| `VNellisJack` | 0015CB8B | 1 | 10 | repeated topics, repeated stages | stages set by `NellisJackSCRIPT (on NPC_ NellisJack)`, `NellisJanetSCRIPT (on NPC_ NellisCrimsonCaravanJanet)`, `NellisPearlScript (on NPC_ NellisPearl)`, `VDialogueCrimsonCaravan dialogue` +2 more | 29 |  |  |  |  |  |
| `VNelson` | 00129445 | 4 | 2 | start game enabled, 0x10 | new game; stages set by `MiloScript (on NPC_ NVNelsonRangerMilo)`, `VNelsonHostageRestrain2 (on NPC_ NelsonNCRTrooperHostage3 +2)` | 33 |  |  |  |  |  |
| `VNightTimeQuest` | 00176538 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VNipton` | 00131E7C | 4 | 2 | start game enabled, 0x10 | new game; stages set by `NiptonSurvivorDeathScript (on NPC_ vNiptonSurvivorCaptive)`, `NiptonSurvivorDeathScript02 (on NPC_ vNiptonSurvivorCaptive02)`, `boxcarscrippledSCRIPT (on NPC_ vNiptonBoxcars2)`, `vNiptonRescueSCRIPT (on ACTI vNiptonRescueTrigger)` | 61 |  |  |  |  |  |
| `VOuterVegasFiends` | 0013AE42 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VOuterVegasLighting` | 0013AACA | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VReactionVeronica` | 001735CE | 0 | 0 |  | started by `VFreeformTheStreet01 dialogue`, `VHDVeronicaReactSCRIPT (on ACTI VHDVeronicaReact)`, `VMS49 dialogue`, `VVeronicaHelloQuestTriggerSCRIPT (on ACTI VVeronicaHelloQuestTrigger)` | 25 |  |  |  |  |  |
| `VStoryStateQuest` | 00179B03 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `VStreetCombat` | 00129014 | 0 | 0 | start game enabled, 0x10 | new game | 13 |  |  |  | `2ELVBStation`, `Lucky38World`, `WastelandNV` |  |
| `VStreetFluffNPC` | 0011F8A2 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 47 |  |  |  | `2ELVBStation`, `Lucky38World`, `WastelandNV` |  |
| `VStreetLighting` | 00127E9D | 0 | 0 | start game enabled, 0x10 | new game; started by `VStreetLightingQuestStarterTrigger (on ACTI VStreetLightingQuestTrigger)` | 3 |  |  |  |  |  |
| `VStreetReputation` | 001602BC | 0 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  |  |  |
| `VStreetSS1` | 0016A997 | 0 | 0 |  | started by `VStripSS0BRandomizerTrigger (on ACTI VStripSS00BRandomizerTrigger)`, `VStripSS1StationTrigger02 (on ACTI VStreetSS1StationTrigger02)` | 3 |  |  |  |  |  |
| `VStreetSS2` | 00151E2B | 0 | 0 | repeated topics | started by `VFSVegasDoorSCRIPT (on DOOR VFSVegasGate)`, `VStripSS2StationTriggerScript (on ACTI VStreetSS2StationTrigger)` | 7 |  |  |  |  |  |
| `VStreetSS3` | 0015B7BC | 0 | 0 |  | started by `VFSVegasDoorSCRIPT (on DOOR VFSVegasGate)`, `VStripSS2StationTriggerScript (on ACTI VStreetSS2StationTrigger)` | 3 |  |  |  |  |  |
| `VStreetSS4` | 0015BD5E | 0 | 0 |  | started by `VStripSS0BRandomizerTrigger (on ACTI VStripSS00BRandomizerTrigger)`, `VStripSS0RandomizerTrigger (on ACTI VStripSS00RandomizerTrigger)` | 4 |  |  |  |  |  |
| `VStreetSS5` | 00163441 | 0 | 0 |  | started by `VStripSS0BRandomizerTrigger (on ACTI VStripSS00BRandomizerTrigger)`, `VStripSS0RandomizerTrigger (on ACTI VStripSS00RandomizerTrigger)` | 4 |  |  |  |  |  |
| `VStreetSS6` | 00163E9F | 0 | 0 |  | started by `VLucky38ExitDoorScript (on DOOR HotelDoor01Lucky38Exit)` | 2 |  |  |  |  |  |
| `VStreetSS7` | 0017A885 | 0 | 0 |  | started by `VStripSS0BRandomizerTrigger (on ACTI VStripSS00BRandomizerTrigger)`, `VStripSS0RandomizerTrigger (on ACTI VStripSS00RandomizerTrigger)` | 3 |  |  |  |  |  |
| `VT74` | 0015412F | 2 | 1 | start game enabled, 0x10 | new game; stages set by `VT74EndTRIG (on ACTI VT74OverseerOffice)`, `VT74stage10triggerSCRIPT (on ACTI VT74stage10TRIG)` | 4 |  |  |  |  |  |
| `VTechatticup` | 0011F935 | 4 | 3 | start game enabled, 0x10 | new game; stages set by `NVTechNCRRenoldsSCRIPT (on NPC_ NVTechatticupNCRRenolds)`, `TecMineHostage (on NPC_ TecMineNCRHostage)` | 17 |  |  |  |  |  |
| `VTopsSecurity` | 001292AC | 0 | 0 |  | started by `TopsExteriorDoorSCRIPT (on DOOR VTopsExteriorDoor)`, `VDialogueTops dialogue` | 13 |  |  |  |  |  |
| `VUltraLuxeSecurity` | 001656FA | 0 | 0 |  | started by `VMS18 dialogue` | 4 |  |  |  |  |  |
| `VVault21FluffNPC` | 0011F460 | 0 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 40 |  |  |  |  |  |
| `VVault21Tour` | 0011B637 | 8 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game; stages set by `PACK VVault21SarahEndTour`, `VFreeformTheStreet01 dialogue` | 9 |  |  |  |  |  |
| `Vault106DialogueShell` | 0007086C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `Vault108DialogueShell` | 00070869 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `WaterBeggars` | 000C6E93 | 1 | 0 | start game enabled, 0x10 | new game | 7 |  |  |  |  |  |
| `nVPrimmVanceGun` | 000E2891 | 0 | 1 | start game enabled, 0x10 | new game | 8 |  |  |  |  |  |
| `nvNelsonExplorerBark` | 0014EA0C | 2 | 0 | start game enabled, 0x10 | new game | 11 |  |  |  | `NelsonBarracks02` |  |
| `vBennySexTimer` | 001672B0 | 0 | 0 |  | started by `VDialogueBenny dialogue` | 4 |  |  |  |  |  |
| `vCCEyeforaneye` | 00134498 | 3 | 10 |  | started by `VMS43 dialogue`; stages set by `AstorDeadScript (on NPC_ CampSearchlightAstor)` | 34 |  |  |  |  |  |
| `vCCLeftMyHeart` | 00135F63 | 2 | 4 |  | stages set by `1EFrankWeathersSCRIPT (on NPC_ 1EFrankWeathers)`, `NVCCKennyWeathersSCRIPT (on NPC_ NVCCKennyWeathers)`, `NVCCMrsWeathersSCRIPT (on NPC_ NVCCMrsWeathers)`, `NVCCSammyWeathersSCRIPT (on NPC_ NVCCSammyWeathers)` | 60 |  |  |  |  |  |
| `vCCmovetothefort` | 001672BD | 0 | 0 |  | started by `NVCCBargeSCRIPT (on DOOR NVCCBarge)`, `NVCCTheFortGateSCRIPT (on DOOR NVCCInvisibleFortGate)`, `VFreeformCottonwoodCove dialogue` | 6 |  |  |  | `WastelandNV` |  |
| `vCampGuardianRadio` | 0014ADAE | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 8 |  |  |  |  |  |
| `vCaravanQuest` | 00157F76 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 17 |  |  |  |  |  |
| `vCasinoCompsAtomicWrangler` | 001614E9 | 0 | 0 |  | started by `casino AtomicWranglerCasinoData (its winnings quest) [G]` | 1 |  |  |  |  |  |
| `vCasinoCompsGomorrah` | 001607FF | 0 | 0 | start game enabled, 0x10 | new game; started by `casino GomorrahCasinoData (its winnings quest) [G]` | 5 |  |  |  |  |  |
| `vCasinoCompsTheTops` | 00145F88 | 0 | 1 |  | started by `casino TheTopsCasinoData (its winnings quest) [G]` | 10 |  |  |  |  |  |
| `vCasinoCompsVikkiAndVance` | 00161E96 | 0 | 0 |  | started by `casino VikkiVanceCasinoData (its winnings quest) [G]` | 6 |  |  |  |  |  |
| `vCaveTransitionISFX` | 0014B0C8 | 0 | 0 |  | started by `NVCGBlackCaveDoorSCRIPT (on DOOR NVCGInvisibleCaveEntDoor +1)` | 4 |  |  |  |  |  |
| `vCountryRadioQuest` | 0016B66E | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 16 |  |  |  |  |  |
| `vCrucifiedFaction` | 00155DBF | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueBoulderCityGeneric` | 00140A7A | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCC` | 00140A97 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCaesarsLegionCrucified` | 0013AC9B | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCaesarsLegionMilitary` | 00129E29 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCaesarsLegionMilitaryCombat` | 0015ACB4 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCaesarsLegionSlave` | 0013FB6C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCampForlornHope` | 001214A9 | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueCampGolf` | 00140A92 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueCasinoCashier` | 00144F34 | 0 | 0 | start game enabled, 0x10 | new game | 19 |  |  |  |  |  |
| `vDialogueCrimsonCaravanGeneric` | 0013FB81 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueEDE` | 001572E8 | 3 | 12 | start game enabled, repeated stages, 0x10 | new game; started by `EDEScriptDisabled (on CREA NVCompanionEdEDdisabled)`; stages set by `DialogueOldLadyGibson dialogue`, `EDEScript (on CREA NVCompanionEdEUpgradeWeapons +2)`, `VDialogueBrotherhoodOfSteelFaction dialogue`, `VDialogueCaesar dialogue` +11 more | 71 |  |  |  |  |  |
| `vDialogueFollowersApocalypse` | 0013FB70 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueFreeside` | 00140A94 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueFreesideDrunk` | 00155ECE | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueGomorrahGeneric` | 001164BF | 0 | 0 | start game enabled, 0x10 | new game | 23 |  |  |  |  |  |
| `vDialogueGoodspringsPowderGanger` | 0015EFB4 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueGreatKhansGeneric` | 0013FB6B | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueGrubnGulp` | 00142969 | 0 | 0 | start game enabled, 0x10 | new game | 10 |  |  |  |  |  |
| `vDialogueGunRunner` | 000EE506 | 0 | 0 | start game enabled, 0x10 | new game | 18 |  |  |  |  |  |
| `vDialogueGunRunnerGeneric` | 0013FB73 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueHooverDam` | 00136B27 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueJacobstown` | 0013BF54 | 0 | 0 | start game enabled, 0x10 | new game | 32 |  |  |  |  |  |
| `vDialogueJacobstownGeneric` | 0013E490 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueJacobstownMercenaryGeneric` | 0014B06A | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueJacobstownNightkinGeneric` | 001466F5 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueKings` | 0013FB6E | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueMedicalClinic` | 001474CB | 0 | 0 | start game enabled, 0x10 | new game | 26 |  |  |  |  |  |
| `vDialogueMojaveCivilian` | 0013FB74 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueNCRCFGeneric` | 0015441C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueNCRCivilian` | 0013FB75 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueNCREldoradoSubstation` | 0015A5FE | 0 | 0 | start game enabled, 0x10 | new game | 12 |  |  |  |  |  |
| `vDialogueNCRMilitary` | 0013FB6D | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueNCRMilitaryCombat` | 0015A79C | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueNightkin` | 0013FB77 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueNovacGeneric` | 0015FFC1 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueOuterVegas` | 00140A91 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialoguePowderGangerGeneric` | 0013FB72 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialoguePrimm` | 0015A388 | 0 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  |  |  |
| `vDialogueRangerStationAlpha` | 0010E007 | 0 | 0 | start game enabled, 0x10 | new game | 3 |  |  |  |  |  |
| `vDialogueRangerStationBravo` | 0010E008 | 6 | 0 | start game enabled, 0x10 | new game | 5 |  |  |  |  |  |
| `vDialogueRangerStationCharlie` | 0010E009 | 0 | 0 | start game enabled, 0x10 | new game | 4 |  |  |  |  |  |
| `vDialogueRangerStationDelta` | 0010E00A | 6 | 0 | start game enabled, 0x10 | new game | 4 |  |  |  |  |  |
| `vDialogueRangerStationEcho` | 0010E00B | 6 | 0 | start game enabled, 0x10 | new game | 5 |  |  |  |  |  |
| `vDialogueRangerStationFoxtrot` | 0010D228 | 0 | 0 | start game enabled, 0x10 | new game | 9 |  |  |  |  |  |
| `vDialogueRedRockCanyon` | 00140A98 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueSL` | 0015F0C0 | 3 | 0 | start game enabled, repeated topics, repeated stages, 0x10 | new game | 2 |  |  |  |  |  |
| `vDialogueSecuritron` | 0014072B | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueSharecropper` | 001400F8 | 0 | 0 | start game enabled, 0x10 | new game | 10 |  |  |  |  |  |
| `vDialogueSharecropperGeneric` | 0013F05D | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueSloanGeneric` | 0013EC0D | 0 | 0 | start game enabled, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueSuperMutant` | 0013FB76 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueSuperMutantFirstGen` | 0015CD9F | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueTestaclesDebug` | 00139B8C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 212 |  |  |  |  |  |
| `vDialogueTheFortGeneric` | 00140A96 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueTheStrip` | 00140A7B | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 24 |  |  |  |  |  |
| `vDialogueTheTops` | 0013FB6F | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 1 |  |  |  |  |  |
| `vDialogueVault19Generic` | 001449C0 | 0 | 0 | start game enabled, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueVault21Guests` | 0015405A | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vDialogueVegasEast2` | 000F95B9 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 58 |  |  |  |  |  |
| `vDialogueWestside` | 000ECE20 | 0 | 0 | start game enabled, 0x10 | new game | 12 |  |  |  |  |  |
| `vElDoradoSubstationNCRBark` | 00160130 | 0 | 0 | start game enabled, 0x10 | new game; started by `ElDoradoNCRBarkActivatorSCRIPT (on ACTI ElDoradoSubstationBarkActivator)` | 15 |  |  |  |  |  |
| `vFreeformCampGuardian` | 0013F400 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 16 |  |  |  |  |  |
| `vFreeformForlornHope02` | 001214AC | 1 | 1 | start game enabled, repeated stages, 0x10 | new game; stages set by `VMS30 dialogue` | 22 |  |  |  |  |  |
| `vFreeformForlornHope03` | 00126D49 | 2 | 1 | start game enabled, repeated stages, 0x10 | new game | 30 |  |  |  |  |  |
| `vFreeformSloan` | 001406A5 | 0 | 0 | start game enabled, 0x10 | new game | 31 |  |  |  |  |  |
| `vFreeformTheStreet02` | 00118D5E | 0 | 0 | start game enabled, 0x10 | new game | 52 |  |  |  |  |  |
| `vGSRadioQuest` | 0014E8B9 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 0 |  |  |  |  |  |
| `vGenericReputationQuest` | 0014D2C3 | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 10 |  |  |  |  |  |
| `vGomorrahRadioQuest` | 0016B66C | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 7 |  |  |  |  |  |
| `vMojave` | 00144122 | 5 | 2 | start game enabled, 0x10 | new game; stages set by `vMORangerJacksonSCRIPT (on NPC_ vMORangerJackson)` | 28 |  |  |  |  |  |
| `vMojaveGhost` | 00144124 | 4 | 3 | start game enabled, 0x10 | new game; stages set by `VFreeformTheFort dialogue`, `VulpesIncultaNiptonSCRIPT (on NPC_ VLegionaryVulpesInculta)`, `VulpesIncultaSCRIPT (on NPC_ FortVulpesInculta)`, `VulpesIncultaStripSCRIPT (on NPC_ VVulpesIncultaStrip)` +1 more | 28 |  |  |  |  |  |
| `vNCREmergencyRadio` | 0014B05B | 0 | 0 | repeated topics, repeated stages | started by `MrHouseSCRIPT (on CREA MrHouse)`, `VHDHouseBattle stage 10`, `VHDLegateCampGateScript (on DOOR VHDLegateCampGate)`, `VHDLegionBattle stage 10` +2 more | 59 |  |  |  | `WastelandNV` |  |
| `vNellisMoveToPearl` | 001672BC | 0 | 0 |  | started by `VFreeformNellis dialogue` | 2 |  |  |  | `NellisBarracks01` |  |
| `vNiptonVulpes` | 001349B5 | 4 | 2 | start game enabled, 0x10 | new game; stages set by `VNipton dialogue`, `VulpesIncultaNiptonSCRIPT (on NPC_ VLegionaryVulpesInculta)`, `VulpesIncultaStripSCRIPT (on NPC_ VVulpesIncultaStrip)`, `vMOSgtKilbornSCRIPT (on NPC_ vMOSgtKilborn)` | 20 |  |  |  |  |  |
| `vSafehouse` | 00159B12 | 0 | 0 | start game enabled, 0x10 | new game | 20 |  |  |  |  |  |
| `vSnowglobeEnabler` | 0016954C | 0 | 0 |  | started by `VDialogueLucky38Penthouse dialogue` | 3 |  |  |  |  |  |
| `vUltraLuxeRadioQuest` | 0016B66D | 0 | 0 | start game enabled, repeated topics, 0x10 | new game | 8 |  |  |  |  |  |
| `vfreeformlucky38` | 0011A0C5 | 9 | 5 | start game enabled, 0x10 | new game; stages set by `EmilyOrtalSCRIPT (on NPC_ Lucky38FollowerEmily)`, `Lucky38BasementTrigger02SCRIPT`, `Lucky38MainframeSCRIPT (on TERM Lucky38MainframeTerminal)`, `NVCRMrHouseSCRIPT (on CREA NVCRMrHouse)` | 44 |  |  |  | `Lucky38BasementFloorB2`, `Lucky38CasinoFloor01`, `Lucky38ControlRoom`, `Lucky38SuiteFloor22` +1 more |  |

### NEVER STARTED (27)

| Quest | Form ID | Stages | Objectives | Flags | Starts | Scripts | Systems missing | Missing functions | Can't run | Moves the player to | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `HD00RobotsDialog` | 0007F47A | 0 | 0 |  | nothing | 13 |  | `ShowBarberMenu` |  |  | Nothing in the data starts it or names it in a script [C]. |
| `1EExtraFiends` | 000F8B6B | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `BH01` | 00038959 | 0 | 0 |  | nothing | 21 |  |  |  |  | Nothing starts it [C]; 20 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `ComplexSceneScriptPS3` | 0017B713 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `FFradioSignals02` | 00093BAD | 0 | 0 | repeated topics | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `Followers` | 000371D1 | 0 | 0 |  | nothing | 8 |  |  |  |  | Nothing starts it [C]; 4 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `Patch04Quest00` | 0017BA15 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VChipHistory` | 00118FD3 | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VConversationRestarter` | 00169030 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VDialogueLucky38Suite` | 0011F3D7 | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VDialogueReactivityLegion` | 00125EA2 | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VES00TemplateScript` | 0015A308 | 0 | 0 |  | nothing | 2 |  |  |  |  | Nothing starts it [C]; 1 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `VES04FactionReactionPrimmArea01PowderGangers` | 0015C846 | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VEndingBinks` | 0015FF6E | 0 | 0 |  | nothing | 1 |  |  |  | `WastelandNV` | Nothing in the data starts it or names it in a script [C]. |
| `VFreeformQuarryJunction` | 000E595A | 0 | 0 |  | nothing | 5 |  |  |  |  | Nothing starts it [C]; 6 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `VLegateShowdown` | 001438E6 | 0 | 0 |  | nothing | 38 |  |  |  |  | Nothing starts it [C]; 45 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `VMQ04` | 001344E8 | 3 | 2 |  | nothing | 4 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMQEndStates` | 001567D7 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMQEndStatesFreeside` | 001615E8 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMQEndStatesTheStrip` | 00161F03 | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMQHouseLockdown` | 001720CD | 4 | 7 |  | nothing | 5 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMS25Support` | 00177D65 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMS27` | 0011F86C | 0 | 0 |  | nothing | 0 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VMS28` | 0011F93C | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
| `VTopsKickedOut` | 00129D88 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing starts it [C]; 1 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `VTopsKickedOutSide` | 00129D87 | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing starts it [C]; 1 other scripts name it (its variables, stages or objectives) without starting it, so it holds data for them [G]. |
| `vCCExplorerBark` | 0013AC9A | 0 | 0 |  | nothing | 1 |  |  |  |  | Nothing in the data starts it or names it in a script [C]. |
