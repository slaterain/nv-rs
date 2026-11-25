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
3. **Turning the game native.** Every translated function is being rewritten as native Rust,
   proven to give the same results as the translation it replaces. It is the biggest job in the
   project and anyone can take a part of it: see
   [Turning the game native](#turning-the-game-native).
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
1.4.0.525, read from the game's data, or measured in the original game. A built-in mod's
behaviour comes from its source, or, for compatibility with mods that expect lStewieAl's Tweaks,
from those mods' files and observed behaviour (see
[What can and can't be accepted](#what-can-and-cant-be-accepted)).

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

## Turning the game native

The translation runs every function of the game through `engine-core`'s model of the original
program: its memory, its calls by address, its x87 floating point. That is exact but slow. A
native rewrite is plain Rust registered at the function's address, so every caller that calls that
address gets it, and it is checked against the translation before it is turned on. The goal is the
whole game native. About 48,000 functions are translated, and about 1,100 of them have a native
rewrite so far (the translation stays, as the reference the native code is tested against).

### Where to help

The maintainer's agents work through these crates, one lane each:

| Crate | Translated functions | Who |
| --- | --- | --- |
| `engine-bs`, `engine-gamebryo` (the engine under the game: drawing, scene graph, animation, sound) | about 9,400 | maintainer |
| `engine-fallout-ai` | about 4,900 | maintainer |
| `engine-fallout-forms`, then `engine-fallout-script` | about 4,800, then 1,550 | maintainer |
| `engine-fallout-cell` | about 4,200 | maintainer |
| `engine-fallout-interface`, then `engine-fallout-facegen` | about 2,700, then 950 | maintainer |
| `engine-havok` (physics) | about 9,000 | open |
| `engine-fallout-pathfinding` | about 1,800 | open |
| `engine-fallout-misc`, `engine-fallout-magic`, `engine-fallout-env` | about 1,300, 1,150, 1,150 | open |
| `engine-fallout-unplaced-a`, `engine-fallout-unplaced-b` | about 1,050, 2,250 | open |
| `engine-speedtree` | about 750 | open |
| `engine-core` (the C runtime) | about 850 | open |

Take a file in an open crate. For a file in a maintainer crate, ask in an issue first: its lane may
already have it in progress. Before you start, open an issue titled `Native: <crate>/<file>` (or
comment on an existing one) with `Claiming this` and your branch name; the claiming rules in
[Pull requests](#pull-requests) apply. Count a crate's translated functions with
`git grep "Translated from" -- crates/<crate> | wc -l`, and the native ones with
`git grep "Typed from" -- crates/engine-typed | wc -l`. Before you start, check which functions of
your file already have a native version: `git grep "Typed from <address>" -- crates`.

### How to convert a file

1. **Pick a family.** Native code lives in `crates/engine-typed/src/<family>/`. A family is a
   group of functions that is switched on and off together (`NV_TYPED=<family>:off`). Use the
   family that already covers the area; there are only 64 ids, so add one only for a new area. A
   new family takes the next number after the highest in
   `git grep "pub const FAMILY: FamilyId" -- crates` (the gaps below it are held by work in
   progress; check the open pull requests too) and declares
   `pub static NAME: Family = Family { name: "<family>", id: FAMILY, past_gate: true, threads: Threads::Both, stage2: false, funcs };`.
   Set `threads` to the threads that run its code (`Main`, `Other` or `Both`): the differential
   test runs under those threads' float precision. Its `mod.rs` starts with a `//!` comment saying
   what it covers, which threads run it and how its floats are done. In
   `crates/engine-typed/src/lib.rs`, add `pub mod <family>;` and
   `&crate::<family>::NAME` at the end of `FAMILIES` (and its length; `units::funcs` provides
   that list, and the tests read it). In that file's tests, update the length `families()`
   returns, the `past` list, the two `count() == N` checks (families
   minus one), and the comment saying when each family went on.
2. **Write each function in plain Rust.** Read the translation and the executable (Ghidra or
   another disassembler) together. Mark it `// Typed from 00a6db50` directly above the function.
   Each function takes `e: &mut Engine` first, then the original's arguments (`u32`, `i32`, `f32`,
   `bool`, `Ptr<T>`). `engine-typed` depends only on `engine-core`, so declare the layouts you read
   with `image!` (or use a family's `layout` module) instead of importing them from a translated
   crate. Read and write the game's memory through `e.mem`. Call a function that is already
   native with `typed_call!(its_family::FAMILY, 0x00a6_db50, path::to_fn(e, ...))`: the family
   that holds the callee (`FAMILY` for your own), with the address as a literal. Call any other
   function by address with `e.call(0x00a6_db50, &args![...])`.
3. **Keep what callers and mods rely on.** The addresses in
   `crates/engine-core/data/overridable.tsv` are always called with `e.call`, never `typed_call!`.
   A virtual call stays `e.vcall(this, slot, &args![...])` unless you check that the slot holds
   exactly the function you call (`slot_target(e, this, slot) == 0x…`, then `typed_call!`). Keep
   every `e.site(...)`, `e.jip_patches()` and `e.hook_users(...)` check the translation makes, at
   the same point (`crates/engine-core/src/extend.rs`). Floats follow the rule in
   [Native code](#native-code). Code that both kinds of thread run is written once over
   `native::Real` (`crates/engine-typed/src/native.rs`), and a float becomes an integer through
   `x87::fistp_chop`, never `as` (clippy and the hygiene test refuse it).
4. **Register it.** Add `entry!(0x00a6_db50, module::name(Ptr, u32) -> u32)` to the family's
   `funcs()` list.
5. **Reach every caller.** `git grep fn_00a6db50 -- crates` finds translations that call the
   function as a plain Rust function. They would skip the native version, so change each one to
   `e.call(0x00a6_db50, &args![...])`. For an address in `overridable.tsv`, also lower its count
   there (the hygiene test asks for it).
6. **Test it against the translation.** Every native function gets a differential test in
   `crates/engine/tests/typed_<family>.rs`. A test such as
   `Diff::new(entry!(...)).family(&facegen::FACEGEN).inputs(...).setup(...).run().assert_same()`
   runs the native function and its translation on the same memory and inputs, and compares the
   results, the memory written and the calls made. Name your own family, and every family your
   `typed_call!`s reach, with `.family`; without it the test runs under both threads' precision
   whatever your `threads` says. Start the file from an existing `typed_*.rs`: it begins with
   `use engine as _;`. Build the memory in the test, with the real layouts where they matter.
   Reach every branch. One setup can serve a whole file.
7. **Prove the tests can fail.** Plant faults: change one line of the native code (a compare, a
   bound, an offset), run the tests, check that one fails, then put the line back. Do four per
   pull request, on the riskiest functions, and list them in the pull request with the test that
   caught each one.
8. **Run the checks.** `cargo fmt --all -- --check`;
   `cargo clippy -p engine-typed --all-targets -- -D warnings` and
   `cargo test -p engine-typed --lib`; `cargo clippy -p engine --test typed_<family> -- -D warnings`
   and `cargo test -p engine --test typed_<family>` (not `--release`: the harness's direct-call run
   needs debug assertions); clippy and lib tests for any crate whose translation you changed; and
   `cargo test -p nvinspect --test repo_hygiene`, which checks that every marked function is
   registered and has a differential test.

A whole file, or a module of 30 to 100 functions, is a good size for a pull request. Say in the
pull request which functions it covers, what you checked against the executable, the planted
faults and the checks you ran. If you find a translation error on the way, fix it in the
translation as well, with its own regression test.

## What can and can't be accepted

Welcome:

- Work traced from the executable, the game's data or recordings of the original game.
- Ports of open-source mods, plugins and script extenders whose licence allows it, with their
  licence notice kept (a `NOTICE` in the crate, `LEGAL.md`, `distribution/THIRD-PARTY-NOTICES.txt`
  and `distribution/notices.txt`).
- Fixes taken from data mods, credited.
- Compatibility with mods that expect lStewieAl's Tweaks, as fix rows: reading their drop-in files
  (`Data\NVSE\Plugins\Tweaks\INIs`, `Data\NVSE\Plugins\Tweaks\GameSettings`), doing the behaviours
  those files name, and answering the commands and checks such mods use (`StewieGetValue`,
  `GetPluginVersion "lStewieAl's Tweaks"`). It is built only from those mods' own files and
  download pages and from behaviour observed in the original game with those mods and Tweaks
  installed, never from any of Tweaks' own files (its code, DLL, readme or its own INI).

Not accepted:

- Code written from lStewieAl's Tweaks itself (its code, DLL, readme or its own INI).
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
2. Never invent game behaviour. Translate from the executable at a named address (a mod's
   behaviour: from the sources [What can and can't be accepted](#what-can-and-cant-be-accepted)
   names), or leave the part out and say so.
3. Mark every translated function with its address and `FalloutNV.exe 1.4.0.525`. Never write
   Ghidra's automatic names (`FUN_…`, `DAT_…`, `LAB_…`).
4. Never commit game files, executable bytes, decompiler output, fnv_decomp's C++, saves, logs,
   local paths or user names.
5. Every fix gets a regression test whose inputs are built in the test.
6. Run the checks above on what you changed and report the real results. Never say a test passed
   unless you ran it and saw it pass; say what you could not run.
7. Keep code working on Windows, Linux and macOS. No Windows-only code without the other systems'
   path in the same change.
8. Don't add external crates. Don't write code from lStewieAl's Tweaks itself (its code, DLL,
   readme or its own INI), closed-source plugins or unlicensed code. Compatibility with mods that
   expect Tweaks is fine when it is built only from those mods' own files and from behaviour
   observed in the game.
9. Say in the pull request that an AI agent helped and with what. A person reviews it and owns it.

## Licence

nv-rs is licensed under the GNU General Public License v3.0 ([LICENSE](LICENSE)); by contributing
you agree that your contribution is licensed the same way. [LEGAL.md](LEGAL.md) explains what the
project contains and what it doesn't.
