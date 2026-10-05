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
  belong in the repository. The identity file writes the recorded exe path
  with your user-profile folder replaced by `%USERPROFILE%`, but treat it as
  private all the same.
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
anything. `analyzeHeadless` exits with 0 even when a script throws, so look
for `REPORT SCRIPT ERROR` in its log (the PowerShell helper below does).

Ghidra compiles every `.java` file below a script directory, sub-folders
included (so `test/` is compiled with the scripts), and when two files define
the same class it keeps only one of them. Never put a copy or a modified copy
of a script anywhere below `research/ghidra`, `target/` included: a mutated
copy there once replaced the real export script in a later run. The tests
build their modified copies in a temporary folder outside the repository, and
check at the end that `target/` holds no `.java` file. Also run the commands
from a folder that has no file named like a script (the repository root is
fine), because Ghidra reads `NvExportProgram.java` as a path before it
searches the script directories.

| Script | What it is for | Methodology step |
| --- | --- | --- |
| `NvExeIdentity.java` | Writes a JSON record of the executable: SHA-256 and MD5, image base, memory blocks, PE header facts, section table with raw-section hashes, Rich header, CodeView/PDB information, function counts. | 1 |
| `NvLabelCommandTables.java` | Names the execute, parse and eval handlers of a command table from the table's own entries. Changes the project. | 2 |
| `NvImportNameMap.java` | Imports `address,name,source,pin` rows with provenance tags. Changes the project. | 2 |
| `NvExportProgram.java` | Writes every function (metadata, callers, callees, references, strings, vtable slots, decompiled C, disassembly) plus a manifest that checks the export is complete. | 3 |
| `NvFunctionCard.java` | Writes a Markdown and JSON card per function: constants decoded by machine, instruction classes, CPU-fidelity tier. | 3 |

### `NvExeIdentity.java` `out=<file.json>`

Records `program_name`, `executable_path`, `executable_file_name`,
`executable_format`, `sha256`, `md5` (from Ghidra's import record),
`image_base`, `ghidra_version`, function counts, every memory block (name,
start, end, size, `r/w/x` flags, initialized), and the PE header read from
the program's header memory:

- TimeDateStamp, Characteristics including the `LARGE_ADDRESS_AWARE` bit,
  SizeOfImage, subsystem, checksum, entry point and the debug directory.
- The CodeView record (GUID, age, PDB path), read both from the debug
  directory and from the program options that Ghidra's loader filled in.
- `rich_header`: product id, build and use count of each entry, the XOR key,
  and whether the key equals the checksum the linker computes (`checksum_valid`).
  `{"present": false}` when the file has none.
- `section_table`: for each section the name, VirtualSize, VirtualAddress,
  SizeOfRawData, PointerToRawData, Characteristics, `raw_in_file` (whether
  the raw range lies inside the file) and `raw_sha256`, the SHA-256 of the
  section's raw bytes in the file. These are what a comparison of two builds
  (for example a Steam and a GOG exe) needs.

`executable_path` is the path Ghidra recorded at import, with the user-profile
folder replaced by `%USERPROFILE%`. The raw-section hashes need the file, not
only the program: the script reads it from the recorded path and uses it
only if its SHA-256 equals the recorded one. `section_hash_status` says what
happened, and `raw_sha256` is `null` when the file could not be used (moved,
or changed since the import). Compare `sha256` with `Get-FileHash` of the file
you imported.

Not recorded: the hash of the original, packed Steam exe (Ghidra only knows
the file that was imported, so record `Get-FileHash` of the original
yourself), the certificate table, the import table, version resources, any
data after the last section, and the names of the Rich header's product ids.

### `NvExportProgram.java` `out=<dir>`

Options: `threads=N` (default the smaller of 8 and the CPU count),
`timeout=<seconds per function>` (default 60), `decompile=1|0` (default 1),
`disasm=1|0` (default 1; `0` leaves the disassembly out and makes the file
smaller), `start=<hex> end=<hex>` (only functions whose entry is in the
range).

Writes `functions.jsonl` (one JSON object per line, sorted by entry address,
byte-identical whatever the thread count) and `manifest.json`.

Each record has: `entry`, `name`, `namespace`, `class`, `name_source`
(Ghidra's source type, so `DEFAULT` means nobody named it), `tags` (the
`src:` and `pin:` tags that say where the name came from), `body` (address
ranges), `size`, `prototype`, `calling_convention`, `param_count`,
`stack_purge`, `is_thunk`, `thunk_target`, `vtable_slots`, `callers`,
`callees` (entry addresses; imported functions appear as
`ext:<library>::<name>`), `data_refs`, `strings` (address and value, cut to
200 characters), `decompile_status` (`ok`, `timeout`, `error` or `skipped`),
`decompile_error`, `c` (the decompiled text) and `disasm`.

- `class` is the parent namespace when Ghidra made it a class (which is what
  its MSVC RTTI analysis does), otherwise empty. `NvImportNameMap` creates
  plain namespaces, so a name such as `Foo::Bar` imported from a CSV shows
  `namespace` `Foo` and an empty `class`.
- `vtable_slots` lists `{table, table_address, table_class, slot}` for every
  slot of every `vftable` symbol (see below) that points at the function.
- `data_refs` holds `address`, reference `type`, `label` (only a real name,
  not a Ghidra placeholder) and `provenance`, the source and pin that
  `NvImportNameMap` stored for a label it created at that address (empty
  when there is none). The provenance of a function itself is in `tags`.

**Why the export is complete by construction.** The script walks
`FunctionManager.getFunctions(true)` and writes one record for each function,
even one whose decompile fails. `getFunctionCount()` counts external
(imported) functions too, so the gate is
`records_written == function_count_api - external functions`. If the two
differ the script throws, writes `manifest.failed.json` instead of
`manifest.json`, and leaves no `manifest.json` behind. It also throws, with
the same file layout, when decompiling was requested and no function
decompiled. Check that `manifest.json` exists before using an export.

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

Option: `depth=N` (1 to 32, default 8), how many call levels the tier looks
down.

Writes `<entry>.md` and `<entry>.json` per function. Every address must be
the entry of a function, or the script throws before writing any card.

A card lists the prototype, callers and callees (with names), the globals
the function touches (address, label, access size, current bytes in the
image, and the same bytes read as u32/i32/f32, or u64/i64/f64 and 80-bit
where the size allows), and:

- **Float constants, decoded by machine.** Every memory operand read by an
  x87 (`F...`) or SSE float instruction is decoded at the access size the
  instruction uses: 4 bytes as f32, 8 as f64, 10 as 80-bit extended, 16 as
  four f32 and two f64. A conversion reads the type that comes first in its
  name, not the one it ends with: `CVTSD2SS` reads a double, `CVTSS2SD` a
  float, `CVTPS2PD` two floats, `CVTPD2PS` two doubles, `CVTDQ2PD` two
  32-bit integers. The memory forms of `CVTTSS2SI`, `CVTTSD2SI` and the
  compares count as SSE although they have no XMM operand. Integer operands
  (`FILD`, `CVTSI2SS`), the x87 control word and `MXCSR` are decoded as such.
  A constant that lives in a writable block is flagged, because it is only
  the value in the image.
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

  `tier` counts the function's own code and the names of the routines it
  calls. `tier_with_callees` also walks the direct callees, their callees and
  so on down to `depth` levels (each function once, shortest call chain
  first), and applies the instruction classes and the routine names at every
  level. A caller of a wrapper around a routine that FunctionID or a name
  import called `_CIsin` is therefore tier D, however many wrappers sit in
  between. `tier_callee_functions_scanned` says how many functions were
  looked at.

  **The tier can be too low.** `tier_is_lower_bound` is `true`, with the
  reasons in `tier_lower_bound_reasons`, when something that could raise the
  tier was not looked at: an indirect call (the target is unknown), a direct
  call to an address that has no function, an imported routine with no usable
  name (`Ordinal_<n>`), callees below the depth limit, or more than 5000
  callees. Writes through pointers and code reached through jump tables are
  not followed. The tier is a heuristic for choosing how to test a function,
  not a proof; a card with `tier_is_lower_bound` set says "a lower bound".
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
Ghidra default (`SourceType.DEFAULT`), unless `force=1`.

- A pointer used by more than one entry is not renamed, because a shared stub
  has no single command name; it is reported and counted once.
- A pointer that one entry uses for two roles (the same function as execute
  and eval) is named for the first role and keeps that name; the second role
  is reported as `also used as execute, keeps <name>`, even with `force=1`.
- Functions the script creates or renames get the tag `src:cmdtable`. When
  `force=1` replaces a name that was not a Ghidra default, the function's
  earlier `src:` and `pin:` tags are removed first, so the tags describe the
  source of the name it has now.
- `dry=1` changes nothing. It keeps track of the functions it would create
  and the names it would give, so its report and counts match the real run on
  the same program (the tests compare them row by row).

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
- **Duplicate names are refused**, because Ghidra itself would accept two
  functions with one name and a lookup by name would then be ambiguous.
  - Two rows that give one name to different addresses are both rejected
    (checked across the whole file, in the same namespace).
  - A name that the program already uses at another address is a `conflict`
    that names the other address, and the row is left alone.
  - `force=1` allows both; the program then has two symbols with that name,
    and the report says so. One address given two different names in the
    file is always rejected.
  - Rows that agree (same name, same address, other source) are fine: the
    first applies the name, the later ones are `unchanged` and add their
    tags.
- Functions get the tags `src:<source>` and `pin:<first 12 characters of
  pin>`. When `force=1` replaces a name that was not a Ghidra default, the
  function's earlier `src:` and `pin:` tags are removed first, so the tags
  say where the current name came from. Labels get an Info bookmark in
  category `nv-names` holding the source and pin; `NvExportProgram` shows it
  as `provenance`.
- Every row is checked before anything changes: first its syntax and the rows
  against each other, then a simulated run of the whole file. If any row
  fails, the report is written, the script throws, nothing is applied, and
  the other rows are marked `not-applied`. As a last safety net the real run
  uses its own transaction, which is aborted if something still goes wrong.
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

# The identity hash must equal the file hash, and the section hashes must
# have been computed from that file.
$id = Get-Content "$Out\identity.json" -Raw | ConvertFrom-Json
$id.sha256 -eq $sha
$id.pe_header.section_hash_status

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
`start=<hex> end=<hex> decompile=0` to export a range quickly.

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
four minutes. `NV_TEST_RUN_FIXTURE=1` also runs the stripped fixture under
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
   checks the outputs with `test/check.py` (standard library only), using
   small helper scripts in `test/` (`InspectNv`, `MakeClass`, `TxProbe`,
   `UnitProbe`) that are not part of the shipped tools.
4. Runs mutated copies of the scripts (made by `test/mutate.py`, which
   fails unless the text it replaces occurs exactly once) to show that the
   safety nets work. The copies, and the helper scripts, are written to a
   temporary folder outside the repository and removed at the end.

What the fixture contains: a table of 40-byte command entries with handlers
that only the table points to, one entry with a null name pointer and one
with a control character in the name, a parse stub shared by two entries, a
function that is both the execute and the eval handler of one entry,
functions that read float, double and 80-bit constants, an SSE function
(`ADDSS`, `SQRTSS`), `RSQRTSS` and `RCPSS` functions, an x87 `FSIN`
function and call chains around it of several depths, a pair of functions
that call each other, an `FLDCW` function, a function that reads a writable
float global, eight functions that read a constant through an SSE conversion
(`CVTSD2SS`, `CVTSS2SD`, `CVTTSS2SI`, `CVTTSD2SI`, `CVTPS2PD`, `CVTPD2PS`,
`CVTDQ2PD`, `CVTSI2SD`), a `thiscall` method, an integer-only function, and a
function reachable only through a function-pointer table.

What the tests check (the last run is recorded in the report that came with
this change; count the `ok` lines of a run to see the current number):

- Identity JSON has the required fields; its SHA-256 and MD5 equal those of
  the analyzed file; TimeDateStamp, Characteristics (the large-address-aware
  bit), SizeOfImage, the CodeView GUID, age and path, and the whole section
  table with the SHA-256 of every section's raw bytes equal values parsed by
  an independent Python PE reader. The recorded path has no user-profile
  folder. A copy of the fixture whose DOS stub holds a hand-made Rich header
  (MinGW makes none) shows the entries decoded and the checksum valid.
- The export manifest has `records_written == function_count_api` minus
  external functions, the SHA-256 of `functions.jsonl` matches the file, the
  records are sorted and complete (with `class`, `vtable_slots`, `provenance`
  and `disasm`), and the file is byte-identical with 4 threads and with 1.
  After the name steps the function in slot 1 of the `Fixture::vftable`
  label carries that slot, and the class is the parent namespace once the
  namespace has been turned into a class.
- A mutated copy of the export script that drops one function makes the
  gate throw and leaves no `manifest.json`.
- Three more mutated copies force the decompile outcomes other than `ok`
  (every fixture function decompiles, so the conditions were replaced):
  the decompiler returning no result at all, the result reported as timed
  out, and the result neither complete nor timed out. Each gives every
  function a record with the matching status and error text, and no
  `manifest.json`.
- Functions that only a data pointer reaches are not found by Ghidra's
  analysis; the uncovered-bytes report contains their addresses. After the
  command-table and name-map steps they appear in the export.
- Cards decode 3.5, 0.0174533, a double, an 80-bit constant and a possible
  float immediate correctly. They decode the constants of the eight SSE
  conversion forms at the right type and size (the double 1.25 for
  `CVTSD2SS`, the float 0.75 for `CVTSS2SD`, the pair {1.5, -2.0} for
  `CVTPS2PD`, and so on) and count the memory forms of `CVTTSS2SI` and
  `CVTTSD2SI` as SSE. The `RSQRTSS` and `RCPSS` functions are tier C, the
  `FSIN` function is tier D, x87 arithmetic is tier B, integer and SSE
  arithmetic is tier A, and the stateful flag is right for the cases tried.
- Tier through callees: an integer-only caller of a wrapper around `FSIN`
  is tier D two levels down; the same code eleven levels down is not seen
  at the default depth 8 (the tier is a lower bound, and says so) and is seen
  with `depth=12`; a pair of functions that call each other is walked once;
  and with the integer-only stub named `_CIsin` by the name import, both the
  wrapper and its caller are tier D, though the code of all three is
  integer-only.
- Command-table labeling reads the 40-byte layout, creates and names the
  handlers, skips the two bad entries, leaves the shared parse stub alone,
  names a pointer used for two roles once, does not change anything on a dry
  run, predicts the real run exactly (also with `force=1`), is idempotent,
  keeps a user name unless `force=1`, and shows a wrong stride (48) as mostly
  invalid entries.
- Name-map import names a stripped function, creates a function at a
  function-pointer-only address, creates labels with Info bookmarks, refuses
  to overwrite a user name without `force=1`, handles `rva=1`, a byte-order
  mark, sanitization and rejected rows (nothing applied), and adds the
  `src:` and `pin:` tags. Duplicate names: two rows that give one name to
  two addresses, and two names for one address, are rejected; a name already
  in the program is a conflict that names the other address; `force=1`
  allows the duplicate and says so; two sources that agree on a name add
  their tags.
- Provenance tags follow the name: after a forced rename by the other
  script the function carries only the tags of the script that set its final
  name.
- Ghidra discards a script run's changes when a nested transaction is
  aborted (`TxProbe`). Both name scripts rely on this, and the tests also
  make each of them fail while applying: copies of the scripts throw after
  an earlier row (or entries) were applied, and the saved project then has
  no rename, no function created, no tag and no bookmark left.
- `NvCommon.f80ToDouble` equals the correctly rounded double (computed with
  exact fractions in Python) for 300+ 80-bit values, including values on or
  next to the halfway point between two doubles, where rounding twice would
  give a different result; the cases are checked to include ones that the
  earlier method gets wrong. The path redaction handles Windows, macOS and
  Linux profile folders.

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
  `timeout` and `error` statuses are unmeasured.
- **Real decompiler failures.** No Ghidra decompile ever timed out or failed
  here. The "decompiler returned no result" branch, the timeout branch and
  the error branch of `NvExportProgram` were reached only by mutated copies
  of the script whose conditions were replaced. That shows what the script
  writes for each outcome, not that Ghidra reports a timeout or an error
  the way the script expects. In particular the error text is only checked
  for the fallback sentence, because Ghidra gave no message of its own.
- **MSVC code.** The fixture is compiled with GCC. The tier heuristics,
  padding detection and the `vftable` check have not been run on
  MSVC-compiled code or on vtables labeled by Ghidra's RTTI analysis; the
  `vftable` check was exercised with a hand-made label. No test used a
  function that Ghidra's FunctionID database named; the CRT-name case is
  made by importing a name.
- **PDB information and the Rich header.** The identity script was checked
  against a CodeView `RSDS` record produced by the MinGW linker and a Rich
  header written by the test. The `NB10` form is untested, and so are the
  real executable's record, its real Rich header and its unpacking (a Steam
  wrapper changes the file). The raw-section hashes need the executable to
  still be at the path Ghidra recorded; otherwise they are `null`.
- **Windows.** `analyzeHeadless.bat`, PowerShell quoting, project copying
  with lock files, and the Windows decompiler binary were not run.
- **Transaction abort is Ghidra behaviour.** It was verified on Ghidra
  12.1.4 (with `TxProbe` and with the injected failures) and could change in
  another version; the tests would then fail.
- **Cards for hand-written assembly or LTCG code.** Custom register
  conventions and indirect calls are not followed (the card says so in
  `tier_is_lower_bound`).
