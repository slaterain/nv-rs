<p align="center">
  <img src=".github/media/logo-ranger-2.png" alt="nv-rs logo: an NCR Ranger with a revolver in front of a Mojave sunset, inside a gear" width="260">
</p>

<h1 align="center">nv-rs</h1>

[![Download](https://img.shields.io/badge/Download-Windows_preview-brightgreen.svg)](https://github.com/slaterain/nv-rs/releases)
[![Rust](https://img.shields.io/badge/Rust-2021_edition-orange.svg)](https://www.rust-lang.org/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows-purple.svg)](#play-the-preview)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2.svg?logo=discord&logoColor=white)](https://discord.gg/pn5kjZJJxx)

**Fallout: New Vegas, rebuilt in Rust, aiming to play exactly like the original.**

nv-rs is a new engine for Fallout: New Vegas, written in Rust. It runs the game from your own
copy's files. The goal: the game looks and plays exactly like the original, while everything
underneath becomes a modern 64-bit engine: no 4 GB memory limit, every CPU core in use, high frame
rates, fast loading, far fewer crashes, built-in modding and more platforms.

> [!IMPORTANT]
> nv-rs is an independent project, not affiliated with or endorsed by Bethesda Softworks, ZeniMax
> Media, Obsidian Entertainment or Microsoft. It contains no game files; you need your own legally
> obtained copy of the game. See [LEGAL.md](LEGAL.md).

---

## Play the preview

**[Download the Windows preview](https://github.com/slaterain/nv-rs/releases)**. You need
Fallout: New Vegas (version 1.4.0.525; only the Steam version has been tested so far) on a Windows PC.

Builds from source (not yet the preview) also recognize one GOG build: the en-US offline installer 1.4.0.525(a), build 55068.

1. Extract the ZIP to a folder of its own, for example `C:\Games\nv-rs`.
2. Double-click `Play.exe`. The launcher opens, in the look of the game's terminals; `> Play` finds
   your game and starts it.
3. `Import my saves.cmd` brings your saves over (in builds from source and the next preview, the
   launcher's `> Import saves` entry does it instead). In builds from source and the next preview, the launcher's `> Settings` has the display options, each game fix on its own, and the new lighting (ambient occlusion and sun shadows, off by default and not drawn by the Direct3D 9 renderer; Shift+F11 switches it while playing).

It never changes your game, your saves or your settings: everything it writes stays in its own
folder. Windows may warn because the program isn't signed. It has only been run on one PC so far:
please report how it runs on yours (press F12 in the game to make a report). Keyboard and mouse,
or an Xbox-style or PlayStation game pad on Windows; mods that need script-extender plugins don't
work yet. The download page lists the known issues.

## Where it stands

**Playable.** The whole game runs on its own translated code: the opening, the open world, quests,
dialogue, combat, V.A.T.S., the Pip-Boy, saves, mods and the DLCs, with the original graphics and
sound. Bugs remain, and it's slower than the original in busy places for now.

**Next:** moving the game's systems to native 64-bit Rust, which lifts the 4 GB memory limit and
brings the speed, and finishing the built-in mod tools. After that: more platforms, co-op and VR.

## Progress

<p align="center">
  <img src=".github/media/progress.svg" alt="Engine port progress: 100% of the game's code translated" width="100%">
</p>

The bar counts how much of the original game's code is translated. That part is done; the table
shows what's left.

| | System | What you get today |
| :---: | --- | --- |
| ✅ | **The whole game** | Combat, AI, quests, dialogue, menus, saves, the open world and the DLCs, checked against the original and playable. |
| ✅ | **Fixes** | Bug fixes on by default, each with its own setting; the original's behaviour is one setting away. |
| ✅ | **Windows preview** | [Released](https://github.com/slaterain/nv-rs/releases); run on one PC so far. |
| 🔨 | **Native 64-bit code** | Under way: threads on their own cores, textures and sounds kept outside the 32-bit memory, and about 2,100 of the game's functions rewritten as native code so far (parts of the script evaluator, the world draw, the actors' AI, forms, menus, cells' references and their extra data, and the scene's lights), each checked against the translation. |
| 🔨 | **Built-in script extender: xNVSE** | All 654 of its script commands and both its script languages, with nothing to install. |
| 🔨 | **Built-in script extender: JIP LN:** | All its script commands (JIP LN: 1,116 and its two version commands) and all of its game patches that do something on this port (174 of 183; three more are folded into others, six do nothing here); on in the default fixes preset, off in vanilla (`bJIPInstalled=0` turns it off). |
| 🔨 | **UI Organizer (UIO)** | UI mods' HUD and menu additions; on in the default fixes preset (`bUIO=0` turns it off). |
| ⬜ | **lStewieAl's Tweaks** | No longer built in. Mods that expect it are to be supported from their own files (CONTRIBUTING.md, "What can and can't be accepted"). |
| 🔨 | **Mod installers (FOMOD)** | Mods with an installer, from .zip or .7z, installed into nv-rs's own folder, never the game's. |
| 🔨 | **One launcher** | `Play.exe` opens a window like the game's terminals to play, install mods, import saves and change settings; its Settings has the display options, each fix on its own and the new lighting. |
| 🔨 | **Modern graphics (wgpu)** | The default renderer, on Vulkan; in the places compared, on one NVIDIA card, its pictures nearly match Direct3D 9's, and at 8x antialiasing it is about 8% slower. New lighting (ambient occlusion and sun shadows) is being tried, off by default, and so is a new way of drawing: the still world drawn again from what the renderer keeps instead of being set up from scratch every frame. |
| 🔨 | **Game pads** | Xbox-style pads on Windows as in the original; other pads, and Linux, not yet tried with a real pad. |
| ⬜ | **More platforms, co-op, VR** | Planned; co-op has no design yet. |

✅ means working, not free of bugs.

## Why it's different

- **Translated, not guessed.** The game's program is translated into Rust piece by piece, each
  piece checked against the original.
- **Your game, your files.** nv-rs reads the game's data from your installation and never changes it.
- **Modern underneath.** The internals become 64-bit Rust, whose memory and thread safety rule out
  whole classes of crashes.
- **Better where it doesn't change the game.** Fixes and upgrades are switchable; the original
  stays available.
- **Mods keep working.** Regular `.esp` and `.esm` mods load as they do in the original.

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
