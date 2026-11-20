#!/bin/sh
# play.sh's tests: made-up folder trees in a temporary folder (no game file, no real player
# folder), a stand-in for the program that prints a given --find. Run by CI under dash, bash,
# BusyBox sh (Linux) and macOS's /bin/sh (bash 3.2):
#   SH=dash sh distribution/unix/test-play.sh [PLAY.SH]
# Prints PASS/FAIL a line and "fails: N"; its status is 0 only when nothing failed.
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
PLAY=${1:-$here/play.sh}
PLAY=$(CDPATH='' cd -- "$(dirname -- "$PLAY")" && pwd -P)/$(basename -- "$PLAY")
SH=${SH:-sh}
T=$(mktemp -d "${TMPDIR:-/tmp}/nv-rs-test-play.XXXXXX") || exit 2
# The real path (macOS: /var is a link to /private/var), as play.sh resolves the game folder.
T=$(CDPATH='' cd -- "$T" && pwd -P) || exit 2
trap 'rm -rf -- "$T"' EXIT
fails=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; fails=$((fails + 1)); fi; }
tab=$(printf '\t')
# The stand-in program: --find prints $FAKE_FIND's file. Otherwise it takes the game by the
# program's own rule (off Windows, nv-viewer's find_data_folder and host::game_exe: FalloutNV.esm,
# any case, in the folder itself or in a folder of it named Data in any case; FalloutNV.exe, any
# case, beside that Data folder) and refuses with status 1 as the program does, so a game play.sh
# takes but the program would not fails a test. Then it says what it was given. CI's Linux job
# also runs the real program on these trees and compares (NV_RS_REAL_PROGRAM, test 24).
cat >"$T/program" <<'EOF'
#!/bin/sh
if [ "${1:-}" = "--find" ]; then cat "$FAKE_FIND"; exit 0; fi
# --build-mods GAME OWN MODS: says what it was given and makes OWN/modgame a copy of GAME.
if [ "${1:-}" = "--build-mods" ]; then
    echo "built: $2 | $3 | $4"
    [ "${FAKE_MODS_STATUS:-0}" = 0 ] || exit "$FAKE_MODS_STATUS"
    rm -rf -- "$3/modgame" && mkdir -p -- "$3/modgame" && cp -R -- "$2/." "$3/modgame/"
    exit $?
fi
# --install-mods GAME INSTALLS MODS: says what it was given and what it read.
if [ "${1:-}" = "--install-mods" ]; then
    echo "installer: $2 | $3 | $4"
    IFS= read -r typed || typed=''
    echo "typed: $typed"
    exit 0
fi
# FOLDER NAME KIND: the entries of FOLDER named NAME without case that are of KIND (-f or -d).
named() {
    ls -1a -- "$1" 2>/dev/null | while IFS= read -r entry; do
        [ "$(printf '%s' "$entry" | tr '[:upper:]' '[:lower:]')" = "$2" ] || continue
        [ "$3" "$1/$entry" ] && printf '%s\n' "$1/$entry"
    done
}
if [ -n "$(named "$1" falloutnv.esm -f)" ]; then
    data=$1
else
    data=$(named "$1" data -d | while IFS= read -r d; do
        if [ -n "$(named "$d" falloutnv.esm -f)" ]; then
            printf '%s\n' "$d"
            break
        fi
    done)
fi
if [ -z "$data" ]; then
    echo "error: neither $1 nor a Data folder inside it contains FalloutNV.esm" >&2
    exit 1
fi
if [ -z "$(named "$(dirname -- "$data")" falloutnv.exe -f)" ]; then
    echo "error: $(dirname -- "$data")/FalloutNV.exe: not found" >&2
    exit 1
fi
echo "program args: $*"
env | grep '^NV_' | sort
exit "${FAKE_STATUS:-0}"
EOF
chmod +x "$T/program"
export NV_RS_PROGRAM="$T/program" FAKE_FIND="$T/find.txt"
# Also the profile variables, so a caller's own (the profiler's NV_PROFILE among them) cannot pass
# or fail the profile checks.
unset NV_RS_GAME NV_RS_ANSWERS_FROM_INPUT NV_RS_PROFILE NV_PROFILE
run() { $SH "$PLAY" "$@"; }
answer() { # TEXT ARGS...: play.sh with TEXT as the typed answers
    text=$1
    shift
    printf "$text" | NV_RS_ANSWERS_FROM_INPUT=1 $SH "$PLAY" "$@"
}
game="$T/steam/steamapps/common/Fallout New Vegas"
mkdir -p "$game/data"
echo x >"$game/data/falloutnv.ESM"
# The exe in another case too (a copy lower-cased by the tool that copied it).
echo x >"$game/falloutnv.exe"
pfx="$T/steam/steamapps/compatdata/22380/pfx/drive_c/users/steamuser/Documents/My Games/FalloutNV"
mkdir -p "$pfx/saves"
echo one >"$pfx/saves/Save 1.fos"
echo co >"$pfx/saves/Save 1.nvse"
echo two >"$pfx/saves/Save 2.fos"
echo n >"$pfx/saves/notes.txt"
own="$T/own"
S2="$own/game/Documents/My Games/FalloutNV/Saves"
printf 'own%s%s\ngame%sSteam%s%s\nsaves%sSteam (Proton)%s%s\n' "$tab" "$own" "$tab" "$tab" "$game" "$tab" "$tab" "$pfx" >"$T/find.txt"
# Everything outside nv-rs's own folder, as checksums: the player's side must never change.
snapshot() { (cd "$T/steam" && find . -type f -exec cksum {} + | sort); }
before=$(snapshot)

check "0 syntax" "$SH -n \"$PLAY\""

# 1. No terminal: the one game found is used and remembered; saves are not copied; start runs.
out=$(run </dev/null 2>&1); st=$?
check "1 status 0" '[ $st = 0 ]'
check "1 game remembered" '[ "$(cat "$own/game-folder.txt")" = "$game" ]'
check "1 no copy without a terminal" '[ ! -e "$S2/Save 1.fos" ]'
check "1 not marked as asked" '[ ! -e "$own/saves-offered.txt" ]'
check "1 program got the game" 'printf "%s" "$out" | grep -qF "program args: $game"'
check "1 save out set" 'printf "%s" "$out" | grep -qF "NV_SAVE_OUT=$S2"'
check "1 sound out" 'printf "%s" "$out" | grep -qx "NV_SOUND=out"'
# A play run, as Play.exe starts one on Windows: the program then draws on Vulkan with the play
# defaults (host fixes::is_play_run; its test play_sh_variables_make_a_play_run_on_vulkan).
# Only where the start goes on (x86-64, and Apple silicon: arm64 on macOS); elsewhere (ARM Linux
# included) the program stops first.
if case "$(uname -s) $(uname -m)" in *" x86_64" | "Darwin arm64") true ;; *) false ;; esac; then
    check "1 play run" 'printf "%s" "$out" | grep -qxF "NV_SETTINGS=$own/settings.ini"'
else
    check "1 no play settings where the start stops" '! printf "%s" "$out" | grep -q "^NV_SETTINGS="'
fi
check "1 log written" '[ "$(ls "$own/logs" | grep -c "^game-.*\.log$")" = 1 ]'
# 1b. A profile's own settings.ini (the launcher's Settings screen, before that profile was made
# ready) is the one named, while there is no shared one.
if case "$(uname -s) $(uname -m)" in *" x86_64" | "Darwin arm64") true ;; *) false ;; esac; then
    mkdir -p "$own/game" && printf 'FrameLimit=60\n' >"$own/game/settings.ini"
    out=$(run </dev/null 2>&1)
    check "1b the profile's own settings" 'printf "%s" "$out" | grep -qxF "NV_SETTINGS=$own/game/settings.ini"'
    rm -f "$own/game/settings.ini"
fi

# 2. Answers given: Enter (and "no") leave the saves, and mark the question asked.
out=$(answer '\n' 2>&1); st=$?
check "2 status 0" '[ $st = 0 ]'
check "2 asked" 'printf "%s" "$out" | grep -q "Type yes to copy"'
check "2 Enter is no" '[ ! -e "$S2/Save 1.fos" ]'
check "2 question noted" '[ -f "$own/saves-offered.txt" ]'
out=$(answer 'yes\n' 2>&1)
check "2 not asked again" '! printf "%s" "$out" | grep -q "Type yes to copy"'

# 3. --import-saves, yes: the saves and the co-save copied, the other file not.
out=$(answer 'yes\n' --import-saves 2>&1)
check "3 save copied" '[ "$(cat "$S2/Save 1.fos")" = one ] && [ "$(cat "$S2/Save 2.fos")" = two ]'
check "3 co-save copied" '[ "$(cat "$S2/Save 1.nvse")" = co ]'
check "3 other file left" '[ ! -e "$S2/notes.txt" ]'
check "3 no part left" '! ls "$S2" | grep -q "\.part$"'

# 4. A different save of the same name: the save and its co-save kept together on Enter,
# replaced together on yes.
echo mine >"$S2/Save 1.fos"
echo mine-co >"$S2/Save 1.nvse"
out=$(answer 'yes\n\n' --import-saves 2>&1)
check "4 asked once for the pair" '[ "$(printf "%s" "$out" | grep -c "already has a different Save 1.fos")" = 1 ]'
check "4 pair kept on Enter" '[ "$(cat "$S2/Save 1.fos")" = mine ] && [ "$(cat "$S2/Save 1.nvse")" = mine-co ]'
out=$(answer 'yes\nyes\n' --import-saves 2>&1)
check "4 pair replaced on yes" '[ "$(cat "$S2/Save 1.fos")" = one ] && [ "$(cat "$S2/Save 1.nvse")" = co ]'
# A co-save left over from another save goes when its save is replaced by one without.
echo mine >"$S2/Save 2.fos"
echo stray >"$S2/Save 2.nvse"
out=$(answer 'yes\nyes\n' --import-saves 2>&1)
check "4 stray co-save removed" '[ "$(cat "$S2/Save 2.fos")" = two ] && [ ! -e "$S2/Save 2.nvse" ]'

# 5. The player's side untouched by all of the above.
check "5 steam folder unchanged" '[ "$(snapshot)" = "$before" ]'

# 6. The remembered game gone, no game found: typed folders.
rm -f "$own/game-folder.txt"
other="$T/elsewhere/FNV copy"
mkdir -p "$other/Data"
echo x >"$other/Data/FalloutNV.esm"
echo x >"$other/FalloutNV.exe"
printf 'own%s%s\n' "$tab" "$own" >"$T/find.txt"
out=$(answer "/nowhere\n$T/elsewhere\n'$other'  \n" 2>&1); st=$?
check "6 missing folder said" 'printf "%s" "$out" | grep -q "that folder does not exist: /nowhere"'
check "6 wrong folder said" 'printf "%s" "$out" | grep -q "no game in $T/elsewhere"'
check "6 quoted folder used" 'printf "%s" "$out" | grep -qF "program args: $other"'
check "6 typed folder remembered" '[ "$(cat "$own/game-folder.txt")" = "$other" ]'
# Dropped into macOS Terminal: backslashes before spaces, a space after.
rm -f "$own/game-folder.txt"
dropped=$(printf '%s' "$other" | sed 's/ /\\ /g')
out=$(answer "$(printf '%s' "$dropped" | sed 's/\\/\\\\/g') \n" 2>&1)
check "6 Terminal's dropped folder used" 'printf "%s" "$out" | grep -qF "program args: $other"'
# GNOME's quoting of a quote inside a name, and a file:// link.
quoted="$T/it's here"
mkdir -p "$quoted/Data"
echo x >"$quoted/Data/FalloutNV.esm"
echo x >"$quoted/FalloutNV.exe"
rm -f "$own/game-folder.txt"
out=$(answer "'$T/it'\\\\''s here'\n" 2>&1)
check "6 GNOME's quoted quote used" 'printf "%s" "$out" | grep -qF "program args: $quoted"'
rm -f "$own/game-folder.txt"
out=$(answer "file://$(printf '%s' "$other" | sed 's/ /%%20/g')\n" 2>&1)
check "6 file link used" 'printf "%s" "$out" | grep -qF "program args: $other"'
# Enter at the prompt: a plain stop, not a silent one.
rm -f "$own/game-folder.txt"
out=$(answer '\n' 2>&1); st=$?
check "6 Enter stops with a word" '[ $st = 1 ] && printf "%s" "$out" | grep -q "stopped: no game folder given"'

# 7. No game, no terminal: a clear stop.
out=$(run </dev/null 2>&1); st=$?
check "7 stops" '[ $st = 1 ] && printf "%s" "$out" | grep -q -- "--game FOLDER"'

# 8. --game: checked, remembered; a folder without the game refused.
out=$(run --game "$T/elsewhere" </dev/null 2>&1); st=$?
check "8 refused" '[ $st = 1 ] && printf "%s" "$out" | grep -q "no game in"'
out=$(run --game "$other/Data" </dev/null 2>&1); st=$?
check "8 Data folder taken" '[ $st = 0 ] && [ "$(cat "$own/game-folder.txt")" = "$other/Data" ]'

# 9. The program's own stop: its status comes back, with where to report.
out=$(FAKE_STATUS=1 run </dev/null 2>&1); st=$?
check "9 status passed on" '[ $st = 1 ] && printf "%s" "$out" | grep -q "nv-rs stopped (status 1)"'
check "9 where to report" 'printf "%s" "$out" | grep -q "github.com/slaterain/nv-rs/issues"'

# 10. --check passes --check.
out=$(run --check </dev/null 2>&1)
check "10 check" 'printf "%s" "$out" | grep -qF "program args: $other/Data --check"'

# 11. Several games: chosen by number; Enter stops with a word.
printf 'own%s%s\ngame%sSteam%s%s\ngame%sLutris%s%s\n' "$tab" "$own" "$tab" "$tab" "$game" "$tab" "$tab" "$other" >"$T/find.txt"
rm -f "$own/game-folder.txt"
out=$(answer '9\nx\n2\n' 2>&1)
check "11 second chosen" 'printf "%s" "$out" | grep -qF "program args: $other"'
rm -f "$own/game-folder.txt"
out=$(answer '\n' 2>&1); st=$?
check "11 Enter stops with a word" '[ $st = 1 ] && printf "%s" "$out" | grep -q "stopped: no copy chosen"'
out=$(run --help)
check "12 help" 'printf "%s" "$out" | grep -q -- "--import-saves"'

# 13. Logs: at most 10 kept.
rm -f "$own/game-folder.txt"
for i in 1 2 3 4 5 6 7 8 9 10 11 12; do touch "$own/logs/game-2000-01-01_00-00-$i.log"; done
run --game "$game" </dev/null >/dev/null 2>&1
check "13 ten logs" '[ "$(ls "$own/logs" | grep -c "^game-.*\.log$")" = 10 ]'

# 14. Escaped fields: a quote and a percent sign in the name; a warning shown; a found folder
# without the game said and left out.
odd="$T/odd/FNV 'copy' 50%"
mkdir -p "$odd/Data"
echo x >"$odd/Data/FalloutNV.esm"
echo x >"$odd/FalloutNV.exe"
rm -f "$own/game-folder.txt"
printf 'own%s%s\nwarning%sown folder inside%%20Documents\ngame%sSteam%s%s\ngame%sLutris%s/nowhere\n' \
    "$tab" "$own" "$tab" "$tab" "$tab" "$(printf %s "$odd" | sed "s/%/%25/g; s/'/%27/g")" "$tab" "$tab" >"$T/find.txt"
out=$(run </dev/null 2>&1); st=$?
check "14 escaped folder decoded" '[ $st = 0 ] && printf "%s" "$out" | grep -qF "program args: $odd"'
check "14 a folder without the game said" 'printf "%s" "$out" | grep -q "found at /nowhere has no FalloutNV.esm"'
check "14 warning shown" 'printf "%s" "$out" | grep -q "WARNING: own folder inside Documents"'

# 15. A stray lower-case "data" (a mod's) beside the game's Data does not hide the game.
stray="$T/stray/FNV"
# Both a lower- and an upper-case stray, so one sorts before Data in any ls order (C or a locale).
mkdir -p "$stray/Data" "$stray/data" "$stray/DATA"
echo x >"$stray/Data/FalloutNV.esm"
echo x >"$stray/FalloutNV.exe"
rm -f "$own/game-folder.txt"
out=$(run --game "$stray" </dev/null 2>&1); st=$?
check "15 the game's Data found" '[ $st = 0 ] && printf "%s" "$out" | grep -qF "program args: $stray"'

# 16. The program refuses nv-rs's own folder: play.sh stops before its first write, with the
# program's reason; a link inside nv-rs's folder stops it too.
fresh="$T/fresh-own"
printf 'own%s%s\nrefused%snv-rs%%27s own folder cannot be written\ngame%sSteam%s%s\n' "$tab" "$fresh" "$tab" "$tab" "$tab" "$game" >"$T/find.txt"
out=$(answer 'yes\n' 2>&1); st=$?
check "16 refused stops" '[ $st = 1 ] && printf "%s" "$out" | grep -q "nv-rs.s own folder cannot be written"'
check "16 nothing written" '[ ! -e "$fresh" ]'
mkdir -p "$fresh" "$T/players-documents"
ln -s "$T/players-documents" "$fresh/game"
printf 'own%s%s\ngame%sSteam%s%s\nsaves%sSteam (Proton)%s%s\n' "$tab" "$fresh" "$tab" "$tab" "$game" "$tab" "$tab" "$pfx" >"$T/find.txt"
out=$(answer 'yes\n' 2>&1); st=$?
check "16 a link in nv-rs's folder stops" '[ $st = 1 ] && printf "%s" "$out" | grep -q "is a link"'
check "16 nothing written through the link" '[ -z "$(ls -A "$T/players-documents")" ] && [ ! -e "$fresh/game-folder.txt" ]'

# 17. A saves folder with no save: no question, and none noted as asked.
empty_pfx="$T/empty-pfx/drive_c/users/u/Documents/My Games/FalloutNV"
mkdir -p "$empty_pfx/Saves"
own2="$T/own2"
printf 'own%s%s\ngame%sSteam%s%s\nsaves%sWine prefix%s%s\n' "$tab" "$own2" "$tab" "$tab" "$game" "$tab" "$tab" "$empty_pfx" >"$T/find.txt"
out=$(answer 'yes\n' 2>&1)
check "17 not asked" '! printf "%s" "$out" | grep -q "Type yes to copy"'
check "17 not noted" '[ ! -e "$own2/saves-offered.txt" ]'

# 18. Through a link to the script (in ~/bin, say): the program is found beside the real script.
mkdir -p "$T/bin"
ln -s "$PLAY" "$T/bin/nv-rs"
out=$(env -u NV_RS_PROGRAM $SH "$T/bin/nv-rs" --check </dev/null 2>&1)
check "18 a linked script looks beside itself" 'printf "%s" "$out" | grep -qF "$(dirname -- "$PLAY")/app/nv-viewer"'

check "19 steam folder still unchanged" '[ "$(snapshot)" = "$before" ]'

# 20. The program's expected stop on a processor nv-rs is not built for (the game does not run
# there yet, status 69; x86-64 and aarch64, Linux and macOS, go on to start it):
# said as expected, with no request for a report; its status comes back.
printf 'own%s%s\ngame%sSteam%s%s\n' "$tab" "$own" "$tab" "$tab" "$game" >"$T/find.txt"
rm -f "$own/game-folder.txt"
out=$(FAKE_STATUS=69 run </dev/null 2>&1); st=$?
check "20 status passed on" '[ $st = 69 ]'
check "20 said as expected" 'printf "%s" "$out" | grep -q "As expected for this preview"'
check "20 no report asked" '! printf "%s" "$out" | grep -q "github.com/slaterain/nv-rs/issues"'
check "20 not called a stop" '! printf "%s" "$out" | grep -q "nv-rs stopped (status"'

# 21. --check: the same stop after the version check passed says so.
out=$(FAKE_STATUS=69 run --check </dev/null 2>&1); st=$?
check "21 check: status passed on" '[ $st = 69 ]'
check "21 check: version said checked" 'printf "%s" "$out" | grep -q "it is the version nv-rs needs"'
check "21 check: said as expected" 'printf "%s" "$out" | grep -q "As expected for this preview"'

# 22. macOS: a program that still has the download's quarantine mark: a plain stop pointing to
# the READ ME's step 2, before the program is started (only where xattr marks can be made).
if [ "$(uname -s)" = Darwin ]; then
    xattr -w com.apple.quarantine '0081;00000000;Safari;' "$T/program"
    out=$(run </dev/null 2>&1); st=$?
    xattr -d com.apple.quarantine "$T/program"
    check "22 quarantine: stops" '[ $st = 1 ]'
    check "22 quarantine: READ ME step 2" 'printf "%s" "$out" | grep -q "the READ ME, step 2"'
    check "22 quarantine: the command" 'printf "%s" "$out" | grep -qF "xattr -dr com.apple.quarantine"'
    check "22 quarantine: program not started" '! printf "%s" "$out" | grep -q "program args:"'
else
    echo "SKIP 22 quarantine (macOS only)"
fi

# 23. A master that is not in the folder or its Data (in another folder of it): no game, said by
# play.sh itself, as the program would refuse it.
elsewhere_master="$T/no-data/FNV"
mkdir -p "$elsewhere_master/Backup"
echo x >"$elsewhere_master/Backup/FalloutNV.esm"
echo x >"$elsewhere_master/FalloutNV.exe"
out=$(run --game "$elsewhere_master" </dev/null 2>&1); st=$?
check "23 refused by play.sh" '[ $st = 1 ] && printf "%s" "$out" | grep -q "no game in"'

# 25. Linux's library check (app/libraries.txt; a stand-in ldconfig and uname, on any system): a
# 32-bit libasound alone is missing, the 64-bit one is found.
pkg="$T/pkg"
mkdir -p "$pkg/app" "$T/fakebin"
cp "$PLAY" "$pkg/play.sh"
printf 'libasound.so.2 libasound2\n' >"$pkg/app/libraries.txt"
printf '#!/bin/sh\necho Linux\n' >"$T/fakebin/uname"
printf '#!/bin/sh\ncat "$FAKE_LDCONFIG"\n' >"$T/fakebin/ldconfig"
chmod +x "$T/fakebin/uname" "$T/fakebin/ldconfig"
printf '\tlibasound.so.2 (libc6) => /usr/lib/i386-linux-gnu/libasound.so.2\n' >"$T/ld32.txt"
printf '\tlibasound.so.2 (libc6,x86-64) => /usr/lib/x86_64-linux-gnu/libasound.so.2\n' >"$T/ld64.txt"
out=$(PATH="$T/fakebin:$PATH" FAKE_LDCONFIG="$T/ld32.txt" $SH "$pkg/play.sh" --game "$game" </dev/null 2>&1); st=$?
check "25 a 32-bit libasound alone is missing" '[ $st = 1 ] && printf "%s" "$out" | grep -qF "libasound.so.2 (install: libasound2)"'
out=$(PATH="$T/fakebin:$PATH" FAKE_LDCONFIG="$T/ld64.txt" $SH "$pkg/play.sh" --game "$game" </dev/null 2>&1); st=$?
check "25 the 64-bit libasound is found" '[ $st = 0 ] && printf "%s" "$out" | grep -qF "program args: $game"'

# 26. --mods, in a fresh nv-rs folder of its own (and a package folder of its own): the mods folder
# is "my mods" in nv-rs's own folder, never beside play.sh, made with its READ ME and said with
# where it is and that it is empty; the program builds nv-rs's modgame from the game and that
# folder; play is from modgame in the mods profile. The saves offer is per profile: a first --mods
# start does not use up plain play's. The plain profile's saves are untouched by a --mods start. A
# mods folder beside play.sh from an earlier version is said, not used. A failed build stops
# before the game starts. The player's side untouched.
pkg="$T/pkg"
own26="$T/own26"
mkdir -p "$pkg/app"
cp "$PLAY" "$pkg/play.sh"
printf 'read me\n' >"$pkg/app/mods READ ME.txt"
printf 'own%s%s\ngame%sSteam%s%s\nsaves%sSteam (Proton)%s%s\n' "$tab" "$own26" "$tab" "$tab" "$game" "$tab" "$tab" "$pfx" >"$T/find26.txt"
out=$(printf '\n' | env FAKE_FIND="$T/find26.txt" NV_RS_GAME="$game" NV_RS_ANSWERS_FROM_INPUT=1 $SH "$pkg/play.sh" --mods 2>&1); st=$?
check "26 status 0" '[ $st = 0 ]'
check "26 no mods folder beside play.sh" '[ ! -e "$pkg/mods" ]'
check "26 mods folder in nv-rs's own folder" '[ -d "$own26/my mods" ]'
check "26 its READ ME" '[ "$(cat "$own26/my mods/READ ME.txt")" = "read me" ]'
check "26 where it is, said" 'printf "%s" "$out" | grep -qxF "The mods folder: $own26/my mods"'
check "26 empty and just made, said" 'printf "%s" "$out" | grep -qF "It is empty (just made)"'
check "26 built from the game and the mods folder" 'printf "%s" "$out" | grep -qxF "built: $game | $own26 | $own26/my mods"'
check "26 plays from modgame" 'printf "%s" "$out" | grep -qF "program args: $own26/modgame"'
check "26 mods profile" 'printf "%s" "$out" | grep -qx "NV_RS_PROFILE=mods"'
check "26 profile selector does not enable the engine profiler" '! printf "%s" "$out" | grep -q "^NV_PROFILE="'
check "26 mods profile saves" 'printf "%s" "$out" | grep -qF "NV_SAVE_OUT=$own26/mods/Documents/My Games/FalloutNV/Saves"'
check "26 saves offered into the mods profile" 'printf "%s" "$out" | grep -qF "Your saves from" && [ -f "$own26/mods/saves-offered.txt" ]'
check "26 saves kept apart, said" 'printf "%s" "$out" | grep -qF "./play.sh --mods --import-saves"'
check "26 plain play's offer not used up" '[ ! -e "$own26/saves-offered.txt" ]'
out=$(printf '\n' | env FAKE_FIND="$T/find26.txt" NV_RS_GAME="$game" NV_RS_ANSWERS_FROM_INPUT=1 $SH "$pkg/play.sh" 2>&1)
check "26 without --mods: the game profile" 'printf "%s" "$out" | grep -qx "NV_RS_PROFILE=game" && printf "%s" "$out" | grep -qF "program args: $game"'
check "26 without --mods: the engine profiler not enabled" '! printf "%s" "$out" | grep -q "^NV_PROFILE="'
check "26 plain play still offers the saves" 'printf "%s" "$out" | grep -qF "Your saves from" && [ -f "$own26/saves-offered.txt" ]'
plain="$own26/game/Documents/My Games/FalloutNV/Saves"
mkdir -p "$plain"
echo mine >"$plain/Plain.fos"
plain_before=$(cd "$plain" && ls -1A && cksum Plain.fos)
mkdir -p "$pkg/mods"
echo old >"$pkg/mods/Old.esp"
echo mod >"$own26/my mods/Mine.esp"
out=$(env FAKE_FIND="$T/find26.txt" NV_RS_GAME="$game" $SH "$pkg/play.sh" --mods </dev/null 2>&1); st=$?
check "26 again: status 0" '[ $st = 0 ]'
check "26 not empty now: nothing said" '! printf "%s" "$out" | grep -qF "It is empty"'
check "26 the old mods folder beside play.sh, said" 'printf "%s" "$out" | grep -qF "is not used any more"'
check "26 the old mods folder untouched" '[ "$(cat "$pkg/mods/Old.esp")" = old ]'
check "26 the plain profile's saves untouched" '[ "$(cd "$plain" && ls -1A && cksum Plain.fos)" = "$plain_before" ]'
out=$(env FAKE_FIND="$T/find26.txt" NV_RS_GAME="$game" FAKE_MODS_STATUS=1 $SH "$pkg/play.sh" --mods </dev/null 2>&1); st=$?
check "26 a failed build stops" '[ $st != 0 ] && ! printf "%s" "$out" | grep -q "program args:"'
check "26 steam folder unchanged" '[ "$(snapshot)" = "$before" ]'

# 27. --install-mods: the program's chooser is given the game, nv-rs's mod installs and the mods
# folder in nv-rs's own folder ("my mods", said), reads the player's typing, and nothing is started
# after it.
out=$(printf 'q\n' | env NV_RS_GAME="$game" $SH "$pkg/play.sh" --install-mods 2>&1); st=$?
check "27 status 0" '[ $st = 0 ]'
check "27 the chooser's folders" 'printf "%s" "$out" | grep -qxF "installer: $game | $own/mod installs | $own/my mods"'
check "27 where the mods folder is, said" 'printf "%s" "$out" | grep -qF "The mods folder: $own/my mods"'
check "27 the typing reaches it" 'printf "%s" "$out" | grep -qxF "typed: q"'
check "27 no game started" '! printf "%s" "$out" | grep -q "program args:"'
check "27 steam folder unchanged" '[ "$(snapshot)" = "$before" ]'

# 28. A package unpacked inside the game's folder: said plainly, and nothing is written in the game's
# folder by a --mods start (the mods folder is in nv-rs's own folder).
game28="$T/game28"
mkdir -p "$game28/Data" "$game28/nv-rs"
echo x >"$game28/Data/FalloutNV.esm"
echo x >"$game28/FalloutNV.exe"
cp "$PLAY" "$game28/nv-rs/play.sh"
printf 'own%s%s\ngame%sSteam%s%s\n' "$tab" "$T/own28" "$tab" "$tab" "$game28" >"$T/find28.txt"
before28=$(cd "$game28" && find . | sort)
out=$(env FAKE_FIND="$T/find28.txt" NV_RS_GAME="$game28" $SH "$game28/nv-rs/play.sh" --mods </dev/null 2>&1); st=$?
check "28 status 0" '[ $st = 0 ]'
check "28 said plainly" 'printf "%s" "$out" | grep -qF "nv-rs is in the game'"'"'s folder"'
check "28 nothing written in the game's folder" '[ "$(cd "$game28" && find . | sort)" = "$before28" ]'

# 24. CI's Linux job (NV_RS_REAL_PROGRAM: the built nv-viewer): the real program takes and refuses
# the same game folders as the stand-in. Taken, it checks the game's FalloutNV.exe before its
# expected stop (69: the game does not run on this system yet); these trees' exe is a made-up file,
# so a taken tree ends with the program's "not 1.4.0.525" refusal (status 1, NOT the expected
# stop: a wrong exe is worth a report); refused, with status 1 and no such refusal.
if [ -n "${NV_RS_REAL_PROGRAM:-}" ]; then
    for tree in "$game" "$other" "$other/Data" "$stray" "$odd" "$elsewhere_master" "$T/elsewhere"; do
        "$T/program" "$tree" >/dev/null 2>&1
        fake=$?
        (cd -- "$T" && NV_RS_NO_WINDOWS=1 "$NV_RS_REAL_PROGRAM" "$tree" >"$T/real.log" 2>&1)
        real=$?
        if [ "$fake" = 0 ]; then
            check "24 a made-up exe is not the expected stop: $tree" '[ "$real" = 1 ]'
        fi
        if [ "$real" = 1 ] && grep -q "other builds are not supported yet" "$T/real.log"; then
            real=0
        fi
        check "24 the program decides as the stand-in ($fake): $tree" '[ "$real" = "$fake" ]'
        [ "$real" = "$fake" ] || cat "$T/real.log"
    done
else
    echo "SKIP 24 the real program (set NV_RS_REAL_PROGRAM)"
fi
echo "fails: $fails"
[ "$fails" = 0 ]
