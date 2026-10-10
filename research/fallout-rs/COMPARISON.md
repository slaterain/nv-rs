# nv-rs and fallout-rs, area by area

Written 2026-10-08 to help nv-rs's maintainer see where fallout-rs's
findings land. nv-rs was read at `main` 56a3d10 (code and docs, nothing
built or run). fallout-rs's numbers are its own checkers' reports. Where
something could only be judged by running nv-rs, this page says so.

## In short

- **nv-rs is further along as a game people play.** Packaged builds that
  people play by hand, the opening from the movie to Goodsprings, menus
  and the Pip-Boy with the mouse, terminals on the terminal's own screen,
  casino tables and Caravan in 3D, grown SpeedTree trees, water
  reflections, a recorded real-game frame the renderer is compared with,
  three acceptance routes run on every change, and a working contributor
  process (issues, CONTRIBUTING, hygiene test, ADRs, CI).
- **fallout-rs is further along as a checked engine.** Two games, every
  record kind decoded and re-encoded byte for byte, every quest and script
  of both games running headless, unattended quest chains, a replay
  against real saves, and an oracle that calls the original programs'
  functions.
- **nv-rs's reading of FalloutNV.exe is accurate.** 73 of the 98 New Vegas
  rules fallout-rs holds agree with nv-rs's code, and several of nv-rs's
  readings corrected fallout-rs. 8 differ and 4 are partly done; those
  are listed with fixes in [NV_FACTS.md](NV_FACTS.md).

## By milestone (docs/MILESTONES.md)

| Milestone | nv-rs | fallout-rs | Who is further, on what evidence | What nv-rs could take |
| --- | --- | --- | --- | --- |
| M1 Opening and persistent world | Played by people from the intro movie through Doc Mitchell's house into Goodsprings; acceptance route `doc`. Open items on the maintainer's list (B14, B15). | A viewer run goes start menu, Doc Mitchell, name, face, SPECIAL, questionnaire, tags, traits, out to Goodsprings; checked by its pictures only, recent, not hand-played. Persistence: its own saves resume 100 % of what is written for Fallout 3. | nv-rs, clearly (played by people, route checked on every change). | Little; see M7 for saves. |
| M2 Core gameplay loop | Sunny's tutorial and the gunfight drive through their scripts in the viewer; melee and V.A.T.S. traced. | Back in the Saddle and They Went That-a-Way complete headless with a test player without aids; damage, V.A.T.S. and AI rules partly oracle-verified. | Split: nv-rs on the loop as played; fallout-rs on checks of the rules. | The DIFFERS rows of NV_FACTS.md (detection, NPC level and health, resistances). |
| M3 Base-game systems and campaign | Topic pages for many systems; Dead Money coverage matrix; base-game matrix is task M3, open. | A sweep that drives every quest's planned stages headless (New Vegas 95.7 % of planned steps) with the blocker per quest. | fallout-rs on coverage numbers; nv-rs on systems a player sees. | The sweep's method for task M3 (see PORTING.md). |
| M4 Stability and performance | Measured frame times, three interleaved runs, stalls removed (PR #12). | Not measured. | nv-rs. | Nothing. |
| M5 DLC and mods | Plugin and archive order, loose files and invalidation traced, generated test plugins. | Real load order read from saves; tools that write plugins from code. | nv-rs on the traced rules. | Nothing pressing. |
| M6 VR | Architecture documented. | None. | nv-rs. | Nothing. |

## By task (docs/TASKS.md, major systems)

| Task | Where fallout-rs can help |
| --- | --- |
| M3 Base-game quest coverage matrix | fallout-rs's quest sweep and quest graph generate exactly this table for every quest of both games, with the blocking stage and reason. The method is in PORTING.md item 2. |
| M4 Weapon class coverage | Rows NV-015 to NV-040 (damage, criticals, armour, spread, V.A.T.S. chance and cost, block, jam, limb cuts) with addresses; NV-022 lists two armour branches nv-rs lacks. |
| M5 Creature AI coverage | NV-014 creature combat skill, NV-033 fighting strength, NV-082 to NV-085 sandbox, wander and follow rules. |
| M6 World map and fast travel | NV-051 full fast-travel refusal order; NV-055 and NV-056 encounter-zone levels and resets (not in nv-rs). |
| M7 Original save files | NV-094 to NV-097: MSTT and PCBE change-form layouts that `docs/FOS_SAVES.md` lists as not decoded; with them fallout-rs decodes every change record of 13 New Vegas saves. |
| M10 Comparison harness | [ORACLE.md](ORACLE.md) (function-level comparison with the program) and the save replay (state-level comparison along a played route, PORTING.md item 1). |
| M11 Factions, crime, karma, disguises | NV-062 to NV-068: all of nv-rs's reputation and karma rules agree with the program, 4 of them oracle-verified; the karma band limits are confirmed inclusive. |

## By topic page

| nv-rs page | nv-rs | fallout-rs | Further along, evidence |
| --- | --- | --- | --- |
| TECHNICAL_REFERENCE (formats) | Every format read; Bink and MP3 decoders bit-exact against the game's own libraries. | Every format of both games read to the last byte; Bink being checked against an external decoder's frame hashes. | Even; nv-rs's decoder proofs are stronger, fallout-rs covers two games. |
| PACKAGES, PATHING | Packages for the Goodsprings routes, path search, doors, offstage walking. | All 9,445 AI packages of both games decode; package choice agrees with real saves for 98-100 % of actors; paths reach every triangle of 14,889 navmeshes. | fallout-rs on breadth and real-save evidence; nv-rs on walking people seen on screen. |
| NPC_COMBAT | Weapon choice by DPS, aim, reloads, flee, combat groups. | The program's attack planner, cover states, grenade window, flee, target score read and ported (Fallout 3), self-checked over 1,000 states. | Split. |
| DIALOGUE | Greeting rules, follow-ups, Goodbye, say-once, speaker turn traced. | Every topic and line of both games decodes and runs headless; greeting sweeps of four towns per game. | fallout-rs on breadth; nv-rs on presentation. |
| SCRIPTS_RUNTIME, SCRIPTGEN | 325 of 622 functions; scripts transpiled to Rust. | Compiled scripts decompile to source that recompiles to the same bytes (99.80 % New Vegas); 94 condition functions; the program's script event system (Fallout 3). | fallout-rs on the round trip and events (NV-075); nv-rs has the transpiler idea. |
| PHYSICS | Own solver with the game's parameters; ragdolls; grab. | A library solver; the game's character controller read from Fallout3.exe; knockdown ragdolls by the program's impulse rule. | Even; neither is Havok. |
| ANIMATION, OPENING_LOOK_IK | Animation picking, furniture, look-IK checked with `nv-call`. | The program's key interpolators; idle tree walked as the program walks it. | Even. |
| WEAPON_EFFECTS, ENERGY_WEAPONS, MELEE_UNARMED, FATIGUE, EXPLOSIVES | Many classes traced and playable. | Oracle-verified damage, condition, critical and armour-wear functions (NV-013 to NV-026). | nv-rs on what plays; agreement on the rules is high. |
| PIPBOY, START_MENU, TERMINALS, HACKING, MENU_FADES, TUTORIALS, HUD_MESSAGES, HTML_TEXT | Usable with the mouse, terminals drawn on the 3D screen, fades, tutorials. | The games' own menu layout engine; every Pip-Boy page filled and compared with saves (0 mismatches over 842 comparisons), but driven by keys. | nv-rs, clearly. |
| CASINO, CARAVAN | Playable on 3D tables. | Rules as a library, 200 blackjack rounds against the casino's ledger; menus open in the viewer with stand-in visuals. | nv-rs. |
| COMPANIONS | Wheel, trading, Nerve, doors, fast travel, gear by DPS. | All 8 companions hired by their own lines and fighting. | Even; Nerve and essential rules agree (NV-027, NV-061). |
| FACTIONS_CRIME | Reputation, crime, karma, disguises. | The same rules, checked by the oracle. | nv-rs implemented them first; they agree. |
| FOS_SAVES, PERSISTENCE | Original saves decoded (nine saves), import, `--load-fos`. | Original saves of both games decoded completely (New Vegas 196,228 records in 13 saves), saves written and read back, a session resumed from a real save and replayed. | fallout-rs on decoding and writing; see NV-094. |
| MODS | Load order and archive rules traced. | Tools to write plugins. | nv-rs. |
| PERFORMANCE | Measured. | Not measured. | nv-rs. |
| Rendering (TECHNICAL_REFERENCE) | Game shader formulas, sky, distant land, grass, water with reflections, grown trees, bloom; one real-game frame compared within 1 %. No shadows. | All 15,791 shaders translated and run; actor shadows by the program's rules; the HDR pipeline read from Fallout3.exe; weather transitions; no grown trees, no planar water reflection, never compared with a real frame. | Split: nv-rs has the real-frame comparison and the visible features; fallout-rs has shadows and shader breadth. |

## By owner (CONTRIBUTING.md "Areas")

| Area and owner | Findings here for them |
| --- | --- |
| Opening, Goodsprings routes (maintainer) | NV-001 and NV-002 change NPC levels and health on every route; NV-043 changes who sees the player sneaking. |
| NPC AI (maintainer) | NV-033, NV-043 to NV-046, NV-082 to NV-086. |
| Player: movement, actions, physics (maintainer) | NV-047 to NV-050; NV-051, NV-052 refusals. |
| Weapon effects (maintainer) | NV-015 to NV-040, NV-022 in particular. |
| Pip-Boy (maintainer) | NV-003 to NV-005 change the values the STATS page shows. |
| Terminals, item scripts, repair, weapon mods (Chazm) | NV-072, NV-073 agree; NV-022 (a) Split Beam threshold; NV-075 events for item scripts. |
| Companions, Caravan, casinos (Chazm) | NV-027, NV-061, NV-084, NV-085. |
| Performance (Chazm) | None. |
| Dead Money, crafting (Dead Money contributor) | NV-075 (OnFire, OnDestructionStageChange are New Vegas-only blocks), NV-080 GetDeadCount (Dead Money's quests count kills). |

## Process, fairly

nv-rs leads on process: claimed tasks, small pull requests, a hygiene test,
generated fixtures, ADRs, acceptance routes, packaged builds and human
playtests with a bug list. fallout-rs's habits that may be worth a look
are narrower: a status per rule (`oracle-verified` / `exe-read` /
`measured`), one `FO_OLD_*` switch per behaviour change so any regression
can be bisected at run time, and the oracle and save replay as two kinds
of comparison with the original. Neither project has yet compared a long
played route state by state with the original game (task M10).
