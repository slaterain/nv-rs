#!/bin/sh
# nv-rs on Linux and macOS: finds your game, offers to copy your saves from Steam's Proton folder,
# and starts the game.
#
#   ./play.sh                  start (the first time it finds your game, or asks for its folder)
#   ./play.sh --game FOLDER    use the game in FOLDER, and remember it
#   ./play.sh --forget-game    forget the remembered game folder, then start (it asks again)
#   ./play.sh --import-saves   offer again to copy the saves found in Steam's Proton folder
#   ./play.sh --check          only check the game and say whether it can start
#   ./play.sh --help           this text
#
# nv-rs never writes into the game's folder, Steam's folders or your Documents. Everything it
# keeps (your nv-rs saves, settings, logs, reports, the game folder it remembers) is in its own
# folder: on Linux $XDG_DATA_HOME/nv-rs (usually ~/.local/share/nv-rs), on macOS
# ~/Library/Application Support/nv-rs.
#
# For testing: NV_RS_PROGRAM=FILE runs FILE instead of app/nv-viewer; NV_RS_GAME=FOLDER is used
# like --game but not remembered; NV_RS_ANSWERS_FROM_INPUT=1 asks questions without a terminal.

set -u

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P) || exit 1
program=${NV_RS_PROGRAM:-$here/app/nv-viewer}

say() { printf '%s\n' "$*"; }
fail() {
    printf 'nv-rs: %s\n' "$*" >&2
    exit 1
}
lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }
# Whether questions can be asked (a terminal is attached).
can_ask() { [ -t 0 ] || [ -n "${NV_RS_ANSWERS_FROM_INPUT:-}" ]; }
# ask PROMPT: the answer in $answer (empty at the end of the input).
ask() {
    printf '%s ' "$1"
    IFS= read -r answer || answer=''
}
typed_yes() {
    case $(lower "$1") in
    y | yes) return 0 ;;
    esac
    return 1
}

# The entry of DIR named NAME without case (the first in name order), or nothing.
child() {
    ls -1 -- "$1" 2>/dev/null | while IFS= read -r entry; do
        if [ "$(lower "$entry")" = "$(lower "$2")" ]; then
            printf '%s\n' "$1/$entry"
            break
        fi
    done
}

# Whether FOLDER is the game's folder or its Data (FalloutNV.esm in it, any case).
has_game() {
    [ -d "$1" ] || return 1
    master=$(child "$1" FalloutNV.esm)
    [ -n "$master" ] && [ -f "$master" ] && return 0
    data=$(child "$1" Data)
    [ -n "$data" ] || return 1
    master=$(child "$data" FalloutNV.esm)
    [ -n "$master" ] && [ -f "$master" ]
}

# A folder as typed or dropped into the terminal: quotes around it and spaces after it taken
# off, a leading ~ read as the home folder.
typed_folder() {
    folder=$(printf '%s' "$1" | sed -e 's/[[:space:]]*$//' -e "s/^'\(.*\)'\$/\1/" -e 's/^"\(.*\)"$/\1/')
    case $folder in
    "~") folder=$HOME ;;
    "~/"*) folder=$HOME/${folder#"~/"} ;;
    esac
    printf '%s' "$folder"
}

usage() {
    sed -n '2,/^$/p' "$0" | sed -e 's/^# \{0,1\}//'
}

# The command line.
opt_game=''
opt_forget=0
opt_import=0
opt_check=0
while [ $# -gt 0 ]; do
    case $1 in
    --game)
        [ $# -ge 2 ] || fail "--game needs the game's folder"
        opt_game=$2
        shift
        ;;
    --forget-game) opt_forget=1 ;;
    --import-saves) opt_import=1 ;;
    --check) opt_check=1 ;;
    -h | --help)
        usage
        exit 0
        ;;
    *) fail "unknown option $1 (./play.sh --help lists them)" ;;
    esac
    shift
done

[ -f "$program" ] || fail "the program $program is missing: extract the whole archive again, keeping its folders"
[ -x "$program" ] || fail "the program $program may not be run: extract the archive with tar (it keeps the files' permissions), or run: chmod +x \"$program\""

# What the program finds: its own folder, the games and the saves folders (only read).
found=$("$program" --find) || fail "the program could not look for your game (above)"
own=$(printf '%s\n' "$found" | awk -F '\t' '$1 == "own" { print $2; exit }')
[ -n "$own" ] || fail "the program did not name nv-rs's own folder"
case $own in
/*) ;;
*) fail "nv-rs's own folder is not a full path: $own" ;;
esac
remembered=$own/game-folder.txt
documents=$own/game/Documents/My\ Games/FalloutNV
saves=$documents/Saves

# The game's folder.
remember=0
game=''
if [ -n "$opt_game" ]; then
    game=$(typed_folder "$opt_game")
    has_game "$game" || fail "no game in $game: it is not the game's folder (the one holding Data, with FalloutNV.esm in it)"
    remember=1
elif [ -n "${NV_RS_GAME:-}" ]; then
    game=$NV_RS_GAME
    has_game "$game" || fail "NV_RS_GAME=$game: no game there"
else
    if [ "$opt_forget" = 1 ] && [ -f "$remembered" ]; then
        rm -f -- "$remembered"
        say "Forgot the game folder nv-rs remembered."
    fi
    if [ -f "$remembered" ]; then
        game=$(head -n 1 -- "$remembered")
        if ! has_game "$game"; then
            say "The game nv-rs remembered is no longer in $game."
            game=''
        fi
    fi
    if [ -z "$game" ]; then
        games=$(printf '%s\n' "$found" | awk -F '\t' '$1 == "game" { print $2 "\t" $3 }')
        count=$(printf '%s' "$games" | grep -c '' || true)
        if [ "$count" = 1 ]; then
            game=$(printf '%s\n' "$games" | cut -f 2-)
            say "Found your game ($(printf '%s\n' "$games" | cut -f 1)): $game"
            remember=1
        elif [ "$count" -gt 1 ]; then
            can_ask || fail "several copies of the game were found; start with --game FOLDER to choose one"
            say "Found these copies of the game:"
            printf '%s\n' "$games" | awk -F '\t' '{ printf "  %d. %s (%s)\n", NR, $2, $1 }'
            while [ -z "$game" ]; do
                ask "Type the number of the one to play:"
                case $answer in
                '' | *[!0-9]*) [ -n "$answer" ] || exit 1 ;;
                *)
                    if [ "$answer" -ge 1 ] && [ "$answer" -le "$count" ]; then
                        game=$(printf '%s\n' "$games" | sed -n "${answer}p" | cut -f 2-)
                    fi
                    ;;
                esac
            done
            remember=1
        else
            can_ask || fail "your game was not found; start with --game FOLDER (the game's folder, the one holding Data)"
            say "nv-rs did not find Fallout: New Vegas. It looks in Steam's libraries (also Steam from"
            say "Flathub), and where Heroic and Lutris say they installed it."
            say "On macOS no store installs the game: copy the game's folder from a Windows PC (or"
            say "another Windows installation) onto this Mac, and name that folder here."
            while [ -z "$game" ]; do
                ask "Type (or drag in) the game's folder, the one holding Data, or press Enter to stop:"
                [ -n "$answer" ] || exit 1
                candidate=$(typed_folder "$answer")
                if has_game "$candidate"; then
                    game=$candidate
                else
                    say "No game there: the game's folder holds a Data folder with FalloutNV.esm in it."
                fi
            done
            remember=1
        fi
    fi
fi
# A full path: the program starts in nv-rs's own folder, and a remembered folder must not depend
# on where play.sh was started from.
game=$(CDPATH='' cd -- "$game" && pwd -P) || fail "could not enter the game's folder"
if [ "$remember" = 1 ] && [ -z "${NV_RS_GAME:-}" ]; then
    mkdir -p -- "$own" || fail "could not make nv-rs's own folder $own"
    printf '%s\n' "$game" >"$remembered.part" && mv -f -- "$remembered.part" "$remembered" ||
        fail "could not remember the game folder in $remembered"
fi

# The saves in Steam's Proton folder (or another Wine prefix): COPIED into nv-rs's own folder on
# the player's typed yes, never moved, never changed there. Offered once (a note in nv-rs's own
# folder says it was), and again with --import-saves.
offered=$own/saves-offered.txt
has_own_saves() {
    [ -d "$saves" ] && ls -1 -- "$saves" 2>/dev/null | grep -qi '\.fos$'
}
copy_saves() { # FROM-NAME FOLDER
    source=$(child "$2" Saves)
    [ -n "$source" ] && [ -d "$source" ] || return 0
    names=$(ls -1 -- "$source" 2>/dev/null | grep -i -e '\.fos$' -e '\.nvse$' || true)
    total=$(printf '%s\n' "$names" | grep -ci '\.fos$' || true)
    [ "$total" -gt 0 ] || return 0
    say ""
    say "Your saves from $1: $total in"
    say "  $source"
    say "nv-rs can COPY them into its own folder ($saves)."
    say "They stay where they are: nothing in that folder is changed."
    can_ask || {
        say "(Not copied: no terminal to ask in. Run ./play.sh --import-saves in a terminal.)"
        return 0
    }
    ask "Type yes to copy them, or press Enter to leave them:"
    typed_yes "$answer" || {
        say "Not copied."
        return 0
    }
    mkdir -p -- "$saves" || fail "could not make $saves"
    copied=0
    kept=0
    # Names one a line (save names have no line breaks).
    while IFS= read -r name <&4; do
        [ -n "$name" ] || continue
        from=$source/$name
        to=$saves/$name
        [ -f "$from" ] || continue
        if [ -e "$to" ]; then
            if cmp -s -- "$from" "$to"; then
                continue
            fi
            ask "nv-rs's folder already has a different $name. Type yes to replace it with the copy:"
            typed_yes "$answer" || {
                kept=$((kept + 1))
                continue
            }
        fi
        if cp -- "$from" "$to.part" && mv -f -- "$to.part" "$to"; then
            copied=$((copied + 1))
        else
            rm -f -- "$to.part"
            say "Could not copy $name."
        fi
    done 4<<EOF
$names
EOF
    say "Copied $copied file(s); kept $kept of nv-rs's own unchanged."
}
if [ "$opt_check" = 0 ] && { [ "$opt_import" = 1 ] || { ! has_own_saves && [ ! -f "$offered" ]; }; }; then
    folders=$(printf '%s\n' "$found" | awk -F '\t' '$1 == "saves" { print $2 "\t" $3 }')
    if [ -n "$folders" ]; then
        while IFS="$(printf '\t')" read -r from folder <&3; do
            copy_saves "$from" "$folder"
        done 3<<EOF
$folders
EOF
        if can_ask; then mkdir -p -- "$own" && say "Asked $(date)" >"$offered"; fi
    elif [ "$opt_import" = 1 ]; then
        say "No saves were found in Steam's Proton folder or in Heroic's or Lutris's prefixes."
    fi
fi

# The start: nv-rs's own folders, the program's switches, its output in a log too.
mkdir -p -- "$saves" "$own/logs" "$own/reports" "$own/machine" || fail "could not make nv-rs's folders in $own"
stamp=$(date +%Y-%m-%d_%H-%M-%S)
log=$own/logs/game-$stamp.log
NV_SOUND=out
NV_SAVE_OUT=$saves
NV_PREFS_OUT=$documents/FalloutPrefs.ini
NV_REPORT_DIR=$own/reports
NV_REPORT_LOG=$log
NV_WGPU_CACHE=$own/machine/wgpu-cache
NV_D3DX_CACHE=$own/machine/d3dx-cache
export NV_SOUND NV_SAVE_OUT NV_PREFS_OUT NV_REPORT_DIR NV_REPORT_LOG NV_WGPU_CACHE NV_D3DX_CACHE
cd -- "$own" || fail "could not enter $own"

if [ "$opt_check" = 1 ]; then
    "$program" "$game" --check
    exit $?
fi

# The newest 10 logs are kept.
ls -1t -- "$own/logs" 2>/dev/null | grep '^game-.*\.log$' | tail -n +10 | while IFS= read -r old; do
    rm -f -- "$own/logs/$old"
done

say "Starting the game ($game). Log: $log"
status_file=$own/logs/.status
{
    "$program" "$game" 2>&1
    printf '%s\n' "$?" >"$status_file"
} | tee -- "$log"
status=$(cat -- "$status_file" 2>/dev/null || echo 1)
rm -f -- "$status_file"
if [ "$status" != 0 ]; then
    say ""
    say "nv-rs stopped (status $status). Everything it said is in $log"
fi
exit "$status"
