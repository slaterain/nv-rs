# M1 custom save persistence

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
