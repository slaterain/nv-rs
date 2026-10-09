#!/usr/bin/env bash
# Replays the shared acceptance routes in the release viewer and reports
# pass/fail for each: the Linux and macOS counterpart of
# scripts/acceptance.ps1. The routes, their success lines and the pass rule
# are the same; its header explains them, and docs/GOODSPRINGS_ROUTE.md and
# docs/PATHING.md document the commands.
#
# A route passes when every one of its success lines appears in the
# viewer's output, no panic does, and the viewer exits with code 0.
#
# Usage:
#   scripts/acceptance.sh --data "<Fallout New Vegas/Data>" [options]
#
#   --data DIR      the game's Data folder (or set NV_DATA)
#   --routes LIST   which routes to run, comma-separated (doc, vcg02, vms16;
#                   default all three)
#   --out DIR       where logs and screenshots go (default
#                   ~/nv-re/acceptance/<time>); they are private, never
#                   commit them
#   --build         build the release viewer first
#   --background    keep the runs out of the way (the viewer's --background:
#                   its window behind the others, without the focus, the
#                   mouse left alone)
#   --headless      run each route inside gamescope's headless backend, so
#                   no window opens at all (needs gamescope)
#
# Examples:
#   scripts/acceptance.sh --data ~/.steam/steam/steamapps/common/"Fallout New Vegas"/Data
#   scripts/acceptance.sh --routes vms16 --build --headless
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data="${NV_DATA:-}"
routes="doc,vcg02,vms16"
out=""
build=0
background=0
headless=0

while [ $# -gt 0 ]; do
    case "$1" in
        --data) data="$2"; shift 2 ;;
        --routes) routes="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --build) build=1; shift ;;
        --background) background=1; shift ;;
        --headless) headless=1; shift ;;
        -h | --help) sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option '$1' (see --help)." >&2; exit 2 ;;
    esac
done

if [ -z "$data" ]; then
    echo "Give --data (the game's Data folder) or set NV_DATA." >&2
    exit 2
fi
if [ -z "$out" ]; then
    out="$HOME/nv-re/acceptance/$(date +%Y%m%d-%H%M%S)"
fi
mkdir -p "$out"

if [ "$build" = 1 ]; then
    (cd "$root/viewer" && cargo build --release) || { echo "viewer release build failed" >&2; exit 1; }
fi
viewer="$root/viewer/target/release/nv-viewer"
if [ ! -x "$viewer" ]; then
    echo "No release viewer at $viewer (run with --build)." >&2
    exit 1
fi
if [ "$headless" = 1 ] && ! command -v gamescope > /dev/null; then
    echo "--headless needs gamescope." >&2
    exit 1
fi

# The routes: the same arguments and success lines as acceptance.ps1.
route_args=()
route_success=()
run_line() { for l in "$@"; do route_args+=(--run "$l"); done; }
run_at() { route_args+=(--run-at "$1" "$2"); }

set_route() {
    route_args=()
    route_success=()
    case "$1" in
        doc)
            route_args=(GSDocMitchellHouse --stage VCG01 110 --wait 50)
            route_success=('Dialogue menu: talking to Doc Mitchell')
            ;;
        vcg02)
            route_args=(WastelandNV --at '-68250,5800,8480,180')
            run_line 'SetStage VCG02 5' 'set VFreeformGoodsprings.bMetSunny to 1' 'StartQuest VCG02' \
                'SetObjectiveCompleted VCG02 3 1' 'SetObjectiveDisplayed VCG02 5 1' 'SetStage VCG02 10'
            run_at 40 'set VCG02.nTargetCount to 3'
            run_at 40 'SetObjectiveCompleted VCG02 10 1'
            run_at 40 'SunnyREF.evp'
            for t in 90 110 130 150 170 190 210; do run_at "$t" 'player.MoveTo SunnyREF'; done
            run_at 230 'player.MoveTo VCG02SunnySneakMarkerREF'
            run_at 240 'VCG02Gecko1REF.Kill'
            run_at 241 'VCG02Gecko2REF.Kill'
            for t in 270 290 310; do run_at "$t" 'player.MoveTo SunnyREF'; done
            # Well 3's geckos (and well 2's if still alive).
            run_at 320 '0010A210.Kill'
            run_at 321 '0010ABB0.Kill'
            run_at 322 '0010A20F.Kill'
            run_at 323 '0010A211.Kill'
            run_at 324 '0010A212.Kill'
            run_at 325 '0010A213.Kill'
            run_at 340 'SunnyREF.StartConversation player'
            route_args+=(--say "Okay, I'm in." --say "Sure, I'll come with you." --say "Couldn't hurt." --wait 370)
            route_success=('Completed: Talk to Sunny about your reward' 'XP +50')
            ;;
        vms16)
            route_args=(WastelandNV --at '-67845,3000,8400,180')
            run_line 'set VMS16.bTrudyHelp to 1' 'SetStage VMS16 65' 'SunnyRef.ResetHealth' \
                'SunnyRef.MoveTo SunnySpawnMarker' 'SunnyRef.AddScriptPackage SunnyTriggerGunfightDialoguePackage' \
                'SunnyRef.RemoveScriptPackage' 'GoodspringsPowderGangMarker.Enable' \
                'SunnyRef.AddScriptPackage SunnyTravelPackage' 'RingoRef.AddScriptPackage RingoTravelPackage' \
                'RingoRef.AddToFaction GoodspringsFaction 1' 'SetStage VMS16 70' 'set VMS16.bGunFightStart to 1' \
                'player.ModAV Health 5000'
            route_args+=(--wait 330 --walk --weapon WeapNV9mmPistol)
            route_success=('XP +50')
            ;;
        *)
            echo "Unknown route '$1' (doc, vcg02, vms16)." >&2
            exit 2
            ;;
    esac
}

IFS=',' read -r -a chosen <<< "$routes"
for r in "${chosen[@]}"; do set_route "${r// /}"; done

marker='acceptance.sh: viewer exit code'
results=()
failed=0
for r in "${chosen[@]}"; do
    r="${r// /}"
    set_route "$r"
    log="$out/$r.log"
    shot="$out/$r.png"
    # --answer-boxes: prompts answered as they appear, as a player would;
    # --box-answers 2: VCG01's "revise your character" box answered
    # "Finished - Travel Onward" (its third button). See acceptance.ps1.
    args=("$data" "${route_args[@]}" --answer-boxes --box-answers 2 --screenshot "$shot")
    if [ "$background" = 1 ]; then args+=(--background); fi
    # Each route is its own viewer run: a new game, so the DLC start
    # messages show again in every route that starts outside Doc's house.
    echo "== $r (new game)"
    start=$SECONDS
    # The viewer's own exit code is written to the log, because under
    # gamescope the exit code is gamescope's.
    run=(bash -c '"$@"; echo "'"$marker"' $?"' bash "$viewer" "${args[@]}")
    if [ "$headless" = 1 ]; then
        run=(env -u WAYLAND_DISPLAY -u DISPLAY gamescope --backend headless
            -W 1920 -H 1080 -w 1920 -h 1080 -- "${run[@]}")
    fi
    # In a subshell, so a crash report from gamescope (which can crash on
    # its own shutdown, after the viewer has exited) goes to the log, and
    # with no core files.
    (ulimit -c 0; "${run[@]}" || true) > "$log" 2>&1
    secs=$((SECONDS - start))
    code="$(grep -a "^$marker " "$log" | tail -n 1 | awk '{print $NF}')"
    missing=()
    for s in "${route_success[@]}"; do
        grep -aqF -- "$s" "$log" || missing+=("$s")
    done
    if grep -aqF 'panicked at' "$log"; then
        detail='panic'
    elif [ -z "$code" ]; then
        detail='no exit code (the viewer did not finish)'
    elif [ "$code" != 0 ]; then
        detail="exit code $code"
    elif [ ${#missing[@]} -gt 0 ]; then
        detail="missing: $(printf '%s; ' "${missing[@]}")"
        detail="${detail%; }"
    else
        detail=''
    fi
    if [ -z "$detail" ]; then pass=True; else pass=False; failed=1; fi
    results+=("$(printf '%-6s %-5s %7s  %s' "$r" "$pass" "${secs}s" "${detail:-$log}")")
done

echo
printf '%-6s %-5s %7s  %s\n' Route Pass Seconds 'Detail or log'
printf '%s\n' "${results[@]}"
exit "$failed"
