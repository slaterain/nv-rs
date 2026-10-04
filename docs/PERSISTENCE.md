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
No play publication yet.
This does not restore mid-animation state or establish full opening acceptance.
