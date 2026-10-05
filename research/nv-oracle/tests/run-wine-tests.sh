#!/usr/bin/env bash
# Build the nv-oracle tools for 32-bit Windows and test them under Wine
# against a synthetic program (tests/fixture/target.c).
#
# Needs: cargo with the i686-pc-windows-gnu target, the MinGW i686 cross
# compiler (i686-w64-mingw32-gcc, -nm, -objdump, -strip), a 32-bit capable
# Wine, and python3. Nothing from any game is used.
#
# Everything is written under research/nv-oracle/target/wine-tests, which is
# ignored by git. Set WINEPREFIX to reuse a Wine prefix; otherwise a private
# one is created there (the first run takes a while).
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/.." && pwd)
OUT=$ROOT/target/wine-tests
# PROFILE=debug builds without optimisation and with overflow checks, to
# catch arithmetic mistakes the release build would hide.
PROFILE=${PROFILE:-release}
REL=$ROOT/target/i686-pc-windows-gnu/$PROFILE
BUILD_FLAGS=()
[ "$PROFILE" = release ] && BUILD_FLAGS=(--release)
PY="python3 $HERE/oracle_tests.py"

export WINEDEBUG=-all
export WINEARCH=${WINEARCH:-win32}
export WINEPREFIX=${WINEPREFIX:-$OUT/wineprefix}
CC=${CC:-i686-w64-mingw32-gcc}
NM=${NM:-i686-w64-mingw32-nm}
STRIP=${STRIP:-i686-w64-mingw32-strip}
export OBJDUMP=${OBJDUMP:-i686-w64-mingw32-objdump}
CFLAGS="-O1 -fno-omit-frame-pointer -msse2"

for tool in cargo wine python3 "$CC" "$NM" "$STRIP" "$OBJDUMP"; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 2; }
done

PASSED=()
FAILED=()
group() { # group <name> <command...>
    local name=$1
    shift
    echo
    echo "== $name"
    if "$@"; then PASSED+=("$name"); else FAILED+=("$name"); echo "   -> group failed: $name"; fi
}

# ---------------------------------------------------------------- build
echo "== build"
mkdir -p "$OUT/work" "$OUT/probe"
(cd "$ROOT" && cargo build "${BUILD_FLAGS[@]}" --target i686-pc-windows-gnu) || { echo "cargo build failed" >&2; exit 1; }

F=$HERE/fixture/target.c
$CC $CFLAGS -o "$OUT/fixture_unstripped.exe" "$F" || exit 1
$STRIP -o "$OUT/fixture.exe" "$OUT/fixture_unstripped.exe"
# The same program linked far from 0x00400000 (outside nv-call's placeholder),
# and one linked where the engine DLL lives, to test the failure message.
$CC $CFLAGS -Wl,--image-base=0x04000000 -o "$OUT/fixture_far_unstripped.exe" "$F" || exit 1
$CC $CFLAGS -Wl,--image-base=0x10000000 -o "$OUT/fixture_collide.exe" "$F" || exit 1
$NM "$OUT/fixture_unstripped.exe" > "$OUT/nm.txt"
$NM "$OUT/fixture_far_unstripped.exe" > "$OUT/nm_far.txt"

cp "$REL/nv-call.exe" "$REL/nv_call_engine.dll" "$OUT/fixture.exe" "$OUT/fixture_far_unstripped.exe" "$OUT/fixture_collide.exe" "$OUT/work/"
cp "$REL/nv-inject.exe" "$REL/nv_probe.dll" "$OUT/fixture.exe" "$OUT/probe/"

# Ground truth: the program calls its own functions on this CPU.
(cd "$OUT" && wine fixture_unstripped.exe selftest > selftest.txt) || { echo "selftest failed" >&2; exit 1; }
(cd "$OUT" && wine fixture_far_unstripped.exe selftest > selftest_far.txt) || exit 1
echo "selftest: $(grep -c '^CASE ' "$OUT/selftest.txt") cases"

# ---------------------------------------------------------- unit tests
unit_host() {
    local out
    out=$(cd "$ROOT" && cargo test -p nv-oracle-core 2>&1) || { echo "$out" | tail -30; return 1; }
    echo "$out" | grep '^test result'
}
group "core unit tests (host)" unit_host

unit_wine() {
    local out
    out=$(cd "$ROOT" && CARGO_TARGET_I686_PC_WINDOWS_GNU_RUNNER=wine \
        cargo test --release --target i686-pc-windows-gnu -p nv-oracle-core -p nv-inject -p nv-probe 2>&1) ||
        { echo "$out" | tail -30; return 1; }
    echo "$out" | grep '^test result' | grep -v ' 0 passed'
}
group "unit tests (32-bit, under Wine)" unit_wine

decode_crosscheck() {
    # The fixture's own code, and a few larger 32-bit programs if Wine's
    # builtin libraries are installed (any 32-bit PE with a .text works).
    local files=("$OUT/fixture_unstripped.exe") d
    for d in /usr/lib/i386-linux-gnu/wine/i386-windows /usr/lib/wine/i386-windows; do
        for f in kernelbase msvcrt user32; do [ -f "$d/$f.dll" ] && files+=("$d/$f.dll"); done
    done
    $PY decode-crosscheck "$OUT/decode" "${files[@]}"
}
group "decoder: instruction lengths agree with objdump on compiled code" decode_crosscheck

# ------------------------------------------------------------- nv-call
W="$OUT/work"
call() { (cd "$W" && wine nv-call.exe "$@"); }

nv_call_selftest() {
    $PY call-vectors "$OUT/selftest.txt" "$OUT/nm.txt" "$W/vectors.jsonl" &&
        call fixture.exe vectors.jsonl --out results.jsonl &&
        $PY call-check "$OUT/selftest.txt" "$W/results.jsonl" "nv-call vs selftest"
}
group "nv-call: every vector matches the program's own results bit for bit" nv_call_selftest

nv_call_extras() {
    $PY extra-vectors "$OUT/nm.txt" "$W/extra.jsonl" || return 1
    $PY snapshot-files "$OUT/nm.txt" "$W/snap" || return 1
    call fixture.exe extra.jsonl --out extra_results.jsonl
    # Six deliberately bad lines make the exit status 2; the run must still finish.
    [ $? -eq 2 ] || { echo "expected exit status 2 for bad vector lines"; return 1; }
    $PY extra-check "$W/extra_results.jsonl" nosnap "faults, traps, registers, bad input" || return 1
    call fixture.exe extra.jsonl --snapshot snap --out extra_snap_results.jsonl
    [ $? -eq 2 ] || return 1
    $PY extra-check "$W/extra_snap_results.jsonl" snapshot "snapshot regions applied" || return 1
    call fixture.exe extra.jsonl --resolve-imports --out extra_resolve.jsonl
    grep -q '"id":"import_trap".*"fault":null' "$W/extra_resolve.jsonl" || { echo "resolved import did not run"; return 1; }
    grep -q '"imports_resolved":true' "$W/extra_resolve.jsonl" || { echo "header does not say imports were resolved"; return 1; }
}
group "nv-call: faults, import traps, register presets, snapshots, bad input" nv_call_extras

nv_call_far() {
    $PY call-vectors "$OUT/selftest_far.txt" "$OUT/nm_far.txt" "$W/vectors_far.jsonl" &&
        call fixture_far_unstripped.exe vectors_far.jsonl --out results_far.jsonl &&
        $PY call-check "$OUT/selftest_far.txt" "$W/results_far.jsonl" "nv-call, image outside the placeholder" &&
        grep -q '"placeholder":null' "$W/results_far.jsonl" && grep -q '"image_base":"0x04000000"' "$W/results_far.jsonl"
}
group "nv-call: an image placed outside the host placeholder" nv_call_far

nv_call_collide() {
    local msg
    msg=$(call fixture_collide.exe vectors.jsonl 2>&1 >/dev/null)
    local code=$?
    echo "   exit $code: $msg"
    [ $code -eq 1 ] && echo "$msg" | grep -q "cannot reserve" && echo "$msg" | grep -q "0x10000000"
}
group "nv-call: a taken address range fails with a clear message" nv_call_collide

nv_call_usage() {
    local out
    out=$(call 2>&1)
    echo "$out" | grep -q "usage" || { echo "no usage text"; return 1; }
    out=$(call missing.exe vectors.jsonl 2>&1) && { echo "a missing image was accepted"; return 1; }
    echo "   $out" | head -1
    echo "$out" | grep -q "cannot read"
}
group "nv-call: usage and missing-file errors" nv_call_usage

# ------------------------------------------------------------ nv-probe
PD="$OUT/probe"
# Start the program suspended, load the probe, run to the end. Prints only the
# program's own output and fails if nv-inject or the program fails.
inj() {
    local raw code
    raw=$(cd "$PD" && wine nv-inject.exe --dll nv_probe.dll --launch fixture.exe "$@" --wait 2>&1)
    code=$?
    printf '%s\n' "$raw" | grep -v '^nv-inject:'
    [ $code -eq 0 ] || { echo "nv-inject exit status $code" >&2; return 1; }
    return 0
}
manifest() { $PY probe-manifest "$1" "$OUT/nm.txt" "$PD/fixture.exe" "$PD/nv-probe.txt" "${@:2}"; }
(cd "$PD" && wine fixture.exe loop 5 > baseline.txt)

probe_main() {
    manifest main output=probe-main.jsonl && rm -f "$PD/probe-main.jsonl" &&
        inj loop 5 > "$PD/prog-main.txt" &&
        $PY probe-check main "$PD/probe-main.jsonl" "$PD/prog-main.txt" "$PD/baseline.txt" "nv-probe hooks"
}
group "nv-probe: arguments, returns, struct dumps, x87 returns, nesting, limits, refusals" probe_main

probe_badsha() {
    manifest badsha output=probe-badsha.jsonl && rm -f "$PD/probe-badsha.jsonl" &&
        inj loop 5 > "$PD/prog-badsha.txt" &&
        $PY probe-check badsha "$PD/probe-badsha.jsonl" "$PD/prog-badsha.txt" "$PD/baseline.txt" "nv-probe wrong host hash"
}
group "nv-probe: a wrong host exe hash refuses every hook" probe_badsha

probe_rel8() {
    manifest rel8 output=probe-rel8.jsonl && rm -f "$PD/nv-probe.jsonl" &&
        inj loop 5 > "$PD/prog-rel8.txt" &&
        $PY probe-check rel8 "$PD/nv-probe.jsonl" "$PD/prog-rel8.txt" "$PD/baseline.txt" "nv-probe short branch"
}
group "nv-probe: a short branch in the stolen bytes is a reported manifest error" probe_rel8

probe_threads() {
    manifest threads output=probe-threads.jsonl flush_ms=100 && rm -f "$PD/probe-threads.jsonl" &&
        inj threads 4 200 > "$PD/prog-threads.txt" &&
        $PY probe-check threads "$PD/probe-threads.jsonl" "$PD/prog-threads.txt" "$PD/baseline.txt" "nv-probe threads"
}
group "nv-probe: four threads, per-thread return tracking" probe_threads

probe_nosuspend() {
    manifest simple 'output=probe-nosuspend-%PID%.jsonl' suspend=0 && rm -f "$PD"/probe-nosuspend-*.jsonl &&
        inj loop 5 > "$PD/prog-nosuspend.txt" &&
        $PY probe-check nosuspend "$(ls "$PD"/probe-nosuspend-*.jsonl | head -1)" "$PD/prog-nosuspend.txt" "$PD/baseline.txt" "nv-probe no suspend"
}
group "nv-probe: patching without suspending threads; %PID% in the output name" probe_nosuspend

probe_pid() {
    manifest simple output=probe-pid.jsonl mode=inject || return 1
    rm -f "$PD/probe-pid.jsonl" "$PD/pidprog.txt"
    (cd "$PD" && wine fixture.exe loop 3 5000 > pidprog.txt) &
    local bg=$! pid=""
    for _ in $(seq 1 100); do
        pid=$(sed -n 's/^waiting pid=\([0-9]*\).*/\1/p' "$PD/pidprog.txt" 2>/dev/null)
        [ -n "$pid" ] && break
        sleep 0.2
    done
    [ -n "$pid" ] || { echo "the program did not start"; return 1; }
    (cd "$PD" && wine nv-inject.exe --dll nv_probe.dll --pid "$pid") || return 1
    wait $bg
    $PY probe-check inject-pid "$PD/probe-pid.jsonl" "$PD/pidprog.txt" "$PD/baseline.txt" "nv-probe injected into a running process"
}
group "nv-probe: injected into a running process (--pid)" probe_pid

probe_auto() {
    manifest simple output=probe-auto.jsonl mode=auto grace=300 && rm -f "$PD/probe-auto.jsonl" &&
        inj loop 5 1500 > "$PD/prog-auto.txt" &&
        $PY probe-check auto "$PD/probe-auto.jsonl" "$PD/prog-auto.txt" "$PD/baseline.txt" "nv-probe auto mode"
}
group "nv-probe: auto mode starts on its own thread when NVSE never calls" probe_auto

nvse_runs() {
    manifest simple output=probe-nvse.jsonl mode=nvse || return 1
    rm -f "$PD/probe-nvse.jsonl"
    (cd "$PD" && wine fixture.exe nvse-sim nv_probe.dll > prog-nvse.txt) &&
        $PY nvse-check "$PD/probe-nvse.jsonl" "$PD/prog-nvse.txt" normal "NVSE query and load" || return 1
    rm -f "$PD/probe-nvse.jsonl"
    (cd "$PD" && wine fixture.exe nvse-sim nv_probe.dll free > prog-nvse-free.txt) &&
        $PY nvse-check "$PD/probe-nvse.jsonl" "$PD/prog-nvse-free.txt" free "NVSE unload restores the code" || return 1
    rm -f "$PD/probe-nvse.jsonl"
    (cd "$PD" && wine fixture.exe nvse-sim nv_probe.dll editor > prog-nvse-editor.txt) &&
        $PY nvse-check "$PD/probe-nvse.jsonl" "$PD/prog-nvse-editor.txt" editor "NVSE editor refusal" || return 1
    manifest simple output=probe-auto-nvse.jsonl mode=auto grace=200 || return 1
    rm -f "$PD/probe-auto-nvse.jsonl"
    (cd "$PD" && wine fixture.exe nvse-sim nv_probe.dll wait 900 > prog-auto-nvse.txt) &&
        $PY nvse-check "$PD/probe-auto-nvse.jsonl" "$PD/prog-auto-nvse.txt" normal "auto mode defers to NVSE"
}
group "nv-probe: NVSE plugin exports, editor refusal, unload, auto mode deferring to NVSE" nvse_runs

# -------------------------------------------------------------- summary
echo
echo "== summary"
echo "passed: ${#PASSED[@]}"
for g in "${PASSED[@]}"; do echo "  ok    $g"; done
echo "failed: ${#FAILED[@]}"
for g in "${FAILED[@]}"; do echo "  FAIL  $g"; done
[ ${#FAILED[@]} -eq 0 ]
