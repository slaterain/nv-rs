# Phase 1: the frame skeleton (proposal)

Drafted 2026-10-09 at the end of Phase 0 ([ENGINE_PORT_PLAN.md](ENGINE_PORT_PLAN.md),
[LEDGER.md](LEDGER.md)). Nothing here is implemented yet. Names are from the
Xbox 360 prototype (Xbox PDB, ADR-0002), PC addresses from
`research/engine-map/engine_map.tsv`.

## The entry point

The frame is `Main::OnIdle` (Xbox PDB): PC `0086e650`, 2,272 bytes, called
once per frame from `main` (`0086a850`, matched by its strings). It was not
named by the matcher; it is the only PC function that calls both
`Main::OnIdle_UpdatePlayer` (`0086f940`) and `Main::OnIdle_PollControls`
(`0086f390`), and its 143 call sites line up with the Xbox `Main::OnIdle`
(`8269f3c8`, 143 calls without the PowerPC register save helpers) apart
from profiling timers and debug-only calls. The PC version polls the
keyboard directly (`GetAsyncKeyState`, six calls).

## Its order (Xbox PDB names, PC addresses where matched)

Grouped into stages for readability; the order inside each stage is the
call order. `?` means not matched yet; Phase 1's first PR fills these from
the PC call list.

| Stage | Calls in order |
| --- | --- |
| 1. Frame start | `Main::UpdateTextures` ?, menu state queries (`Interface::IsInMenuMode` `00702360`, `IsPipboyOpening` `00709bc0`, `InDialog` `007050d0`, `FaderManager::IsFaderVisible` `00701450`, `IsConsoleVisible` `00703d50`, `GetCurrentRenderedMenu` `00707ad0`, `GetPipboy` `00705990`), `MemoryLevelManager::RunNonDestructiveFree` ? |
| 2. Player | `Main::OnIdle_UpdatePlayer` `0086f940`, `Main::OnIdle_UpdateImageSpace` `0086fd90` |
| 3. Housekeeping | `Pathing::ProfilePathing` `006da7c0`, `BGSSaveLoadManager::UpdateQueuedSaves` `00851d90`, `BSConsoleCommand::PollForCommands`, `Main::OnIdle_FixActorBones`, `Main::OnIdle_UpdateMessageBox`, `Main::OnIdle_UpdateTimer`, `Main::OnIdle_PollControls` `0086f390`, `IOManager::UpdateQueue`, `LoadingMenu::SuspendBackgroundThread` `0078cfc0`, `TES::ShowLoadingMenu`, `Main::OnIdle_ScaleLOD`, `BSTextureManager::Update`, `Main::OnIdle_HandleMenuBackground` `0086f450`, `FaderManager::UpdateFaders` `007011d0`, `ScreenSplatter::Update`, `ScreenCustomSplatter::Update` |
| 4. World and time | `NiParallelUpdateTaskManager::BeginUpdate`, `TES::TestAllCells` `004556d0`, `Calendar::Update` `00867a40`, `TES::RunAnimations`, `ProcessLists::RunActorScripts`, `ProcessLists::UpdateRadiationList`, `ProcessLists::ChangeProcessLevelTempList` `0096eb40`, `ProcessLists::UpdateFollowerTempList`, `MemoryLevelManager::CheckMemoryLevel`, `BSTexturePalette::PurgeUnusedTextures` `00a61cd0`, `GarbageCollector::Update` `00868850`, `GarbageCollector::ClearTempEffects` `00868d10`, `BSTreeManager::Update` `006652e0`, `Main::OnIdle_UpdateCurrentGridCell` |
| 5. Interface and scene | `Interface::PreIdleStuff`, `Interface::Idle`, `Interface::PostIdleStuff`, `Main::DisplayDebugText`, `BGSDecalManager::UpdateDecals`, `BSSceneGraph::SetCameraFOV` `00c52020`, `BSShaderManager::SetFOV`, `TES::ResetAllMultiBoundNodes`, `ShadowSceneNode::UpdateOcclusionPlaneVisibility` `00b5ac90`, `TES::UpdateMultiBoundVisibility` |
| 6. AI threads start | `AILinearTaskThreadManager::SetMainRendering`, `StartThreads`, `AITaskManager::StartTasksDuringRendering`, `Main::OnIdle_UpdateAnimationsAndEffects`, `Interface::PreIdleStuff`/`Idle`/`PostIdleStuff` again, `Interface::LastMinuteUpdate` `007058e0`, `PathManager::Update`, `NavMeshRender::Update`, `NavMeshObstacleManager::Update`, `CombatManager::Update`, `NiParallelUpdateTaskManager::EndUpdate`, `Interface::UpdateSleeping` `007056f0` |
| 7. Render | `Main::RenderMenuBackground` `00871dc0`, `CheckWithinMultiBoundTask::DoAttachments`, `ShadowSceneNode::ProcessAllQueuedLights`, `XGamerProfile::Update`, `Main::Swap`, `Main::PostSwapProcess` |
| 8. Threads join | `AILinearTaskThreadManager::WaitForThreads`, `AITaskManager::WaitForTasksDuringRendering`, `Main::UpdateNonRenderSafeAITasks` `0086f6a0`, `Main::OnIdle_PostThreadsProcess`, `Interface::OpenConsole` `00703e10`, `Script::ClearOptimizations`, `ScriptLocals::ClearOptimizations` |

Not visible at this level: the Havok step. `bhkWorld::Update` (`00c6ae70`)
is called only through its vtable; where the frame reaches it (the AI task
threads, `TES::RunAnimations` or the player update) is the first thing
Phase 1 traces. The same holds for actor updates, which run in the AI tasks
started in stage 6 and joined in stage 8.

Today the ledger marks 14 of the 143 PC call sites `traced` (the
`Interface` queries, the menu background, the fader update, the calendar and
others), none `translated`; everything else in this list is `open` or
`platform`.

## PR sequence

Each PR names one next action, regenerates the ledger and passes the
acceptance routes, as in B1.

1. **Frame map.** Extend `research/engine-map` with a `frame` step that
   writes the `Main::OnIdle` tree (depth 3, in call order, PC and Xbox
   addresses, names, ledger status) as a committed table, and show its
   coverage in LEDGER.md. Pair the remaining `?` rows by aligning the PC
   and Xbox call lists (the two are close to identical here). Trace where
   the Havok step and the actor updates are called from (vtable calls and
   the AI task lists). No Rust behaviour changes.
2. **`world::frame`.** Translate `0086e650`'s control flow: which stages run
   in menu mode, while loading, with the console or Pip-Boy open, in
   V.A.T.S. and while sleeping, as an ordered `FrameStep` list. Each step
   either calls our existing system or is marked `open` with its address.
   Tests: the step order equals the traced order; the mode gates match the
   branches of `0086e650`.
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
