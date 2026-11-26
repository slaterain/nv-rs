<p align="center">
  <img src=".github/media/logo-ranger-2.png" alt="nv-rs logo: an NCR Ranger with a revolver in front of a Mojave sunset, inside a gear" width="260">
</p>

<h1 align="center">nv-rs</h1>

[![Download](https://img.shields.io/badge/Download-preview_2026--11--26.1-brightgreen.svg)](https://github.com/slaterain/nv-rs/releases/tag/preview-2026-11-26.1)
[![Rust](https://img.shields.io/badge/Rust-2021_edition-orange.svg)](https://www.rust-lang.org/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows_|_Linux_|_macOS-purple.svg)](#play-the-preview)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2.svg?logo=discord&logoColor=white)](https://discord.gg/pn5kjZJJxx)

**Fallout: New Vegas, rebuilt in Rust, aiming to play exactly like the original.**

nv-rs is a new engine for Fallout: New Vegas, written in Rust, that runs the game from your own
copy's files. The game should look and play exactly like the original, while underneath it becomes
a modern 64-bit engine: high frame rates, fewer crashes, built-in modding and more platforms.

> [!IMPORTANT]
> nv-rs is an independent project, not affiliated with or endorsed by Bethesda Softworks, ZeniMax
> Media, Obsidian Entertainment or Microsoft. It contains no game files; you need your own legally
> obtained copy of the game. See [LEGAL.md](LEGAL.md).

---

## Play the preview

**[Download the preview](https://github.com/slaterain/nv-rs/releases/tag/preview-2026-11-26.1)**
for Windows, Linux or macOS (Apple silicon). You need Fallout: New Vegas, version 1.4.0.525.

- **Windows:** extract the ZIP to a folder of its own (not Documents, Program Files or the game's
  folder) and double-click `Play.exe`.
- **Linux:** extract the archive in your home folder and start `./nv-rs`.
- **macOS:** unpack it, allow it once in Terminal (`xattr -dr com.apple.quarantine` and the
  folder), then start `./nv-rs`.

The launcher finds your game and plays it, imports your saves, installs mods and has the settings.
It never changes your game, your saves or your settings: everything it writes stays in its own
folder. It asks once whether crash reports may be sent, and sends nothing unless you agree. Press
F12 in the game to save a report of anything that looks wrong. The download page has the details
and the known issues.

The team has tested on Windows only; players have run it on Linux and on Apple silicon Macs.
Please tell us how it runs on yours.

## Where it stands

**Playable.** The whole game runs on its own translated code: the opening, the open world, quests,
dialogue, combat, V.A.T.S., the Pip-Boy, saves, mods and the DLCs, with the original graphics and
sound. Bugs remain.

**Next:** turning the game's code into native 64-bit Rust and a new renderer, for speed and to lift
the 4 GB memory limit. After that: co-op and VR.

| | What | Today |
| :---: | --- | --- |
| ✅ | **The whole game** | Playable from start to finish, DLCs included. |
| ✅ | **Fixes** | Bug fixes on by default, each with its own setting. |
| ✅ | **Launcher** | Play, mods, saves, settings and crash reports in one window. |
| ✅ | **Windows, Linux, macOS** | A preview for each; tested by the team on Windows. |
| 🔨 | **Mods** | `.esp`/`.esm` mods and mod installers work; xNVSE, JIP LN NVSE and UI Organizer are built in. Players report Tale of Two Wastelands working. Other `.dll` plugins don't load yet. |
| 🔨 | **Native code** | About 2,550 of the game's functions rewritten in native Rust so far. |
| 🔨 | **Graphics** | Vulkan by default. Optional new lighting: ambient occlusion and sun shadows (Shift+F11 in the game). |
| ⬜ | **Co-op, VR** | Planned. |

✅ means working, not free of bugs.

## Why it's different

- **Translated, not guessed.** The game's program is translated into Rust piece by piece, each
  piece checked against the original.
- **Your game, your files.** nv-rs reads the game's data from your installation and never changes it.
- **Better where it doesn't change the game.** Fixes and upgrades are switchable; the original
  stays available.

---

## How it's made

To be upfront: nv-rs is built almost entirely with AI coding agents (mainly Claude). The maintainer
directs the project: the plan, the priorities, playtesting and what gets merged.

What keeps it honest is the method: agents don't get to invent behaviour. Every piece of logic is
translated from the original game's program, marked with where it came from, and checked against
the original's own code and recordings of the original game. Judge it by how it plays.

## Get involved

- **Play and report:** try the preview and [open an issue](https://github.com/slaterain/nv-rs/issues)
  for anything that differs from the original.
- **Contribute:** read [CONTRIBUTING.md](CONTRIBUTING.md) (and point your agent at it), say
  in an issue what you'd like to work on, then open a small pull request.
- **Chat** on the project's [Discord](https://discord.gg/pn5kjZJJxx).
- **Support** the project on [Ko-fi](https://ko-fi.com/slater41n). It pays for the AI time the
  work runs on. It's a voluntary gift: it buys nothing and promises no release date.
- **Build from source:** with Rust installed (the version is pinned in `rust-toolchain.toml`),
  `cargo build --release -p nv-viewer`; [Build and run](CONTRIBUTING.md#build-and-run) says how to
  start it.

Final decisions rest with the maintainer, **slaterain**.

## Credits

nv-rs builds these mods in, translated into Rust from their published source and distributed as
their licences allow (each crate's `NOTICE` and every package carry the notices). No C++ of theirs
is included; one data file, UI Organizer's list of the UI mods it supports (`supported.txt`), is
included unchanged.

- **[xNVSE](https://github.com/xNVSE/NVSE)**, the New Vegas Script Extender: the original NVSE by
  Ian Patterson, Stephen Abel, Paul Connelly and Hugues LE PORS (ianpatt, behippo, scruggsywuggsy
  the ferret and hlp); xNVSE by korri123, cnf13, jazzisparis, Demorome and its contributors, with
  the code xNVSE carries from Algohol (entim) and the MT19937 generator (Makoto Matsumoto and
  Takuji Nishimura).
- **[JIP LN NVSE](https://github.com/jazzisparis/JIP-LN-NVSE)** by jazzisparis, with its game
  patches and the commands that began as the lutana NVSE plugin.
- **[UI Organizer](https://github.com/jazzisparis/UIO-User-Interface-Organizer)** by jazzisparis
  (Jef).
- **[JohnnyGuitar NVSE](https://github.com/carxt/JohnnyGuitarNVSE)** by carxt and its
  contributors: its editor ids and its corrections to JIP LN's co-save.
- The fixes preset's "No LOD Water" flag on The Strip, found by Aqua Performa's author.

The community decompilation [fnv_decomp](https://github.com/momusdigital/fnv_decomp) is read
alongside the game's program as a reference for names, types and logic.

## Legal and license

Original code is licensed under the [GNU GPL v3 or later](LICENSE). The licence covers only
original work. Fallout and Fallout: New Vegas are trademarks of their respective owners. Read
[LEGAL.md](LEGAL.md) for the full notice.
