# Contributing to nv-rs

nv-rs is Fallout: New Vegas rebuilt in Rust, played from your own copy of the game. The game's
program, `FalloutNV.exe` 1.4.0.525, is translated into Rust function by function, and the
translated systems are being rewritten as fast native 64-bit code. The aim is a game that looks,
sounds and plays exactly like the original, without its crashes, stutters and memory limit.

This page is for people and for their AI agents. **If you use an agent, point it at this file.**
Everything it needs to follow is here; the section [For AI agents](#for-ai-agents) is the short
list of rules it must not break.

## The short version

1. Fork the repository and work on a topic branch made from the latest `main`.
2. For anything bigger than a small fix, open an issue first and say what you plan to do.
3. Every behaviour comes from the original game. Nothing is invented or "tuned by feel".
4. Keep each pull request small and about one thing. Run the checks and say how you verified it.

## What helps most

1. **Play and report bugs.** A crash, a freeze, or anything that differs from the original game.
   Open an issue saying where you were, what you did, what the original game does there, and
   attach the log (the newest `game-*.log`) and an F12 report if you made one. When the game's
   code stops, the log's last line names the place:
   `Game's main stopped in frame N: <why> (in <exe addresses>)`. Those addresses are what we need.
2. **Fix translation errors.** Compare a function's Rust translation with the executable (in a
   disassembler such as Ghidra) and fix what differs, with a regression test.
3. **Speed and stutters.** Rewriting hot code as native Rust, proven to give the same results as
   the translation it replaces (see [Native code](#native-code)).
4. **Linux and macOS.** Builds exist but have barely been played. Reports and fixes are welcome.
5. **Mod and script-extender compatibility.** xNVSE, JIP LN NVSE and UI Organizer are built in,
   and JohnnyGuitar NVSE's editor ids. Fixes to them, and ports of other open-source plugins whose licence
   allows it, are welcome (see [What can and can't be accepted](#what-can-and-cant-be-accepted)).

## Build and run

You need Rust (the version is pinned in `rust-toolchain.toml`; `rustup` picks it up) and your own
installed copy of Fallout: New Vegas 1.4.0.525. The repository contains no game files and never
will.

```sh
cargo build --profile play -p nv-viewer      # quick play build: target/play/nv-viewer(.exe)
cargo build --release -p nv-viewer           # the build the packages use
```

Run it with the game's `Data` folder (or the installation folder above it). Started without the
launcher, it needs two variables: which renderer draws, and sound on.

```powershell
$env:NV_D3D9 = 'wgpu'; $env:NV_SOUND = 'out'
.\target\play\nv-viewer.exe 'C:\Games\Steam\steamapps\common\Fallout New Vegas\Data'
```

`NV_D3D9=wgpu` draws with wgpu (Vulkan, Direct3D 12 or Metal); on Windows, `real` uses the
system's Direct3D 9 instead.

On Windows the game also reads tables that describe this machine (its graphics adapter, the
graphics driver's answers, the sound device). The launcher, `Play.exe`, makes them at every start
with the capture tools in `scripts/` (`d3d9-capture`, `nvapi-capture`, `dsound-capture`; two of them
build for 32-bit Windows, so run `rustup target add i686-pc-windows-msvc` once). Started without the
launcher, a run that lacks a table stops, and its last line names the tool to run and the variable
that points at its file. Write a table from `cmd`, for example
`cmd /c "cargo run --release --manifest-path scripts/d3d9-capture/Cargo.toml > d3d9_adapter.tsv"`
for the adapter (`NV_D3D9_ADAPTER`, default `%LOCALAPPDATA%\nv-rs\d3d9_adapter.tsv`): Windows
PowerShell's `>` writes UTF-16, which the game cannot read.

nv-rs never changes the game's folder, your saves or your settings. On Linux and macOS,
`distribution/unix/play.sh` finds the game and starts it (`--check` checks your game's version
first); for a source build, give it your program's full path:
`NV_RS_PROGRAM="$PWD/target/play/nv-viewer" distribution/unix/play.sh`.

## Checks

Run these before you push. GitHub runs them too, and they must pass before a merge.

```sh
cargo fmt --all -- --check
cargo clippy -p <each crate you changed> --all-targets -- -D warnings
cargo test -p <each crate you changed>
cargo test -p nvinspect --test repo_hygiene
```

- `cargo test --workspace` runs everything, but the engine's integration tests (`crates/engine/tests`)
  are large: each needs several GB of memory to build. Run the ones that cover what you changed and
  leave the rest to CI.
- The hygiene test rejects game files and other binaries, absolute paths with user names, and
  Ghidra's automatic names (`FUN_…`, `DAT_…`): write the address instead.
- Code must build on Windows, Linux and macOS. Check a crate for macOS with
  `rustup target add aarch64-apple-darwin` (once), then
  `cargo check --target aarch64-apple-darwin --tests -p <crate>`. A Linux check from Windows needs a
  Linux system's libraries for the sound crates; GitHub's Linux job runs on every pull request.
  A pull request that touches files, windows, input, sound, drawing, the launcher or packaging gets
  the `other-systems` label so the macOS job runs too (ask in the pull request if you can't set it).
- If you change `Cargo.lock`, update `distribution/DEPENDENCY-LOCK.sha256`: the SHA-256 of the
  committed `Cargo.lock` with LF line endings (`git show HEAD:Cargo.lock | sha256sum`). New external
  crates need the maintainer's agreement first.
- If you change `crates/host/data/game_ini.tsv`, regenerate `distribution/nv-rs-sections.ini` with
  `scripts/game-ini-sections.sh`.

## How the code is organised

| Where | What |
| --- | --- |
| `crates/engine-fallout-*`, `engine-bs`, `engine-gamebryo`, `engine-havok`, ... | The game's own code, translated function by function, each function marked with its address |
| `crates/engine-core` | The model the translation runs on: the game's memory, calls by address, threads, the extenders' hook points |
| `crates/engine-typed` | Native rewrites of the game's functions ("typed families"), registered at the functions' addresses |
| `crates/engine-platform` | What the game asks of Windows, DirectX and the C runtime, answered on every system |
| `crates/host` | Starts the game, holds the optional fixes ("rows"), runs the frames |
| `viewer/` | `nv-viewer`, the program around the host |
| `crates/nv-launcher` | `Play.exe`, the launcher players start |
| `crates/d3d9-wgpu` | The renderer: the game's Direct3D 9 drawing on wgpu (Vulkan, Direct3D 12, Metal) |
| `crates/nvse`, `jip`, `johnnyguitar`, `uio` | The built-in script extenders and plugins |
| `research/engine-map` | The map of every function in `FalloutNV.exe` that the build and tests read |
| `distribution/`, `scripts/` | What goes into the packages, and the scripts that build and check them |
| `vendor/` | Third-party crates kept with small patches |

## Rules for game code

**Provenance.** Every behaviour comes from the original game: translated from `FalloutNV.exe`
1.4.0.525, read from the game's data, or measured in the original game.

- Mark each translated function with its address:
  `// Translated from 007b5400 (decompiled, FalloutNV.exe 1.4.0.525)`.
- Names, types and layouts from the Xbox 360 prototype's symbols may be used, marked `(Xbox PDB)`.
- The community decompilation [fnv_decomp](https://github.com/momusdigital/fnv_decomp) may be read
  alongside the executable for names, types and logic. The executable stays the reference: settle
  every difference there, add `, read against fnv_decomp` to the marker, and never commit its C++.
- Nothing invented: no stand-in values, "close enough" constants, or behaviour the game doesn't have.
  If you couldn't trace something, leave it out and say so in the pull request.
- Never commit game files, extracted assets, executable bytes, decompiler databases or exports,
  saves, recordings or logs.

**Tests.** Every behaviour fix comes with a regression test. Tests build their inputs from scratch:
they never need the game's files.

**Changes beyond the original.** A bug fix the original game doesn't have, or anything else a
player could notice, is a fix row: an entry in `crates/host/src/fixes.rs` with its variable
(`NV_...`) and its `Fallout.ini` key in `crates/host/data/game_ini.tsv`. Rows are on in the default
"fixes" preset and off in "vanilla", which stays exactly the original game.

**Speed.** The game may do its work differently (in another order, cached, on another thread,
skipping what no one can see) as long as a player can't tell: the same gameplay, results, look and
feel. A change that alters order or timing goes behind a switch, off by default, until it has been
checked.

### Native code

Native rewrites in `crates/engine-typed` (and `crates/engine-core/src/typed`) replace a translated
function at the same address. Mark each one `// Typed from 00a6db50` directly above the function
(`, read against fnv_decomp` may follow). Call a function that a fix or built-in may replace (the
addresses in `crates/engine-core/data/overridable.tsv`) with `e.call`, not `typed_call!`. Each
one needs a differential test that runs it against the translation it replaces and gets the same
results. Logic, integers and pointers must match exactly. For floating-point maths: plain `f32` in
code only the main thread runs; in code other threads run, `f64` rounded to `f32` wherever the
original stores a float.

## What can and can't be accepted

Welcome:

- Work traced from the executable, the game's data or recordings of the original game.
- Ports of open-source mods, plugins and script extenders whose licence allows it, with their
  licence notice kept (a `NOTICE` in the crate, `LEGAL.md`, `distribution/THIRD-PARTY-NOTICES.txt`
  and `distribution/notices.txt`).
- Fixes taken from data mods, credited.

Not accepted:

- lStewieAl's Tweaks, or code written from it.
- Code written from a closed-source plugin (its DLL's disassembly) or from source published without
  an open-source licence.
- Behaviour that wasn't traced, however good it looks.
- Game files or anything taken from the game's executable as bytes.

## Pull requests

- Work on a topic branch of your fork, made from the latest `main`. Keep it rebased on `main`
  (`git fetch upstream && git rebase upstream/main`), at least daily while it's open and right
  before you ask for review.
- Don't build on the maintainer's `claude/*` or `codex/*` branches: they move and are deleted.
- One system per pull request. Big work goes in as a series of small pull requests, each
  reviewable on its own. A draft is fine while it's unfinished.
- Fill in the pull request template: what changed and why, the evidence (addresses, records,
  recordings), the checks you ran and their results, what you checked by playing, and which AI tools
  helped.
- Before starting on an issue someone else might take, comment `Claiming this` with your branch
  name. First claim wins; a claim with no update for 48 hours can be released.
- Every pull request is reviewed independently before merge: reviewers check its translations
  against the executable and these rules, and only findings that hold up are sent back. Pull
  requests are squash-merged into `main`. The maintainer decides priorities, design and what is
  merged.

## For AI agents

Follow everything above. These rules matter most:

1. Work only on what the task asks. Don't reformat, rename or "tidy" unrelated code.
2. Never invent game behaviour. Translate from the executable at a named address, or leave the
   part out and say so.
3. Mark every translated function with its address and `FalloutNV.exe 1.4.0.525`. Never write
   Ghidra's automatic names (`FUN_…`, `DAT_…`, `LAB_…`).
4. Never commit game files, executable bytes, decompiler output, fnv_decomp's C++, saves, logs,
   local paths or user names.
5. Every fix gets a regression test whose inputs are built in the test.
6. Run the checks above on what you changed and report the real results. Never say a test passed
   unless you ran it and saw it pass; say what you could not run.
7. Keep code working on Windows, Linux and macOS. No Windows-only code without the other systems'
   path in the same change.
8. Don't add external crates. Don't port lStewieAl's Tweaks, closed-source plugins or unlicensed
   code.
9. Say in the pull request that an AI agent helped and with what. A person reviews it and owns it.

## Licence

nv-rs is licensed under the GNU General Public License v3.0 ([LICENSE](LICENSE)); by contributing
you agree that your contribution is licensed the same way. [LEGAL.md](LEGAL.md) explains what the
project contains and what it doesn't.
