# Native config, form and theft script commands

Source inspected: Agua Fria 1.0.5.3's 46 readable script blocks, compiled-line
framing, master list, loose scripts, INI and MCM JSON. No mod assets or extracted
scripts are included in this repository. This is a local implementation candidate,
not a claim that the whole mod or menu has passed gameplay testing. The commands
are reusable extender APIs; the mod supplies its own gameplay scripts.

## Implemented

- JohnnyGuitar `EditorIDToFormID` (3128): the original game lookup, returning form
  bits, including the missing-name and console paths. Its existing editor-ID row
  supplies the expanded lookup when enabled.
- JohnnyGuitar `Clamp` (3155): three float arguments and the original ordered
  comparisons, including reversed bounds and NaN behavior.
- JohnnyGuitar `GetFactionFlags` (31e0): required form argument, faction type
  check, unsigned flag word and original console-output behavior. This additional
  API is used by minigame-bypass scripts, not by the six direct Agua Fria calls.
- JohnnyGuitar `SendStealingAlarm` (3199): actor/ref checks, ownership, optional
  inventory-stack owner matching and the game's own theft alarm (008bfa40).
- ShowOff `GetINIFloatOrDefault_Cached` / `GetINIFloat_Cached` (3d29): expression
  arguments, defaults, the config directory, caller-derived name, SimpleIni values
  and the shared case-insensitive file cache.
- ShowOff `ClearFileCacheShowOff` / `CloseFileSO` (3d2c): INI-cache invalidation
  (mode 0). There is no JSON-producing API yet, hence no JSON cache to clear.
- Stewie `StewieGetValue` / `StewVal` (2139): expression argument and ambiguous
  numeric/string results, key/section split, prefix selector and missing-value
  behavior. The settings queried are nv-rs's loaded settings, including its
  documented VNV defaults. This does not undo the port's deliberate default policy.

All three plugins remain unlisted as complete installations. No version checks
are bypassed. JohnnyGuitar's commands work independently of its editor-ID row;
Stewie's command is installed when the existing Stewie row is enabled. ShowOff's
individual commands are installed after Stewie, before UIO. The existing
SimpleIni parser is shared through engine-core; Stewie's old module re-exports
it so its callers and parser tests retain their behavior.

## Provenance and checks

JG source: 59049a0c844111dab69aed372eb8e9c30f5cc5ae, functions fn_math,
fn_utility, fn_form and fn_gameplay, plus JohnnyCommands and their parameter tables.
ShowOff source: f45d069d5357968f01246d87bbdcdee4b030dc90, SO_fn_Files,
NVSEParamInfoList and SimpleIni. The case-insensitive map keys follow its
MapKey/StrCompare. Stewie's released 10.05 DLL was inspected: registration
10081a16..10081bdb (eighth command after base 2132, return type 5), command
record 100fb16c, execute 10062ff0..10063289 and type-selector jump table
1006328c. Notices give the DLL hash; no binary data is committed.

Focused tests exercise the real registered command handlers and argument readers:
float rounding and branch order, raw form results, theft ownership and optional
inventory matching, expression INI reads/defaults/clear and expression numeric and
string Stewie results. Existing JG and Stewie tests also run; the latter cover
the unchanged SimpleIni rules and loaded settings. Startup checks and play build
are run headlessly. Detailed local results belong in the contributor's build log.

## Still needed for the full mod

MCM Extender's JSON/INI writing, maps, timers and helpers are separate work;
classic MCM support alone does not provide that framework. The direct plus menu
inventory totals 26 canonical APIs; only the six direct ones above are covered
here. No truthful full-plugin registration is possible yet.

Local gameplay has exercised Agua Fria in a combined build. That is not an
original Windows runtime comparison or proof of the full mod. The complete
nested-expression inventory, full 97-option MCM, and formal new/load/save
validation remain open. MCM Extender is not supplied by these APIs. Full
workspace tests and Windows/Linux runtime validation remain outstanding.

Next: validate the complete mod's script and menu paths separately from these
registered-command tests. No plugin identity/version is spoofed, and no mod
assets, installation profiles or rewritten mod scripts are shipped here.
