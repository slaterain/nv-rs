#!/usr/bin/env bash
# Tests the Ghidra scripts in research/ghidra against a synthetic 32-bit PE.
#
# What it does:
#   1. Builds test/fixture.c with the MinGW i686 cross compiler (-O1 -msse2),
#      keeps an unstripped copy only to read symbol addresses with nm, and
#      strips the copy that Ghidra analyzes.
#   2. Imports the stripped exe into a throwaway Ghidra project under
#      research/ghidra/target/ with analyzeHeadless.
#   3. Runs each script (read-only for the exports, on a project copy for the
#      labeling scripts) and checks the outputs with test/check.py.
#   4. Runs mutated copies of the scripts (made with test/mutate.py) to show
#      that the safety nets work: the export gate, the decompiler check and
#      the rollback when applying names fails halfway.
#
# Needs: Ghidra 12.1.x (GHIDRA_INSTALL_DIR; the Linux test container has it in
# /opt/tools/ghidra_12.1.4_PUBLIC, which is the fallback),
# a JDK on PATH, i686-w64-mingw32-gcc/nm/strip, python3. No game files and no
# game executable are used or needed. Everything is written under target/.
#
# Usage: research/ghidra/test/run-tests.sh

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)" # research/ghidra
TARGET="$ROOT/target"
OUT="$TARGET/test-out"
LOGS="$OUT/logs"
# Ghidra 12.1.x install directory: GHIDRA_INSTALL_DIR, or the location the
# Linux test container uses.
GHIDRA="${GHIDRA_INSTALL_DIR:-/opt/tools/ghidra_12.1.4_PUBLIC}"
HEADLESS="$GHIDRA/support/analyzeHeadless"
CC="${NV_TEST_CC:-i686-w64-mingw32-gcc}"
NM="${NV_TEST_NM:-i686-w64-mingw32-nm}"
STRIP="${NV_TEST_STRIP:-i686-w64-mingw32-strip}"
PROJ_NAME=nvtest

for tool in "$HEADLESS" "$(command -v "$CC")" "$(command -v "$NM")" "$(command -v "$STRIP")" "$(command -v python3)"; do
    if [ -z "$tool" ] || [ ! -e "$tool" ]; then
        echo "missing tool: $tool (see the header of this script)" >&2
        exit 2
    fi
done

now_ms() { echo $(($(date +%s%N) / 1000000)); }
SUITE_START=$(now_ms)
declare -a TIMINGS=()
note_time() { # name start_ms
    TIMINGS+=("$(printf '%-34s %6d ms' "$1" $(($(now_ms) - $2)))")
}

fail() {
    echo "FAILED: $*" >&2
    exit 1
}

# run_ghidra <name> <ok|fail> <analyzeHeadless args...>
# Runs analyzeHeadless, keeps the log, and decides success from the log:
# analyzeHeadless exits 0 even when a script throws, so the log is the evidence.
run_ghidra() {
    local name="$1" expect="$2"
    shift 2
    local t0 rc=0 script_error=0
    t0=$(now_ms)
    "$HEADLESS" "$@" >"$LOGS/$name.log" 2>&1 || rc=$?
    note_time "ghidra: $name" "$t0"
    if grep -q "REPORT SCRIPT ERROR" "$LOGS/$name.log"; then
        script_error=1
    fi
    if [ "$rc" -ne 0 ]; then
        tail -30 "$LOGS/$name.log" >&2
        fail "analyzeHeadless exited with $rc ($name)"
    fi
    if [ "$expect" = ok ] && [ "$script_error" -eq 1 ]; then
        grep -A6 "REPORT SCRIPT ERROR" "$LOGS/$name.log" | head -20 >&2
        fail "a script failed in $name"
    fi
    if [ "$expect" = fail ] && [ "$script_error" -eq 0 ]; then
        fail "$name should have failed with a script error but did not"
    fi
    return 0
}

check() { # stages...
    local t0
    t0=$(now_ms)
    python3 "$HERE/check.py" "$@" --out "$OUT" --nm "$TARGET/nm.txt" --exe "$TARGET/nvfixture.exe" ||
        fail "check.py $*"
    note_time "check: $*" "$t0"
}

sym() { # symbol name in the fixture -> 8 hex digits
    local v
    v=$(awk -v s="_$1" '$3 == s { print $1; exit }' "$TARGET/nm.txt")
    [ -n "$v" ] || fail "symbol $1 not found in nm output"
    echo "$v"
}

# mutate <dir> <script.java> <anchor> <replacement>: puts a copy of a script with one exact text
# replaced into $TARGET/<dir>, next to a copy of NvCommon.java. mutate.py fails if the anchor
# is not found exactly once, so a moved anchor cannot turn a test into a no-op.
mutate() {
    local dir="$TARGET/$1"
    mkdir -p "$dir"
    cp "$ROOT/NvCommon.java" "$dir/"
    python3 "$HERE/mutate.py" "$ROOT/$2" "$dir/$2" "$3" "$4" || fail "mutation of $2 did not apply"
}

# ---------------------------------------------------------------- build
rm -rf "$TARGET"
mkdir -p "$TARGET" "$LOGS"
T0=$(now_ms)
(
    cd "$TARGET"
    # --pdb makes ld write a CodeView record (RSDS) into the exe so the
    # identity script has one to read. The .pdb itself is deleted straight
    # away: if it sat next to the exe, Ghidra would load names from it.
    "$CC" -O1 -msse2 -Wall -Wl,--large-address-aware -Wl,--pdb=nvfixture.pdb \
        -o fixture_unstripped.exe "$HERE/fixture.c"
    rm -f nvfixture.pdb
    cp fixture_unstripped.exe nvfixture.exe
    "$STRIP" nvfixture.exe
    "$NM" fixture_unstripped.exe >nm.txt
)
note_time "build fixture" "$T0"
echo "built $(wc -c <"$TARGET/nvfixture.exe") byte stripped exe; unstripped copy used only for nm"

# Optional: prove the fixture is a working PE by running the stripped exe under Wine.
if [ "${NV_TEST_RUN_FIXTURE:-0}" = 1 ] && command -v wine >/dev/null; then
    T0=$(now_ms)
    WINEDEBUG=-all WINEPREFIX="${WINEPREFIX:-$TARGET/wineprefix}" wine "$TARGET/nvfixture.exe" >"$LOGS/fixture-run.log" 2>&1 ||
        fail "the fixture exe did not run under Wine"
    grep -q '^fixture ' "$LOGS/fixture-run.log" || fail "unexpected fixture output"
    note_time "run fixture under wine" "$T0"
fi

# ---------------------------------------------------------------- import and analyze
mkdir -p "$TARGET/proj"
run_ghidra import ok "$TARGET/proj" "$PROJ_NAME" -import "$TARGET/nvfixture.exe" -overwrite
grep -q "Import succeeded" "$LOGS/import.log" || fail "import did not succeed"

SP=(-scriptPath "$ROOT")
RO=(-process nvfixture.exe -noanalysis -readOnly)
RW=(-process nvfixture.exe -noanalysis)

CARD_ADDRS=$(
    for n in fx_angle_to_unit fx_double_scale fx_extended fx_possible_float_immediate fx_sse_scalar fx_rsqrt \
        fx_rcp fx_sin fx_set_cw fx_sum fx_gain Obj_Compute main \
        fx_cvt_sd2ss fx_cvt_ss2sd fx_cvt_tss2si fx_cvt_tsd2si fx_cvt_ps2pd fx_cvt_pd2ps fx_cvt_dq2pd fx_cvt_si2sd \
        fx_sin_top fx_chain0 fx_chain8 fx_ping fx_crt_top; do
        printf '%s,' "$(sym $n)"
    done
)
CARD_ADDRS="${CARD_ADDRS%,}"

# ---------------------------------------------------------------- A: identity, export, cards (read-only)
run_ghidra read_only_scripts ok "$TARGET/proj" "$PROJ_NAME" "${RO[@]}" "${SP[@]}" \
    -postScript NvExeIdentity.java out="$OUT/identity.json" \
    -postScript NvExportProgram.java out="$OUT/export1" threads=3 \
    -postScript NvFunctionCard.java addr="$CARD_ADDRS" out="$OUT/cards" \
    -postScript NvFunctionCard.java addr="$(sym fx_chain0)" depth=12 out="$OUT/cards_deep"
check identity export1 cards

# A bad card address must fail loudly and write nothing.
run_ghidra card_bad_address fail "$TARGET/proj" "$PROJ_NAME" "${RO[@]}" "${SP[@]}" \
    -postScript NvFunctionCard.java addr="$(sym fx_sum),00000001" out="$OUT/cards_bad"
grep -q "no function starts at 00000001" "$LOGS/card_bad_address.log" || fail "card error text missing"
[ ! -e "$OUT/cards_bad" ] || fail "a card was written for a bad address list"
echo "  ok   card with an address that is not a function fails and writes nothing"

# The Rich header cannot come from MinGW, so the test writes one into a copy of the fixture's DOS
# stub, imports that copy into its own project and reads its identity.
check richgen
mkdir -p "$TARGET/proj_rich"
run_ghidra identity_rich ok "$TARGET/proj_rich" "$PROJ_NAME" -import "$OUT/nvrich.exe" -overwrite -noanalysis \
    "${SP[@]}" -postScript NvExeIdentity.java out="$OUT/identity_rich.json"
check rich

# ---------------------------------------------------------------- B: labeling on a copy of the project
cp -a "$TARGET/proj" "$TARGET/proj_named"
TABLE=$(sym g_commands)
T=0x$TABLE
run_ghidra label_1 ok "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvLabelCommandTables.java table="$T" count=6 stride=48 dry=1 report="$OUT/label_stride48.csv" \
    -postScript NvLabelCommandTables.java table="$T" count=8 dry=1 report="$OUT/label_dry.csv" \
    -postScript NvLabelCommandTables.java table="$T" end=0x$(printf '%08x' $((0x$TABLE + 8 * 40))) report="$OUT/label_real.csv" \
    -postScript NvLabelCommandTables.java table="$T" count=8 report="$OUT/label_again.csv"
grep -q "dry run, nothing changed. Would create" "$LOGS/label_1.log" || fail "dry run message missing"

# Names import: main file, dry run, repeat, RVA form, force.
A_ANGLE=$(sym fx_angle_to_unit)
A_FP=$(sym fp_only_target)
A_DISPATCH=$(sym g_dispatch)
A_ALPHA=$(sym cmd_alpha_execute)
A_GAIN_VAR=$(sym g_gain)
A_EXT=$(sym fx_extended)
A_SUM=$(sym fx_sum)
A_GAIN_FN=$(sym fx_gain)
A_SSE=$(sym fx_sse_scalar)
RVA_SUM=$(printf '0x%x' $((0x$A_SUM - 0x400000)))

cat >"$OUT/names_main.csv" <<EOF
address,name,source,pin,kind
$A_ANGLE,Fixture::AngleToUnit,own,0123456789abcdef0123,
$A_FP,FpOnlyTarget,xnvse,fb0f4e95ac46c7cf49b2c68b2d222b362ea250ef,
$A_DISPATCH,Fixture::vftable,own,,label
$A_ALPHA,UserAlpha,jip,1111111111111111,
$A_GAIN_VAR,g_Gain,own,,
$A_EXT,  Fixture::Ext Math  ,own,aaaa,
EOF
cat >"$OUT/names_dry.csv" <<EOF
address,name,source,pin,kind
$A_GAIN_FN,DryRunOnly,own,deadbeef,
EOF
cat >"$OUT/names_force.csv" <<EOF
address,name,source,pin,kind
$A_ALPHA,UserAlpha,jip,1111111111111111,
EOF
cat >"$OUT/names_rva.csv" <<EOF
address,name,source,pin,kind
$RVA_SUM,FxSum,own,cafebabe1234,function
EOF
# A CSV as Windows PowerShell writes it: byte-order mark and CRLF line ends.
printf '\357\273\277address,name,source,pin,kind\r\n%s,BomName,own,deadbeef,\r\n' "$A_GAIN_FN" >"$OUT/names_bom.csv"
run_ghidra names_1 ok "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvImportNameMap.java csv="$OUT/names_dry.csv" dry=1 report="$OUT/names_dry.report.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_bom.csv" dry=1 report="$OUT/names_bom.report.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_main.csv" report="$OUT/names_main.report.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_main.csv" report="$OUT/names_again.report.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_rva.csv" rva=1 report="$OUT/names_rva.report.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_force.csv" force=1 report="$OUT/names_force.report.csv"

# A CSV with bad rows must apply nothing. The first file has rows that fail the row checks. In
# the second, a row passes those but cannot be applied (it names a function at a data address):
# the simulated run of the whole file finds that, so the earlier row is never applied. The
# rollback of a failure that only shows up while applying is tested further down with mutated
# copies of the scripts.
cat >"$OUT/names_bad.csv" <<EOF
address,name,source,pin,kind
$A_SSE,ShouldNotApply,own,abc,
$(sym fx_rcp),bad name!,own,abc,
$(sym fx_rsqrt),OkName,bad source,abc,
00001000,Unmapped,own,abc,
EOF
cat >"$OUT/names_unappliable.csv" <<EOF
address,name,source,pin,kind
$A_SSE,ShouldNotApply,own,abc,
$A_GAIN_VAR,NotAFunction,own,abc,function
EOF
run_ghidra names_bad fail "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvImportNameMap.java csv="$OUT/names_bad.csv" report="$OUT/names_bad.report.csv"
grep -q "row(s) rejected" "$LOGS/names_bad.log" || fail "rejected-row error text missing"
run_ghidra names_unappliable fail "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvImportNameMap.java csv="$OUT/names_unappliable.csv" report="$OUT/names_unappliable.report.csv"
grep -q "cannot create a function here" "$OUT/names_unappliable.report.csv" || fail "unappliable-row report text missing"

# After the forced name import the function has the import's tags and no src:cmdtable.
mkdir -p "$TARGET/testscripts"
cp "$ROOT/NvCommon.java" "$ROOT"/test/{InspectNv,TxProbe,MakeClass,UnitProbe}.java "$TARGET/testscripts/"
run_ghidra inspect_names ok "$TARGET/proj_named" "$PROJ_NAME" "${RO[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript InspectNv.java out="$OUT/inspect_names.json" addr="$A_ALPHA,$A_FP"
check names_tags

# The label script keeps a non-default name unless force=1. Its dry run must predict the real run
# (here with force=1, which renames functions that already have names).
run_ghidra label_2 ok "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvLabelCommandTables.java table="$T" count=8 report="$OUT/label_kept.csv" \
    -postScript NvLabelCommandTables.java table="$T" count=8 force=1 dry=1 report="$OUT/label_force_dry.csv" \
    -postScript NvLabelCommandTables.java table="$T" count=8 force=1 report="$OUT/label_force.csv"
check label names

# Bad arguments must fail with a clear message and change nothing.
printf 'address,name,source,pin,oops\n' >"$OUT/names_badheader.csv"
run_ghidra bad_args fail "$TARGET/proj_named" "$PROJ_NAME" "${RW[@]}" "${SP[@]}" \
    -postScript NvExeIdentity.java foo=1 out="$OUT/never.json" \
    -postScript NvExportProgram.java out="$OUT/never" start=4015a2 \
    -postScript NvLabelCommandTables.java table="$T" end=$(printf '0x%x' $((0x$TABLE + 41))) report="$OUT/never.csv" \
    -postScript NvLabelCommandTables.java table="$T" count=3 end=$(printf '0x%x' $((0x$TABLE + 120))) report="$OUT/never.csv" \
    -postScript NvImportNameMap.java csv="$OUT/names_badheader.csv" report="$OUT/never.csv" \
    -postScript NvImportNameMap.java csv="$OUT/does-not-exist.csv" report="$OUT/never.csv" \
    -postScript NvFunctionCard.java addr="$(sym fx_sum)" depth=0 out="$OUT/never_cards"
for msg in "unknown argument 'foo'" "start and end must be given together" "not a positive multiple of stride 40" \
    "give exactly one of count=<n> or end=<hex>" "unknown CSV column 'oops'" "does-not-exist.csv" \
    "depth must be in 1..32"; do
    grep -q -- "$msg" "$LOGS/bad_args.log" || fail "bad-argument message missing: $msg"
done
[ "$(grep -c 'REPORT SCRIPT ERROR' "$LOGS/bad_args.log")" -eq 7 ] || fail "expected seven script errors in bad_args"
[ ! -e "$OUT/never.json" ] && [ ! -e "$OUT/never" ] && [ ! -e "$OUT/never.csv" ] && [ ! -e "$OUT/never_cards" ] ||
    fail "a failed script wrote output"
echo "  ok   seven bad invocations each fail with a clear message and write nothing"

# ---------------------------------------------------------------- C: export the labeled project
mkdir -p "$TARGET/testscripts"
cp "$ROOT/NvCommon.java" "$ROOT/test/InspectNv.java" "$TARGET/testscripts/"
run_ghidra export_labeled ok "$TARGET/proj_named" "$PROJ_NAME" "${RO[@]}" "${SP[@]}" \
    -postScript NvExportProgram.java out="$OUT/export2" threads=4 \
    -postScript NvExportProgram.java out="$OUT/export2_t1" threads=1 \
    -postScript NvExportProgram.java out="$OUT/export_range" start="$A_ANGLE" end="$A_EXT" decompile=0 disasm=1
run_ghidra inspect ok "$TARGET/proj_named" "$PROJ_NAME" "${RO[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript InspectNv.java out="$OUT/inspect.json" addr="$A_DISPATCH,$A_GAIN_VAR,$A_FP"
check export2 determinism range inspect

# Ghidra semantics the scripts rely on: aborting a nested transaction discards the whole run.
cp -a "$TARGET/proj" "$TARGET/proj_tx"
cp "$ROOT/test/TxProbe.java" "$TARGET/testscripts/"
run_ghidra tx_probe ok "$TARGET/proj_tx" "$PROJ_NAME" "${RW[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript TxProbe.java addr="$A_GAIN_FN,$A_SUM"
run_ghidra tx_inspect ok "$TARGET/proj_tx" "$PROJ_NAME" "${RO[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript InspectNv.java out="$OUT/inspect_tx.json" addr="$A_GAIN_FN,$A_SUM"
check tx

# The original project must be untouched by the labeling steps.
run_ghidra export_original ok "$TARGET/proj" "$PROJ_NAME" "${RO[@]}" "${SP[@]}" \
    -postScript NvExportProgram.java out="$OUT/export1_again" threads=2 decompile=0
python3 - "$OUT" <<'EOF' || fail "the original project changed"
import json, sys
a = json.load(open(sys.argv[1] + "/export1/manifest.json"))
b = json.load(open(sys.argv[1] + "/export1_again/manifest.json"))
assert a["function_count_api"] == b["function_count_api"], (a["function_count_api"], b["function_count_api"])
print("  ok   the original (read-only) project still has %d functions" % b["function_count_api"])
EOF

# ---------------------------------------------------------------- D: the completeness gate must trip
# A copy of the export script with one function dropped from the walk. The
# gate compares the records written with Ghidra's own function count, so it
# has to refuse. The mutation is applied with sed and verified.
cp "$ROOT/NvCommon.java" "$TARGET/testscripts/"
SKIP=$(sym fx_sum)
sed "s|functions.add(it.next());|Function dropped = it.next(); if (dropped.getEntryPoint().getOffset() != 0x$SKIP) { functions.add(dropped); }|" \
    "$ROOT/NvExportProgram.java" >"$TARGET/testscripts/NvExportProgram.java"
cmp -s "$ROOT/NvExportProgram.java" "$TARGET/testscripts/NvExportProgram.java" && fail "gate mutation did not change the script"
run_ghidra gate_mutation fail "$TARGET/proj" "$PROJ_NAME" "${RO[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript NvExportProgram.java out="$OUT/export_gate" decompile=0
grep -q "GATE FAILED" "$LOGS/gate_mutation.log" || fail "gate failure text missing"
check gate

# A second mutation: the decompiler never returns a result. Every function
# gets status "error" and the export must refuse to produce a manifest.json,
# because an export with no decompiled text would otherwise look complete.
cp "$ROOT/NvExportProgram.java" "$TARGET/testscripts/NvExportProgram.java"
sed -i "s|DecompileResults res = di.decompileFunction(f, timeoutSec, TaskMonitor.DUMMY);|DecompileResults res = null;|" \
    "$TARGET/testscripts/NvExportProgram.java"
cmp -s "$ROOT/NvExportProgram.java" "$TARGET/testscripts/NvExportProgram.java" && fail "decompile mutation did not change the script"
run_ghidra decompile_failure fail "$TARGET/proj" "$PROJ_NAME" "${RO[@]}" -scriptPath "$TARGET/testscripts" \
    -postScript NvExportProgram.java out="$OUT/export_nodecomp" threads=2
grep -q "no function decompiled successfully" "$LOGS/decompile_failure.log" || fail "decompiler-failure text missing"
check nodecomp

echo
echo "timings:"
printf '  %s\n' "${TIMINGS[@]}"
echo "  total: $(($(now_ms) - SUITE_START)) ms"
echo "ALL TESTS PASSED"
