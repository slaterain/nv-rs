# Ghidra scripts for the private research workflow

These are Java scripts for Ghidra 12.1.x's headless mode (`analyzeHeadless`).
They implement steps 1 to 3 of [docs/METHODOLOGY.md](../../docs/METHODOLOGY.md):
a reproducible record of which executable was analyzed, a safe way to put
names into a copy of the project, and an export of every function that
cannot silently leave one out.

Nothing here contains game data, and nothing here runs against the game
installation. The scripts read and (for two of them) change a Ghidra project
that you built yourself from your own executable.

## Rules for everything that comes out of these scripts

- **The output is private.** Exports hold decompiled C and disassembly of a
  program you own; cards, name reports and the identity file describe it.
  Keep them in the private research tree (`%USERPROFILE%\nv-re\...`).
  Never commit them, paste them into the repository or put them in a pull
  request. Only facts that you have re-derived and written in your own words
  belong in the repository.
- **The name scripts change the project, so use a copy.** Run the export,
  card and identity scripts with `-readOnly` against the project. Run
  `NvLabelCommandTables.java` and `NvImportNameMap.java` on a copy of the
  project, without `-readOnly`, and review the result before anyone adopts it
  (the methodology keeps a single writer for the canonical database).
- **Imported names are hypotheses.** Use only names whose source and version
  you can state (`source` and `pin` in the name map). The methodology
  explains which sources are excluded and why; follow it before importing
  anything from outside the executable.

## The scripts

All scripts live in this directory. Ghidra compiles them when they run.
`NvCommon.java` is not a script: it holds helper code the others share, so
keep it next to them. Arguments are `key=value` pairs after the script name.
Unknown or malformed arguments make the script throw before it writes
anything.

| Script | What it is for | Methodology step |
| --- | --- | --- |
| `NvExeIdentity.java` | Writes a JSON record of the executable: SHA-256 and MD5, image base, every memory block, PE header facts, CodeView/PDB information, function counts. | 1 |
| `NvLabelCommandTables.java` | Names the execute, parse and eval handlers of a command table from the table's own entries. Changes the project. | 2 |
| `NvImportNameMap.java` | Imports `address,name,source,pin` rows with provenance tags. Changes the project. | 2 |
| `NvExportProgram.java` | Writes every function (metadata, callers, callees, references, strings, decompiled C, optional disassembly) plus a manifest that checks the export is complete. | 3 |
| `NvFunctionCard.java` | Writes a Markdown and JSON card per function: constants decoded by machine, instruction classes, CPU-fidelity tier. | 3 |

### `NvExeIdentity.java` `out=<file.json>`

Records `program_name`, `executable_path`, `executable_format`, `sha256`,
`md5` (from Ghidra's import record), `image_base`, `ghidra_version`,
function counts, every memory block (name, start, end, size, `r/w/x` flags,
initialized), and the PE header read from the program's header memory:
TimeDateStamp, Characteristics including the `LARGE_ADDRESS_AWARE` bit,
SizeOfImage, the debug directory and the CodeView record (GUID, age, PDB
path). It also copies the PDB fields that Ghidra's loader stored in the
program options. The Rich header is not recorded. Compare `sha256` with
`Get-FileHash` of the file you imported.

### `NvExportProgram.java` `out=<dir>`

Options: `threads=N` (default the smaller of 8 and the CPU count),
`timeout=<seconds per function>` (default 60), `decompile=1|0` (default 1),
`disasm=0|1` (default 0), `start=<hex> end=<hex>` (only functions whose entry
is in the range).

Writes `functions.jsonl` (one JSON object per line, sorted by entry address,
byte-identical whatever the thread count) and `manifest.json`.

Each record has: `entry`, `name`, `namespace`, `name_source` (Ghidra's
source type, so `DEFAULT` means nobody named it), `tags`, `body` (address
ranges), `size`, `prototype`, `calling_convention`, `param_count`,
`stack_purge`, `is_thunk`, `thunk_target`, `callers`, `callees` (entry
addresses; imported functions appear as `ext:<library>::<name>`),
`data_refs` (address, reference type, label when it has a real name),
`strings` (address and value, cut to 200 characters), `decompile_status`
(`ok`, `timeout`, `error` or `skipped`), `decompile_error`, `c` (the
decompiled text) and, with `disasm=1`, `disasm`.

**Why the export is complete by construction.** The script walks
`FunctionManager.getFunctions(true)` and writes one record for each function,
even one whose decompile fails. `getFunctionCount()` counts external
(imported) functions too, so the gate is
`records_written == function_count_api - external functions`. If the two
differ the script throws, writes `manifest.failed.json` instead of
`manifest.json`, and leaves no `manifest.json` behind. Check that
`manifest.json` exists before using an export.

**Matching counts are not enough.** Code that Ghidra never made into a
function is invisible to that walk. The manifest therefore also reports:

- `uncovered_executable`: bytes in initialized executable blocks that are in
  no function. Runs made only of padding (`0x00`, `0x90`, `0xCC` or
  multi-byte x86 NOPs) are counted separately; the other runs are listed
  (`has_instructions` says whether Ghidra disassembled them).
- `vtable_checks`: for every symbol whose name is or ends with `vftable`
  (what Ghidra's MSVC RTTI analysis creates), the 4-byte slots are walked
  while they point into executable memory, and each slot whose target is not
  the entry of a defined function is listed with the reason.

The manifest also holds the exe SHA-256, Ghidra version, program name, image
base, status counts, the SHA-256 of `functions.jsonl`, the options used and
Ghidra's analysis options.

### `NvFunctionCard.java` `addr=<hex>[,<hex>...] out=<dir>`

Writes `<entry>.md` and `<entry>.json` per function. Every address must be
the entry of a function, or the script throws before writing any card.

A card lists the prototype, callers and callees (with names), the globals
the function touches (address, label, access size, current bytes in the
image, and the same bytes read as u32/i32/f32, or u64/i64/f64 and 80-bit
where the size allows), and:

- **Float constants, decoded by machine.** Every memory operand read by an
  x87 (`F...`) or SSE float instruction is decoded at the access size the
  instruction uses: 4 bytes as f32, 8 as f64, 10 as 80-bit extended, 16 as
  four f32 and two f64. Integer operands (`FILD`, `CVTSI2SS`), the x87
  control word and `MXCSR` are decoded as such. A constant that lives in a
  writable block is flagged, because it is only the value in the image.
- **Possible float immediates.** 32-bit immediates that would be sensible
  f32 values (finite, magnitude between 1e-4 and 1e8). Many integers look
  like that by chance, so they are only flagged `possible float`.
- **Instruction classes** (x87 arithmetic, compare, store-to-int,
  transcendental, load/store, control; SSE arithmetic, approximation,
  move/logic, control; `FLDCW`, `LDMXCSR`; direct and indirect calls).
- **CPU-fidelity tier**, the highest class present:
  - A: integer and/or SSE arithmetic and square root only.
  - B: x87 arithmetic, compares or float-to-int stores.
  - C: SSE reciprocal approximations (`RSQRTSS`, `RSQRTPS`, `RCPSS`, `RCPPS`).
  - D: x87 transcendentals (`FSIN`, `FCOS`, `FSINCOS`, `FPATAN`, `FPTAN`,
    `F2XM1`, `FYL2X`, `FYL2XP1`, `FSCALE`) or a direct call to a CRT
    transcendental function recognised by name.

  `tier` counts the function's own code and names; `tier_with_callees` also
  scans the direct callees one level down, which is how a wrapper around a
  stripped CRT `sin` is caught. The tier is a heuristic for choosing how to
  test a function, not a proof.
- **`stateful`**: true when the function writes a global (a write reference
  to non-code memory) or calls another function. Writes through pointers are
  not detected.

### `NvLabelCommandTables.java` `table=<hex>` (`count=<n>` or `end=<hex>`) `report=<csv>`

Options: `stride=40`, `prefix=Cmd_`, `dry=1`, `force=0`. `end` is the
address just past the last entry.

The entry layout for stride 40 is an ABI fact documented by xNVSE's
`CommandInfo`: `+0` long-name pointer, `+4` short-name pointer, `+8` opcode
(u32), `+12` help pointer, `+16` needs-parent (u16), `+18` parameter count
(u16), `+20` parameter list pointer, `+24` execute function, `+28` parse
function, `+32` eval function, `+36` flags.

**The stride is an argument because this repository's notes disagree.**
`docs/ENGINE_REFERENCE.md` describes console command entries as 48 bytes,
while the public xNVSE structure is 40 bytes. The script reads the same
field offsets whatever the stride, so a stride other than 40 is only a
probe: the right stride gives a valid long name for every real entry, and a
wrong one gives mostly invalid entries.

For each entry the long-name pointer must lead to a NUL-terminated printable
ASCII string, otherwise the entry is reported and skipped. For each
non-null execute, parse and eval pointer the script creates a function if
none exists (disassembling first) and renames it
`<prefix><LongName>_Execute`, `_Parse` or `_Eval`, with characters outside
`[A-Za-z0-9_]` turned into `_`. It renames only when the current name is a
Ghidra default (`SourceType.DEFAULT`), unless `force=1`. A pointer used by
more than one entry is not renamed, because a shared stub has no single
command name; it is reported. Functions the script creates or renames get
the tag `src:cmdtable`.

The report has one row per entry: `index,address,opcode,long,short,params,
execute,parse,eval,flags,action`. If anything throws, the script aborts its
own transaction and Ghidra discards its changes.

### `NvImportNameMap.java` `csv=<file>` `report=<csv>`

Options: `rva=0|1`, `dry=1`, `force=0`.

Input header: `address,name,source,pin[,kind]`.

| Column | Meaning |
| --- | --- |
| `address` | Hex virtual address; with `rva=1` it is an RVA and the image base is added. |
| `name` | The name, optionally qualified as `Class::Method` or `Foo<int>::Bar`. |
| `source` | Where the name came from (`rtti`, `xnvse`, `jip`, `jg`, `bgs`, `llm`, `own`, ...): 1 to 32 characters of `[A-Za-z0-9_.-]`. |
| `pin` | Commit hash or other version stamp of that source. Empty becomes `none`. |
| `kind` | `function` or `label`. Empty means function if one exists or can be created at the address, otherwise label. |

Sanitization: surrounding whitespace is trimmed and each run of whitespace
inside becomes one `_`. After that the name may only use
`[A-Za-z0-9_:~<>]`; anything else rejects the row. `::` separates
namespaces (not when inside `<...>`); missing namespaces are created.

Rules:

- A name that is not a Ghidra default is never overwritten unless `force=1`.
  The row is reported as `conflict`.
- Functions get the tags `src:<source>` and `pin:<first 12 characters of
  pin>`. Labels get an Info bookmark in category `nv-names` holding the
  source and pin.
- Every row is checked before anything changes: first its syntax, then a
  simulated run of the whole file. If any row fails, the report is written,
  the script throws, nothing is applied, and the other rows are marked
  `not-applied`. As a last safety net the real run uses its own transaction,
  which is aborted if something still goes wrong.
- `dry=1` reports what would happen and changes nothing.
- A CSV written by Windows PowerShell (byte-order mark, CRLF) is accepted.

The report columns are `row,address,requested_name,applied_name,kind,source,
pin,action,detail`. Actions: `applied`, `would-apply`, `unchanged`,
`conflict`, `rejected`, `not-applied`.

## Commands for the Windows machine

These were **not run on Windows** (the test container has no PowerShell or
Windows Ghidra). They are the same `analyzeHeadless` invocations the tests
make on Linux, written for PowerShell. Check the quoting once with a small
run first; avoid `%`, `|` and `^` in arguments, and prefer paths without
spaces.

```powershell
# Edit these to match your layout. Close Ghidra's GUI first: a project that
# is open there is locked.
$NvRe     = Join-Path $env:USERPROFILE 'nv-re'
$Headless = Join-Path $NvRe 'tools\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat'
$Project  = Join-Path $NvRe 'ghidra'                  # folder that holds FalloutNV.gpr
$ProjName = 'FalloutNV'
$Program  = 'FalloutNV_unpacked.exe'                  # program name inside the project
$Exe      = Join-Path $NvRe $Program                  # the file that was imported
$Scripts  = (Resolve-Path 'research\ghidra').Path     # run from the repository root

$sha = (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()
$Out = Join-Path $NvRe "exports\$($sha.Substring(0, 12))"   # private tree, never committed
New-Item -ItemType Directory -Force $Out | Out-Null

# analyzeHeadless exits with 0 even when a script throws, so look at the log.
function Invoke-Nv {
    param([string]$Log, [string[]]$HeadlessArgs)
    & $Headless @HeadlessArgs *> $Log
    if (Select-String -Path $Log -Pattern 'REPORT SCRIPT ERROR' -Quiet) {
        Get-Content $Log -Tail 30
        throw "A script failed; see $Log"
    }
}
```

### 1. Identity, full export and cards (read-only, on the canonical project)

```powershell
Invoke-Nv "$Out\identity-export.log" @(
    $Project, $ProjName, '-process', $Program, '-noanalysis', '-readOnly',
    '-scriptPath', $Scripts,
    '-postScript', 'NvExeIdentity.java', "out=$Out\identity.json",
    '-postScript', 'NvExportProgram.java', "out=$Out\export", 'threads=8')

# The identity hash must equal the file hash.
(Get-Content "$Out\identity.json" -Raw | ConvertFrom-Json).sha256 -eq $sha

# The export is only complete if manifest.json exists and its gate passed.
$m = Get-Content "$Out\export\manifest.json" -Raw | ConvertFrom-Json
$m.gate.passed; $m.records_written; $m.function_count_api; $m.status_counts
$m.uncovered_executable.uncovered_non_padding_bytes; $m.vtable_checks.bad_slots_total

# Cards for chosen functions (every address must be a function entry).
Invoke-Nv "$Out\cards.log" @(
    $Project, $ProjName, '-process', $Program, '-noanalysis', '-readOnly',
    '-scriptPath', $Scripts,
    '-postScript', 'NvFunctionCard.java', 'addr=005cc4f0,00c755e0', "out=$Out\cards")
```

The first command also needs the decompiler for every function, so expect a
long run for the whole program. Progress is printed every 10%. Use
`start=<hex> end=<hex> decompile=0 disasm=1` to export a range quickly.

### 2. Command tables and name maps (on a copy)

```powershell
$Work = Join-Path $NvRe 'ghidra-named'
Remove-Item $Work -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item $Project $Work -Recurse
Get-ChildItem $Work -Recurse -Force -Include '*.lock', '*.lock~' | Remove-Item -Force

# Check the entry stride first. Dry runs change nothing. The public xNVSE
# sources give the console table at 0x0118E8E0 and the script table at
# 0x01190910 for FalloutNV.exe 1.4.0.525.
foreach ($stride in 40, 48) {
    Invoke-Nv "$Out\stride$stride.log" @(
        $Work, $ProjName, '-process', $Program, '-noanalysis', '-readOnly',
        '-scriptPath', $Scripts,
        '-postScript', 'NvLabelCommandTables.java', 'table=0x0118E8E0', 'count=206',
        "stride=$stride", 'dry=1', "report=$Out\console-stride$stride.csv",
        '-postScript', 'NvLabelCommandTables.java', 'table=0x01190910', 'count=640',
        "stride=$stride", 'dry=1', "report=$Out\script-stride$stride.csv")
    Select-String -Path "$Out\stride$stride.log" -Pattern 'entries,'
}

# Label for real (no -readOnly), then review the reports.
Invoke-Nv "$Out\label.log" @(
    $Work, $ProjName, '-process', $Program, '-noanalysis',
    '-scriptPath', $Scripts,
    '-postScript', 'NvLabelCommandTables.java', 'table=0x0118E8E0', 'count=206',
    "report=$Out\console-labels.csv",
    '-postScript', 'NvLabelCommandTables.java', 'table=0x01190910', 'count=640',
    "report=$Out\script-labels.csv")

# Name map: dry run first, then apply.
Invoke-Nv "$Out\names-dry.log" @(
    $Work, $ProjName, '-process', $Program, '-noanalysis', '-readOnly',
    '-scriptPath', $Scripts,
    '-postScript', 'NvImportNameMap.java', "csv=$NvRe\names\names.csv", 'dry=1',
    "report=$Out\names-dry.csv")
Invoke-Nv "$Out\names.log" @(
    $Work, $ProjName, '-process', $Program, '-noanalysis',
    '-scriptPath', $Scripts,
    '-postScript', 'NvImportNameMap.java', "csv=$NvRe\names\names.csv",
    "report=$Out\names.csv")
```

Add `rva=1` when the CSV holds RVAs (the methodology notes that BGS uses
RVAs while xNVSE, JIP and JG use virtual addresses). Export the copy again
(command 1, pointed at `$Work`) to see the effect of the names.

### Reading the stride check

Each run prints `<n> entries, <v> valid, <s> skipped`. The script table has
640 slots, so some entries may legitimately be empty (null long-name
pointer); compare the two strides, not the absolute counts. The stride that
gives valid long names for the entries is the real one. If it is not 40,
update the script comment, `docs/METHODOLOGY.md` and the notes that
disagree, and record the evidence (executable hash, table address and the
report).

## Tests

```sh
research/ghidra/test/run-tests.sh
```

It needs Ghidra 12.1.x (`GHIDRA_INSTALL_DIR`; the Linux test container has
it in `/opt/tools/ghidra_12.1.4_PUBLIC`), a JDK, the MinGW i686 cross
compiler (`i686-w64-mingw32-gcc`, `-nm`, `-strip`) and Python 3. It writes
everything under `research/ghidra/target/` (ignored by git) and takes about
two minutes. `NV_TEST_RUN_FIXTURE=1` also runs the stripped fixture under
Wine as a check that it is a working program.

What it does:

1. Builds `test/fixture.c` (written for this test, not derived from any
   game) with `-O1 -msse2` into a 32-bit PE. It keeps an unstripped copy only
   to read symbol addresses with `nm`, strips the copy that Ghidra
   analyzes, and links with `--pdb` so the exe carries a CodeView record
   (the `.pdb` file itself is deleted, so Ghidra cannot load names from it).
2. Imports and analyzes the stripped exe in a throwaway project.
3. Runs every script with the same flags the commands above use (read-only
   for exports and cards, on a project copy for the name scripts) and
   checks the outputs with `test/check.py` (standard library only).

What the fixture contains: a table of 40-byte command entries with handlers
that only the table points to, one entry with a null name pointer and one
with a control character in the name, a parse stub shared by two entries,
functions that read float, double and 80-bit constants, an SSE function
(`ADDSS`, `SQRTSS`), `RSQRTSS` and `RCPSS` functions, an x87 `FSIN`
function, an `FLDCW` function, a function that reads a writable float
global, a `thiscall` method, an integer-only function, and a function
reachable only through a function-pointer table.

What the tests check (186 checks, all passing on the last run, in about
2 minutes):

- Identity JSON has the required fields; its SHA-256 and MD5 equal those of
  the analyzed file; TimeDateStamp, Characteristics (the large-address-aware
  bit), SizeOfImage and the CodeView GUID, age and path equal values parsed
  by an independent Python PE reader.
- The export manifest has `records_written == function_count_api` minus
  external functions, the SHA-256 of `functions.jsonl` matches the file, the
  records are sorted and complete, and the file is byte-identical with 4
  threads and with 1.
- A mutated copy of the export script that drops one function makes the
  gate throw and leaves no `manifest.json`. Another mutated copy whose
  decompiler never returns a result gives every function the status `error`
  and also leaves no `manifest.json`.
- Functions that only a data pointer reaches are not found by Ghidra's
  analysis; the uncovered-bytes report contains their addresses. After the
  command-table and name-map steps they appear in the export.
- Cards decode 3.5, 0.0174533, a double, an 80-bit constant and a possible
  float immediate correctly; the `RSQRTSS` and `RCPSS` functions are tier C,
  the `FSIN` function is tier D, x87 arithmetic is tier B, integer and SSE
  arithmetic is tier A, and the stateful flag is right for the cases tried.
- Command-table labeling reads the 40-byte layout, creates and names the
  handlers, skips the two bad entries, leaves the shared parse stub alone,
  does not change anything on a dry run, is idempotent, keeps a user name
  unless `force=1`, and shows a wrong stride (48) as mostly invalid entries.
- Name-map import names a stripped function, creates a function at a
  function-pointer-only address, creates labels with Info bookmarks, refuses
  to overwrite a user name without `force=1`, handles `rva=1`, a byte-order
  mark, sanitization and rejected rows (nothing applied), and adds the
  `src:` and `pin:` tags.
- Ghidra discards a script run's changes when a nested transaction is
  aborted, which both name scripts use as their last safety net.

## What was not tested here, and needs the real executable

The container has no game files and no `FalloutNV.exe`. The scripts were
tested only on the synthetic MinGW program above. On the Windows machine:

- **The command-table stride and addresses.** Run the stride check above on
  `FalloutNV.exe` 1.4.0.525 (console table `0x0118E8E0`, script table
  `0x01190910`, both from public xNVSE sources, not verified by us). The
  fixture proves the layout code reads a 40-byte layout correctly; it cannot
  say what the real table uses.
- **Scale and time.** The fixture has 115 functions. A full export of the
  real program has tens of thousands; the time, memory use and the number of
  `timeout` and `error` statuses are unmeasured. The `timeout` and `error`
  paths were not exercised (every fixture function decompiles).
- **MSVC code.** The fixture is compiled with GCC. The tier heuristics,
  padding detection and the `vftable` check have not been run on
  MSVC-compiled code or on vtables labeled by Ghidra's RTTI analysis; the
  `vftable` check was exercised with a hand-made label.
- **PDB information.** The identity script was checked against a CodeView
  `RSDS` record produced by the MinGW linker. The `NB10` form is untested.
  The real executable's record, its Rich header and its unpacking (a Steam
  wrapper changes the file) are not covered.
- **Windows.** `analyzeHeadless.bat`, PowerShell quoting, project copying
  with lock files, and the Windows decompiler binary were not run.
- **Cards for hand-written assembly or LTCG code.** Custom register
  conventions and indirect calls are not followed.
