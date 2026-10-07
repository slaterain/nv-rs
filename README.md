# nv-rs

[![Rust](https://img.shields.io/badge/Rust-2021_edition-orange.svg)](https://www.rust-lang.org/)
[![Engine](https://img.shields.io/badge/Engine-Bevy_0.16-blue.svg)](https://bevyengine.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE-APACHE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS-purple.svg)](docs/PLAYTESTING.md)
[![Status](https://img.shields.io/badge/Status-experimental-lightgrey.svg)](docs/MILESTONES.md)

A from-scratch reimplementation of **Fallout: New Vegas** in **Rust** with the
**Bevy** engine. It runs the game from the `Data` folder of an installation you
own and aims to behave exactly like the original (1:1).

> [!IMPORTANT]
> nv-rs is an independent project, not affiliated with or endorsed by
> Bethesda Softworks, ZeniMax Media, Obsidian Entertainment or Microsoft. It
> contains no game files; you need your own legally obtained copy of the game.
> See [LEGAL.md](LEGAL.md).

---

## How it works

- **Original files at runtime.** nv-rs reads the game's own plugins, archives,
  meshes, textures, sounds, menus and shaders directly. There is no
  conversion step, and the installation is never modified.
- **Behaviour from the original program.** Rules, constants and formulas come
  from the game's program (`FalloutNV.exe`, read in Ghidra), its data, and
  recordings of the original game. Nothing is invented or tuned by eye. The
  method is described in [docs/METHODOLOGY.md](docs/METHODOLOGY.md) and the
  decisions behind it in [docs/adr/](docs/adr/README.md).
- **Checked against the original.** Tools in [`research/`](research/) record
  what the original program actually does (function results on the real CPU,
  live arguments and state) so that Rust code can be tested against it.
- **Simulation separate from presentation.** Game rules live in plain Rust
  crates with no external libraries. The Bevy app only draws, plays sound
  and reads input. That split also keeps the planned VR support possible
  ([docs/VR.md](docs/VR.md)).

---

## Repository layout

```
nv-rs/
├── Cargo.toml         # core workspace (no external crates)
├── crates/            # readers, game rules, tools and test fixtures (table below)
├── viewer/            # the Bevy app (its own workspace, Bevy 0.16)
├── research/          # Ghidra scripts and native oracle tools
│   ├── ghidra/        # identity, complete export, function cards, name import
│   └── nv-oracle/     # nv-call, nv-probe, nv-inject (own workspace, 32-bit Windows)
├── docs/              # roadmap, method, decisions, topic findings
├── scripts/           # packaging
└── distribution/      # files shipped in playtest packages
```

| Group | Crates | What they do |
| --- | --- | --- |
| **Formats** | `esm`, `bsa`, `nif`, `dds`, `mp3`, `speedtree`, `shaders`, `assets` | Read the game's files: plugins and records, archives, meshes and animations, textures, audio, trees, compiled shaders, and the loose-file/archive lookup. |
| **Game** | `world`, `physics`, `script`, `ui`, `preview`, `cellview` | The game's rules: world state, actors, AI, combat, dialogue, quests, inventory, saves, collision, the script engine, menus built from the game's XML, and per-cell scene data. |
| **Tools** | `nvinspect` | Command-line inspection of real game data: records, cells, meshes, collision walks, coverage tables and more. |
| **Test content** | `testdata` | Builds synthetic plugins, archives and meshes for tests, so tests never need game files. |
| **Engine** | `viewer` (binary `nv-viewer`) | The Bevy app: rendering, audio and input on top of the game crates. |

[ADR-0004](docs/adr/0004-mudcrab-style-layout.md) plans to merge these groups
into five crates (`formats`, `game`, `tools`, `test-content`, `engine`) and to
regroup the docs.

---

## Roadmap

Milestones are worked in order, and each has an acceptance gate checked
against the original game. Status and evidence are in
[docs/MILESTONES.md](docs/MILESTONES.md).

- [ ] **M1: Opening and persistent world** (active): the opening movie and
  wakeup, Doc Mitchell's character creation, leaving for Goodsprings, and
  saving and reloading with all state intact.
- [ ] **M2: Core gameplay loop:** Sunny's tutorial and a Goodsprings quest
  branch: movement, weapons, damage, AI, dialogue, loot, trade, progression
  and V.A.T.S.
- [ ] **M3: Base-game systems and campaign:** coverage of quests, creatures,
  weapons, effects, factions, companions, travel and menus, up to campaign
  completion.
- [ ] **M4: Stability and performance:** long play without crashes or lost
  state, with measured frame times and streaming.
- [ ] **M5: DLC and mods:** official DLC and documented plugin, archive and
  script-extension behaviour.
- [ ] **M6: VR:** shared simulation with independent aim and views, tested on
  a headset.

The opening has been played live from Doc's vigor tester through the SPECIAL
menu. That segment is not the whole opening, and nothing here is a complete
playthrough yet. Native NVSE plugin DLLs are not supported, original-game
saves are not interchangeable with nv-rs saves, and VR and performance targets
have not been validated.

---

## Quick start

### Play a test build

Download a Windows x64 ZIP from
[Releases](https://github.com/slaterain/nv-rs/releases), extract it, and run
`Play.cmd`. Point it at your own `Fallout New Vegas\Data` folder. Read
[docs/PLAYTESTING.md](docs/PLAYTESTING.md) first.

### Build from source

Install stable Rust, then:

```sh
# Core workspace: readers, game rules, nvinspect
cargo build --release
cargo test --workspace

# The Bevy app (separate workspace)
cd viewer
cargo run --release -- "<path to Fallout New Vegas\Data>" GSDocMitchellHouse
```

The core crates and their tests need no game files. Running the viewer and
`nvinspect` on real assets needs your `Data` folder.

### Build and run on macOS

The source build and Bevy viewer support native macOS. The viewer selects
Metal on macOS. Install stable Rust and the Xcode Command Line Tools, then
build the core crates and viewer as above. On Apple Silicon, Cargo builds a
native arm64 viewer binary by default. Pass the absolute path to the `Data`
folder from your own **PC** game installation:

```sh
cd viewer
cargo run --release -- "/path/to/Fallout New Vegas/Data" GSDocMitchellHouse
```

The project does not include game files or provide a macOS game installer. A
Windows PC install's `Data` folder can be read directly from a disk or copied
to the Mac. Native config discovery checks `~/Documents/My Games/FalloutNV`
and the install's `Fallout_default.ini`. If your Windows game runs in a
compatibility-layer prefix and you want its mod list or settings, pass the
prefix's `plugins.txt` with `--plugins` and `Fallout.ini` with `--ini`.
Windows release ZIPs and the research tools that inspect `FalloutNV.exe`
remain Windows-specific.

---

## Documentation

| Read this | For |
| --- | --- |
| [docs/MILESTONES.md](docs/MILESTONES.md) | Active work, blockers, evidence and the next action |
| [docs/METHODOLOGY.md](docs/METHODOLOGY.md) | How the game's program is read and reimplemented, with sources |
| [docs/adr/](docs/adr/README.md) | Decisions: method, symbols, decompiled code, layout, tools |
| [docs/RESEARCH_WORKFLOW.md](docs/RESEARCH_WORKFLOW.md) | Day-to-day research steps and provenance |
| [research/ghidra](research/ghidra/README.md), [research/nv-oracle](research/nv-oracle/README.md) | The research tools and how to run them |
| [docs/TECHNICAL_REFERENCE.md](docs/TECHNICAL_REFERENCE.md) | Detailed feature descriptions |
| [docs/OPENING.md](docs/OPENING.md), [docs/OPENING_LOOK_IK.md](docs/OPENING_LOOK_IK.md), [docs/FACE_CREATION.md](docs/FACE_CREATION.md), [docs/PERSISTENCE.md](docs/PERSISTENCE.md), [docs/VIGOR.md](docs/VIGOR.md) | M1 topic findings |
| [docs/VR.md](docs/VR.md) | VR boundaries and proposals |
| [docs/PLAYTESTING.md](docs/PLAYTESTING.md), [docs/MAINTAINING.md](docs/MAINTAINING.md) | Testing builds and maintaining releases |

---

## Contributing

nv-rs is a personal project maintained by **slaterain**. Contributions are
welcome through forks and pull requests when they fit the active milestone.
Read [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/GOVERNANCE.md](docs/GOVERNANCE.md)
first.

## Legal and license

Original code is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. The licence covers only original
work. Fallout and Fallout: New Vegas are trademarks of their respective owners.
Read [LEGAL.md](LEGAL.md) for the full notice, including how material learned
from the game's program is handled.
