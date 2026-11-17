#!/bin/sh
# Runs scripts/package-playtest.sh whole on this checkout with stand-ins for the two programs it
# would build (--use-built: no Rust build), and checks the archive it makes: every step after the
# build (the licence checks on cargo tree's real list, the staging, the notices copy, BUILD.txt,
# the archive and its listing) runs as in a real package. The stand-in package-check passes
# everything, so its own checks are not tested here (scripts/package-check has its tests).
#
#   sh scripts/test-package-playtest.sh linux-x86_64 | macos-arm64
#
# Needs a clean checkout (the script refuses another), cargo (for cargo tree; nothing is built), git
# and tar. Run by the play.sh workflow on Linux and macOS.
set -eu

system=${1-}
case $system in
linux-x86_64 | macos-arm64) ;;
*) echo "usage: $0 linux-x86_64|macos-arm64" >&2; exit 2 ;;
esac
here=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
trap 'exit 1' HUP INT TERM

# The stand-ins: a program and a package-check that says yes.
mkdir -p -- "$tmp/target/release" "$tmp/out"
printf '#!/bin/sh\necho stand-in program\n' >"$tmp/target/release/nv-viewer"
printf '#!/bin/sh\nexit 0\n' >"$tmp/target/release/package-check"
chmod 755 "$tmp/target/release/nv-viewer" "$tmp/target/release/package-check"

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

status=0
sh "$here/scripts/package-playtest.sh" --version selftest --system "$system" --test-build --use-built \
    --target "$tmp/target" --out "$tmp/out" >"$tmp/log" 2>&1 || status=$?
cat "$tmp/log"
name=nv-rs-preview-$system-test
archive=$tmp/out/$name.tar.gz
check "package-playtest.sh ended with status 0 (was $status)" [ "$status" = 0 ]
check "it printed no write error (a pipe closed early)" sh -c '! grep -q "write error" "$1"' x "$tmp/log"
check "the archive is there" [ -s "$archive" ]
check "its checksum file is there" [ -s "$archive.sha256" ]
if [ -s "$archive" ]; then
    tar -tzf "$archive" >"$tmp/entries"
    has() { grep -qxF -e "$name/$1" "$tmp/entries"; }
    for file in play.sh app/nv-viewer BUILD.txt LICENSE LEGAL.md THIRD-PARTY-NOTICES.txt "READ ME.txt"; do
        check "the archive holds $file" has "$file"
    done
    # Every notice of distribution/notices.txt under its own name.
    tab=$(printf '\t')
    tr -d '\r' <"$here/distribution/notices.txt" | grep -v '^#' >"$tmp/notices"
    while IFS="$tab" read -r file named; do
        [ -n "$file" ] || continue
        check "the archive holds licenses/$named (from $file)" has "licenses/$named"
    done <"$tmp/notices"
    # And a crates.io crate's licence folder.
    first=$(cd -- "$here/distribution/licenses" && git ls-files | sed -n 1p)
    check "the archive holds licenses/$first" has "licenses/$first"
fi
[ "$failed" = 0 ] && echo "test-package-playtest.sh: passed" || echo "test-package-playtest.sh: FAILED"
[ "$failed" = 0 ]
