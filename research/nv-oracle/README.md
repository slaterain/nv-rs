# nv-oracle

Ground-truth tools for 32-bit Windows. They answer the question "what does
the original program really do?" by running or watching the original code
on the real CPU, so Rust ports can be checked against facts instead of
guesses. They are step 6 ("Oracles") of the decompilation and rewrite
methodology.

The tools never contain or produce game code for the repository. They read a
private copy of the user's own game exe, and what they write is a private
recording (see "Safety" below).

This directory is its own Cargo workspace. It has no external crates, only
`std` and `core::arch`, and it is not part of the main nv-rs workspace or the
viewer.

## What each tool is for

| Tool | What it does | Typical use |
| --- | --- | --- |
| `nv-call` | Maps the private unpacked exe into its own process at its preferred base and calls one function by address on the real CPU, with the FPU control word and MXCSR you choose. Writes registers, x87 and SSE results and memory buffers. | Golden vectors for pure or nearly pure functions: integer logic, SSE arithmetic, x87 math, and the cases software emulators get wrong (reciprocal square root estimates, x87 transcendentals, precision-control and rounding effects). |
| `nv-probe` | A DLL that sits inside a running game. It patches the entry of functions you list and logs each call's arguments, memory it points at, and (optionally) the return value, as JSON lines. | Live state that cannot be derived offline: real arguments in the opening sequence, a struct's bytes before and after a call, post-INI values of global settings. |
| `nv-inject` | Starts a program suspended and loads a DLL into it, or loads a DLL into a running process. | Putting `nv-probe` into the game on a private copy. |

How they map to the milestone blockers: the Doc look-IK work in
[../../docs/OPENING_LOOK_IK.md](../../docs/OPENING_LOOK_IK.md) needs two
kinds of evidence this directory provides. The helper that normalises a
quaternion with `RSQRTSS` plus one Newton step produces SSE-approximation
results that only the real CPU can confirm bit for bit; `nv-call` records
those as vectors, including which CPU produced them. The angle cap and the
`fAngleMax:LookIK` values depend on post-INI numbers and on real arguments;
`nv-probe` can log a global's value (`dump=0xADDRESS:4`, or
`dump=[0xADDRESS]+0x760:4` for a field of the object a global points to) and the arguments
and returned rotation of a helper on every call. The same pattern fits any
function whose address and calling convention have been confirmed in Ghidra.

Calling conventions and addresses are never guessed by the tools. Take them
from the disassembly, and note that link-time optimisation can create
custom register conventions (see "Known limitations").

## Safety

- Work on a private copy, never on the real installation. Build an "oracle
  install" by copying the game folder, or inject into a running game. Do not
  put these programs, DLLs, manifests or logs into the real game folder.
- Everything the tools write is a private recording: `nv-call` results,
  `nv-probe` logs, snapshot regions. They can contain game data. Do not
  commit them. Commit only small curated test vectors and the facts derived
  from them (addresses with the exe build, constants with units), after
  deciding with the maintainer that they count as acceptable recordings.
  `.gitignore` here already excludes `*.jsonl` and `*.bin`.
- The tools check what they run against: `nv-call` reports the SHA-256 of the
  image in its header, and `nv-probe` refuses to install any hook when the
  host exe's SHA-256 differs from `host_sha256` in its manifest, or when the
  bytes at a hook address differ from the bytes you expected. Every artifact
  derived from a run should carry that exe hash.
- `nv-probe` refuses to load in the editor (`isEditor != 0`).
- Antivirus programs may flag any DLL injector. Use an exclusion for the
  private oracle folder rather than weakening protection elsewhere.
- Launching the game exe directly may fail with a Steam error. Setting
  `SteamAppId=22380` in the environment before running `nv-inject --launch`
  is the first thing to try; it is inherited by the launched program.

## Building

On the maintainer's Windows machine (PowerShell), from this folder:

```powershell
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
```

The programs land in `target\i686-pc-windows-msvc\release\`:

- `nv-call.exe` and `nv_call_engine.dll` (keep them in the same folder),
- `nv-inject.exe` and `nv_probe.dll`.

The tools must be 32-bit because they load, call and hook 32-bit code. Use
the MSVC target on Windows. The workspace is only tested with the MinGW
target below; the MSVC build is expected to work (the build script passes
the MSVC linker options) but has not been run.

On Linux, for the Wine tests (needs the MinGW i686 cross toolchain):

```sh
rustup target add i686-pc-windows-gnu
cargo build --release --target i686-pc-windows-gnu
```

`.cargo/config.toml` sets the MinGW linker for that target.

`NV_CALL_RESERVE_MB` (build-time environment variable, default 24) sets how
much address space `nv-call.exe` reserves at 0x00400000 (see below). The
default covers 0x00400000 to 0x01C00000.

The link addresses are set by the build scripts: `nv-call.exe` at 0x00400000
and `nv_call_engine.dll` at 0x10000000, both with address randomisation off
(`-Wl,--image-base=... -Wl,--disable-dynamicbase` for MinGW,
`/BASE:... /DYNAMICBASE:NO` for MSVC). Do not change them without reading
"Why there are two files" below.

`cargo test` on any host runs the unit tests of `nv-oracle-core` and the
command-line quoting tests of `nv-inject`. On a Linux host the Windows-only
crates compile to empty programs.

## nv-call

```
nv-call <image.exe> <vectors.jsonl | -> [--snapshot <dir>] [--resolve-imports]
        [--fpcw <hex>] [--mxcsr <hex>] [--out <file>] [--arena <hex>]
```

### Why there are two files

The image being measured (the game exe) must be mapped at its preferred base,
normally 0x00400000, because the code contains absolute addresses. Every
Windows process starts with a stack, heaps and system data at low addresses
that are not under our control, so a program that asks for 0x00400000 after it
has started may find it taken (under Wine this always happens). `nv-call.exe`
is therefore only a small host, linked at 0x00400000 with a large zero-filled
placeholder, so the operating system reserves the whole range before
anything else can land in it. It loads `nv_call_engine.dll`, which is linked
at 0x10000000 (with ASLR off), and the engine replaces the placeholder with
the image. After that the host's own code no longer exists, so the engine ends
the process itself. If the image's range is not inside the placeholder, the
engine asks the system for it with `VirtualAlloc` and fails with a clear
message when it is taken (the message names what is there).

### What it does

1. Reads the image file, takes its SHA-256 and parses the PE32 headers.
2. Maps headers and sections at the preferred base. No relocations are
   applied and no CRT, TLS or static constructors are run: only what is in the
   file exists. Anything the game initialises at run time (singletons, post-INI
   settings) has to come from a snapshot.
3. Imports are not resolved by default. Every import-table slot points at a
   small per-import stub. If the called code reaches an import, the call stops
   and the result reports the import's name (`KERNEL32.dll!Sleep`) and the
   caller. `--resolve-imports` fills the table with this system's real DLL
   functions instead (anything missing keeps a stub). Resolving is only
   sensible for imports that are safe to run, such as simple kernel32 calls.
4. `--snapshot <dir>` applies every `<hexaddr>.bin` file in the folder after
   mapping (for example `011dea3c.bin`). A region outside the image is
   allocated at that exact address. The header lists each region with its size
   and SHA-256.
5. Runs each vector (below), one JSON line each, after one header line.

### Vector file (JSON lines)

```json
{"id":"lerp_1","fn":"0x00401230","cc":"cdecl",
 "args":["f32:1.0","f32:3.0","f32:0.25"],
 "buffers":{"obj":{"size":16,"init":"0100000002000000"}},
 "ret":"f32_x87","fpcw":"0x027F","mxcsr":"0x1F80","regs":{"esi":"0x5"}}
```

| Field | Meaning |
| --- | --- |
| `id` | Name copied into the result. |
| `fn` | Function address (hex string or number). |
| `cc` | `cdecl` (default), `stdcall`, `thiscall` or `fastcall`. |
| `args` | Strings written `kind:value`, placed by the convention: `i8 u8 i16 u16 i32 u32 bool i64 u64` (decimal or `0x` hex, negative allowed); `f32`/`f64` (a decimal, or `0x` and the exact bit pattern such as `f32:0x3fc00000`); `ptr:NAME`, `ptr:NAME+0x10` (address of a buffer), `ptr:0x1234` or `ptr:null`; `raw:0x3f800000,0x1` (raw dwords pushed as they are). For `thiscall` the first argument is `this` (ECX); for `fastcall` the first two integer-like single dwords go in ECX and EDX; floats and 64-bit values always go on the stack. |
| `buffers` | Named blocks, each `{"size": n, "init": "hex"}`, zero-filled beyond `init`. They are placed in an arena (at 0x30000000 when free, so stored pointers repeat between runs), made fresh for each vector, and reported after the call. |
| `ret` | `void i32 u32 ptr i64 f32_x87 f64_x87 xmm0` (default `i32`). Decides whether ST0 is read and how the value is summarised. |
| `fpcw`, `mxcsr` | Control words set before the call. Defaults: `0x027F` (53-bit precision, round to nearest, exceptions masked) and `0x1F80`, or the `--fpcw`/`--mxcsr` option. Set `0x007F` for a thread whose Direct3D device did not use `FPU_PRESERVE`. Direct3D 9 sets single precision on the creating thread without that flag; confirm what the game's thread really uses. |
| `regs` | Registers to preset, for custom conventions: `eax ecx edx ebx esi edi`, and `xmm0` to `xmm3` as hex bytes in memory order. A preset that conflicts with the convention's own argument registers is an error. |

Blank lines and lines starting with `#` are skipped. A bad line produces a
`{"type":"error",...}` line and the run continues. Exit status: 0 when every
vector ran (faults are results, not errors), 1 when the setup failed (image,
snapshot, address range, input file), 2 for a usage error or when any vector
line was rejected.

### Result line

```json
{"type":"result","id":"lerp_1","fn":"0x00401230","cc":"cdecl","ret":"f32_x87",
 "fpcw_in":"0x0000027F","mxcsr_in":"0x00001F80","fault":null,
 "regs":{"eax":"0x00000000","edx":"0x00000000",
         "st0":{"raw":"00000000000000c0ff3f","f32_bits":"0x3FC00000","f32":1.5,
                "f64_bits":"0x3FF8000000000000","f64":1.5},
         "fpu_depth":1,"xmm0":"000000...","fsw":"0x00003800","fcw":"0x0000027F",
         "mxcsr":"0x00001F80","callee_popped":0,"cleanup":"caller"},
 "value":{"f32_bits":"0x3FC00000","f32":1.5},"stack_bytes":12,"buffers":{}}
```

- `regs` is `null` after a fault. `fault` is `null` or
  `{"kind":"exception","code","name","eip","address","access"}` (access
  violations, illegal instructions, integer divide by zero, FPU traps that the
  control word unmasks, breakpoints) or
  `{"kind":"import","name","index","caller"}`.
- `st0.raw` is the 10-byte x87 register in memory order (significand first),
  captured only for x87 return kinds and only when the FPU stack is not
  empty. `f32_bits` rounds the 80-bit value straight to single precision, as
  `fstp dword` would; `f64_bits` rounds to double.
- `xmm0` is always captured (16 bytes, memory order).
- `fpu_depth` is how many x87 registers the callee left in use. A clean
  function that returns nothing in ST0 leaves 0.
- `callee_popped` and `cleanup` show who removed the stack arguments:
  `caller` (cdecl style), `callee` (`ret N`), `none` (no stack arguments) or
  `other`. This is a cheap check that the declared convention is right.
- The low six bits of `mxcsr` are sticky exception flags that SSE code may
  set; compare only the control bits.
- The header line has the CPU vendor and brand string (RSQRTSS and RCPSS
  estimates are reported to differ between Intel and AMD, so record which CPU
  made a vector), the image SHA-256 and base, the control words, import
  counts, the placeholder and the snapshot regions.

Faults inside a call are caught by a vectored exception handler that records
them, resets the FPU (`fninit`, control word restored) and returns to the
harness, so one bad vector does not stop the batch. C++ exceptions and
Windows' own debug-print exceptions are not touched.

## nv-probe

`nv_probe.dll` reads `nv-probe.txt` from its own folder. The full manifest
format is documented at the top of
[nv-oracle-core/src/manifest.rs](nv-oracle-core/src/manifest.rs); in short:

```
output = nv-probe-%PID%.jsonl
host_sha256 = <64 hex digits of the host exe file>
mode = inject
hook look_helper addr=0x00401000 cc=thiscall bytes="55 8B EC 83 E4 F0" steal=6 args=ptr,f32 ret=void dump=ecx+0x170:16,arg0+0x10:12 regs=1 max=500
```

Hook keys: `addr`, `cc`, `bytes` (what must be at the address), `steal`
(bytes to replace, at least 5 and ending on an instruction boundary, or
`auto`), `args` (types), `ret` (turns on return capture), `dump`
(`expr:length`, e.g. `ecx+0x170:16`, `arg0+0x10:12`, `[0x011dea3c]+0x760:4`,
`esp+4:8`), `regs=1` (log all eight registers) and `max` (records limit).
Settings: `output`, `host_sha256`, `mode`, `nvse_grace_ms`, `flush_ms`,
`suspend_threads`, `max_records`. A relative `output` is relative to the DLL's
folder. Mistakes in the manifest are reported with line numbers, and a hook
whose stolen bytes include something the decoder cannot relocate is a manifest
error. A manifest error installs nothing and is written to the log header
(`manifest_error`), in `nv-probe.jsonl` next to the DLL when `output` itself
could not be read. A hook whose expected bytes do not match memory is refused
on its own and the others still install.

### Loading it: injected or as an NVSE plugin

The DLL exports `NVSEPlugin_Query(const NVSEInterface*, PluginInfo*) -> bool`
and `NVSEPlugin_Load(const NVSEInterface*) -> bool`, using only these public
ABI facts: `PluginInfo = {u32 infoVersion = 1, const char *name, u32
version}` and `NVSEInterface` starting with `u32 nvseVersion, runtimeVersion,
editorVersion, isEditor`. Both refuse when `isEditor != 0`. It does not export
`NVSEPlugin_Version`; confirm on the real NVSE build that the legacy `Query`
path is accepted.

Which mode applies is the `mode` setting:

| `mode` | When the hooks are installed |
| --- | --- |
| `inject` | Inside `DllMain`, before `LoadLibrary` returns. Use with `nv-inject`. With `--launch` the program has not run a single instruction yet, so every call is seen. |
| `nvse` | Only in `NVSEPlugin_Load`. Use when the DLL sits in NVSE's plugin folder. |
| `auto` (default) | `DllMain` cannot tell the two apart, so a thread waits `nvse_grace_ms` (default 1500) for NVSE to call `Query` or `Load`. If NVSE calls, it does the work and the thread does nothing; if not, the thread installs the hooks itself. Nothing is installed twice. |

Unloading the DLL (`FreeLibrary`) puts the original bytes back. The generated
stubs are left allocated so a thread still inside one does not fault.

### How a hook works

1. The bytes at the address are compared with `bytes`; any difference refuses
   that hook (the log says where). The decoder checks that the stolen bytes end
   on an instruction boundary and contain no short branch; `E8`, `E9` and
   `0F 8x` with 32-bit displacements are relocated.
2. A trampoline is built in executable memory: the relocated stolen
   instructions and a jump back.
3. A per-hook entry stub saves flags, all integer registers and the FPU/SSE
   state (`FXSAVE`, 16-byte aligned), resets the FPU and MXCSR so the Rust
   logger runs in a known state, calls the logger with the hook number and the
   saved state, restores everything and continues in the trampoline.
4. With `ret=`, the logger replaces the return address with the hook's return
   stub and remembers the original on a per-thread shadow stack. The return
   stub logs EAX, EDX, ST0 (read from the `FXSAVE` image; ST0 is the register
   the status word's TOP field points at, valid if its tag bit is set) and
   XMM0, then returns to the original caller. It works for caller-cleaned and
   callee-cleaned conventions alike.
5. The patch is a 5-byte `jmp rel32` written with one locked 8-byte
   compare-and-exchange (keeping the three bytes that follow), after every other
   thread has been suspended (`suspend_threads = 1`). A suspended thread whose
   instruction pointer is inside the replaced bytes is moved to the same place
   in the trampoline. All memory the patch routine needs is allocated before
   any thread is stopped, so a suspended thread holding the heap lock cannot
   stall it.

### Log format (JSON lines)

If the output file cannot be created the probe installs nothing and leaves a
note, `nv-probe-error.txt`, next to the DLL.

- A header line with the origin, process id, host exe path and hash check, each
  hook's status (`installed` with the stolen byte count, or `refused` with the
  reason), how many threads were suspended and moved, or a `manifest_error`.
  It is written before any call record.
- `{"type":"call","seq","tid","hook","tick","args":[...],"regs":{...},"dumps":[...]}`.
  `seq` increases in file order across threads; `tick` is `GetTickCount`.
  Arguments are written by declared type, for example `{"t":"f32","bits":
  "0x3F800000","v":1.0}` or `{"t":"ptr","v":"0x0061FD90"}`. A dump is
  `{"expr","addr","hex"}`, or `{"expr","error"}` when the memory could not be
  read (readability is checked with `VirtualQuery`, so a bad pointer in the game
  becomes a logged error, not a crash).
- `{"type":"ret","seq","tid","hook","tick","call","eax","edx","ret"|"st0"|"xmm0",
  "dumps_after":[...]}` for hooks with `ret=`. `call` is the `seq` of the
  matching call record. `dumps_after` re-reads the same addresses that were
  resolved at entry, which is how a function's output structure is captured.
- Output is buffered and written when the buffer fills, when `flush_ms` has
  passed at the next record (`0` writes every record), on unload and at process
  exit. A crash can lose the last buffered records.

## nv-inject

```
nv-inject [--dll <path>] --launch <exe> [args...] [--wait]
nv-inject [--dll <path>] --pid <n>
```

The DLL defaults to `nv_probe.dll` next to `nv-inject.exe`. `--launch` creates
the program suspended, with the exe's folder as its working directory, loads
the DLL with `CreateRemoteThread(LoadLibraryW)`, then resumes it. `--wait`
waits for the exit and prints the exit code. The tool reports the module handle
or an error. It relies on kernel32 sitting at the same address in the target
as in `nv-inject.exe`, which Windows does for system DLLs within a boot
session. Both processes must be 32-bit.

## Typical workflows

Golden vectors for a helper: find the address and convention in Ghidra, write
`fn`, `cc`, `args` and `ret`, set `fpcw`/`mxcsr` to what the calling thread
really uses, run `nv-call`, and keep the `value` or `regs` fields that the Rust
test compares against. Mark the test with the CPU vendor from the header and
with the fidelity tier from the methodology (A and B must match bit for bit; C
and D against the oracle with a stated tolerance).

Live state: write an `nv-probe.txt` with `host_sha256`, the hooks and dumps,
start the private copy through `nv-inject --launch`, play to the point of
interest, and read the log. For a function that writes through a structure
pointer, dump the structure at entry and, with `ret=`, again at return.

State that `nv-call` needs from a running game: dump the regions with
`nv-probe` or a debugger into `<hexaddr>.bin` files and pass the folder as
`--snapshot`.

## Tests

`tests/run-wine-tests.sh` builds everything, builds a synthetic program from
`tests/fixture/target.c` (original code written for the test), and runs 16
groups, most of them under Wine. `PROFILE=debug` runs the same tests on
unoptimised builds with overflow checks. It needs `cargo`, `wine`, `python3`
and the MinGW i686 tools, and writes only under `target/wine-tests`.

- the decoder cross-check: the length decoder is run over every instruction
  of the fixture and of a few large 32-bit programs and must agree with
  `objdump` on each instruction it accepts (the fixture alone has about 9,000;
  with Wine's builtin libraries present, roughly 400,000; no disagreement);
- the program's own `selftest` calls each of its functions on this CPU
  (cdecl, stdcall, thiscall, fastcall; float returns through x87; an
  `RSQRTSS` quaternion normalise; a struct writer; fixed control words 24-,
  53- and 64-bit with different rounding), and every `nv-call` result is
  compared bit for bit: EAX, float bits, raw 80-bit ST0, XMM0 and buffers;
- `nv-call` on faults (null read, write, illegal instruction, divide by
  zero), import traps, register presets, a wrongly declared convention, bad
  input lines, snapshots (including a region outside the image) and resolved
  imports; the image at a different base; the clear failure when the address
  range is taken;
- `nv-probe` through `nv-inject --launch` (call counts, argument values by
  type, struct bytes before and after, return values, x87 return, recursion
  order, per-hook limit, refusal on wrong expected bytes, refusal on wrong host
  hash, reported manifest error, a hooked function whose stolen bytes contain
  a relative call, a hooked jump thunk), four threads calling a hooked recursive
  function, injection into a running process, patching without suspending
  threads, `auto` mode both with and without NVSE, and the NVSE exports
  (`Query`/`Load`, editor refusal, unload restoring the code);
- the unit tests of the portable crates, on the host and as 32-bit programs
  under Wine.

`cargo clippy --workspace --all-targets --target i686-pc-windows-gnu -- -D
warnings` is clean.

## What was tested here and what still needs the real game

Tested in this repository's environment: everything above, against a
synthetic program built from scratch, under Wine 9 on an Intel Xeon with the
MinGW i686 target. The hardware results (SSE, x87 including transcendentals)
are real CPU results because Wine runs the code natively; only the Windows
loader, exception dispatch and API behaviour are Wine's.

Not tested, and to be checked on the maintainer's Windows machine with the
real exe:

- the MSVC target build, and the maintainer's Windows version's loader (the
  placeholder, `VirtualProtect` over image pages, ASLR placement of other
  allocations);
- mapping the real unpacked exe (its size against `NV_CALL_RESERVE_MB`, its
  import table, its TLS and security-cookie use);
- real functions of the game, their conventions and which of them depend on
  state a snapshot must supply;
- loading `nv_probe.dll` through the real NVSE, and in the real game process
  with its own hooks, DRM and anti-tamper behaviour;
- the probe's thread-suspension patching against the game's real threads,
  and the overhead of hooks on hot functions;
- the Steam launch behaviour of `nv-inject --launch`.

## Known limitations

- Shadow stack: the return capture relies on every hooked call returning
  through the stub. A function left by an exception, `longjmp` or a thread
  killed inside it never reaches the stub. Stale entries are dropped the next
  time a hooked function is entered from a higher stack position, and that
  call's return record is simply never written. If a return stub cannot find
  its entry at all the original return address is lost, so the probe logs a
  `fatal` record and ends the process rather than jump somewhere wrong.
- Functions the decoder cannot handle: only a defined subset of instructions
  is accepted at a function's start (listed in
  [nv-oracle-core/src/decode.rs](nv-oracle-core/src/decode.rs)). A function
  whose first five bytes include a short branch, a `ret`, or anything else
  outside the subset is refused. A jump from elsewhere in the function into
  the middle of the replaced bytes cannot be detected and would break.
- Write race: the jump is one atomic 8-byte write and other threads are
  suspended, which removes the race for threads that exist at that moment. A
  thread created during the patch, or `suspend_threads = 0`, leaves a small
  window; patching early (`--launch`) avoids it. When patching a running
  process, a thread that is currently inside a call that is itself among the
  replaced bytes would return into the middle of the jump; only the
  instruction pointers of suspended threads are corrected.
- Custom conventions: link-time code generation can pass arguments in other
  registers. `nv-call` presets any register with `regs`, `nv-probe` logs all
  registers with `regs=1`, and dumps can use any register as a base, but typed
  argument decoding only follows the four standard conventions.
- `nv-call` runs the function on the harness's main thread, with the harness's
  stack and thread-local storage: code that uses thread-local slots, the game's
  allocator state, or anything a constructor sets up sees an empty world unless
  a snapshot provides it. It catches hardware faults, not C++ exceptions thrown
  out of the called code, and cannot recover from a callee that corrupts its
  own stack badly enough to break exception dispatch. Code that calls
  `ExitProcess` (through a resolved import) ends the harness.
- Unloading the probe while a thread is inside one of its stubs or handlers
  crashes that thread.
- Logging flushes on a timer only when another record arrives, not from a
  background thread, because waiting for a thread inside `DllMain` can
  deadlock the loader.
- Readability checks use `VirtualQuery`, so memory unmapped by another thread
  between the check and the read can still fault.
- Only one image can be mapped, at its preferred base, and only PE32 images
  for 32-bit x86. Delay-load and bound imports are not read.
