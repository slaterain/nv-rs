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

# This script runs under every POSIX sh: dash, BusyBox, bash, and macOS's /bin/sh (bash 3.2, which
# cannot parse a `case` inside `$( )`: every `case` here is outside one, and every pattern has its
# opening bracket). CI runs its tests under all of them (distribution/unix/test-play.sh).

set -u

say() { printf '%s\n' "$*"; }
# The line every stop ends with: where to report.
REPORT="This preview has not been played by the team on Linux or macOS. Please report what happened at https://github.com/slaterain/nv-rs/issues with the newest log from nv-rs's logs folder."
fail() {
    printf 'nv-rs: %s\n' "$*" >&2
    exit 1
}
lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }
tab=$(printf '\t')

# The script's own folder, through any links to the script (a link in ~/bin, for example).
script=$0
while [ -L "$script" ]; do
    link=$(readlink -- "$script") || break
    case $link in
    (/*) script=$link ;;
    (*) script=$(dirname -- "$script")/$link ;;
    esac
done
here=$(CDPATH='' cd -- "$(dirname -- "$script")" && pwd -P) || fail "could not find the folder play.sh is in"
program=${NV_RS_PROGRAM:-$here/app/nv-viewer}

# Whether questions can be asked (a terminal is attached).
can_ask() { [ -t 0 ] || [ -n "${NV_RS_ANSWERS_FROM_INPUT:-}" ]; }
# ask PROMPT: the answer in $answer (empty at the end of the input).
ask() {
    printf '%s ' "$1"
    IFS= read -r answer || answer=''
}
typed_yes() {
    case $(lower "$1") in
    (y | yes) return 0 ;;
    esac
    return 1
}

# The entries of DIR named NAME without case, one a line: the exact name first (as the program
# prefers it), then the others in ls's order.
children() {
    [ -e "$1/$2" ] && printf '%s\n' "$1/$2"
    ls -1 -- "$1" 2>/dev/null | while IFS= read -r entry; do
        [ "$entry" = "$2" ] && continue
        if [ "$(lower "$entry")" = "$(lower "$2")" ]; then
            printf '%s\n' "$1/$entry"
        fi
    done
}
# The first entry of DIR named NAME without case (the exact name first), or nothing.
child() { children "$1" "$2" | head -n 1; }

# Whether FOLDER holds FalloutNV.esm (any case) as a file.
has_master() {
    [ -n "$(children "$1" FalloutNV.esm | while IFS= read -r m; do
        if [ -f "$m" ]; then
            echo y
            break
        fi
    done)" ]
}
# Whether FOLDER is the game's folder or its Data: FalloutNV.esm in it or in any of its Data
# folders (any case: a stray lower-case "data" from a mod does not hide the game's Data).
has_game() {
    [ -d "$1" ] || return 1
    has_master "$1" && return 0
    [ -n "$(children "$1" Data | while IFS= read -r data; do
        if [ -d "$data" ] && has_master "$data"; then
            echo y
            break
        fi
    done)" ]
}

# Each field of --find is escaped: every byte but ASCII letters, digits and ` /._-+,:@=~()` is
# written %XX, so no field holds a tab or a line break. decode undoes it, byte for byte (through
# printf's octal escapes; CI checks it against the program's own escaping). A shell's $(...) drops
# line breaks at the end of what it catches, so a game folder whose name ends in a line break is
# left out below with a warning (nv-rs's own folder ends in nv-rs, a saves folder in FalloutNV).
decode() {
    octal=$(printf '%s' "$1" | LC_ALL=C awk '{
        s = $0; r = ""; hex = "0123456789ABCDEF"
        while ((i = index(s, "%")) > 0) {
            h = toupper(substr(s, i + 1, 2))
            n = (index(hex, substr(h, 1, 1)) - 1) * 16 + index(hex, substr(h, 2, 1)) - 1
            r = r substr(s, 1, i - 1) sprintf("\\0%03o", n)
            s = substr(s, i + 3)
        }
        printf "%s", r s
    }')
    decoded=$(printf '%bx' "$octal")
    printf '%s' "${decoded%x}"
}

# A folder as typed, pasted or dropped into the terminal: spaces around it taken off; quotes
# around it taken off (GNOME's '\'' for a quote inside undone); without quotes, backslashes
# before characters undone (macOS Terminal writes Fallout\ New\ Vegas); a file:// link read
# (its %XX undone); a leading ~ read as the home folder.
typed_folder() {
    folder=$(printf '%s' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
    case $folder in
    ("'"*"'") folder=$(printf '%s' "$folder" | sed -e "s/^'//" -e "s/'\$//" -e "s/'\\\\''/'/g") ;;
    ('"'*'"') folder=$(printf '%s' "$folder" | sed -e 's/^"//' -e 's/"$//') ;;
    (file://*)
        folder=${folder#file://}
        folder=${folder#localhost}
        folder=$(decode "$folder")
        ;;
    (*) folder=$(printf '%s' "$folder" | sed -e 's/\\\(.\)/\1/g') ;;
    esac
    case $folder in
    ("~") folder=$HOME ;;
    ("~/"*) folder=$HOME/${folder#"~/"} ;;
    esac
    printf '%s' "$folder"
}
# Why FOLDER is not a game folder, in plain words.
not_a_game() {
    if [ ! -e "$1" ]; then
        printf 'that folder does not exist: %s' "$1"
    else
        printf "no game in %s: it is not the game's folder (the one holding Data, with FalloutNV.esm in it)" "$1"
    fi
}

usage() {
    sed -n '2,/^$/p' "$script" | sed -e 's/^# \{0,1\}//'
}

# The command line.
opt_game=''
opt_forget=0
opt_import=0
opt_check=0
while [ $# -gt 0 ]; do
    case $1 in
    (--game)
        [ $# -ge 2 ] || fail "--game needs the game's folder"
        opt_game=$2
        shift
        ;;
    (--forget-game) opt_forget=1 ;;
    (--import-saves) opt_import=1 ;;
    (--check) opt_check=1 ;;
    (-h | --help)
        usage
        exit 0
        ;;
    (*) fail "unknown option $1 (./play.sh --help lists them)" ;;
    esac
    shift
done

# Never as root: nv-rs's folder and saves in your home would become root's, and later starts
# could not write them.
if [ "$(id -u)" = 0 ] && [ -z "${NV_RS_ALLOW_ROOT_FOR_TESTS:-}" ]; then
    fail "do not run nv-rs as root (with sudo): run ./play.sh as yourself"
fi

[ -f "$program" ] || fail "the program $program is missing: extract the whole archive again, keeping its folders"
if [ ! -x "$program" ]; then
    if [ -n "$(find "$program" -perm -u+x 2>/dev/null)" ]; then
        fail "the drive or folder $here does not allow programs to run (it is mounted noexec): extract the archive into your home folder and start it there"
    fi
    fail "the program $program may not be run: extract the archive with tar (it keeps the files' permissions), or run: chmod +x \"$program\""
fi

# The system libraries the program needs, named in app/libraries.txt (one a line: the library's
# file name, then what to install; empty while the program needs none beyond the C library):
# the missing ones named plainly, before anything else (Linux; macOS has them all).
check_libraries() {
    list=$here/app/libraries.txt
    [ -f "$list" ] || return 0
    [ "$(uname -s)" = Linux ] || return 0
    known=$({ ldconfig -p || /sbin/ldconfig -p; } 2>/dev/null) || return 0
    missing=$(while read -r lib package; do
        if [ -z "$lib" ] || [ "${lib#\#}" != "$lib" ]; then
            continue
        fi
        if ! printf '%s\n' "$known" | grep -qF "$lib ("; then
            printf '  %s (install: %s)\n' "$lib" "${package:-see the READ ME}"
        fi
    done <"$list")
    [ -z "$missing" ] || fail "these system libraries are missing; install them, then start again:
$missing"
}
check_libraries

# What the program finds: its own folder, the games and the saves folders (only read).
found=$("$program" --find) || fail "the program could not look for your game (above). $REPORT"
field() { # KIND N: field N of the first record of KIND, still escaped
    printf '%s\n' "$found" | awk -F '\t' -v kind="$1" -v n="$2" '$1 == kind { print $n; exit }'
}
own=$(decode "$(field own 2)")
[ -n "$own" ] || fail "the program did not name nv-rs's own folder"
case $own in
(/*) ;;
(*) fail "nv-rs's own folder is not a full path: $own" ;;
esac
# The program refuses nv-rs's own folder (a `..` in its path, a path inside Steam's or Wine's
# folders, a link into your own folders): nothing is written, with its reason.
refusal=$(field refused 2)
[ -z "$refusal" ] || fail "$(decode "$refusal")"
printf '%s\n' "$found" | awk -F '\t' '$1 == "warning" { print $2 }' | while IFS= read -r warning; do
    printf 'nv-rs: WARNING: %s\n' "$(decode "$warning")" >&2
done
remembered=$own/game-folder.txt
offered=$own/saves-offered.txt
documents=$own/game/Documents/My\ Games/FalloutNV
saves=$documents/Saves
# Nothing nv-rs writes is a link: mkdir -p and cp would follow it out of nv-rs's folder.
for path in "$own" "$own/game" "$own/game/Documents" "$own/game/Documents/My Games" "$documents" \
    "$saves" "$own/logs" "$own/reports" "$own/machine" "$remembered" "$remembered.part" "$offered"; do
    [ ! -L "$path" ] || fail "$path is a link: nv-rs writes only into plain folders and files of its own there. Make it a plain folder (or remove the link), then start again."
done

# The copies of the game found that really hold it (still escaped, STORE<TAB>FOLDER a line); the
# others are said and left out.
usable_games() {
    printf '%s\n' "$found" | awk -F '\t' '$1 == "game" { print $2 "\t" $3 }' |
        while IFS="$tab" read -r store folder; do
            case $folder in
            (*%0A | *%0a)
                printf 'nv-rs: WARNING: %s\n' "a copy of the game in a folder whose name ends in a line break is left out (nv-rs cannot pass such a name on: rename the folder): $(decode "$folder")" >&2
                continue
                ;;
            esac
            if has_game "$(decode "$folder")"; then
                printf '%s\t%s\n' "$store" "$folder"
            else
                printf 'nv-rs: WARNING: the copy %s found at %s has no FalloutNV.esm in its Data folder: left out\n' "$store" "$(decode "$folder")" >&2
            fi
        done
}

# The game's folder.
remember=0
game=''
if [ -n "$opt_game" ]; then
    game=$(typed_folder "$opt_game")
    has_game "$game" || fail "$(not_a_game "$game")"
    remember=1
elif [ -n "${NV_RS_GAME:-}" ]; then
    game=$NV_RS_GAME
    has_game "$game" || fail "NV_RS_GAME: $(not_a_game "$game")"
else
    if [ "$opt_forget" = 1 ] && [ -f "$remembered" ]; then
        rm -f -- "$remembered"
        say "Forgot the game folder nv-rs remembered."
    fi
    if [ -f "$remembered" ]; then
        # The whole file but its last line break.
        game=$(cat -- "$remembered" && printf x)
        game=${game%x}
        game=${game%"
"}
        if ! has_game "$game"; then
            say "The game nv-rs remembered is no longer in $game."
            game=''
        fi
    fi
    if [ -z "$game" ]; then
        games=$(usable_games)
        count=$(printf '%s' "$games" | grep -c '' || true)
        if [ "$count" = 1 ]; then
            game=$(decode "$(printf '%s\n' "$games" | cut -f 2-)")
            say "Found your game ($(printf '%s\n' "$games" | cut -f 1)): $game"
            remember=1
        elif [ "$count" -gt 1 ]; then
            can_ask || fail "several copies of the game were found; start with --game FOLDER to choose one"
            say "Found these copies of the game:"
            number=0
            printf '%s\n' "$games" | while IFS="$tab" read -r store folder; do
                number=$((number + 1))
                say "  $number. $(decode "$folder") ($store)"
            done
            while [ -z "$game" ]; do
                ask "Type the number of the one to play (or press Enter to stop):"
                [ -n "$answer" ] || fail "stopped: no copy chosen"
                if [ "$answer" -ge 1 ] 2>/dev/null && [ "$answer" -le "$count" ]; then
                    game=$(decode "$(printf '%s\n' "$games" | sed -n "${answer}p" | cut -f 2-)")
                else
                    say "Type a number from 1 to $count."
                fi
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
                [ -n "$answer" ] || fail "stopped: no game folder given"
                candidate=$(typed_folder "$answer")
                if has_game "$candidate"; then
                    game=$candidate
                else
                    say "No game: $(not_a_game "$candidate")."
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
    rm -f -- "$remembered.part"
    printf '%s\n' "$game" >"$remembered.part" && mv -f -- "$remembered.part" "$remembered" ||
        fail "could not remember the game folder in $remembered"
fi

# The saves in Steam's Proton folder (or another Wine prefix): COPIED into nv-rs's own folder on
# the player's typed yes, never moved, never changed there. A save (.fos) and its script
# extender co-save (.nvse) go together. Offered once (a note in nv-rs's own folder says the
# question was asked), and again with --import-saves.
has_own_saves() {
    [ -d "$saves" ] && ls -1 -- "$saves" 2>/dev/null | grep -qi '\.fos$'
}
asked=0
# copy_file FROM TO: FROM copied to TO through TO.part (a link at TO.part removed first, so cp
# never writes through one; mv replaces a link at TO instead of following it).
copy_file() {
    rm -f -- "$2.part" && cp -- "$1" "$2.part" && mv -f -- "$2.part" "$2" && return 0
    rm -f -- "$2.part"
    return 1
}
copy_saves() { # FROM-NAME FOLDER
    source=$(child "$2" Saves)
    [ -n "$source" ] && [ -d "$source" ] || return 0
    names=$(ls -1 -- "$source" 2>/dev/null | grep -i '\.fos$' || true)
    total=$(printf '%s' "$names" | grep -c '' || true)
    [ "$total" -gt 0 ] || return 0
    say ""
    say "Your saves from $1: $total in"
    say "  $source"
    say "nv-rs can COPY them (each with its .nvse file when there is one) into its own folder"
    say "($saves). They stay where they are: nothing in that folder is changed."
    can_ask || {
        say "(Not copied: no terminal to ask in. Run ./play.sh --import-saves in a terminal.)"
        return 0
    }
    asked=1
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
        base=${name%.*}
        from=$source/$name
        [ -f "$from" ] || continue
        co=$(child "$source" "$base.nvse")
        [ -z "$co" ] || [ -f "$co" ] || co=''
        to=$saves/$name
        to_co=$saves/$base.nvse
        # The same save already there (and the same co-save, or none on both sides): nothing to do.
        if [ -e "$to" ] && cmp -s -- "$from" "$to"; then
            if { [ -n "$co" ] && cmp -s -- "$co" "$to_co"; } || { [ -z "$co" ] && [ ! -e "$to_co" ]; }; then
                continue
            fi
        fi
        if [ -e "$to" ] || [ -e "$to_co" ]; then
            ask "nv-rs's folder already has a different $name (or its .nvse file). Type yes to replace both with the copy:"
            typed_yes "$answer" || {
                kept=$((kept + 1))
                continue
            }
        fi
        if copy_file "$from" "$to"; then
            if [ -n "$co" ]; then
                copy_file "$co" "$to_co" || say "Could not copy $base.nvse."
            else
                rm -f -- "$to_co"
            fi
            copied=$((copied + 1))
        else
            say "Could not copy $name."
        fi
    done 4<<EOF
$names
EOF
    say "Copied $copied save(s); kept $kept of nv-rs's own unchanged."
}
if [ "$opt_check" = 0 ] && { [ "$opt_import" = 1 ] || { ! has_own_saves && [ ! -f "$offered" ]; }; }; then
    folders=$(printf '%s\n' "$found" | awk -F '\t' '$1 == "saves" { print $2 "\t" $3 }')
    if [ -n "$folders" ]; then
        while IFS="$tab" read -r from folder <&3; do
            copy_saves "$(decode "$from")" "$(decode "$folder")"
        done 3<<EOF
$folders
EOF
        if [ "$asked" = 1 ]; then
            mkdir -p -- "$own" && say "Asked $(date)" >"$offered"
        fi
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
rm -f -- "$status_file"
{
    "$program" "$game" 2>&1
    printf '%s\n' "$?" >"$status_file"
} | tee -- "$log"
status=$(cat -- "$status_file" 2>/dev/null || echo 1)
rm -f -- "$status_file"
if [ "$status" != 0 ]; then
    say ""
    say "nv-rs stopped (status $status). Everything it said is in $log"
    say "Problem reports made with F12 in the game are in $own/reports."
    say "$REPORT"
fi
exit "$status"
