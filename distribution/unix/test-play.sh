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
unset NV_RS_GAME NV_RS_ANSWERS_FROM_INPUT
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
check "1 log written" '[ "$(ls "$own/logs" | grep -c "^game-.*\.log$")" = 1 ]'

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

# 20. The program's expected stop in this preview (the game does not run here yet, status 69):
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
