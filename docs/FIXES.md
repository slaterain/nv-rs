# The fixes preset

A player who changes nothing gets the `fixes` set; `vanilla` is what FalloutNV.exe 1.4.0.525 does, and
every comparison with the original, gate and recording runs on it (the maintainer, 2026-11-02;
[ADR-0014](adr/0014-typed-migration-in-two-stages.md) point 5, `crates/host/src/fixes.rs`).

## Choosing

| Where | What | Who |
| --- | --- | --- |
| The run's `Fallout.ini`, `[nv-rs General]`, `[nv-rs Display]`, `[nv-rs Threads]` and `[nv-rs Fixes]` | `sPreset=fixes` or `vanilla`; `bBorderless`, `iFrameLimit=n`, `iMenuFrameLimit=n`; `bSteppedThreads`, `bRenderThread`; a fix's own line (`bUncap=0` or `1`, ...: `distribution/nv-rs-sections.ini`), which turns that fix alone on or off without making the run a test; a commented line leaves it to the preset. Read only in a launcher's play run (`NV_SETTINGS` set) that is no test | The player |
| `userdata\settings.ini` (older previews' Settings.cmd; today's only before Play.exe's first start) | `Preset=fixes` (the default) or `Preset=vanilla`; `FrameLimit=n` (0: none); `MenuFrameLimit=n` (120 when missing or empty, 0: none of its own). Play.exe moves it into both profiles' `Fallout.ini` at its next start and renames it `settings.ini.migrated`. While it is there Play.exe moves it in at every start, its lines winning over the files (PLAYTESTING.md, "The game's Fallout.ini"), and the game reads these three lines of it itself, each only when neither `Fallout.ini` nor a variable sets it | The player |
| `NV_PRESET=fixes` or `vanilla`, `NV_FRAME_LIMIT=n`, `NV_MENU_FRAME_LIMIT=n` | Over the files | Tests, scripts |
| `NV_UNCAP=0` or `1`, and each fix's own variable below | That fix alone; with no `NV_PRESET` the rest of the run is vanilla | Gates, tests |
| A recording or scripted run: `NV_TRACE_LIST`, `NV_MOVE_KEYS`, `NV_MOVE_MOUSE`, `NV_SOUND_RECORD`, `NV_SHOT`, `NV_FRAME_LOG`, the probes and `*_CHECK` switches (the list is `TEST_VARIABLES` in `fixes.rs`), with no `NV_PRESET` | Vanilla; and, with no `NV_THREADS`, stepped threads whatever the play default is (`Choice::threads`); with no `NV_RENDER_THREAD`, no render thread (`Choice::render_thread`); with no `NV_BORDERLESS`, the window as the game makes it (`Choice::borderless`) | A script that was never told about the preset or the threads still measures the original, and doesn't change when a play default does. `NV_SAVE_OUT` (the preview launcher's) does not pin |
| `NV_SETTINGS=FILE` | The settings file, when it isn't `settings.ini` beside the program or in the folder above it | Launchers |

It is read once at start-up and bound before the first frame (`fixes::install`): the log's `Preset ...`
line says what the run is. Anything it can't read stops the start with a message. A settings file with a UTF-8 mark or in UTF-16 (PowerShell 5.1's `>`) is decoded; one that exists and can't be read stops the start, unless `NV_PRESET` is set, which beats the file whatever it holds. `Host::start` runs
vanilla, for tests and tools; the viewer reads the choice and calls `Host::start_with`.

Anything that compares with the original says `NV_PRESET=vanilla`. Speed gates say `NV_PRESET=vanilla
NV_UNCAP=1` as before.

The game's threads are not part of the preset. In play they run side by side on their own cores
(`fixes::PLAY_THREADS`); `bSteppedThreads=1` in `Fallout.ini`'s `[nv-rs Threads]` (Settings.cmd's
"Game threads": "One at a time") runs them one at a time, the fallback. A test or scripted run without
`NV_THREADS` runs them stepped whatever play's default is (the table above).

The render thread isn't part of the preset either: it changes where the real Direct3D 9 calls are
made, not which calls or in what order. In play they are made on a render thread of their own
(`fixes::PLAY_RENDER_THREAD`, since measure/render-thread: town +14%, the crowded main street +16%;
#347); `bRenderThread=0` in `Fallout.ini`'s `[nv-rs Threads]` (Settings.cmd's "Drawing thread":
"Main thread") makes them on the game's threads, as the original does, the fallback. A test or scripted
run without `NV_RENDER_THREAD` makes them on the game's threads whatever play's default is.

The borderless window isn't part of the preset either. In a launcher's play run (`NV_SETTINGS` set
and no test: the run that reads `Fallout.ini`'s sections) the game's window has no frame and fills
the screen (`fixes::PLAY_BORDERLESS`; the maintainer, 2026-11-09); `bBorderless=0` in `Fallout.ini`
(Settings.cmd's "Display mode": "Window") keeps the window as the game makes it. Every other run
keeps that window too unless its `NV_BORDERLESS=1` says otherwise: a test, a scripted or measured
run, and a start no launcher made. So a run in the background never covers the screen, and no
recording or measurement changes with play's default. The default is the host's alone: its table
`crates/host/data/game_ini.tsv` is held to `PLAY_BORDERLESS` by a test, and the generated sections
and Play.exe take the value from the table.

## How a fix is built

- An override keyed by exe address, a plain function registered over the translation when it is on.
  Off, nothing is registered, so the translation runs as it is and no call tests a flag.
- Translations are never edited for a fix.
- Its addresses are in `crates/engine-core/data/overridable.tsv`, so typed code keeps calling them by
  address (`docs/ENGINE_CRATE.md`, "Typed code"; a test checks).
- Tests in both modes: the translation's behaviour stays what its own tests say; the fix's tests
  install it.
- Its row says why it is in the set, from measurements.

## Measuring

`NV_FRAME_LOG=FILE` writes a line per main frame (clock, step, Havok accumulator, player position)
without a per-call cost, so the frame rate stays the player's. The game's own `iFPSClamp` INI line
(`[General]`, 1 to 199) gives exact frame steps at any wall rate, which is how a recording sees a
high frame rate's steps: a trace itself holds the game near 30 frames a second.

`NV_LOCK_STATS=FILE` (real threads only) writes a line per main frame with the game clock and how long
the main thread and the other game threads waited that frame for the kernel lock, for critical
sections, for events, semaphores and threads, and in sleeps, how often a section leave woke waiters, and
how long the host held the kernel lock before the frame (joined with `NV_FRAME_LOG` by the clock), then
how long main waited in the frame-end barrier (`barrier_us`) and the waits for the Direct3D device lock,
main's and the other threads' (`main_device_*`, `game_device_*`). Beside it, `FILE.sites` lists, for each
frame in which main's sleeps and object waits passed 50 ms, those sleeps and waits and the other threads'
by the exe function that made them (the caller of `Sleep` or the wait), with the sleep's milliseconds,
the first handle main waited on, or the thread's id. `NV_FRAME_LOG`'s last column, `qpc`, is the
Windows performance counter, for an outside sampler's timestamps.

`NV_PHYS_BLEND_LOG=FILE`, with the smooth physics drawing on (say `NV_PRESET=fixes`: the variable pins
vanilla otherwise), writes per drawn frame each free body's node as the original draws it and as the
fix draws it, with the drawing time and the world's last step end; `NV_SHOT_FRAMES=FROM-TO` limits
`NV_SHOT` to those presents, so a short stretch can be taken every frame.

## The fixes

The game's own `iFPSClamp` ignores `FrameLimit`: a clamp is a fixed step with no wait, which the uncapped step doesn't pace, so a recording with `iFPSClamp` runs as fast as it can.

| Fix | Address | Switch | What it does |
| --- | --- | --- | --- |
| Uncapped frame rate | `00aa4ee0` | `NV_UNCAP` | `BSTimer`'s step without its sleep and its 10 ms floor, on the microsecond clock. `FrameLimit` paces it. Where the original sleeps (a step under 10 ms by the tick count) the other threads get a `Sleep(0)` pass instead, about one frame in three, and the first step after a disable answers as the original does (10 ms) |
| High-frame-rate landing | `00c73170` | `NV_HF_LANDING` | The player's character, landing standing still in a frame under 16 ms, falls on as far as at 16 ms frames: one more integration with `Move`'s own velocity, large-creature mode word and step. Measured on the player only, so other characters run `Move` as it is. The extra time is 32 ms minus the frame, so it jumps at 16 ms (15.9 ms gets about 32 ms in all, 16 ms none); the fall stops at the floor either way, and the difference is under half a unit (0.43 at 16 ms frames), not visible, so it is not smoothed |
| High-frame-rate slow motion | `00aa4e40` | `NV_HF_SLOWMO` | The time multiplier moves to its target by the time passed, not by the frame |
| Menu frame limit | none of its own (paces the uncapped step) | `NV_MENU_CAP` | A menu (main menu, Pip-Boy, dialogue, inventory, pause, message boxes) runs at most `MenuFrameLimit` frames a second (120 unless set; `FrameLimit` when lower), a loading screen with no limit, even under a `FrameLimit` (the maintainer, 2026-11-07). What a frame shows is the game's own test (`Main::OnIdle`, `0086e650`): a menu is the menu-mode byte `011dea2b` the frame sets first (`Interface::IsInMenuMode` or the Pip-Boy opening); loading is the loading menu up (`LoadingMenu::pMe`) with neither the main menu (`XUIIsUp`) nor the credits over it, the frames in which `OnIdle` asks `TES` whether the load is done. The load's own work, while the main thread is busy and the loading menu's thread draws, has no main-timer step and was never limited. With the uncapped step off nothing changes. `NV_FRAME_LOG` has a `screen` column (play, menu, loading) |
| Dragged bodies by time | `0095f6c0`, `009613c0`, `00960520` | `NV_GRAB_PUSH` | The grab update (`00960520`) pulls a held body that is more than `fZKeyComplexHelperMinDistance` (6) units from its place by adding `(distance - 6) x fZKeyComplexHelperScale (2) / distance x -1.0 x` a weight factor (`fZKeyComplexHelperWeightMin` 0 to `Max` 100) to its velocity, once a rendered frame (`AddVelocity` for a ragdoll or a flagged body, the velocity plus the pull for others), so at 144 frames a second it is pulled 2.4 times as hard as at 60. Its three entries (the player's grab handling and the dropped-object spring, which call it in Rust, and the grab update itself, which telekinesis calls) run with `fZKeyComplexHelperScale` scaled by the frame's seconds times 60, at most 4 (TEMPO's rule in Fallout 3), and the setting back afterwards; the grab update is its only reader. The spring itself is a Havok action on the 16 ms steps and is left alone |
| Smooth physics drawing | `00c65a80`, `008706b0` | `NV_PHYS_BLEND` | Free bodies (dropped and thrown items, ragdolls, debris) are drawn between Havok's 16 ms steps instead of frozen on frames without a step: around the frame's drawing only, each moving body's node shows the body at the game's time minus one step, blended with Havok's own swept-transform formula, and is put back after. The simulation and what game logic reads are the original's. Bodies are drawn one step (16 ms) behind the clock, about 5 ms later than the original on average. Below |
| Mod fit worn question | `007838a0` | `NV_MOD_WORN` | Fitting a weapon mod asks whether the mod is worn before the mod is removed from the inventory, not after. The original asks after (a slip: it means the weapon), through the mod's extra list the removal has just deleted; it nearly always answers no, but a yes rebuilds the inventory list, which frees the weapon entry the mod menu keeps, and closing the menu then re-equips through it and the game stops. A mod is never worn, so the answer is the original's usual no, and no released memory is read. Every other click of the menu is the original's. Below |
| Dialogue topic index | `00619820`, `0061a310`, `0061a380`, `0061a5a0`, `0045dfe0` | `NV_TOPIC_INDEX` | Finding which topic holds a dialogue info (`TESTopic::GetOwnerTopic`) no longer walks the topic list and every info in it: the original does that for each info a plugin changes (YUP and the add-ons: 5,837 times), so a dialogue mod's load grew with its size times the game's. Every topic `TESTopic::InsertInfo` links an info into (the one place an info enters a topic) is kept beside it; a search asks only those topics, with the original's own search (`00619e10`), which hold this info first by its id, and takes the first of them in the topic list's order, from a copy of the list's order that is made again whenever the list may have changed in a way that matters (its sort, the data handler's `ClearData`, an insert of a topic the copy holds, which can list it again earlier, a changed head node, or a topic inserted since that the copy doesn't hold). So the answer is the walk's, tie-breaks included (an info in two topics, a twin id, the list's order and end). `NV_TOPIC_INDEX_CHECK=1` with `NV_TOPIC_INDEX=1` (alone it is a test variable, which makes the run vanilla) runs the original's walk beside every search and stops the game at the first difference. With the fix on, the typed `topic_search` family (the walk) goes by address for the run, since the fix registers over one of its members. Below |
| Shaders compiled ahead of first use | none (a flag the real Direct3D 9 backend reads) | `NV_SHADER_WARM` | The graphics driver compiles a shader when it is first drawn with, and the game makes all 960 of its packages' shaders at start-up but first draws some of them in play: the first shot draws 7 for the first time (`SKIN2005.vso`, `SKIN2002.pso`, `SLS2136.pso`, `SLS2148.pso`, `SM3000.pso`, `NOLIGHTTEXVCPMA.pso`; `d3d9-first-use`), and its frame took 29 ms where the others take 12.5 (46 ms without the render thread). It recurs at every launch, whatever the exe's path. With the row the real backend draws once with every shader the game makes, into a 1x1 target of its own, after a `Present`, and sets the game's bindings again with its own commands: 960 draws in 2 ms of the render thread at the first `Present`, compiled by the driver while the menus run. The first shot's frame: 28.6 and 29.2 ms without, 19.3 and 19.8 with (the shooting route, BAAB, 2026-11-09); loads not slower (Doc's house 2.26 and 2.28 s without, 2.14 and 2.10 s with); the real device's frames identical to wgpu's while it runs. The NVIDIA driver also compiles a pixel shader again for a multisampled target, so the shaders warmed into the plain 1x1 target were compiled again at the first shot, drawn into the game's 8x scene target: the warm-up now also draws each shader into a 1x1 target of each multisample type the game's render targets and back buffer use (at 8x antialiasing: 8x for the scene and 4x for its 1024x1024 targets; 2880 draws, 5-12 ms of the render thread over the first 2-4 presents in the game). Found by replaying a recording of the shooting route into a real device with one change at a time to an extra warm-up before the shot: drawing each shader also into a multisampled 1x1 target alone removed the compile (A8R8G8B8 or A16B16G16R16F, 4x or 8x, with or without depth); without multisampling, a float format, the real partner shaders, textures and render states did not; adding the real vertex layout, textures and states on top of multisampling changed nothing. Replay, the shot's two presents, ms inside Direct3D 9 (calm presents 6-7): with main's warm-up 19.5-29.3 and 19.8-33.5, with this one 6.8-8.7 and 6.9-8.5 (with no warm-up at all 28.2-42.5 and 16.4-21.5). In the game (the shooting route cut to its five shots, 2560x1440 with 8x antialiasing, 2026-11-10): without the render thread (BAAB) the first shot's two slow frames were 39.6 and 32.6, 29.3 and 30.8 ms before, 21.8 and 20.6, 23.4 and 22.2 after (the part's median 16-17; what is left is the game's own work at the shot), later shots unchanged (worst 20.5-21.5 ms both). With the render thread (play's setting; 6 runs of main, 4 of the change: too few to stand alone, the replay and the runs without the render thread carry the claim) main had frames over 18 ms in the first shot's window in 3 of 6 runs (22.3, 19.5, 19.4 ms), the change in none (worst 15.9); the later shots' worst frames 17.7-34.6 ms on main, 18.5-25.6 with the change. Start-up (38.5-38.9 s from the first frame to play) and the Goodsprings load unchanged |
| Co-save keeps other plugins | none (a flag the extender reads) | `NV_COSAVE_KEEP` | A co-save's sections of plugins that are not built in (JIP LN NVSE's, until it is) are kept as they were read and written back on each save while the files loaded are the ones that co-save lists, so a save brought from a real modded installation keeps them. xNVSE writes only the plugins it has loaded, so off (vanilla) they go at the next save, as with xNVSE. Below |
| Script error loop guard | none (a flag the extender reads) | `NV_NVSE_ERROR_GUARD` | An error raised while a script's `RuntimeScriptError` handler runs (in the handler itself, or at the call depth limit when handlers call functions that fail) is not sent to the handlers again. xNVSE sends it again, which recurses until the stack runs out and the game stops; off (vanilla), the port does the same. Below |
| Image-space event form check | none (a flag the extender reads) | `NV_NVSE_IMOD_CHECK` | When an image-space modifier instance runs out, OnRemoveIMOD is raised only for an instance of a modifier form (the game's own `AsForm` test, as its `Stop` makes it), with that form. xNVSE reads a form at the same place in every instance, and a depth-of-field or radial blur instance (iron sights, V.A.T.S., `SetGlobalRadialBlur`) has a number there: a handler then reads garbage as a form. Off (vanilla), the word is read as xNVSE reads it, and a word that is not a readable form drops the event instead of stopping the game. Below |
| Script reference checks | none (a flag the extender reads) | `NV_NVSE_INVREF_CHECK` | (Once "Inventory reference checks": the variable and the Fallout.ini key `bNVSEInvRefCheck` keep that name, so a setting already made still works.) A reference handed to a script with parts missing, where xNVSE reads or writes through the missing part and the game stops; with the row the command gives its "nothing" answer instead (0, or nothing set). Off (vanilla), the port stops there too. **Inventory references:** an inventory reference's temporary reference is in no cell, and has no base form once released (after `RemoveMeIR`, or a loop's reference kept past its loop): `GetDoorTeleportX`, `GetDoorTeleportY`, `GetDoorTeleportZ`, `GetDoorTeleportRot`, `SetDoorTeleport`, `GetParentCellOwner` and `GetParentCellOwningFactionRequiredRank` read through the missing base form or cell. A container whose base list has an entry without a form stops the game in `ForEach` and `GetInventoryItems` (xNVSE reads the missing form's type), where the row passes over it; the commands that look an item up by its id (`PickOneOf`, `GetTokenValue` and the rest) read its id through it, where the row takes it as another item (`GetNumItems` and `GetInventoryObject` count it as xNVSE does, in both presets). **Doors:** a door whose teleport has no data or no linked door, or whose lock has no data (a mod's broken door data; the game itself makes a lock without data only when an allocation fails, and never a teleport without data): `GetLinkedDoor` reads through it, `SetDoorTeleport` and `SetOpenKey` write through it. **Owners and cell scans:** `HasOwnership` and `IsOwned` read the item's parent cell (for the cell's owner) and the asker's base form through nothing for a reference without them, where the row answers 0; the cell scans read a cell reference's base form for its type, where the row passes over a reference without one. **Scan records:** `GetNextRef` with no scan reports "GetFirstRef MUST be called before GetNextRef" and leaves a record it never builds, and a second one reads that record's uninitialised memory; a console scan that has ended stays under the console's key (xNVSE drops the script's own id instead), and the next console `GetNextRef` reads it through its missing cell. With the row both answer as the first call did: the error line again and 0. A scan whose walk finds no cell of its area in the worldspace's map reads the missing cell's list too; with the row it answers 0. **Form lists and doors** (`Commands_Misc.cpp`): `GetDoorSound` reads the id of the door's open or close sound through nothing for a door record without that sound (the game allows one), and in the console prints the id of a missing form through nothing; `GetLevCreatureRefs` and `GetLevCharacterRefs` read the id of a leveled entry's missing form; `GenericCheckForm` reads the id of an empty node's missing form in the middle of the list. With the row the door answers 0 and nothing is printed, and the entry or node is passed over. `IsRefInList` (`Commands_List.cpp`) reads the id of a reference's missing base form (an inventory reference's, once released) when the list holds a form; with the row nothing is found there. **And two of xNVSE's own slips with inventory references:** `RemoveMeIR` on an unworn stack with data of its own (a weapon with its condition) frees that data through the entry the reference holds, which for a reference from `GetInvRefsForItem`, `ForEach`, OnSell or OnRepair is the reference's own copy: the container's entry keeps the freed data and its count, so the item stays with freed memory as its data; with the row the data leaves the container's own entry too (its count with it) before it is freed, and nothing is freed when no entry of the container holds it. OnSell's item stays kept after the event when no handler ran (every handler filtered out), so a later `GetSoldItemInvRef` reads a sale that is over; with the row it goes when the event ends. Off (vanilla), all of these are xNVSE's, the stops included. Below |
| Script array and string size caps | none (a flag the extender reads) | `NV_NVSE_SIZE_CAPS` | An array `ar_Range`, `ar_Init`, `ar_Resize` or `ar_Generate` makes or grows stops at 8,388,608 elements (about 335 MB), and `sv_Replace` of an empty text a given number of times stops at 64 MiB. The keys `MenuTapKey`, `MenuHoldKey` and `MenuReleaseKey` queue for the game's buffered key reads (drained only when the game reads them, in menus) stop at 4,096; the rest are dropped. A format string the C runtime cannot make is made safely: a precision past 965,624,998 (more working memory than the runtime gets in a 32-bit process) gives a number with a point the runtime's own fallback of 163 digits and an integer the zeros asked, and a format the runtime refuses (`%Ld`, `%5n`, a second size such as `%hld`) gives an empty text. In xNVSE such an integer precision crashes the runtime, a refused format ends the game (the runtime's default invalid-parameter handler: nothing in xNVSE's process installs another, measured in the real game), and whether a number's precision gets its memory depends on the game's; off (vanilla), the port stops the game at all three. xNVSE goes on until it runs out of memory and the game stops; off (vanilla), the port stops too: an array past 33,554,432 elements or a string past 1 GiB or more than 67,108,864 queued menu keys stops the game as out of memory, about where xNVSE's 32-bit process runs out, so the computer is never asked for more. A range that never ends (a step of 0, or one that wraps past its end), `ar_Cat` of an array with itself and an empty text replaced without a count never end in xNVSE and are capped the same in both presets. Below |
| Script missing array guard | none (a flag the extender reads) | `NV_NVSE_NULL_GUARD` | `ar_Count` and `ar_GetNth` on a missing array answer 0, and a format string's `%s` (or `%S`, `%Z`) pointing outside the game's memory ends the text. xNVSE reads through the null array or the pointer and the game stops; off (vanilla), the port does the same. Below |
| Script array cycle guard | none (a flag the extender reads) | `NV_NVSE_CYCLE_GUARD` | A deep copy (`ar_DeepCopy`), a deep compare (`ar_DeepEquals`) or a dump (`ar_Dump`, `ar_DumpF`) of an array that holds itself, directly or through others, ends where it meets the array again: the copy holds its own copy there, the compare counts that pair as equal, the dump writes `[...]`. xNVSE walks on until the stack runs out and the game stops; off (vanilla), the port does the same. Below |
| Script list and scan guard | none (a flag the extender reads) | `NV_NVSE_LIST_GUARD` | (Once "Script inventory list guard": the variable and the Fallout.ini key `bNVSEListGuard` keep that name.) Lists and walks that cannot end are ended. A game list the inventory commands walk (a container's stacks, a stack's extra data lists), or one the form list commands walk (a form list's forms for `ListGetCount`, `ListGetFormIndex`, `IsRefInList`, `ListAddForm`, `ListRemoveForm`, `ListClear`, `ForEachInList` and the rest; the lists `GetListForms`, `GetRaceHairs` and the other array commands and the generic list commands walk), that runs back into itself ends the walk after 1,048,576 nodes, and `GetPaired` ends where it would ask for the same stack again (a stack list with an empty node). A cell scan (`GetFirstRef`, `GetNextRef`, `GetNumRefs`, `GetRefs`, their `InCell` forms, `GetInGrid`, `GetInGridInCell`) whose depth takes the walk past the cell grid's 8-bit positions (the cell's x or y plus the depth above 127, as with `GetNumRefs 0 200`, or a depth of -2, taken as 254) ends when the walk is back where it started, every cell position it can reach visited once: `GetNumRefs` and `GetRefs` give the references of the cells passed; `GetFirstRef` and `GetNextRef` go on answering the same references round and round as xNVSE does, and only a call that goes the whole way round without finding one answers 0. xNVSE walks on for ever and the game hangs; off (vanilla), the port stops the game there instead. A name xNVSE copies into a 256- or 512-byte buffer on its stack, its terminator included, is used whole with the row. `GetAgeClass`'s race name in capitals runs into the saved frame and the return address at 256 bytes or more; off (vanilla), the port stops the game there, where xNVSE faults. `AddAnimation`'s and `DelAnimation`'s name argument: off (vanilla), the port also stops at 256 bytes, which is a deviation: there `AddAnimation` in xNVSE adds the name with the bytes that follow it in its frame (an answer that depends on the run), and `DelAnimation` compares the node's index byte as the name's 257th (an answer the list fixes, kept under the same stop as one rule) and faults only from 257 or 260 bytes (`AddAnimation`, without or with a form argument) and 265 or 272 (`DelAnimation`); see below. The same rule for the 512-byte name arguments of `GetNumericGameSetting`, `SetNumericGameSetting`, `GetNumericIniSetting`, `SetNumericIniSetting`, `IsModLoaded` and `GetModIndex` (a `$` string variable's text is copied whole): off (vanilla), the port stops at 512 bytes, where `IsModLoaded` faults; the others first write their next bytes into locals (see below). And for the 256-byte plugin names of `IsPluginInstalled` and `GetPluginVersion`, where both fault (the saved frame is right above their buffers). And two places where xNVSE itself stops the game no longer do: the extra data list `SetHotkeyItem` gives a stack that had none (stimpaks, for one) has the game's own vtable, so the game can free it when the stack is used up, dropped or the inventory torn down; the token commands (`GetTokenValue`, `GetTokenRef`, `SetTokenValue`, `SetTokenRef`, `SetTokenValueAndRef`, `GetPaired`) on an item that is in the reference's base container without a stack of its own (the player's Pip-Boy, for one) answer with that item's base health and no owner, and the setters add it and set its new stack to the value given, as they do for an item not held (except that `SetTokenRef`, which gives no value, keeps the item's base health, where xNVSE's not-held path would store 0). xNVSE's list has no vtable, and its token search reads the item's health through the missing changes entry or counts on through an index the command did not pass: the game crashes; off (vanilla), the port stops there too. Below |
| IO wait ends on completion | `00c3dfa0` | `NV_IO_WAIT` | The IO manager's wait for queued loads (`00c3dfa0`; main at a cell border, a load, a save) sleeps 50 ms a turn while the completed work is short of what is in flight, then takes what came in. Here that sleep ends as soon as it is not (looked at after each 1 ms sleep; a turn still ends after 50 ms by the clock, or after 50 looks), so the wait for queued loads ends when they are done, not at its next 50 ms step: the same work in the same order. With stepped threads each 1 ms sleep is a switch point, where the IO thread runs. `IOManager::LoadQueuedPriority` (`00456520`) calls it by address now, so the row applies there too. |
| Half-loaded actor's level change | `00881d30`, `00882b90`, `00883240`, `00883800`, `00932a00`, `00932b70` | `NV_HALF_LOADED_MOVE` | An actor moved to another AI level while its saved data loads (one loading thread between the two passes over a cell's saved changes, another setting its animation: running fast across cells) has its saved reference ids taken out for the move and put back where the original's copy puts them, for the second pass to resolve. The original follows the saved target id as an object (marks it, then calls through the word at the id as a vtable): its own crash. Below |
| Thread races | `00aa42e0`, `00aa3a40`, `00a0e770`, `00a0df80`, `00a0f8b0`, `0071d0a0`, `00652140`, `00652160`, `00652190`, `004b5d10` | `NV_THREAD_RACES` | Four places where the original shares state between its threads without the lock it needs, which the port meets because it runs the game's threads side by side: a new thread's scrap heap (the first look-up on a thread inserts into an unlocked map, so two threads starting at once can lose one, and a block is later freed into the wrong heap), the debug texts and the console (printed from any thread while the main thread handles and times out the same queue), the face-gen lock's count (counted down after the release with a plain read and write, so counts are lost and the model cache is never emptied, or emptied under another thread), and the editor-marker pass every loaded model gets (its controller in a global that a pass on another thread clears between two reads, and the virtual call then goes through 0). The first three from lStewieAl's Tweaks, the fourth from Psycho. Below |
| Texture files read before the renderer's lock | `00e6dc40` | `NV_TEXTURE_READ_FIRST` | The loading thread makes a texture's renderer data (`00e6dc40`) under the renderer's source-data lock (`m_kSourceDataCriticalSection`, the renderer's +0x180), and the making reads the texture's file and unpacks it from its archive inside that lock (`00e68a80`: `NiFile::GetFile`, `00462d80`), so main, which takes the same lock to bind or make another texture's data, waited for the read: 6-13 ms in 5-6 frames of each run across cell borders (measure/smooth-200/BORDERS.md). Here the file is read first, on the same thread with the same calls, when the texture is loaded by its file name (the common case: each making opens its own file; a texture holding a file object of its own is made as before, its file read under the lock), and only the rest stays under the lock in the exe's order: the renderer data's allocation and construction, the image description, the Direct3D texture's creation, the texture taking its data. The lock guards the textures' renderer data (+0x24) and the renderer's texture state; the read touches only the file and a buffer of its own. A texture another thread gives data meanwhile frees the buffer read for nothing (its file, opened for this making alone, is already deleted). The same textures with the same bytes, made and handed to the renderer in the same order; what differs with the row on is the guest heap's order of allocation (the file's buffer is taken before the renderer data's object), so heap addresses differ, not what is drawn. |
| Cell entry waits for its own cell | `00446b50` | `NV_CELL_ENTRY_WAIT` | Crossing into a new outdoor cell, the game waits for the loads the new cell needs at once. The original waits for more: at the same crossing it queues the row of cells three cells ahead as a pre-load, at the same top priority, and the wait takes them too, one cell after another on the one loading thread: a wait of 45 to 82 ms for 1.5 ms of the entered cell's own loads, 68 to 150 ms with YUP. With the row the cells ahead that are still queued get the priority their references get (1 or 3, `TES::GetCellPriority`'s rule) before the wait, through the IO manager's own requeue, and load right after it, in the background. A cell load already running, and one for the entered cell itself, are waited for as before. Measured, one run each: the wait went from 82 to 20 ms in the base game and from 68 to 4 ms with YUP; the frame it falls in went from 122 to 55 ms and from 102 to 31 ms. The frames next to it keep the border's other work, so the crossing's longest frame went from 122 to 70 ms and from 102 to 65 ms: about half. Below |
| Cell offsets for every plugin | `00583560`, `005854f0` | `NV_CELL_OFFSETS` | The game finds a cell of a master file by the table of cell positions the master carries (`OFST`); a plugin, and a master whose tables were stripped (YUP's, TTW's), it searches record by record each time a cell loads, on the one loading thread. With the row each such world space record gets the table its search would give, built from the plugin's own records at load on a host thread (Cell Offset Generator's idea), and a plugin's cells are found by it as a master's are. The same cells are found: where a table could not give the search's answer for every position, the world space is searched as before. Below |
| V.A.T.S. waits only for the camera | `007f3e00` | `NV_VATS_LAG` | V.A.T.S. measures a target's body parts one by one as the camera turns to it; the parts it has not measured get the average of the measured ones only after a fixed timer: 2000 ms since the scan began (`007f4058`). With the row they also get it as soon as the camera's heading is reached (the menu's `bHeadingReached`), once at least one part is measured, so the chances show when the camera arrives instead of up to 2 seconds later. The measured parts keep their own chances; a part on screen that the scan has not measured yet when the heading is reached (the camera arrived within a few frames) gets the measured parts' average instead of the chance the original would measure for it in the following frames. lStewieAl's VATS Lag Fix, except that it does not end the wait before a part is measured (its average would be 0/0: no chance at all on the other parts). Below |
| Causes keep no witness lists | `0066e010`, `0066e440` | `NV_ACTOR_CAUSE` | An actor's projectile or explosion carries a cause (`ActorCause`), which copied the actor's whole detection list when it was made; every saved cause wrote all of it (one 42-byte record per actor detected), so saves grew with the shots fired in company. With the row the cause copies nothing (its constructor no longer asks the actor's process for the copy) and a save's records are read and dropped, so they are not written again. Nothing in the game reads the list but the cause's own functions (freeing it, removing an actor from it, saving and loading it), and a cause made for an actor without a full process never had one. Saves stay readable both ways. lStewieAl's ActorCause Save Bloat Fix. Below |
| Radio song lengths | `008300c0`, `00830b30` | `NV_RADIO_LENGTHS` | Two bugs of the music and radio channels, from Psycho. A file opened on a channel left that channel's last duration in place until the new file's thread ran, and a radio station asks for the new song's duration at once and keeps a positive answer: a song opened while the channel's last song still faded out (a quick change of station) got that song's length, so it was cut short or followed by silence. With the row the duration is cleared when the file opens, and a channel's thread writes its file's duration only while the channel is still that file's (the fading thread wrote its length back once more before it noticed the new song). And the music thread brings the time since a station's start into the song's length by taking the length off while it is longer: a length of 0 never ends that loop, and the music thread spins for good (silence, one core busy). With the row a length of 0 skips that seek, and the rest gets the loop's own answer in one step. Below |
| No distant water under the Strip | `00583560` | `NV_STRIP_LOD_WATER` | Patch data (below): The Strip's world space (`TheStripWorldNew`) gets the "No LOD Water" flag as it loads. Its distant water lies under the Strip's ground and is never seen, but the game makes it and turns its water system on for it. YUP, UPNVSE Plus and Tale of Two Wastelands 3.3.3b set the same flag; Aqua Performa's author reported about 20 frames a second on the Strip from it (not measured in nv-rs). Below |
| Direct file access | `00aa14d0`, `00aa15f0`, `00aa16d0`, `00aa85c0`, `00aa8610`, `00aa8660`, `00aa7eb0`, `00aa81d0`, `00ec9907`, `00aff300`, `00b004e0`, `00afb2a0`, `00afa550`, `00afa750`, `00afb190`, `00afac20`, `00afac90`, `00afad00`, `0043c3b0`, `00c3d0e0`, `00c3d440` | `NV_DIRECT_FILES` | The game's files are opened, read and written by the thread that asks, as Mod Limit Fix 7 and Cell Offset Generator have them: no IO thread that every read waits for, no limit of 512 open files (about 140 plugins), and files and archives up to 4 GB: seeks and sizes past 2 GB, and an archive's files stored past 2 GB found and read, as with JohnnyGuitar NVSE (the original stops at 2 GB, where TTW's and VNV's generated LOD archives go on). Below |

### What high frame rates do to the original's code

Measured on the port's vanilla translation at exact frame steps (the game's `iFPSClamp`), against the
real game on the same route (2026-11-06, the recorded route's Doc Mitchell's hall, Vikki and Vance,
Goodsprings; real frames 10, 15 and 16 ms):

| Frame step | 16.67 ms | 10 ms | 8.33 ms | 5.03 ms | Real game |
| --- | --- | --- | --- | --- | --- |
| Height above the floor after a standing jump | 0.4 | 3.2 to 5.1 | 4.0 to 5.5 | 6.6 to 7.0 | 1.1 to 2.8 |
| Jump height | 61.54 | 62.52 | 62.77 | 63.26 | 61.5 to 61.9 |
| Rise at rest after a load: player | 0.269 | 0.007 | 0.006 | 0.004 | 0.029 |
| Rise at rest after a load: an NPC in Vikki and Vance | 0.599 | 0.336 | 0.186 | 0.006 | 0.346 |
| Player's run, units a second | 334 | 334 | 334 | 334 | 336 |
| Dropped stimpak, steps until asleep | 93 | 101 | 93 | 85 | 101 |
| Doc's body, steps until asleep | 220 | 176 | 151 | 173 | 128 (140, 232, 140 in other real runs) |
| Havok steps | 16 ms | 16 ms | 16 ms | 16 ms | 16 ms |

- **Rigid bodies don't care:** the world steps 16 ms with 4 solver steps at any frame rate
  (`00c66760`, `00c6af..`); under 8 ms a frame's time is carried and it has no step. Settling
  has no trend with the frame step. NVTF sets `fMaxTime` to the frame's time; nothing measured here asks for
  that, and it changes the solver's work a second. It goes with the fixed-rate simulation, later.
- **The game's clock.** A frame shorter than a `GetTickCount` tick is reported as 10 ms, so the game's
  time runs ahead of the real time as frames get short: the real game at 94 frames a second in the hall has
  13.7 ms of game time a frame in 10.66 ms of real time, 1.29 times real time. Above 100 frames a second
  the original would go faster still (the "sped-up game" of the Viva New Vegas guide's FAQ). The uncapped step follows the real time.
- **The original's sleep.** `Sleep(9)` in about one frame in three in the hall (732 of 2300), and that frame
  takes 11.8 ms against 7.6 without: it isn't rounded up to the 15.6 ms tick.
- **Rest height follows the frame step alone.** A still character's rise from its start stops at the first step
  whose contacts move under 0.001 Havok units. Nothing here depends on foot placement.
- **Not settled:** animation (NVTF's `bSpiderHandsFix`) has no measure here. The jump is 2.4% higher at
  200 frames a second, which is the integration's own step, and stays. An NPC at rest after a load stands 0.34 units
  lower than in the real game at 200 frames a second and 0.25 higher at 60; left.

### Before and after

Height above the floor (7360.36 in Doc's hall) where the player rests after three standing jumps, one
after the other. Real frames with `NV_FRAME_LOG`, the `iFPSClamp` rows with a recording:

| Frames | Uncapped alone | With the landing fix |
| --- | --- | --- |
| Limit 60 | 0.69 | 0.69 |
| Limit 120 (the hall reaches 93 a second here) | 4.57, 3.04, 1.36 | 0.69 |
| No limit (about 86 in the hall) | 2.83, 3.01, 1.78 | 0.69 |
| Exactly 120 | 3.96, 5.53, 4.65 | 0.69 |
| Exactly 199 | 6.98, 6.77, 6.55 | 0.69 |
| The real game at its own rate | 1.10, 1.90, 2.83 | |

It holds up to the 199 a second tested. Frames of 16 ms and longer run `Move` untouched. The game's time against the
real time is 1.001 to 1.005 at every limit.

Frame rate at the maintainer's settings (2560x1440, 8x anti-aliasing, borderless; about two minutes a run,
2026-11-06): Doc's room 70.2 frames a second on vanilla, 104.7 with the fixes; Goodsprings about 43 and about 44 (the original never sleeps there; the second from 15 s only).

### The menu frame limit, measured

Port runs on the play build (2026-11-07, 1280x720 window in the background, `NV_FRAME_LOG`'s `screen`
column; Save 138 from the main menu, then the Pip-Boy in Doc's house):

| | Main menu | Pip-Boy | Play (Doc's house) | Save load |
| --- | --- | --- | --- | --- |
| `NV_MENU_CAP=0` | 179.9 | 157.2 | 74 | 3.87 s, no main-loop frame |
| The fixes preset (120) | 119.9 | 119.0 | 76 | 4.08 s, no main-loop frame |

A load draws on the loading menu's own thread (a round every 50 ms, as the original) while the main
thread does the load, so there is no main-timer step to limit: the save load has no frame of its own
and a `coc` cell load one (with `FrameLimit=90`), which ran unpaced. The loading screens' speed is the
original's either way.

### Smooth physics drawing

Havok steps the world 16 ms at a time at any frame rate, and the original draws each free body as the
scene graph's update last set its node (`hktoNiTransform`, `00c65a80`): above 62.5 frames a second many
frames have no step, so dropped items, ragdolls and debris stand still on them and jump on the next. The
fix keeps the simulation and every node game logic reads exactly as the original's; only around the
frame's drawing (`008706b0`) it writes each moving body's node as the body was one step before the game's
time, blended between the two Havok steps around it with Havok's own swept-transform formula
(`approxTransformAt`: centre of mass and rotation moved linearly, rotation normalised), refreshes the
nodes below it with the game's own world update, draws, and puts every node and bound back as it was
(`crates/host/src/phys_blend.rs`). Nodes whose class has its own world update (world-space particle systems,
fixed-rotation `BSValueNode`s, billboards) and everything below them, and keyframed bodies, are drawn as the
original draws them.
The original's nodes are set before the frame's world step, so they show the body 11 ms behind the game's
clock on average (0 to 23 ms at 144 frames a second); the fix draws it 16 ms behind, about 5 ms later.

Measured on the hf-motion route in Doc's hall (three dropped items and Doc's ragdoll, 60 to 76 moving
bodies; `NV_PHYS_BLEND_LOG`, both series from the same frames; 2026-11-07). Frozen: a frame in which the
body's path moves and the drawn body doesn't. Shown/ideal: the drawn move in a frame against the path's.

| Frames | Frozen, original | Frozen, fix | Over 1.5 times the path's move, original / fix | Shown/ideal p10, p50, p90, original | Fix |
| --- | --- | --- | --- | --- | --- |
| Exactly 144 (`iFPSClamp`) | 56.6% | 0.4% | 43.2% / 0.5% | 0.00, 0.00, 2.31 | 0.98, 1.00, 1.02 |
| Limit 60 (68.5 a second of game time measured) | 7.7% | 0.0% | 9.9% / 1.0% | 0.53, 1.03, 1.50 | 0.99, 1.00, 1.03 |

The fix's frozen frames are bodies that just woke or were moved by the game (a `SetPosition`): they are
drawn from their first step's start until the drawing time reaches it, one or two frames. Off the path:
0.04 units (rms) at 144 against the original's 0.75. With `NV_PHYS_BLEND=0` the same route shows 56.5%
frozen, as before the fix, and the three dropped items settle step for step as with the fix on (same
steps active, speeds and resting places). Vanilla at exactly 60: one step in 95.8% of frames, two in 4.2%,
as before.

On screen (`NV_SHOT` every frame at exactly 144, looking down at the dropped shovel): with the fix off every
second frame of its fall is the same picture as the one before and the next moves twice as far; with the fix
on the picture changes every frame, by half as much (one frame stays still where the shovel hits the floor).

### Mod fit worn question

`ItemModMenu::DoClick` (`007838a0`), on a click of a mod in the item mod menu, fits the mod to the weapon, removes one
of the mod from the player (`007839ca`), plays the sound, takes the entry out of the list and rebuilds the menu, and
only then asks `ItemChange::GetWorn` of the mod's list entry (`00783a3f`). The removal deletes the mod's extra list
when the last one goes, so the question reads a released block. In the original the block nearly always still says
"not worn" (the list's flags were cleared before the delete, and the 32-byte pool keeps a freed block's bytes past
the first eight). A "worn" makes the player update its equipment and rebuilds the inventory list
(`InventoryMenu::UpdateInventoryMenu`, `00782a90`), which frees the weapon entry the mod menu keeps (`011d9f58`);
closing the menu (E) re-equips the weapon through it and the game stops: the play report of 2026-11-07
(measure/weapon-mod-crash). The port's heap hands freed blocks back more widely than the original's pools, which
makes the "worn" more likely; that is fixed in the allocator, separately.

The fix runs the list's click as the original's, step for step, with the worn question asked of the same entry just
before the removal (`crates/host/src/mod_fit_worn.rs`). A mod is never worn, so the answer is the original's usual
"no" and nothing more happens; no released memory is read. Tests: with the fix the click reads nothing released and
rebuilds nothing; vanilla still asks after the removal and rebuilds on a "yes".

### Half-loaded actor's level change

The maintainer's play run of 2026-11-09 ("moving too fast": a console speed change, crouch and stand,
then running across cells) stopped with `read of unmapped memory at 00154145 (in 00564db0 < 0091ea40 <
00907510 < 008d0660 < 00881d30 < 00931920)` on a loading thread (measure/crash-1109).

- **The gap.** When a cell loads with saved changes (`BGSSaveLoadGame::LoadCell`, `00849b10`, on a
  loading thread), each changed actor gets two passes: its `LoadGame` (`00932a00`, the process's own
  last) stores the references its AI process holds (the target `+0x40`, the furniture, its package's
  target, ...) as saved form ids, and its `InitLoadGame` (`00932b70`) a moment later looks them up.
  Between the two another loading thread can set the actor's animation (`00572e50`), which moves it to
  the high process (`00881d30`). A recording of the original on the same route shows exactly that
  interleaving for one actor (its target was empty that time).
- **What the original does with a saved target.** The move copies the process (`00907510` gives the
  target to `SetTarget`, which marks it targeted through the flags at `id + 8`), then
  `HighProcess::SetLastSeenLocation` (`008f6ea0`) takes the word at the id as the target's vtable and
  calls through it; for a follower the move also asks the target whether it is an actor the same way.
  For an id such as `0x0015413d` that is a call through low memory: the original's own crash. The port
  stops at the first read. Not reproduced in six fast runs of the port (the gap was met in every run,
  never with a saved target); the evidence is the exe, the recording of the interleaving and the
  engine-level test of the whole move.
- **The row.** The gap is tracked as it is: from the return of the actor's `LoadGame` to the start of
  its `InitLoadGame` (the form flag `0x200000` covers more: `LoadGame`'s own level change, `009319c0`,
  runs before any id is written). A level change of an actor in the gap takes the saved ids out of its
  process first (they read 0: the move runs as for an actor without a target, as in the recorded race),
  then puts each back where the original's copy would have put it: into the new process when the level
  copy carries that field (the target; the middle-high `+0x140`, `+0x158`, `+0x164`; a package's target
  and, for a created package, its id with its "still an id" flag, which `ActorPackage::MoveTo` leaves
  behind), nowhere when the copy drops it (as in the original), back into the same process when the move
  keeps it. The second pass then resolves them as in a load without the move: the target marked and its
  marker reserved once. If the second pass reaches the actor while the move runs, it puts the ids back
  first. Vanilla keeps the original's path (the port stops where the original crashes).
- **Not covered.** The id lists (`+0x6c`; middle-high `+0xc8`, the active effects, the queued items;
  high `+0x38c`, `+0x394`, the detection states) stay in place; the first pass still writing into the
  old process while the other thread replaces it is the original's own race.

### Thread races

Found by lStewieAl's Tweaks (`ThreadSafeThreadScrapHeap`, `ConsolePrintMultithreadingFix`,
`patchFixFaceGenRaceDataRace`; none of the three depends on JIP LN or JohnnyGuitar), built here from
the exe (`crates/host/src/thread_races.rs`). In the original these are races between the game's own
threads on Windows; the port runs those threads side by side in play (`NV_THREADS`), so it has them
too. None has been seen in a port run: the evidence is the exe and a test with real threads
(`crates/host/tests/thread_races.rs`).

- **A new thread's scrap heap.** `MemoryManager::GetThreadScrapHeap` (`00aa42e0`) looks the calling
  thread's id up in a map of 37 buckets; on the first call of a thread it allocates a heap and inserts
  it with `SetAt` (`00aa4860`): read the bucket's head, link the new node in front of it, write the head
  back. No lock. Two threads inserting into one bucket at the same moment both read the old head, and
  one node is lost: that thread's next call makes a second heap, so a block taken from the first heap
  is freed into the second. In the port two ids share a bucket when they differ by a multiple of 37
  (game threads are numbered from 1, render helpers from 65,537), so it is rare today; the original's
  Windows ids collide more often. With the row each OS thread keeps the heap it found (for its run,
  its game thread, and until the next install or the memory manager's teardown, `00aa3a40`). The
  exe's own look-up and insert run under one lock, with the stepped host's time slice taken out
  around them (as the render helpers' fork does for its caller), so no turn ends while the lock is
  held. So the map is filled as in the original, and the teardown frees every heap. A heap looked up
  while a teardown runs is not kept. Not fixed: the teardown's own walk of the map, racing another
  thread's first call, is the original's race and stays.
  After its first call a thread pays one thread-local read, where the original makes an import call
  and a map look-up through three virtual calls.
- **The debug texts and the console.** The debug texts' queue is handled and timed out on the main
  thread (`DebugText::HandleQueuedPrints`, `00a0e770`; `00a0df80`), which take the critical section
  at `011f33dc` only while the debug texts are shown; `DebugText::Print` (`00a0f8b0`) and the console
  print (`0071d0a0`, which scripts and the script extender's commands print with) take none, from any
  thread. With the row the four run whole inside that section (it is recursive: their own takes inside
  stay). Two differences from the plugin: the console print is wrapped whole, its log-file write
  included, where the plugin takes the section from the line break on; and the functions' own
  takes, made only while the debug texts show, stay, where the plugin removes them.
- **The face-gen lock's count.** `00652140` takes the face-gen lock and counts its depth (`011d59dc`)
  up, `00652160` tries it and counts up when it got it, `00652190` releases it and only then counts
  down, outside the lock. A release and a take on two threads at once lose a count. The memory-freeing
  step (`00866a90`) empties the face-gen model cache only when the depth is exactly 1 after its own
  take: a depth that drifted up keeps the cache full for good (the plugin's author links it to endless
  loading on some saves), one that drifted down empties it while another thread uses it. With the row
  the counts are locked increments and decrements at the same points. Tweaks also takes the lock out
  of three of the model map's functions (`00650200`, `00650930`, `00650ba0`); the exe gives no reason
  why that is safe, so that part is not built.
- **The editor-marker pass of a loaded model** (from Psycho, `docs/model_postprocess_serialization_engine_fix.md`
  in its repository). `InitModel` (`0043ace0`) runs `RemoveEditorMarkers` (`004b5d10`) on each model it
  loads, at `0043afac`, the pass's only caller from outside (`004b5efb` is its recursion): on the IO
  thread for queued models, on the main thread for models loaded at once. The pass keeps its root
  (`011c6328`) and a controller value (`011c6324`) in globals, tests the value once and reads it again
  before each of its four virtual calls; a pass on another thread that ends meanwhile clears it, and
  the call goes through 0 (Psycho's crash dump, in the original: `004b5e11`). With the row the outer
  call runs whole inside a critical section of the row's own, made the game's way at start-up
  (`InitializeCriticalSection`; recursive, as Psycho's lock is), so the recursion is inside it and the
  lock is taken once per model. A thread waiting for it waits as on any of the game's sections, under
  either threads scheme.
- **Tested.** The real-thread tests (`crates/host/tests/thread_races*.rs`) run four OS threads side
  by side (skipped below four cores): up to 20,000 rounds of first calls into a one-bucket map, so
  every insert meets another, and 100,000 takes and releases each of the face-gen lock. One binary
  installs the row before the first fork; the other runs the original. In 10 runs of each on this
  machine (2026-11-10, commit b45bc457): with the row, 0 rounds lost a heap and the depth ended at 0
  every time. Without it, 72 to 248 of the 20,000 rounds lost a heap, and the depth ended between
  -3,161 and 4,513 after 400,000 takes and releases. A stepped-threads test (the reviewer's) ends a
  time slice at the map look-up; it finishes, where the first version hung. Single-thread tests
  install the row and drive all nine addresses: 200 engines in a row each find their own heap, a
  heap looked up during the teardown is not kept, and the debug-text section is taken first and left
  as often as taken. The editor-marker pass: with the row its outer call enters the run's section
  first and leaves it last, also when the pass stops; without it no section is taken. Not run with
  real threads: the race needs two models with editor markers loading at once.

### Cell entry waits for its own cell

Crossing a cell border outdoors made main wait 45 to 82 ms in one frame, where frames take 12, and 68
to 150 ms with YUP and SR556 loaded: the largest regular pause in play. Measured with a log of the wait
and of the loading thread's tasks (measure/smooth-200/BORDERS.md, runs fd-dg1, fd-dg2 and ce-dg-*,
2026-11-10).

- **The wait.** When the player's cell changes and the new cell still has references queued for
  loading (`00447950`), `PlayerCharacter::Update` (`0094391c`..`00943964`) gives every queued
  reference its priority again (`00446b50`: 0 for the entered cell's 3D-critical ones, else
  `TES::GetCellPriority`, 1 near and 3 far) and waits in the IO manager until the priority-0 work is
  loaded and post-processed (`00c3dfa0`).
- **The cells ahead.** At the same crossing `TES::UpdateCurrentGridCell` queues the row of cells
  three cells ahead of the cell just left (`004528ba`, `00452920`, `0045294a`): a pre-load into the
  cell buffer, so that the grid's next shift finds them loaded. The exterior cell loader makes each
  of these tasks at priority 0 (`00527c00`), and the walk above knows only the model loader's
  references. So the wait takes the row too. The one loading thread runs its cells one after
  another (5.5 to 7.5 ms each in the base game, 14 to 25 ms with YUP, which has no cell offset
  table), and in the priority-0 order the entered cell's own loads come after them: in the
  measured crossing 29 small file loads, 1.5 ms in all.
- **Nothing needs the row there.** The entered cell is loaded and in the grid already. The cells
  ahead are outside the grid until it shifts, half a cell further on; at the shift the game cancels
  the cell loads still queued (`005281f0`) and loads what is missing itself. And when the entered
  cell has no queued references there is no wait at all, so the code after the wait never had a
  promise about them.
- **The row.** Registered over the walk (`crates/host/src/cell_entry_wait.rs`). After the
  translation's walk, at a cell entry, it walks the exterior cell loader's tasks (`011c9618`, as
  `00528110` walks them) and requeues each whose priority differs from its cell's, through the
  task's own requeue (`00440660`, `IOManager::RequeueTask`), as the walk does for references. A
  queued cell task has no cell object yet (the task makes or finds it when it runs), so the
  priority is `TES::GetCellPriority`'s arithmetic on the task's coordinates, 1 or 3 (a test holds
  the two together over the grid); the function's cache of the last cell asked is left alone. A
  task at the entered cell's own coordinates stays at 0. The requeue refuses a running or finished
  task (`00c3e690`), so a cell load already running when the player crosses is waited for, one at
  most. The walk's other caller (`0094cfd6`) passes no cell and waits for nothing: there the cell
  tasks stay as they are.
- **What stays the original's.** The wait itself: it post-processes whatever has finished, of any
  priority, and sleeps a turn while fewer tasks have finished than are in flight (50 ms in the
  original; the IO wait row ends a turn as soon as that is no longer so). The row's cells load at
  the priority of their own references, so each cell's task is still post-processed before its
  references' tasks, and the loads, their results and what is in the cell buffer afterwards are
  the same; the order of loading differs.
- **Measured** (2026-11-10, one run each; the smoothness route with the game's threads side by side
  and the render thread, the PR's exe with a log of the wait added; a check run, not a quiet timing
  sitting):

  | | Base game, row off | Row on | YUP and SR556, row off | Row on |
  | --- | --- | --- | --- | --- |
  | The cell-entry wait | 82.3 ms | 20.3 ms | 67.9 ms | 3.8 ms |
  | The frame it falls in | 121.7 ms | 55.3 ms | 102.4 ms | 31.1 ms |
  | The crossing's longest frame (3 before to 40 after) | 121.7 ms | 69.9 ms (the next) | 102.4 ms | 64.7 ms (two after) |
  | Cells ahead loaded inside the wait | all five | the one already running | all five | none |

  With the row on, the wait is the entered cell's finished loads post-processed (3.7 ms with YUP,
  7.7 ms in the base game) and, in the base game, the one cell ahead that was already running when
  the player crossed (12 ms). The other cells ahead ran at priority 3, starting up to 72 ms after the
  wait began. All of them loaded before the next row was queued. Row off and on loaded the same
  cells (40 in each base run; with YUP the 40 the shorter run reached), in the same order apart from
  the row's own. The frames next to the wait keep the border's other work (the grid's shift,
  terrain points, the renderer's lock, post-processing), so the crossing still has one long frame,
  about half as long as before. In the vanilla preset a recorded run is byte-identical to main's. In
  repeatable runs (stepped threads) the loading thread keeps up and the entered cell never has
  queued loads at the crossing: there is no wait, and the row changes nothing (the same state every
  frame, the same loading tasks in the same order).

### Cell offsets for every plugin

Each plugin that holds a world space record keeps an `OFFSET_DATA` for it: the record's position, the
grid its `NAM0` / `NAM9` corners span, and a table of where each cell record of the grid is (`OFST`,
made by Bethesda's tools). `TESWorldSpace::FindCellInFile` (`005854f0`) uses the table only for a
master file (`00585502`), and `TESWorldSpace::Load` (`00583560`) stores the position and corners only
for a master (`005835bc`, `00583dc0`, `00583ea8`). Any other file, and a master whose tables were
stripped (YUP's and TTW's: the tools that make them drop `OFST`), is searched record by record from the
world space record each time one of its cells loads, once for each such file, on the one loading thread
(Cell Offset Generator's author measured up to 20 ms a file a cell in the original). A cell task took 5.5
to 7.5 ms in the base game and 14 to 25 ms with YUP (measure/smooth-200/BORDERS.md, run fd-dg2), which
was thought to be mostly that search; it is not (below).

- **The row** (Cell Offset Generator's idea; `crates/host/src/cell_offsets.rs`). Registered over `Load`
  and `FindCellInFile`. `Load` runs the translation, then gives the record's `OFFSET_DATA` the table its
  search would give, built from the plugin's own records (`crates/esm/src/cell_offsets.rs`: one pass over
  the plugin's world space group, on a host thread that the run's first world space record starts for
  every plugin of the load order, in load order; `Load` waits for a plugin that is not done yet). A
  master keeps the position and corners the translation stored and gains a table only where it has
  none; any other file gets them as a master would, and its table replaces an `OFST` it may carry,
  which the original never reads for it. `FindCellInFile` finds a cell of a file that is not a master
  by that table, as the translation finds a master's; anything else goes to the translation. The tables
  are in the game's heap like the masters' own and go with the world space (`00588c80`).
- **The same cells.** The builder runs the search's own walk once per block and sub-block key: the
  groups it enters and skips, the records flagged ignored that `TESFile::NextForm(1)` passes over (but
  not right after a skipped group), `TLOD`, where it stops; a persistent cell is never found, and a cell
  without a grid position is at 0, 0. Each position's entry is the record the search finds first. Where
  a table cannot give the search's answer for every position (a cell the search finds outside the
  corners, no corners, a grid of no squares or more than 4,194,304, two records of one world space in a
  file, a cell that does not inflate) that world space gets none and is searched as before. The one
  place a table answers differently is a cell asked for at x = 2,147,483,647, where the search finds
  the persistent cell; no cell can be there. Built from the base game's masters' records, every one of
  their 41 tables is the masters' own `OFST`, entry for entry (`crates/esm/tests/cell_offsets_real.rs`,
  run on FalloutNV.esm and the four story add-ons, Dead Money, Honest Hearts, Old World Blues and Lonesome
  Road).
- **Not taken from Cell Offset Generator.** Its cache of tables on disk (`Data\CellOffsets`, checked
  with an xxHash of the whole plugin): it builds a table by running the game's search for every square
  of the grid, which is slow; one pass over the records reads less of a plugin than hashing it would,
  so nothing is kept between runs and nothing is written. Its other patches, which the cell loads don't
  need: `LoadPartial`'s master checks (the row stores the corners itself), the world space's and the
  cell's `FindInFileFast` (`005858dc`, `005502a1`: a plugin's `TESFile::FindForm`), the editor-ID
  lookup's "optimized file" test (`00461db1`, used by the console's `coc`) and its null check
  (`0058598f`: an `OFFSET_DATA` with no table, which the row never makes).
- **Measured** (2026-11-11, one check run each way: the short route, `NV_REPEATABLE=5400` with stepped
  threads, YUP and SR556 loaded, the fixes preset with the row off and on; other agents' builds were
  running, so the times are a check, not a timing sitting). Both runs pass the route check and load the
  same 26 cells in the same order (the coc into Goodsprings). YUP's 28 world spaces got their tables in
  8.4 ms on the host thread (the builder alone, release build, warm: 1.9 ms; FalloutNV.esm, which needs
  none, is passed over in 3.2 ms). The cell loads took 335.6 ms in all with the row off and 317.6 ms with
  it on (mean 12.9 and 12.2 ms, median 11.7 and 11.2): about 0.7 ms a cell, inside this run's noise (the
  same cell's two loads differ by up to 10 ms either way). So in nv-rs the search of one plugin is a
  small part of a cell load; most of it is the cell's own records, which the row does not change, and
  the 14 to 25 ms cell tasks with YUP above are mostly those. What the row saves is one search per cell
  for each plugin that edits a world space, so it grows with long load orders and TTW. The tables (about
  370 KB of the game's heap with YUP) move later heap blocks, so the two runs are not byte for byte the
  same: the player's height on the street at the end differs by 0.02 units.

### Direct file access

Mod Limit Fix 7 and Cell Offset Generator 1.08 (WallSoGB, GPL-3.0; Viva New Vegas and TTW's guide install
both) change how the game reaches its files, and JohnnyGuitar NVSE (LGPL-2.1, in both guides) lets its
archives' files lie anywhere in an archive of up to 4 GB (its `bFullOffsetRange`, on by default, which
needs Mod Limit Fix). The row does the same, built from the exe (`crates/host/src/direct_files.rs`).

- **The original.** `NiFile` and its subclass `BSFile`, which read every plugin, archive and loose file,
  keep a C runtime `FILE` and reach it through Obsidian's "Serialized IO Thread": each read, write and
  seek is queued to that one thread, which runs `fread`, `fwrite` or `fseek` and then wakes the caller.
  The C runtime holds at most 512 open files, and the game keeps one on each plugin for each of its
  form-loading threads: about 140 plugins fill it (Mod Limit Fix's author). Its `fseek` takes a signed
  32-bit offset and its `ftell` refuses a position past 2 GB, so a file larger than 2 GB can't be read
  past that point, and its size reads as 0. And an archive keeps each file's offset in 31 bits: the
  32nd is a mark the PC game never sets (the check that a file has data compares it with the archive's
  headers-only bit), so a file stored at 2 GB or more in an archive has no data. Generated object LOD
  textures are "often exceeding 2GB, especially with Tale of Two Wastelands" (VNV's LOD guide), and
  packed into one archive: in the original, every texture stored past 2 GB is missing.
- **The row.** `NiFile`'s and `BSFile`'s opens (`00aa14d0`, `00aff300`) open the file with the platform's
  `CreateFileA` as the C runtime's `fopen` of the same mode does (the same access and creation), and keep
  the handle, marked by its low bit, where the `FILE` was. The IO thread's three calls (`00aa85c0`,
  `00aa8610`, `00aa8660`) read, write and seek it on the calling thread with `ReadFile`, `WriteFile` and
  `SetFilePointer`; `NiFile`'s seek (`00aa16d0`) moves it with 64-bit positions; the sizes (`00aa15f0`,
  `00b004e0`) come from `GetFileSizeEx`; `fclose` (`00ec9907`) closes it. The IO thread is not started
  (`00aa7eb0`) and its stop at the end has nothing to stop (`00aa81d0`); nothing else in the exe waits on
  it. A `BSFile` in text mode (the game's own dumps) and a `FILE` handed to one keep the C runtime, whose
  calls now run on the calling thread too. The archive functions that read a file's offset word take all
  32 bits of it, at JohnnyGuitar's sites: the check that a file has data (`00afb2a0`), the making of a
  file's object by its indices and by its entry (`00afa550`, `00afa750`), the invalidations of a file
  that is loose on disk (`00afb190`, `00afac20`, `00afac90`, `00afad00`: they clear the whole word, since
  an offset of 0 is what marks a file with no data), and the readers for a log line, a queued file's
  description and its place in the loading queue (`0043c3b0`, `00c3d0e0`, `00c3d440`). An archive's read
  (`00afb7a0`) moves its file by the difference of two 32-bit positions, a relative seek: the row's seek
  lands such a move where the game's 32-bit sum puts it, so every position of an archive of up to 4 GB is
  reached (JohnnyGuitar instead splits the move at `00afb99b` into steps that Mod Limit Fix's signed seek
  can take).
- **What differs from the original.** No limit on open files. A read, write or seek runs on the thread
  that asks instead of waiting for the IO thread's turn (with real threads the platform's file calls
  still run one at a time, under its lock, and an archive keeps its own lock around each seek and read,
  as in the original). A seek from the start reads the game's 32-bit position as unsigned, so positions
  and sizes up to 4 GB work; past 2 GB the original refused the seek and gave the size as 0 (a file of
  4 GB or more still answers 0). A relative seek of 2 GB or more, on or back, goes where the game's 32-bit
  sum points (the original's `fseek` refused it or went astray). A move before the start is still refused
  in a file under 2 GB, where its sum is always past the end; in a file of 2 GB or more it lands at the
  sum even past the end, as a move on past the end does, so an archive's bad entry reads nothing instead of
  leaving the whole archive unreadable. An archive's files stored at 2 GB or more are found and read;
  in an archive under 2 GB nothing changes (no offset there sets bit 31). The C runtime's undefined cases
  answer as Windows does, as with Mod Limit Fix: a read of a file opened only for writing answers 0 (the
  C runtime answered 0, or copied bytes out of its own write buffer), and a write right after a read
  through one file opened for update is written (the C runtime dropped it, or refused it).
- **Not taken from Mod Limit Fix.** Its opens differ from the C runtime's in the write modes (`"wb"`
  opened without emptying the file, `"ab"` failing on a missing file), it reads every seek offset as
  signed, and it opens text-mode files in binary. The row keeps each mode's meaning, and text mode on the
  C runtime.
- **Tested** (`direct_files.rs`; the archive functions also in `engine-bs`'s `archive.rs` and
  `queuedfiles.rs`, registered from the one list the row registers, `archive::full_offset_range`, and
  called at their addresses): the same answers, bytes, positions and files as the C runtime's path for reading (31
  steps: through, around, inside and outside the buffer, at and past the end, a refused seek), for
  `NiFile`'s two write modes and for `BSFile`'s three binary modes, a missing file in each (and only the
  file's own handle left open), and its size asked before any open in each binary mode; 600 files held
  open at once and each read while all 512 of the C runtime's are taken, where the original's open fails;
  a 3 GB sparse scratch file read at 2.25 GB and at its end, reached from the start, the position and the
  end, with its size (the original: the seek refused, the size 0), and relative seeks of 2 GB and more
  on and back reaching their markers, and one before the start whose sum is past the end landing there,
  the file still good; a 5 GB sparse file answering size 0, as the original does past 2 GB; a 2.5 GB
  sparse archive with files below, across and past 2 GB, made by `00afa550` (the original makes none for
  the one past 2 GB) and read by `00afb7a0` in an order that moves its file back and on by more than 2 GB;
  an entry past such an archive's end, read after one near its start: nothing read, and the archive still
  good for the next file; the archive functions on a file at
  `0x80000300`: data, a file object starting there, the lookup finding it, and each invalidation clearing
  the whole word; the IO thread not started, and its calls answered at once.
- **Measured** (2026-11-11, the short route, real threads, Vulkan, the play profile; a warm-up run, then the
  row off, on, on, off, on a machine busy with builds): the Goodsprings `coc` load took 3.0 and 3.1 s with
  the row and 4.0 to 5.7 s without it (every run without it, the warm-up included); the save's load from
  the menu 3.7 and 4.4 s against 4.1 to 7.4 s. The route check passed in all five
  (`measure/vnv-batch1/host-file-io/runs/c1`).

### Dialogue topic index

`TESTopic::GetOwnerTopic` (`0061a310`) walks the data handler's topic list and asks each topic (`00619e10`, first to
last) for the info's id until a topic answers with this very info. Loading a plugin that changes an existing info
asks it once per such info (`TESTopicInfo::ClearData`, to take the info out of its owner): with YUP and the add-ons
5,837 searches of about 68,000 infos each. The fix (`crates/host/src/topic_index.rs`) notes every topic
`TESTopic::InsertInfo` (`00619820`, the one place an info enters a topic) links the info into, asks only those with
the same `00619e10`, and of the ones answering this info takes the first in the list's order. The order comes from a
copy of the list, made again after the sort (`0061a5a0`), the data handler's `ClearData` (`0045dfe0`), an insert
(`0061a380`) of a topic the copy already holds (it can list the topic again, earlier), a changed head node, or when
an answer is missing from it after an insert. So the answer is the walk's, tie-breaks included. JIP LN answers from
a word set as the load links the info, which is the last topic linked, not always the walk's answer. With the fix on,
the typed `topic_search` family goes by address for the run (the fix registers over its member `0061a310`).

Measured 2026-11-07 (measure/topic-index; YUP, the five add-ons and SR556, vanilla preset with only `NV_TOPIC_INDEX`
switched, default typed families, so "off" is the typed walk):

- `NV_TOPIC_INDEX_CHECK=1` with `NV_TOPIC_INDEX=1`: over 5,000 searches (the load, the main menu and Continue), the
  index gave the walk's answer every time; the list was copied twice.
- Profiled load (`NV_PROFILE`): the search goes from 27 of 32 profiled seconds to too small to list. The profiler
  slows every call, so this says where the time was, not how much.
- The final build, fix on, four runs: the first main frame 4.39-5.01 s from the process start every time, the load
  done (its last sort) at 3.2-3.8 s. The index's counters, the same in all four: 5,837 searches, each answered by
  the one topic holding the info, 3 copies of the list's order, 54 ms in all (7 ms of it copying).
- Off (the typed walk), six runs: 7.85-8.61 s. Add-ons only, no YUP, is 4.3 s (measure/yup-load).
- Five earlier runs with the fix on (builds before the counters' line printed) took 7.86-8.02 s, as slow as off;
  three took 4.49-4.64 s. Why is not known. The review suspects a wait before the first main frame that has
  nothing to do with the index: even in the fast runs the first main frame comes about 1.2 s after the load's
  end. So no fixed saving is claimed here.
- Continue (save to world) 3.2-4.0 s with the fix on and off.

### V.A.T.S. waits only for the camera

lStewieAl's VATS Lag Fix (`VATSLagFix.dll`, read in its disassembly), built from the exe. In mode 3
the V.A.T.S. menu's scan (`007f3e00`) measures the target's parts one by one (their place on screen,
`00800a00`; whether each is visible, `007f06d0`); a part not measured yet stays without a chance until
the scan gives up on it: at `007f4058` the code that gives every such part the average chance of the
measured ones (and marks it measured and shown) runs only when more than 2000 ms (`0x7d0`) have passed
on the interface timer (`011f6394`) since the scan began (`011db0dc`); then the menu goes to mode 2 and
shows the chances. The plugin keeps that jump but sends it to its own test: once the menu's
`bHeadingReached` (+0xf8, Xbox PDB; set by `007f3bd0` when the camera has turned to the target) is set,
the give-up runs at once. The row is that test, in a copy of the scan registered at `007f3e00` (the
menu's update calls the scan by address now); vanilla's scan is the translation as it was. One
difference from the plugin: the heading ends the wait only once at least one part has been measured.
With none, the give-up's average is 0/0, and every part still without a chance would get no number
at all (shown as 0%, and V.A.T.S. takes no attack on it). The row then waits for the first measured
part, or for the 2000 ms as the original.

What a player sees: the hit chances appear as soon as the camera arrives, where the original could sit
for up to 2 seconds first, mostly when a part is hidden or off screen. The measured parts keep their own
chances, and the parts the original gives up on get the same average, only sooner. One more difference,
the plugin's too: the scan shows and measures one part a frame, so when the camera arrives within a few
frames of the scan's start, a part on screen that it has not measured yet gets the measured parts'
average at once, where the original would measure it in the following frames and give it its own
chance.

Tested: `vatsmenu.rs`'s `the_scan_waits_2000_ms_unless_the_heading_ends_the_wait` runs the scan in mode 3
with a measured part (40) and one without: the original gives the second the average only after 2000 ms
whatever the heading; the row gives it at once with the heading reached, and waits as the original
without it. `at_the_heading_the_row_averages_a_part_the_original_would_measure_next` takes a part on
screen not yet shown when the heading is reached: the original shows it (to measure it next), the row
gives it the measured part's 40 and marks it measured; with no part measured yet the row shows it as
the original does.

### Causes keep no witness lists

lStewieAl's ActorCause Save Bloat Fix (`ActorCauseSaveBloatFix.dll`, read in its disassembly), built from
the exe. An `ActorCause` (Xbox PDB; 0x1c bytes) is made for each projectile, explosion and other thing an
actor causes (`0066e010`): its actor, a list at +4 (`pWitnesses`, Xbox PDB), its origin, an id. The
constructor fills the list with what the actor's process answers at its slot +0xf4: a copy of the
actor's detection states, one 0x24-byte record per actor it detects. Every cause on the game's cause list
is saved (`0066e390`): each record as a form id and eight values with their spacers, 42 bytes (40 before
the save's form version 8). The plugin skips the copy (a jump at `0066e099`), skips the save's loop
(`0066e3de`) and, on load, reads the count and skips its records by their size (`0066e499`).

The row: the constructor (`0066e010`) asks for the actor's process as before but does not call its slot
+0xf4, so the list stays empty, as it is for a cause of an actor without a full process in the original
too; the load (`0066e440`) reads each saved record with the original's reader (`008d7140`) into a record
on the stack and drops it, so a save written by the original or by vanilla is read to the same byte
(spacers and the version's extra byte included) and keeps no list. The save itself is left as it is: with
no list made or loaded it writes a count of 0, the same bytes the plugin writes. Who reads the list:
only the cause's own functions (`0066df40` frees it, `0066e100` removes an actor's record, `0066e200`
resets it, `0066e390`/`0066e440` save and load it, `0066e640` resolves its form ids after a load); the
Xbox PDB has no accessor for it. So what a player sees is smaller saves that save and load faster in long
fights (the guides quote 15-30%; not measured in nv-rs). A save made with the row loads in vanilla and in
the original (count 0), and one made without it loads with the row.

Tested: `run_0066e010.rs`'s `without_witnesses_the_process_is_asked_but_nothing_is_copied` (the same words
as the original's except +4, and the process's slot never called) and `actorcause.rs`'s
`load_game_without_witnesses_reads_each_record_and_keeps_none` (three records read by `008d7140`, nothing
allocated, no list) and `the_list_loads_each_cause_by_address`.

### Radio song lengths

From Psycho's radio fix (`psycho-engine-fixes/src/mods/perf/radio/music.rs` and
`docs/radio_scan_hitch_evidence.md` in its repository), built from the exe; two of its parts.

- **The duration left on a channel.** Music plays on two channels that cross-fade (`FalloutAudioMedia`).
  Opening a file (`008300c0`) sets up the next channel and wakes its thread, which publishes the file's
  position and duration (`011dd320` for channel A, `011dd354` for B) once it runs. Until then the channel
  holds what its last file published: a channel is cleared only when its file's thread ends, after its
  fade-out. A radio station asks for the duration right after opening the song (`008308c0`) and keeps a
  positive answer as the song's length, which decides when it moves to the next song. A song opened while
  the channel's last song still faded out (a quick change of station) got that song's length: cut short,
  or followed by silence. With the row the opened channel's duration is cleared under the lock before its
  thread is woken, so the station gets 0 and asks again later, as it does for any song whose thread has
  not answered yet. The clear alone leaves a window: each channel has one thread, and the last song's
  thread, still fading out, runs on until the end of its round (up to one update, at most 100 ms), where
  it sees the new ID and stops. That round first writes its own length to the channel again, and the new
  song's thread starts only after it, so a station asking meanwhile still got the old length. So the row
  also takes Psycho's check at the four places the thread writes the duration (its start-up and each
  round, for either channel: `00830c97`, `00830d2c`, `0083135d`, `008313d7`): the write happens only while
  the channel's ID is still the thread's, under the same lock as the open. The duration then stays 0 from
  the open until the new song's own thread publishes its length. Psycho also ties the station's progress
  to the songs' ends; that part is not taken.
- **The sync loop.** Each round the music thread (`00830b30`, the loop at `00831790`) brings the time since
  the channel's synch start into the file's length by taking the length off while it is longer, then seeks
  when the position is off by more than the tolerance. With a length of 0 (none known: the thread's first
  rounds, or a failed duration query) that loop never ends: the music stops and the thread spins on one
  core for good. With a clock behind the synch start the time wraps to about four billion milliseconds,
  taken down one length at a time before the clamp to 0. With the row a length of 0 with time to bring in
  skips the seek (playback goes on where it is, as Psycho does), and otherwise the time is
  `(time - 1) % length + 1`, the loop's own answer, in one step. Where the loop ends the row's answer is
  the loop's.

The media object's open (`00830010`) and the channel thread (`0082f7b0`) call the two by address now, so the
row applies there too. Tested: `falloutaudiomedia.rs`'s `the_radio_open_clears_the_duration_before_waking_the_thread`
(the duration the thread sees when woken: the last file's without the row, 0 with it; the other channel
keeps its own), `a_fading_thread_no_longer_writes_its_length_over_the_next_song` (a song opened on a
channel while its last song's thread runs one more round: with the original's thread proc that round
writes the old 10 s back and the station gets it; with the row's the station gets 0 until the new song's
thread publishes its own 20 s), `bring_into_length_is_the_loops_answer` (against the loop for lengths 1 to 2^31-1),
`the_bounded_sync_plays_as_the_original_within_the_length` (the same calls as the original's thread
proc on the play test) and `the_bounded_sync_skips_the_seek_without_a_length` (a length of 0 ends the
run with no seek, where the original's never ends).

### Patch data

Data mods come as patch data (the maintainer, 2026-11-01): each author's change, kept as the record, the
field and the value in nv-rs's own table (`crates/host/data/patch_data.tsv`, read by
`crates/host/src/patch_data.rs`), applied to the player's own game files as they load. No game file ships;
each change belongs to a fixes row, so it is switched with that row and is off in vanilla, and each row
names who made the change.

- **No distant water under the Strip** (`NV_STRIP_LOD_WATER`): FalloutNV.esm's world space `0013b308`
  (`TheStripWorldNew`), whose `DATA` is 0x01, gets 0x10, "No LOD Water". The flag is the world space's
  `cFlags` bit 4 (+0x4c), and the exe reads it in one place: `004e4c80`, which makes the distant water
  (a temporary reference of the world's LOD water with a water shader, in a fade node) and turns the water
  system on, returns nothing when the current or the given world space has it. The Strip's distant water
  lies under its ground and is never seen. Applied after `TESWorldSpace::Load` (`00583560`, reached only
  through the form's vtable): the record loads as the original reads it, then the bit is set; a plugin that
  overrides the record later loads it again and gets the bit again. Credit: Aqua Performa's author found
  it (about 20 frames a second on the Strip on their machines); YUP, UPNVSE Plus and Tale of Two Wastelands
  3.3.3b ship the same flag. Not measured in nv-rs: the short route does not reach the Strip.
  VNV's and TBoT's `bForceHighDetailReflections=0` (water reflecting the distant world instead of drawing
  the scene a second time) changes the picture, so it is not part of this row; it is left to the
  maintainer as a preset choice.

Tested: `patch_data.rs`'s `the_table_reads_and_every_row_is_applied`, `the_strip_gets_no_lod_water` and
`the_load_in_both_modes` (the record's flags as loaded in vanilla, 0x11 for the Strip with the row, nothing
for another world space or another record type).

### The script extender

The built-in xNVSE (ADR-0016) is the same in both presets, except for these rows and the
differences listed first below. Like xNVSE, it
writes a `.nvse` co-save beside every save, an empty one too, in both presets: a save without
one loads as a new game for the extender without resetting what the game before held
(`Serialization.cpp`, `HandleLoadGame`), which only a save made without the extender (a vanilla
save) should get.

- **Which xNVSE it is, and its plugins.** The built-in matches xNVSE 6.4.9: `GetNVSEVersion`
  answers 6, `GetNVSERevision` 4 and `GetNVSEBeta` 9 (xNVSE's `nvse_version.h`:
  `NVSE_VERSION_INTEGER`, `NVSE_VERSION_INTEGER_MINOR`, `NVSE_VERSION_INTEGER_BETA`), and the
  co-save header says 6.4. All of them read one constant, `nvse::VERSION`
  (`crates/nvse/src/lib.rs`): change it there when the built-in catches up with a newer release.
  `IsPluginInstalled` and `GetPluginVersion` answer by what nv-rs builds in, since no plugin DLL
  ever loads in nv-rs: no plugin is built in yet, so every name answers not installed (0, and
  -1 for the version: xNVSE's answers for a plugin it did not load), JIP LN NVSE's names
  included. That changes when JIP LN NVSE's commands are built in (ADR-0016's I-13): JIP puts
  its own versions of these two into xNVSE's records when it loads, and they answer for its
  names.
- **Where the port differs from xNVSE in both presets** (none is a row). `FireChallenge` with a challenge type
  outside 0 to 13: the game's function returns nothing and, for such a type, leaves its stack
  cookie in the upper bytes of the register xNVSE reads its answer from (`005F5970`), so xNVSE
  answers a number that changes from run to run and is not 0 in practically every run; the port
  answers the translated function's 0. A mod's `if FireChallenge <bad type> ...` is therefore true
  in the real game and false here. (For a type of 0 to 13 both answer 1.) `AddAnimation` and
  `DelAnimation` with a name of 256 bytes or more, and `GetNumericGameSetting`,
  `SetNumericGameSetting`, `GetNumericIniSetting`, `SetNumericIniSetting` and `GetModIndex` with
  one of 512 bytes or more, in the vanilla preset: see "Script list and scan guard" below.
  `AddSpellNS` silences the game's `QueueUIMessage` while AddSpell runs as xNVSE does (a `ret`
  written at its entry, here a wrap), and the silenced call answers 0, the function's own answer,
  where xNVSE's `ret` leaves the caller's register as it was; no translated caller reads it.
  A format string of 16 KB or more (`MessageEx`, `sv_Construct` and every command taking one,
  the text given or a `$` variable's): xNVSE copies it into its 16 KB buffers without a bound
  (`s_tempStrArgBuffer`, then `ExtractFormattedString`'s `fmtBuffer` on its stack) and writes
  past them; the port takes it whole, and cuts the formatted text at 16 KB less 3 bytes only
  where the C runtime formats it (a format with specifiers).
- **Co-save keeps other plugins** (the row above): the maintainer's decision, 2026-11-08. The
  sections are kept byte for byte from the co-save of the game loaded (none after a new game or a
  save without a co-save) and written after xNVSE's own section, with that co-save's file list:
  only while today's files are the same ones in the same order. Their data holds mod indexes of
  that list (JIP LN NVSE maps them through the list it was saved with), so under another list
  they go, as with xNVSE.
- **Script error loop guard** (the row above): the maintainer's decision, 2026-11-08 (review of
  #329). One error reaches the handlers once; the console still shows every error. Kept per game
  thread: an error another thread raises meanwhile is its own and reaches the handlers.
- **Image-space event form check** (the row above): from the review of #340, 2026-11-08. Vanilla
  keeps xNVSE's read and only refuses a word that is no readable form, since ordinary play must
  not stop.
- **Script reference checks** (the row above, once "Inventory reference checks"): from the review
  of #349, 2026-11-08: `RemoveMeIR`'s freed data, OnSell's kept item and `GetInventoryItems`' base
  entry without a form, all xNVSE 6.4.9's own (`InventoryReference.cpp`'s `RemoveFromContainer`,
  `EventManager.cpp`'s `OnSell`, `GameObjects.cpp`'s `GetInventoryItems`). The reviews of #384
  added the lookups by id over such an entry (`InventoryInfo.h`'s `ContainerFindRefId`), changing
  only the stop: the counts stay xNVSE's in both presets. The reviews of #388 (the coordinator's
  decision, 2026-11-10) widened the row to a reference handed to a script with parts missing: an
  inventory reference with no base form or cell (`Commands_MiscRef.cpp`'s door teleport and
  parent cell commands), and a door with no teleport data, linked door or lock data
  (`GetLinkedDoor`, `SetDoorTeleport`, `SetOpenKey`). The coordinator's decision, 2026-11-10
  (the MiscRef batch's cell scans and owners): the ownership tests' missing cell and base form, a
  cell reference without a base form in the scans, and `GetNextRef` on a scan record that has ended
  in the console or was never built (read from `Commands_MiscRef.cpp`'s `CellScan` and the shipped
  DLL, where a new record's iterator alone is zeroed; not run in the real game) went under this row
  rather than a new one: few switches, and no player would want one of these on and its neighbour
  off. The same decision (the Misc batch, 2026-11-10): the door without a sound, the leveled entry without a form and `GenericCheckForm`'s empty node (read in xNVSE's source and the shipped DLL, `10035F30`, `10034835`, `100354C6`; not run in the real game). And `IsRefInList`'s reference without a base form (the List batch, `10031B20`). Under the row only what would otherwise stop
  the game changes; where xNVSE answers, the port answers the same in both presets. Vanilla is
  1:1 with xNVSE, its stops included.
- **Script array and string size caps, missing array guard, array cycle guard** (the rows
  above): the coordinator's decision, 2026-11-08 (review of #346). Vanilla is xNVSE's string
  and array commands as they are, their crashes included, except an outright hang or an
  allocation without end, which is capped in every preset; every other difference is one of
  these rows, on by default. The out-of-memory stop (an array past 33,554,432 elements, a
  string past 1 GiB): the coordinator's decision, 2026-11-08 (follow-ups to #346); vanilla
  stops where xNVSE's 32-bit process would, instead of taking the computer's memory. The format
  strings' part (issue #357): the coordinator's decision, 2026-11-08: where the C runtime
  crashes, ends the process or needs output without bound, the size caps row makes the text
  safely and vanilla stops the game. That a refused format ends the game was measured in the real
  game (xNVSE 6.4.9 with JIP LN 57.30: `sv_Construct "refused [%Ld]" 5` ends it with
  0xC0000409, the next line never runs); the memory limit was measured in a 32-bit harness and
  moves a little from run to run.
- **Script list and scan guard** (the row above, once "Script inventory list guard"): the cell
  scans' part (the coordinator's decision, 2026-11-10): xNVSE keeps a scan's position as two signed
  bytes and its depth as an unsigned byte (read in the shipped DLL, xNVSE 6.4.9's `nvse_1_4.dll`:
  the scan record's constructor and `NextCell`, whose end test compares the byte widened to 32 bits,
  less the cell's own x or y, with the depth); past 127 the byte wraps and the test is never met.
  The hang was read, not run in the real game. With the row the walk ends when it is back at its
  starting position (it has been through every position it can reach, each once, at most 65,536),
  so `GetNumRefs` and `GetRefs` count and list the references of the cells passed, and
  `GetFirstRef`/`GetNextRef`, which walk only until they find a reference, go on answering the
  same references round and round as xNVSE does (in both presets): only a call that goes the whole
  way round from where it began without finding one is the hang, and with the row it answers 0 (from
  the review of #409). A depth that ends is walked as xNVSE walks it in both presets. The name buffers' part (the coordinator's decision, 2026-11-10, the Misc batch): read in xNVSE's source and the shipped DLL (`GetAgeClass`'s buffer at `ebp-0x100` with the saved frame above it; `AddAnimation`'s at `ebp-0x104` and `DelAnimation`'s at `ebp-0x110`, the form argument and the command's counters above them); not run in the real game. `GetAgeClass`'s stop is exact (xNVSE faults there). For `AddAnimation` and `DelAnimation` the vanilla stop at 256 bytes is a deviation the coordinator chose (the review of #415): `AddAnimation` at 256 bytes does not stop in xNVSE but adds the name followed by run-dependent bytes (the form pointer's and the frame pointer's), its first real fault at 257 bytes without a form argument and 260 with one; `DelAnimation` from 256 to 264 bytes compares the first 256 bytes and then the node's index byte, so it can delete only the list's first name, its first fault at 265 or 272 bytes. `AddAnimation`'s answer depends on the run and cannot be reproduced; `DelAnimation`'s is fixed by the list and could be, but its stop is kept with `AddAnimation`'s as one rule, since no script means to give such a name. The Game batch's 512-byte names (2026-11-10, the same rule; read in the shipped DLL, not run in the real game): `IsModLoaded`'s buffer (`ebp-0x200`) has the saved frame right above it, so its stop is exact. `GetNumericGameSetting` and `GetNumericIniSetting` (`ebp-0x208`) have the 8-byte number above it, written only after the name is used, so xNVSE reads names of up to 519 bytes whole and faults from 520; `SetNumericGameSetting` and `SetNumericIniSetting` (`ebp-0x204`, the float read after the name above it) and `GetModIndex` (`ebp-0x204`, the name's own pointer stored above it after the read) look up a different name from 512 to 515 bytes and fault from 516. The form lists' part (the coordinator's decision, the reviews of #415 and #420): the same bound as the inventory walks (1,048,576 nodes, `invref::MAX_LIST_NODES`); a walk the row ends answers what it found so far (a count, no index, no node removed, the calls made). The inventory lists' part: from the review of #365, 2026-11-09. The game's
  own lists never run back into themselves; the bound only turns a hang into a stop, or with the row
  into the end of the walk; `GetPaired` (`Commands_Inventory.cpp`) asks again with an index its
  search did not count on, so it ends at the first such answer (the review of #384). The hotkey
  list's part (the coordinator's decision, 2026-11-09, from the review of #380): xNVSE's
  `SetHotkeyItem` gives a stack with no extra data a `new ExtraDataList()`, all zero, and the game
  frees a stack's lists through their vtable (`ItemChange::DeleteAllExtra`, `004BC780`). Measured in
  the real game (xNVSE 6.4.9 with JIP LN 57.30, run `hk1`): `SetHotkeyItem 8` on the player's
  stimpaks, then all of them removed, ends the game with an access violation (0xC0000005). With the
  row the list gets `ExtraDataList`'s vtable (`010143E8`, named by its RTTI); the stack still counts
  one more item, as with xNVSE (run `inv4c`). The token commands' part (the coordinator's decision,
  2026-11-09): xNVSE's `GetItemWithHealthAndOwnershipByRefID` (`InventoryInfo.cpp`) looks for the
  item's stack in the changes, then in the base container, and there reads `pEntryData->type` with
  no entry and adds 1 to `*inOutIndex` with no index. Measured in the real game (run `inv6r1`):
  `player.GetTokenValue` of the Pip-Boy, the first entry of the player's base container, ends the
  game with an access violation (0xC0000005). With the row the search goes on without those two
  reads: the health is the found item's own, nothing is counted on, and the command answers as for
  any item; an item with a stack of its own is not touched by this. (Container changes without
  data, which xNVSE makes itself after `CopyIR` or `RemoveMeIR` into a container that had none, are
  read as no changes in every preset, as xNVSE reads them: not a fix.)
- **Input** (I-11): the queue of menu keys is the size caps row's (review of #362); otherwise no
  row. xNVSE's DirectInput hook stops the game only on calls the game never makes (a buffered read
  that peeks with records, a mouse read of another size, a record size other than DirectInput 8's);
  every keyboard and mouse read the game makes goes through it as with xNVSE, in both presets.

## Credits

Written with the fix plugins' knowledge, by their authors' permission (2026-11-03); no code is
translated from them in these fixes.

- **New Vegas Tick Fix (NVTF)**, carxt, lStewieAl and jazzisparis, LGPL-2.1: `BSTimer`'s fixed
  step from the measured frame and the survey of what goes wrong at high frame rates.
- **ITR NVSE** (Into the Rough contributors, MIT): `SlowMotionPhysicsFix`, read for what slow motion does
  to physics.
- **TEMPO** (Fallout 3): scaling the grab's per-frame pull by the frame's time times 60, at most 4.
- **Mod Limit Fix 7** and **Cell Offset Generator 1.08**, WallSoGB (Mod Limit Fix first by iranrmrf),
  GPL-3.0: which file calls to take off the C runtime and the IO thread, and that the IO thread can go.
- **JohnnyGuitar NVSE** (its contributors, LGPL-2.1): the full archive offset range, which places read an
  archive file's offset word and that its 32nd bit is free on PC.
- **Viva New Vegas** (ModdingLinked, MIT): the guide's frame-rate rules, which these numbers were checked
  against.
- **Cell Offset Generator**, WallSoGB, GPL-3.0: offset tables for every plugin and the engine's master-only
  checks (`CellOffsetGenerator.cpp`), the idea of the cell offsets row; its tables are built another way here
  and none of its code is used.
- **lStewieAl's Tweaks, VATS Lag Fix and ActorCause Save Bloat Fix**: the thread races' first three
  places, the V.A.T.S. wait that ends with the camera, and the causes' witness lists.
- **Psycho** (acidpointer): the editor-marker pass's lock and the radio's two music fixes.
- **Aqua Performa**'s author, **YUP** (Yukichigai Unofficial Patch), **UPNVSE Plus** and **Tale of Two
  Wastelands**: "No LOD Water" on the Strip, as patch data.
