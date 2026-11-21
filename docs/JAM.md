# JohnnyGuitar form, Pip-Boy and render-event APIs

Inspected JAM 4.6's supplied plugin, embedded source, compiled scripts and UI
files. No mod assets or extracted scripts are part of this repository. This
slice adds six reusable JohnnyGuitar APIs directly used by its loot menu
and weapon wheel; it does not establish that all of JAM works.

## Commands and events

- `GetBaseEffectAV` (3103), `GetBaseEffectArchetype` (3104): original effect-setting
  class check, associated-value predicate and minus-one defaults.
- `TogglePipBoy` (3109): original allowed menu IDs and manager state transitions,
  including success in intermediate states without opening or closing again.
- `GetCalculatedWeaponDPS` (310a): uses the game's formula (00645380) and the
  player's actor-value owner, with JohnnyGuitar's world/inventory condition and
  ammo selection. The process's temporarily cleared weapon is restored even if
  the formula stops the run. This does not introduce a second DPS formula.
- `IsCrimeOrEnemy` (31a5): activation ownership, teammate/combat checks and the
  player's perceived hostile targets. The hostile list is searched for any
  matching hostile entry, rather than replacing it with the game's first-match
  helper.
- `SetJohnnyOnRenderUpdateEventHandler` / `SetOnRenderUpdateEventHandler` (319f):
  queues registration/removal through JohnnyGuitar's main-loop update, retains
  lambda variables, and runs zero-argument function callbacks at the original
  game-render (870244), rendered-menu (8702a9) and game-pre (943748) call sites.
  The ignored third flags argument stays ignored; the fourth selects event
  buckets. Render registrations survive load/new-game messages, as in the
  original. These are script callbacks; messages to other native DLL plugins
  are not emitted because those plugins do not load in this port.

The command/event listener is independent of the editor-ID restoration row.
JohnnyGuitar remains unlisted as a complete installation: no version gate is
bypassed, and missing commands retain their existing unsupported behavior.

## Provenance

JohnnyGuitarNVSE at `59049a0c844111dab69aed372eb8e9c30f5cc5ae`:
`JG/functions/fn_form.cpp`, `fn_gameplay.cpp`, `fn_event.cpp`, their parameter
definitions and `internal/JG/JohnnyCommands.cpp`; `events/JohnnyEvents.cpp`,
`EventFramework.cpp` and `LambdaVariableContext.cpp`; associated game-class
declarations and helpers. The three hook sites were checked against the owned
FalloutNV.exe 1.4.0.525. Game calls and layouts were read against the existing
engine translations. No executable bytes or decompiler exports are included.

## Validation boundary

Focused synthetic tests cover the registered command records/argument readers,
ownership and hostile-target cases, allowed Pip-Boy modes, condition/ammo/DPS
argument selection, state restoration, and callback ordering and lifetime.
Translation tests exercise the render seams before the original calls and
preserve the original dialog-check result. These are headless tests, not
Windows-runtime comparisons or proof of gameplay compatibility.

The initial synthetic audit decoded 43 of 52 compiled-script blocks. With this
slice it decodes 45, including the previously failing loot inventory handler and
weapon-wheel menu handler. Seven remain unresolved: hold breath, visual
objectives, two optional yUI integration functions, and three bullet-time
handlers. Fixture limitations must be distinguished from missing engine
behavior before treating those failures as engine defects.

The remaining JohnnyGuitar commands found in the wider JAM source inventory are
`DeactivateAllHighlightsAlt`, `GetActorValueAlt`, `GetCustomMapMarker`,
`HighlightBodyPartAlt`, `ToggleDisableSaves` and `WorldToScreen`. A selective
loot-menu/wheel configuration still needs runtime testing; the full assortment
is not supported by this slice alone. UI assets, feature configuration,
save/load and gameplay remain unverified. A successful parse proves command
recognition, not handler execution or correct UI behavior.

## Submission scope

This builds on the native config/form command-registration slice. The API and
handler modules are named for their capabilities (`gameplay_commands`,
`weapon_dps`, `render`), independently of the consuming mod. Command opcodes,
parameter records and the three original hook sites are preserved. No JAM
scripts, UI files, INI presets, altered startup globals or other mod assets are
included. No plugin installation gate is bypassed.

The selective loot/wheel configuration is a private test setup. User interaction,
UI integration, save/load and original Windows-runtime comparison remain open;
these command and seam tests do not establish complete JAM compatibility.

After capability-module naming cleanup and the current-main rebase, the local
checks passed: 39 JohnnyGuitar tests (one existing ignored), two ShowOff tests,
31 Stewie tests, 12 frame-end tests, the pre-dialog seam regression, two existing
player frame-model tests, and 47 extender-table tests. Strict affected-crate
Clippy, explicit ARM test compilation, all 86 repository-hygiene tests, formatting
and optimized play build passed. A stale shared target cache initially supplied
a hook table from another worktree; rebuilding engine-core in the default and
ARM targets resolved that build-only mismatch without changing product code.
These are local checks, not Windows/Linux gameplay or complete workspace proof.
