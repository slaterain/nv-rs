# Phase 1: the frame skeleton (proposal)

Drafted 2026-10-09 at the end of Phase 0 ([ENGINE_PORT_PLAN.md](ENGINE_PORT_PLAN.md),
[LEDGER.md](LEDGER.md)); PR 1 (the frame map) and PR 2 (`world::frame`) done 2026-10-09, the rest is
not implemented yet. Names are from the Xbox 360 prototype (Xbox PDB,
ADR-0002), PC addresses from `research/engine-map/engine_map.tsv` and
`research/engine-map/frame.tsv`.

## The entry point

The frame is `Main::OnIdle` (Xbox PDB): PC `0086e650`, 2,272 bytes, called
once per frame from `main` (`0086a850`, matched by its strings; the call is
at `0086b3e3`, its only caller). It was not named by the matcher; it is the
only PC function that calls both `Main::OnIdle_UpdatePlayer` (`0086f940`)
and `Main::OnIdle_PollControls` (`0086f390`), and its 143 call sites line
up with the Xbox `Main::OnIdle` (`8269f3c8`, 142 calls without the PowerPC
register save helpers) apart from profiling timers and debug-only calls.
The PC version polls the keyboard directly (`GetAsyncKeyState`, six calls).

## Its order (Xbox PDB names, PC addresses where matched)

Grouped into stages for readability; the order inside each stage is the
call order. Plain addresses come from the engine map; **bold** ones were
paired for the frame in Phase 1 PR 1 (evidence below; `CONFIRMED` in
`research/engine-map/src/bin/frame.rs`); *lead* marks a pairing by
position only; `?` means no PC counterpart was found. The full tree to
depth 3 is `research/engine-map/frame.tsv`, its coverage in
[LEDGER.md](LEDGER.md) ("Frame").

"Threads" below is the setting `iNumHWThreads:General` (`011c3ea4`, name
string at `0101801c`, static default 1 from `00f38360`), read through
`0043d4d0`. `main` sets it to `GetSystemInfo`'s processor count
(`0086a950`-`0086a966`) and raises 1 to 2 (`0086a977`-`0086a990`); whether
the INI can lower it afterwards is not traced. With threads > 1 `main`
creates the AI linear task threads (`0086b117`-`0086b12d`, below).

| Stage | Calls in order |
| --- | --- |
| 1. Frame start | `Main::UpdateTextures` ? (the PC's first call, `0086a830`, only returns 0), menu state queries (`Interface::IsInMenuMode` `00702360`, `IsPipboyOpening` `00709bc0`, `InDialog` `007050d0`, `FaderManager::IsFaderVisible` **`00701450`**, `IsConsoleVisible` `00703d50`, `GetCurrentRenderedMenu` `00707ad0`, `GetPipboy` `00705990`), `MemoryLevelManager::RunNonDestructiveFree` **`008782b0`** |
| 2. Player | `Main::OnIdle_UpdatePlayer` `0086f940`; PC only: the Steam API object (`006ff580`) and `SteamAPI_RunCallbacks` (`006ff860`); `Main::OnIdle_UpdateImageSpace` `0086fd90` |
| 3. Housekeeping | `Pathing::ProfilePathing` `006da7c0`, `BGSSaveLoadManager::UpdateQueuedSaves` `00851d90`, `BSConsoleCommand::PollForCommands` ?, `Main::OnIdle_FixActorBones` **`0086f190`**, `Main::OnIdle_UpdateMessageBox` ?, PC only: a requested `Sleep` of up to 20 ms (`004e1610`), `Main::OnIdle_UpdateTimer` **`0086f260`**, `Main::OnIdle_PollControls` `0086f390`, `IOManager::UpdateQueue` **`00c3dbf0`**, `LoadingMenu::SuspendBackgroundThread` `0078cfc0`, `Interface::IsInGameLoadingMenuOpen` `00705ea0`, `TES::ShowLoadingMenu` **`00457d70`**, `Main::OnIdle_ScaleLOD` **`0086efe0`**, `BSTextureManager::Update` *lead* `00b6dd00`, `Main::OnIdle_HandleMenuBackground` `0086f450`, `FaderManager::UpdateFaders` **`007011d0`**, `ScreenSplatter::Update` **`004e0110`**, `ScreenCustomSplatter::Update` **`004de600`** |
| 4. World and time | `NiParallelUpdateTaskManager::BeginUpdate` *lead* `00a81a20`, `TES::TestAllCells` `004556d0`, `Calendar::Update` **`00867a40`**, `TES::RunAnimations` **`00455640`** (threads = 1 only), `ProcessLists::RunActorScripts` **`00978550`**, `ProcessLists::UpdateRadiationList` **`009777a0`**, `ProcessLists::ChangeProcessLevelTempList` `0096eb40`, `ProcessLists::UpdateFollowerTempList` **`0096e9b0`**, `MemoryLevelManager::CheckMemoryLevel` ?, `BSTexturePalette::PurgeUnusedTextures` `00a61cd0`, `GarbageCollector::Update` `00868850`, `GarbageCollector::ClearTempEffects` `00868d10`, `BSTreeManager::Update` **`006652e0`**, `Main::OnIdle_UpdateCurrentGridCell` **`0086fbe0`** |
| 5. Interface and scene | `Main::OnIdle_DoInterfaceIdle` **`0086fd70`** (threads = 1 only; it calls `Interface::PreIdleStuff` **`007027e0`**, `Interface::Idle` **`00702810`**, `Interface::PostIdleStuff` **`00702840`**), `Main::DisplayDebugText` ?, `BGSDecalManager::GetInstance` **`0049fef0`**, `BGSDecalManager::UpdateDecals` **`0049fff0`**, `BSSceneGraph::SetCameraFOV` `00c52020`, `BSShaderManager::SetFOV` **`00b54000`**, `TES::ResetAllMultiBoundNodes` **`0045bc80`**, `ShadowSceneNode::UpdateOcclusionPlaneVisibility` `00b5ac90`, `TES::UpdateMultiBoundVisibility` **`0045b070`** |
| 6. AI threads start | threads > 1: `AILinearTaskThreadManager::SetMainRendering` **`008c80e0`**, `StartThreads` **`008c78c0`**; threads = 1: `AITaskManager::StartTasksDuringRendering` **`008ca070`**; then `Main::OnIdle_UpdateAnimationsAndEffects` **`0086fc60`**, `Main::OnIdle_DoInterfaceIdle` **`0086fd70`** again (threads > 1), `Interface::LastMinuteUpdate` `007058e0`, `PathManager::Update` **`006ebc50`**, `NavMeshRender::Update` **`006a61b0`**, `NavMeshObstacleManager::Update` **`006c3640`** and `CombatManager::Update` **`00991500`** (both threads = 1 only), `NiParallelUpdateTaskManager::EndUpdate` *lead* `00a81a80`, `Interface::UpdateSleeping` `007056f0` |
| 7. Render | `Main::RenderMenuBackground` `00871dc0`, `CheckWithinMultiBoundTask::DoAttachments` *lead* `0057ab70`, `ShadowSceneNode::ProcessAllQueuedLights` *lead* `00b60040`, `XGamerProfile::Update` ? (the PC calls `TESActorBaseData::GetAlignmentForKarma` `0047e040` there), `Main::Swap` **`0086ff70`**, `Main::PostSwapProcess` **`008705d0`** |
| 8. Threads join | threads > 1: `AILinearTaskThreadManager::WaitForThreads` **`008c7990`**; threads = 1: `AITaskManager::WaitForTasksDuringRendering` *lead* `008ca300`; `Main::UpdateNonRenderSafeAITasks` `0086f6a0`, `Main::OnIdle_PostThreadsProcess` **`00870610`**, `Interface::OpenConsole` `00703e10`, `Script::ClearOptimizations` **`005ae270`**, `ScriptLocals::ClearOptimizations` **`005a9d60`**; PC only: `00a29680`, the frame time (`0084d030`), `00950090`, `00aa7290` |

The remaining `?` rows are Xbox debug calls the PC does not make at this
level (`BSConsoleCommand::PollForCommands`, `Main::DisplayDebugText`,
`XGamerProfile::Update`) or were not found (`Main::UpdateTextures`,
`Main::OnIdle_UpdateMessageBox`, `MemoryLevelManager::CheckMemoryLevel`).
The Xbox build also brackets most stages with `BSPerformanceTimerInternal`
start/stop calls, which the PC build does not make.

### Evidence for the pairs

Each pair compares the PC function's direct calls (names from the engine
map), strings and unit with the Xbox function's.

- `FaderManager::IsFaderVisible` `00701450`: the only call between
  `InDialog` and `IsConsoleVisible`, both times, on both sides; `Main::Swap` calls
  it twice on both sides; unit `fadermanager.cpp`.
- `MemoryLevelManager::RunNonDestructiveFree` `008782b0`: calls
  `BSFaceGenManager::GetModelCache` three times and `Tile::GetMenuByClass`,
  as the Xbox function does; next to
  `MemoryLevelManager::FreeReleasedObjects` (`00878250`).
- `Main::OnIdle_FixActorBones` `0086f190`: calls
  `MobileObject::GetCurrentProcessType` and twice an empty function
  (`00483710`) where the Xbox calls the empty `RemoteLog::OnConnected` twice.
- `Main::OnIdle_UpdateTimer` `0086f260`: calls `bhkWorld::SetDeltaTime`
  (`00c66760`, docs/PHYSICS.md) and a `bstimer.cpp` function (`00aa4e40`),
  as the Xbox one calls `BSTimer::Update` and `bhkWorld::SetDeltaTime`.
- `IOManager::UpdateQueue` `00c3dbf0`: calls `BSPrecisionTimer::GetTimer`
  twice and an `iomanager.obj` function (`00c3e420`).
- `TES::ShowLoadingMenu` `00457d70`: 8 shared callees and strings; `Main::Swap`
  calls it on both sides.
- `Main::OnIdle_ScaleLOD` `0086efe0`: calls `TES::GetWorldSpace` twice and a
  `tesworldspace.cpp` function (`00586390`).
- `FaderManager::UpdateFaders` `007011d0`: the same `FaderManager.cpp`
  source-path string.
- `ScreenSplatter::Update` `004e0110` and `ScreenCustomSplatter::Update`
  `004de600`: both call `NiObjectNET::GetExtraData`; the second calls
  `004b3ab0` twice and the first once, as the Xbox custom splatter calls
  `FLerp` twice and the plain one once.
- `Calendar::Update` `00867a40`: unit `calendar.cpp`; calls `004b10d0`, a
  12-entry lookup by month, where the Xbox calls `Date::GetDaysInMonth`.
- `TES::RunAnimations` `00455640`: calls `00553820` (`tesobjectcell.cpp`)
  and `004baba0` (`gridcell.cpp`, which calls `GridCellArray::Get` and
  `00553820`), as the Xbox one calls `TESObjectCELL::RunAnimations` and
  `GridCellArray::RunAnimations` (so `00553820` and `004baba0` are those
  two).
- `ProcessLists::RunActorScripts` `00978550`: calls
  `TESObjectREFR::RunScript`.
- `ProcessLists::UpdateRadiationList` `009777a0`: `ExtraDataList::GetRadius`,
  `GetDistanceFromReference`, `Actor::GetRadiationResistanceMult`,
  `ShouldActorAvoidRadiation`, `HighProcess::AddAvoidPathingArea` and more,
  in the Xbox order.
- `ProcessLists::UpdateFollowerTempList` `0096e9b0`:
  `Actor::GetCurrentPackageTarget` twice, `Actor::AddFollower`.
- `BSTreeManager::Update` `006652e0`: `CSpeedTreeRT::SetCamera`, unit
  `bstreemanager.cpp`.
- `Main::OnIdle_UpdateCurrentGridCell` `0086fbe0`:
  `TES::UpdateCurrentGridCell`, `TESObjectREFR::GetWorldSpace`,
  `TES::GetWorldSpace`.
- `Main::OnIdle_DoInterfaceIdle` `0086fd70`: calls three wrappers
  (`007027e0`, `00702810`, `00702840`, which call
  `InterfaceManager::PreIdleStuff`, `InterfaceManager::Idle` and
  `00711ea0`), as the Xbox one calls `Interface::PreIdleStuff`, `Idle` and
  `PostIdleStuff`; it is also the first call of the combined AI thread
  function (`008c7bd0`), as on the Xbox.
- `BGSDecalManager::GetInstance` `0049fef0`: allocates (`00401000`) and
  constructs (`0049fcd0`); `UpdateDecals` `0049fff0`: calls
  `BGSDecalManager::UpdateSimpleDecals`.
- `BSShaderManager::SetFOV` `00b54000`: two sine/cosine pairs (`00eca0a0`,
  `00ec9f70`) as the Xbox `sin`/`cos`; unit `bsshadermanager.cpp`.
- `TES::ResetAllMultiBoundNodes` `0045bc80`: `BSCompoundFrustum::SetCamera`,
  `TESWorldSpace::GetTerrainManager`, `GridCellArray::Get`.
- `TES::UpdateMultiBoundVisibility` `0045b070`: `nicullingprocess.cpp`
  functions (`00a69400`, `00a694a0`), `GetTerrainManager`,
  `GridCellArray::Get`, `shadowscenenode.cpp` functions.
- `AITaskManager::StartTasksDuringRendering` `008ca070`: unit
  `aitaskmanager.cpp`, calls `MobileObjectTaskletData::RunToCompletion`
  twice, as the Xbox one does.
- `AILinearTaskThreadManager::SetMainRendering` `008c80e0`: the last call of
  `Main::Swap` on both sides. `StartThreads` `008c78c0` resets the 12 + 8
  event handles of the thread manager (`011dfa50`, returned by `00713d80`),
  sets the flag `011dfa19` and releases each thread slot's semaphore
  (`008c9fb0`, `ReleaseSemaphore`), as the Xbox `StartThreads` calls
  `ReleaseSemaphore`; `WaitForThreads` `008c7990` waits on each slot
  (`008c7490`, `WaitForSingleObject`, as on the Xbox) and clears the flag.
- `Main::OnIdle_UpdateAnimationsAndEffects` `0086fc60`:
  `ProcessLists::UpdateTempEffects` `00974420` (threads < 2) or
  `UpdateTempEffectsParallel` `009746c0` (which calls `00974420`, as on the
  Xbox), then `TES::UpdateCellAnimations` `00453550` or
  `TES::UpdateCellMainThread` `004537c0`, then
  `BSParticleSystemManager::UpdateParallel` (`00c50610`,
  `bsparticlesystemmanager.obj`).
- `TES::UpdateCellAnimations` `00453550`: wind (`00c468c0` in
  `bswindmodifier.obj`, `00c74550` in BSHavok) where the Xbox calls
  `BSWindModifier::SetWind` and `bhkWindListener::SetWind`;
  `TES::LockHavokUpdateMT` (`00453860`) before and after
  `TESObjectCELL::UpdateManagedNodes` (`00551890`) or
  `GridCellArray::UpdateManagedNodes` (`004ba9a0`); `TaskQueueInterface`
  (`0087aa90`), `focollisionlistener.cpp` (`00623640`),
  `Interface::InDialog`, `Sky::Update`, `TES::UpdateCellMainThread`,
  `processlists.cpp` (`00975080`): all in the Xbox order.
- `PathManager::Update` `006ebc50` (`pathmanager.cpp`) and
  `NavMeshRender::Update` `006a61b0` (`navmeshrender.cpp`): unit, and
  position after `PathManager::QInstance`.
- `NavMeshObstacleManager::GetInstance`/`Update` `006c0720`/`006c3640` and
  `CombatManager::Update` `00991500`: callee overlap, and the same calls in
  the combined AI thread function as on the Xbox.
- `Main::Swap` `0086ff70`: 11 shared callees and strings (`Main::RenderMenuBackground`,
  `FaderManager::RemoveFader`, `Main::KillMenuBGTexture`,
  `MTRenderingSystem::SetThreadStage`, `Main::UpdateOffscreenInterface`,
  `Interface::IsMenuIDVisible` eight times and others).
- `Main::PostSwapProcess` `008705d0`: calls
  `Main::OnIdle_UpdateProcessLists`.
- `Main::OnIdle_PostThreadsProcess` `00870610`: 4 shared callees and strings.
- `Script::ClearOptimizations` `005ae270`,
  `ScriptLocals::ClearOptimizations` `005a9d60`: the last two game calls of
  `Main::OnIdle` and of the AI thread functions, as on the Xbox.

Leads by position only: `00b6dd00` (`BSShader`, next to
`BSTextureManager::ReturnRenderedTexture` `00b6da10`); `00a81a20` and
`00a81a80` (`NiMain`, each behind a test, where `BeginUpdate` and
`EndUpdate` stand); `0057ab70` and `00b60040` (after
`RenderMenuBackground`; `00b60040` calls one function, as
`ProcessAllQueuedLights` calls `ProcessQueuedLights`); `008ca300` (the
only call between the threads-join branch and `UpdateNonRenderSafeAITasks`).

## The Havok step

`bhkWorld::Update` (`00c6ae70`) is slot `+0xc4` of the `bhkWorld` vtable
(`010c40b4`, slot at `010c4178`) and of the `bhkWorldM` vtable (`010c69f4`,
slot at `010c6ab8`). Three code sites call slot `+0xc4` on a world:

- `00554780` (`tesobjectcell.cpp`), on the exterior world in the global
  `011ca0d8`. `TESObjectCELL::InitStatics` (`00541b80`) stores there the
  world that `00554010` creates with `00c99ff0`, which writes the
  `bhkWorldM` vtable `010c69f4` (`00c9a00c`).
- `TESObjectCELL::UpdateManagedNodes` (`00551890`, call at `005518df`), on
  the cell's own world from `004543c0` (cell byte `+0x24` bit 0 set:
  `0041b9a0` on the cell's extra data at `+0x28`; clear: `00451010`, the
  global `011ca0d8`), only when that bit is set and `00450ff0` holds
  (`00450fd0` returns 6).
- `00554010` itself, once, right after creating the world (`0055450b`).

The per-frame path is `TES::UpdateCellAnimations` (`00453550`; `this` is
`[011dea10]` at the call from `0086fc60`). It adds the frame time to `011c3c08` and, between
`TES::LockHavokUpdateMT(1)` and `(0)` (`00453860`, a critical section only
when threads ≠ 1), calls either `TESObjectCELL::UpdateManagedNodes`
(`00551890`) on the cell at `this+0x34` when there is one, or
`GridCellArray::UpdateManagedNodes` (`004ba9a0`, on `this+0x8`). The
latter steps the exterior world first (`00554780`) and then runs
`00551890` on every loaded grid cell (`GridCellArray::Get`); those cells
have bit 0 clear, so the exterior world is stepped once per frame.

`TES::UpdateCellAnimations` has three callers:

- `Main::OnIdle_UpdateAnimationsAndEffects` (`0086fc60`, call at
  `0086fd08`) when threads = 1 and the menu flag `011dea2b` (set at the start of
  the frame from `IsInMenuMode`/`IsPipboyOpening`) is clear (otherwise it calls
  `TES::UpdateCellMainThread` `004537c0`);
- the combined AI linear task thread function `008c7bd0` (`008c7d03`);
- AI linear task thread 2's function `008c7f50` (`008c803d`).

So with threads > 1, which `main` makes the normal case on PC, the Havok
step runs on an AI linear task thread between `StartThreads` (stage 6) and
`WaitForThreads` (stage 8), after that thread's actor updates.

## The actor updates

`AILinearTaskThreadManager::CreateThreads` (`008c7290`, called from `main`
at `0086b12d` when threads > 1) creates one thread on `008c7bd0` ("AI Linear
Task Thread", string `0108561c`) when threads ≤ 2, else two, on `008c7da0`
("AI Linear Task Thread 1", `0108564c`) and `008c7f50` ("AI Linear Task
Thread 2", `01085634`). Their call lists line up with the Xbox
`AILinearTaskThreadManager::AIThreadCombined`, `AIThread1TaskFunc` and
`AIThread2TaskFunc`. `008c7bd0` calls, in order (Xbox PDB names from that
alignment; leads unless bold in the table above):

`Main::OnIdle_DoInterfaceIdle` **`0086fd70`**,
`TaskQueueInterface::ThreadBeginInput` (`004537b0`, `0087a6b0`),
`CombatManager::Update` **`00991500`**,
`ProcessLists::RunActorAnimationUpdates` `0096cca0`,
`ProcessLists::ParallelActorAnimationMovementUpdates` `0096cda0`,
`TES::RunAnimations` **`00455640`**,
`ProcessLists::RunActorMagicUpdates` `009784c0`,
`CombatManager::UpdateCombatants` `00991dc0`,
`ProcessLists::UpdateHighListPackages` `0096bcd0`,
`ProcessLists::UpdatePlayerFollowers` `0096d520`,
`ProcessLists::RunDetectionForAllActors` `0096c330`,
`ProcessLists::UpdateActorsMovement` `0096db30`,
`ProcessLists::RunActorUpdates` `0096c7c0`,
`ProcessLists::RunActorRagdollAnimationUpdates` `0096cb50`,
`BGSDestructibleObjectForm::UpdateDestructibleObjects` `004772f0`,
`TES::UpdateCellAnimations` **`00453550`** (the Havok step),
`NavMeshObstacleManager::GetInstance`/`Update` **`006c0720`**/**`006c3640`**,
`TaskQueueInterface::ThreadEndInput` (`004537b0`, `0087a6d0`),
`Script::ClearOptimizations` **`005ae270`**,
`ScriptLocals::ClearOptimizations` **`005a9d60`**. Between the groups it
signals and waits on thread stages (`00713d80` with `008c79e0`, `008c7a70`
or `008c7d80`). `008c7f50` runs the detection, movement, ragdoll,
destructible, cell-animation and script-clearing calls of that list;
`008c7da0` calls the combat, animation, magic, package, follower and
actor-update ones (callers of each from `get_xrefs_to`). `ProcessLists::RunActorUpdates` (`0096c7c0`) has no
caller besides `008c7bd0` and `008c7da0`.

The threads = 1 path (`008ca070`, the AI task queue with
`MobileObjectTaskletData::RunToCompletion`) is not followed further here.

## Coverage today

[LEDGER.md](LEDGER.md) ("Frame") counts the rows of `frame.tsv` by status
whenever it is regenerated. On 2026-10-09, after the rolling translation
batches up to #81, of the 143 depth-1 call sites 22 are `translated`, 37
`traced`, 5 `platform` and 79 `open`.

## PR 2 result: `world::frame`

`crates/world/src/frame.rs` (2026-10-09) is the control flow of
`0086e650`, translated (ADR-0003 marker): `STEPS` lists its 143 calls in
the exe's order (a test loads `frame.tsv` and compares address and name
row by row), each with a `Stage` (the table above), a `Gate` and a
`Wiring`. `steps_run(&FrameState)` says which run for given inputs. The
disassembly and the decompiler agree on every branch (the decompiler only
drops two blocks behind a local that is always 0, `0086ebd7`, `0086ec00`).

The gates (branch addresses in `0086e650`; each has a test):

| Gate | Steps | Branches |
| --- | --- | --- |
| Tab and Alt both held: return after step 1 | 2-143 | `0086e682`, `0086e69a` |
| Menu mode (`[011dea2b]` = `IsInMenuMode` or `IsPipboyOpening`; V.A.T.S.'s menu, sleep/wait, dialogue and the pause menu count) or the free camera's frozen world (`Main` +7, set only for `TFC` 1, 2 or 5: `005bc260` → `00961e30` → `00961f50`) stops the screen splatters, the cell tests, the sky update, `Calendar::Update` and (one thread) `TES::RunAnimations` | 46-47, 50-58 | `0086e918`, `0086e923`, `0086e946`, `0086e955`, `0086e9dc` |
| The radiation, process-level and follower lists, and the AI work's start and join, run when menu mode is clear or the fader is visible (`[011dea2d]`), the console is hidden (`[011dea2e]`) and the world is not frozen | 62-68, 98-102, 128-131 | `0086ea0c`-`0086ea84`, `0086ec1d`-`0086ec74`, `0086ee0c`-`0086ee45` |
| With threads > 1 and no AI work this frame, the main thread runs the interface idle, and `LastMinuteUpdate` in menu mode | 105-107 | `0086ecad`, `0086ecb5`, `0086ecc9` |
| One thread: the interface idle before the AI work, the obstacle manager (with `bUseObstacleAvoidance`, `011d73e4`) and `CombatManager::Update` on the main thread | 80, 113-116 | `0086eb31`, `0086ed00`, `0086ed11` |
| V.A.T.S. playback (manager mode 4, `[011f2250]`+8) skips `BSSceneGraph::SetCameraFOV` | 87-89 | `0086eb6d` |
| Loading: the start menu with flag 0x10000 skips the block; up without flag 1 it suspends the loading menu's thread; otherwise an open in-game loading menu is shown | 36-39 | `0086e8a6`, `0086e8b2`, `0086e8c5` |
| Sleeping or waiting: top menu 1012 asks `UpdateSleeping`; when true the menu background is redrawn | 120-121 | `0086ed75`, `0086ed81` |
| `PathManager::Update` stops only with the frozen world | 108-109 | `0086ecd9` |
| The memory free: interface mode 3 and no rendered menu other than the Pip-Boy | 13 | `0086e74e`, `0086e756` |
| The game-mode frame counter (`0086ef40`) | 31 | `0086e87a` |
| The heap sort (`00aa7290`): menu mode, the view key held, or 45 s gathered | 142-143 | `0086eeb9`, `0086eec2`, `0086eed4`, `0086eee7` |
| Requests and short-circuits: pathing profile `[011deefc]`, texture purge `[011f4461]`, display mode change `[011c6fbb]`, console open `[011dea2f]`, the parallel update's begin and end, the menu queries | 4, 9-10, 19, 23-24, 49, 51-52, 73, 118, 127, 135-136 | `0086e6c0`, `0086e714`, `0086e7a8`, `0086e7fd`, `0086e936`, `0086e96b`, `0086e97d`, `0086ead8`, `0086ed64`, `0086edfe`, `0086ee79` |

There is no separate "paused" test: the pause menu is a menu-mode menu.
The mode-dependent arguments are modelled too: `Main::OnIdle_UpdateImageSpace`
gets "the world runs", `BSTreeManager::Update` its negation and
`CombatManager::Update` "menu mode, the fader or the frozen world".

Steps wired to existing nv-rs code (checked by reading it):

- **Is the step**: 44 `Main::OnIdle_HandleMenuBackground` (`0086f450`) →
  `world::menu_background::Background::update`, run by the viewer's
  `menu_background::update`.
- **Partly there** (still open): 32 `Main::OnIdle_UpdateTimer` →
  `physics::havok::Clock` (the frame timer and Havok's delta only); 45
  `FaderManager::UpdateFaders` → `world::living::sleep::Fade` and the
  fade to black in the viewer's `game_menus` (two faders); 56
  `Calendar::Update` → `GameState::advance_clock` (its calendar is a
  labelled guess); 77 `BSTreeManager::Update` → `speedtree::wind` (its
  `006658b0` only); 78 `Main::OnIdle_UpdateCurrentGridCell` →
  `world::ref_scripts`' grid-move test (`00452580`).
- **Open**: the other 137.

Left for PR 3: the viewer's per-frame systems still run in their own
`.chain()`/`.before`/`.after` order, with menu mode approximated per
system (`menus::Menus::is_open`, the dialogue test, Bevy's virtual clock
paused in V.A.T.S.). PR 3 makes `Stage`/`FrameStep` system sets, puts each
system in the set of the step it belongs to, and runs each set under its
`Gate`, evaluated from one `FrameState` filled once per frame. Inputs with
no viewer counterpart yet (fader 1, the frozen world, the interface mode,
the thread count) need a source or a fixed value.

## PR sequence

Each PR names one next action, regenerates the ledger and passes the
acceptance routes, as in B1.

1. **Frame map.** Extend `research/engine-map` with a `frame` step that
   writes the `Main::OnIdle` tree (depth 3, in call order, PC and Xbox
   addresses, names, ledger status) as a committed table, and show its
   coverage in LEDGER.md. Pair the remaining `?` rows by aligning the PC
   and Xbox call lists (the two are close to identical here). Trace where
   the Havok step and the actor updates are called from (vtable calls and
   the AI task lists). No Rust behaviour changes. *Done*: the ledger
   computes the status instead of the table storing it (it would go
   stale); 38 depth-1 functions (42 call sites) newly paired, the rest
   listed above.
2. **`world::frame`.** Translate `0086e650`'s control flow: which stages run
   in menu mode, while loading, with the console or Pip-Boy open, in
   V.A.T.S. and while sleeping, as an ordered `FrameStep` list. Each step
   either calls our existing system or is marked `open` with its address.
   Tests: the step order equals the traced order; the mode gates match the
   branches of `0086e650`. *Done*: "PR 2 result" above.
3. **Bevy order from the frame.** The viewer's per-frame systems are ordered
   by `FrameStep` system sets instead of their own `.before`/`.after`
   chains; remove the chains it replaces. This is where "runs in the wrong
   order" bugs get fixed; acceptance routes must pass unchanged or with
   explained differences.
4. **Player stage.** `Main::OnIdle_UpdatePlayer` (`0086f940`) and the call
   order inside `PlayerCharacter::Update` (`0093e860`, 22.6 KB, the largest
   game function on this path): order only, calling our systems.
5. **World and time stage.** `TES::TestAllCells`, `Calendar::Update`,
   `Main::OnIdle_UpdateCurrentGridCell` (cell attach and detach), the
   process-level temp lists, `GarbageCollector::Update`.
6. **AI task stage.** The tasks `AITaskManager` starts in stage 6 and joins
   in stage 8: actor process updates, animation, the Havok step. Our single
   thread runs them in the game's order; the thread split itself is
   `platform`.
7. **Interface and render stage.** `Interface::Idle` and its pre/post
   steps, `LastMinuteUpdate`, the menu background, `Main::Swap`'s
   pre/post-swap work (the renderer itself stays Bevy).

After PR 7 every call in `Main::OnIdle` to depth 2 is either wired to a
system or listed as `open` in the ledger, which is Phase 1's gate. Phase 2
(the state model) can start in parallel after PR 2.
