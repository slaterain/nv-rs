#!/bin/sh
# Runs scripts/package-playtest.sh whole on this checkout with stand-ins for the two programs it
# would build (--use-built: no Rust build), and checks the archive it makes: every step after the
# build (the licence checks on cargo tree's real list, the staging, the notices copy, BUILD.txt,
# the archive and its listing) runs as in a real package. The stand-in package-check passes
# everything, so its own checks are not tested here (scripts/package-check has its tests).
#
#   sh scripts/test-package-playtest.sh linux-x86_64|macos-arm64 [--checker BUILT-PACKAGE-CHECK]
#
# Needs a clean checkout (the script refuses another), cargo (for cargo tree; nothing is built), git
# and tar. Run by the play.sh workflow on Linux and macOS.
#
# The stand-in package-check writes down the arguments it was given: every needle of
# scripts/package-needles.txt must reach it byte for byte after a --needle, also from a copy of the
# list with CRLF line ends and no line end after its last needle (a local clone of this checkout,
# committed there, never pushed). With --checker, the real package-check is also run with those
# needles over a file holding a Windows user folder in UTF-16, which it must refuse.
set -eu

system=${1-}
case $system in
linux-x86_64 | macos-arm64) ;;
*) echo "usage: $0 linux-x86_64|macos-arm64 [--checker FILE]" >&2; exit 2 ;;
esac
checker=''
if [ "${2-}" = --checker ]; then
    checker=${3-}
    [ -x "$checker" ] || { echo "test-package-playtest.sh: no package-check at $checker" >&2; exit 2; }
fi
here=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
trap 'exit 1' HUP INT TERM

# The stand-ins: the programs, and a package-check that says yes and writes down its arguments.
mkdir -p -- "$tmp/target/release"
printf '#!/bin/sh\necho stand-in program\n' >"$tmp/target/release/nv-viewer"
printf '#!/bin/sh\necho stand-in launcher\n' >"$tmp/target/release/nv-rs"
printf '#!/bin/sh\nprintf "%%s\n" "$@" >"$0.args"\nexit 0\n' >"$tmp/target/release/package-check"
chmod 755 "$tmp/target/release/nv-viewer" "$tmp/target/release/nv-rs" "$tmp/target/release/package-check"
recorded=$tmp/target/release/package-check.args

failed=0
check() { # NAME, then a command that must succeed
    what=$1
    shift
    if "$@"; then
        printf 'ok   %s\n' "$what"
    else
        printf 'FAIL %s\n' "$what"
        failed=1
    fi
}

# The needles the list holds, as package-check must get them.
tr -d '\r' <"$here/scripts/package-needles.txt" | grep -v -e '^#' -e '^$' >"$tmp/needles"
check "the shared list holds needles" [ -s "$tmp/needles" ]

# package-playtest.sh in the checkout REPO (its output in OUT), then the archive and the needles.
package() {
    repo=$1
    label=$2
    out=$tmp/out-$label
    mkdir -p -- "$out"
    rm -f -- "$recorded"
    status=0
    sh "$repo/scripts/package-playtest.sh" --version selftest --system "$system" --test-build --use-built \
        --target "$tmp/target" --out "$out" >"$tmp/log" 2>&1 || status=$?
    cat "$tmp/log"
    name=nv-rs-preview-$system-test
    archive=$out/$name.tar.gz
    check "$label: package-playtest.sh ended with status 0 (was $status)" [ "$status" = 0 ]
    check "$label: it printed no write error (a pipe closed early)" sh -c '! grep -q "write error" "$1"' x "$tmp/log"
    check "$label: the archive is there" [ -s "$archive" ]
    check "$label: its checksum file is there" [ -s "$archive.sha256" ]
    if [ -s "$archive" ]; then
        tar -tzf "$archive" >"$tmp/entries"
        for file in play.sh nv-rs app/nv-viewer BUILD.txt LICENSE LEGAL.md THIRD-PARTY-NOTICES.txt "READ ME.txt"; do
            check "$label: the archive holds $file" grep -qxF -e "$name/$file" "$tmp/entries"
        done
        # Every notice of distribution/notices.txt under its own name.
        tab=$(printf '\t')
        tr -d '\r' <"$repo/distribution/notices.txt" | grep -v '^#' >"$tmp/notices"
        while IFS="$tab" read -r file named; do
            [ -n "$file" ] || continue
            check "$label: the archive holds licenses/$named (from $file)" grep -qxF -e "$name/licenses/$named" "$tmp/entries"
        done <"$tmp/notices"
        # And a crates.io crate's licence folder.
        first=$(cd -- "$repo/distribution/licenses" && git ls-files | sed -n 1p)
        check "$label: the archive holds licenses/$first" grep -qxF -e "$name/licenses/$first" "$tmp/entries"
        # A test build never sends crash reports: no crash service file in it.
        check "$label: the test build holds no crash-reports.txt" sh -c '! grep -qxF -e "$1" "$2"' x "$name/crash-reports.txt" "$tmp/entries"
    fi
    if [ -f "$recorded" ]; then
        check "$label: package-check was not asked for a crash service file" sh -c '! grep -qxF -e --crash-reports "$1"' x "$recorded"
    fi
    # Every needle of the list after a --needle, byte for byte.
    if [ -f "$recorded" ]; then
        awk 'previous == "--needle" { print } { previous = $0 }' "$recorded" >"$tmp/given"
        while IFS= read -r needle; do
            check "$label: package-check got the needle $needle" grep -qxF -e "$needle" "$tmp/given"
        done <"$tmp/needles"
    else
        check "$label: package-check was run" false
    fi
}

package "$here" checkout

# The same from a clone whose list has CRLF line ends and no line end after the last needle.
git clone -q -- "$here" "$tmp/crlf"
awk '{ printf "%s%s", (NR > 1 ? "\r\n" : ""), $0 }' "$tmp/needles" >"$tmp/crlf/scripts/package-needles.txt"
git -C "$tmp/crlf" -c core.autocrlf=false add -- scripts/package-needles.txt
git -C "$tmp/crlf" -c user.name=test -c user.email=test@example.invalid commit -q --allow-empty -m "CRLF list (test only)"
check "crlf: the list has CRLF line ends" grep -q "$(printf '\r')" "$tmp/crlf/scripts/package-needles.txt"
check "crlf: the list has no line end after its last needle" sh -c '[ "$(tail -c 1 "$1" | od -An -c | tr -d " ")" != "\n" ]' x "$tmp/crlf/scripts/package-needles.txt"
package "$tmp/crlf" crlf

# A release package made where the game is not (--game-check-later, as the package workflow makes
# it), by this checkout's script on another checkout (--source): a clone whose origin is a local
# copy with this commit as its main, so the script's "HEAD is on origin's main" holds without the
# network. It is named and filled as a release package (crash-reports.txt in it, package-check
# asked to compare it with the repository's) and BUILD.txt says the game check comes later.
git clone -q --bare -- "$here" "$tmp/origin.git"
git -C "$tmp/origin.git" update-ref refs/heads/main "$(git -C "$here" rev-parse HEAD)"
git -C "$tmp/origin.git" symbolic-ref HEAD refs/heads/main
git clone -q -- "$tmp/origin.git" "$tmp/rel"
out=$tmp/out-release
mkdir -p -- "$out"
rm -f -- "$recorded"
status=0
sh "$here/scripts/package-playtest.sh" --source "$tmp/rel" --version selftest --system "$system" --game-check-later \
    --use-built --target "$tmp/target" --out "$out" >"$tmp/log" 2>&1 || status=$?
cat "$tmp/log"
name=nv-rs-preview-$system
archive=$out/$name.tar.gz
check "release: package-playtest.sh ended with status 0 (was $status)" [ "$status" = 0 ]
check "release: the archive has the release name" [ -s "$archive" ]
check "release: its checksum file is there" [ -s "$archive.sha256" ]
if [ -s "$archive" ]; then
    tar -tzf "$archive" >"$tmp/entries"
    check "release: the archive holds crash-reports.txt" grep -qxF -e "$name/crash-reports.txt" "$tmp/entries"
    tar -xzf "$archive" -C "$tmp" "$name/BUILD.txt" "$name/crash-reports.txt"
    check "release: its crash-reports.txt is the repository's" cmp -s "$tmp/rel/distribution/crash-reports.txt" "$tmp/$name/crash-reports.txt"
    check "release: BUILD.txt names the source commit" grep -qxF -e "Source commit: $(git -C "$here" rev-parse HEAD)" "$tmp/$name/BUILD.txt"
    check "release: BUILD.txt says the game check was not made where it was built" grep -q '^Checked against the game: not where it was built' "$tmp/$name/BUILD.txt"
    check "release: BUILD.txt says it is published only with its CHECKED.txt" grep -qF 'A release package: it is published only with that CHECKED.txt' "$tmp/$name/BUILD.txt"
fi
if [ -f "$recorded" ]; then
    check "release: package-check was asked to compare the crash service file" grep -qxF -e --crash-reports "$recorded"
    check "release: package-check was not given a game" sh -c '! grep -qxF -e --game "$1"' x "$recorded"
else
    check "release: package-check was run" false
fi
# The refusals: a release package without the game and without --game-check-later, a package of
# both kinds, and --game-check-later with a game.
refused() { # NAME, TEXT the refusal must hold, then package-playtest.sh's arguments
    rwhat=$1
    rwant=$2
    shift 2
    status=0
    sh "$here/scripts/package-playtest.sh" --source "$tmp/rel" --version selftest --system "$system" --use-built \
        --target "$tmp/target" --out "$tmp/out-refused" "$@" >"$tmp/log" 2>&1 || status=$?
    check "refused: $rwhat (status $status)" [ "$status" != 0 ]
    check "refused: $rwhat says why" grep -qF -e "$rwant" "$tmp/log"
    check "refused: $rwhat made no archive" sh -c '! ls "$1"/*.tar.gz >/dev/null 2>&1' x "$tmp/out-refused"
}
refused "a release package without the game" "needs --game"
refused "a package of both kinds" "give one of them" --game-check-later --test-build
printf 'not the game\n' >"$tmp/FalloutNV.exe"
refused "--game-check-later with a game" "with --game the check is made here" --game-check-later --game "$tmp/FalloutNV.exe"

# The real package-check refuses a file holding a Windows user folder in UTF-16 with these needles.
if [ -n "$checker" ]; then
    mkdir -p -- "$tmp/utf16"
    # (The backslash as \134: printf's \\ before an octal escape reads differently between shells.)
    printf 'x\000 \000C\000:\000\134\000U\000s\000e\000r\000s\000\134\000x\000' >"$tmp/utf16/notes.txt"
    set --
    while IFS= read -r needle; do set -- "$@" --needle "$needle"; done <"$tmp/needles"
    status=0
    "$checker" "$tmp/utf16" "$@" >"$tmp/real" 2>&1 || status=$?
    cat "$tmp/real"
    check "package-check refuses C:\Users\ in UTF-16 (status $status)" [ "$status" = 1 ]
    check "package-check names what it found" grep -q 'FOUND' "$tmp/real"
fi
[ "$failed" = 0 ] && echo "test-package-playtest.sh: passed" || echo "test-package-playtest.sh: FAILED"
[ "$failed" = 0 ]
