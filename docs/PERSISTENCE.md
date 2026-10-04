# M1 custom save persistence

## Camera continuation, 2026-10-04

Active branch `codex/m1-camera-persistence`, based on appearance PR #6.
The next bounded batch captures first-person special-idle clocks, frozen
blend poses, loop counters, package cache, pending requests, hand-follow and
view pitch in the custom save. It does not infer an animation from quest stage
or claim native `.fos` compatibility. F9 preflights the skeleton and sequence
assets before committing the running world.

Changed files: animation.rs and animation/snapshot.rs; save.rs and
save/camera.rs; scripting.rs; viewer player_idle.rs, sitting.rs, scripts.rs
and actors.rs. Implementation and local checks are complete; no camera
build is published yet. All test viewer processes have exited.
Private design and eventual checks: nv-re/work/m1-overnight-2026-10-04/
camera-persistence. PR #6 checks are running separately; play/app remains
the verified PR #5 binary. Exact next action: submit the camera batch through
a checked PR after appearance PR #6, then publish its verified binary.

Implemented camera state is a versioned, bounded hex payload on one `camera`
line. Saves without that line still load; older viewers will reject the new
line. It preserves full active animation state, including frozen blend
sources, live group data and pending package release. Sequence paths resolve
through the same Seats cache used for new requests, preserving Arc identity.
Ordered bone names/parents and sequence timing/track/group signatures reject
incompatible assets before world replacement. These are structural checks,
not file hashes: restoration requires unchanged assets and load order;
same-path transform-only edits are not detected. No KF bytes are embedded.
NPC pending requests wait until the destination scene is installed, but
active NPC animations and dialogue continuation remain outside this batch.

Generated checks pass: 925 root tests/doc tests and 85 viewer tests; both
workspaces' clippy, formatting and release builds. A private installed-
data harness round-tripped all four VCG01 player camera clips at 0, 0.05,
0.25, 1 and 4 seconds, then compared every pose for 240 continuation frames
and 30 release frames per case: all 20 passed exactly. This proves custom
snapshot continuation against our animation engine with actual assets, not
original-game visual equivalence. Native request/transition provenance stays
with `world::animation` and [OPENING.md](OPENING.md).

Live installed-data F5/F9 preserved `SpecialIdle_NVCG01PlayerBedsit.kf`,
package001055C3, physical feet/heading and pitch. A second save confirmed an
advancing animation clock (2.2223833 to 5.2226577 in its loop). A fresh-process
F9 also restored that clip from disk and saved it at clock2.5011308. Images
were inspected and both startup logs contained only Vulkan present-mode
warnings. The name menu consumed F5/F9 until closed, so those attempts were
not counted as loads. Dialogue continuation remains incomplete after restart;
camera restoration does not establish the full opening-save acceptance gate.
Private logs/saves and decoded summaries: camera-persistence/live/.
Verified binary SHA256:
`39A4219C0B9B3D6562E1673E70AB5112AC4E83FD3B5D62FF378DEA5FCC9BCBC8`.

## Failed writes and failed destination loads, 2026-10-04

This batch concerns nv-rs's own text saves, not original `.fos` compatibility
or a newly inferred native gameplay rule. Source inspection found two failure
paths: direct writes could truncate the previous save before an I/O failure,
and F9 replaced GameState/trigger/camera resources before loading the saved
cell or worldspace. A destination failure therefore mixed old geometry with
new quest state.

`world::save::save_to_path` now writes a uniquely created sibling temporary
file, syncs and closes it, then renames it over the destination. Failed writes
or renames remove only the temporary file. Quicksave and scripted autosave,
forced-save and system-save requests use this writer. Existing save syntax
is unchanged. This is not a promise of power-loss durability on every filesystem.

The viewer prepares the saved interior scene or world grid before replacing
GameState and clearing trigger/camera state. Parse or destination errors leave
those resources and pending destinations alone. A successful load replaces
the pending destination so a stale transition cannot win later. Exterior
chunk streaming still happens afterward and is not rolled back by this work.

Generated regressions exercise partial-write failure, replacement failure,
Windows readers that prevent replacement, successful overwrite/reload,
malformed saves, nonexistent cells/worldspaces, and successful interior
placement. The existing same-cell tester-trigger regression remains relevant.

Active branch: `codex/m1-opening-overnight`. Changed runtime files:
`crates/world/src/save.rs`, `viewer/src/scripts.rs`. Check logs are private in
`%USERPROFILE%/nv-re/work/m1-overnight-2026-10-04/persistence-*`.
Checks: 913 core tests/doc tests and 84 viewer tests passed, including the
Windows file-lock regression. Both workspaces passed clippy, formatting and
release builds. An isolated release run loaded the installed
GSDocMitchellHouse (417 objects, 406 meshes), executed `ForceSave`, wrote the
custom save and exited after its inspected screenshot. Startup diagnostics
showed only Vulkan present-mode warnings. Private runtime files are in
`persistence-smoke/`; user play saves and reports were not touched.
No original-game comparison is claimed for these custom-format I/O changes.
Merged through [PR #5](https://github.com/slaterain/nv-rs/pull/5) after both
GitHub checks passed, commit `637b28d28dbe27718d0e2d749f6136798e341b7c`.
Installed the verified build in `nv-rs-play/app`; SHA256
`2A42932E91AD206184E1D700120C93188CD4D2BA572E9A2AD4F684B918EAAE7B`.
The merged tree was verified identical to tested source `94983c0`. The
prebuilt executable was retained privately while the next batch built;
publication used that exact artifact and the existing helper. The current
appearance build was restored afterward. Saves/reports were preserved.
This does not restore mid-animation state or establish full opening acceptance.

Additional live F5/F9 check: the isolated `persistence-live/` run saved,
rejected a deliberately nonexistent saved cell, then saved the unchanged
player position/heading and quest state (the injected stage60 did not leak).
A subsequent valid F9 load succeeded and restored Doc's seated state. The
test process exited; `RESULT.txt`, before/after saves and logs stay private.

## Native save gating investigation

The scripted Autosave/ForceSave opcode handlers (`005c5560`, `005c5590`)
check their INI permission settings; these handlers contain no package,
dialogue or animation condition. This does not establish manual F5 routing.
`0070c4a0` has quicksave/quickload refusal feedback via `0070edf0` and the
current StartMenu's+0x1a8 bit1 (`004a4080`); the quicksave string says the
game is paused. Exact acceptance during each opening phase remains unresolved.

An original-game launch through Steam reached the main menu, but computer-use
click and Return did not drive its DirectInput. The test was closed with
Alt-F4; no save was loaded or overwritten. Do not invent a phase-specific
save prohibition from this failed experiment. Native evidence and executable
hash: `nv-re/work/m1-overnight-2026-10-04/save-gating/RESULT.md`.

## CI fixture collision found during camera PR #7

The first core CI run failed before package-action assertions: parallel
tests used the same tag/PID temporary directory, so another fixture could
delete or truncate its generated FalloutNV.esm. `TempData` now exclusively
creates unique tag/PID/counter directories; every fixture builder uses that
allocator. A deterministic same-tag regression verifies independent contents
and that dropping one fixture preserves the other. No check was disabled.
The amended root suite passes 926 tests/doc tests; viewer remains 85.
Both workspaces' checks were rerun. This changes test data only; the rebuilt
release also passed an isolated installed-data opening smoke (situp/bedsit,
Doc speech, inspected screenshot, normal exit; only known Vulkan warnings).
Its SHA256 is `1219AD13BC6190B208C76C84B0F29D856A10324DA1AE6C0D1206A4F4FB277560`.
Private logs/image: `camera-persistence/fixture-smoke` under the overnight
research directory. CI must pass the amended head before camera publication.
