#!/bin/sh
# The licence checks of the Linux and macOS packages (scripts/package-playtest.sh runs them on
# every package, test builds too: a test archive handed to players carries every licence as a
# release does). Run from the repository's root, with the program's crates as `cargo tree --locked
# --target TRIPLE -p nv-viewer -e normal --prefix none --format '{p}'` listed them in CRATES:
#
#   sh scripts/package-licences.sh CRATES            all the checks; each problem a line on stderr,
#                                                    status 1 when there is one
#   sh scripts/package-licences.sh --crates CRATES   the crates.io crates, NAME-VERSION a line
#   sh scripts/package-licences.sh --folders CRATES  the path crates' folders (the workspace's and
#                                                    vendor/), as cargo tree gave them, a line each
#
# The checks:
# - every package of Cargo.lock comes from crates.io (source "registry+"): there is no rule yet for
#   a crate from Git or another source, so such a crate is refused;
# - THIRD-PARTY-NOTICES.txt has an entry `NAME VERSION` for each crates.io package of Cargo.lock;
# - every crates.io crate of the program has its folder distribution/licenses/NAME-VERSION;
# - every path crate's folder is inside the repository, and each of its LICENSE*, NOTICE* and
#   COPYING* files Git tracks is listed in distribution/notices.txt (which the package copies).
#
# cargo tree writes " (*)" after a crate it listed before and " (proc-macro)" after a macro crate;
# both marks are taken off first, so only a path crate's own folder is left in brackets (a "*" once
# taken for a folder matched every licence file in the repository). scripts/test-package-licences.sh
# tests this on made-up lists.
set -eu

problems=0
problem() {
    printf 'package-licences.sh: %s\n' "$*" >&2
    problems=1
}

# The lines of cargo tree's list without its marks.
plain() {
    tr -d '\r' <"$1" | sed -e 's/ (\*)$//' -e 's/ (proc-macro)$//'
}
crates_of() {
    plain "$1" | grep -v ' (' | sed -e 's/ v/-/' | sort -u
}
folders_of() {
    plain "$1" | sed -n -e 's/^[^ ]* v[^ ]* (\(.*\))$/\1/p' | sort -u
}

case ${1-} in
--crates) [ $# = 2 ] || { echo "usage: $0 --crates CRATES" >&2; exit 2; }; crates_of "$2"; exit 0 ;;
--folders) [ $# = 2 ] || { echo "usage: $0 --folders CRATES" >&2; exit 2; }; folders_of "$2"; exit 0 ;;
esac
[ $# = 1 ] || { echo "usage: $0 CRATES (see the script's first lines)" >&2; exit 2; }
list=$1
[ -s "$list" ] || { problem "the list of the program's crates is empty or missing: $list"; exit 1; }
root=$(pwd -P)

work=$(mktemp -d) || exit 1
trap 'rm -rf -- "$work"' EXIT
trap 'exit 1' HUP INT TERM

# Cargo.lock's packages with a source: crates.io's own, or refused.
[ -f Cargo.lock ] || { problem "no Cargo.lock here (run from the repository's root)"; exit 1; }
tr -d '\r' <Cargo.lock | awk '
    /^\[\[package\]\]/ { name = ""; version = "" }
    /^name = / { name = $3 }
    /^version = / { version = $3 }
    /^source = / { source = $3; gsub(/"/, "", name); gsub(/"/, "", version); gsub(/"/, "", source)
        print name " " version " " source }' >"$work/packages"
foreign=$(awk '$3 !~ /^registry\+/ { printf "%s %s (%s), ", $1, $2, $3 }' "$work/packages")
[ -z "$foreign" ] || problem "Cargo.lock has packages that are not from crates.io (from Git or another source), and there is no rule yet for their licences: $foreign"
awk '$3 ~ /^registry\+/ { print $1 " " $2 }' "$work/packages" | sort -u >"$work/registry"
# No crates.io package at all is a lock file read wrongly (or not the repository's), not a pass.
[ -s "$work/registry" ] || problem "no crates.io package read from Cargo.lock"

# (grep reads the notices from a file: fed through a pipe, grep -q stops reading at the first match
# and the writer fails with "Broken pipe".)
if [ -f distribution/THIRD-PARTY-NOTICES.txt ]; then
    tr -d '\r' <distribution/THIRD-PARTY-NOTICES.txt >"$work/noticed"
    unnoticed=$(while IFS= read -r package; do
        grep -qxF -e "$package" "$work/noticed" || printf '%s, ' "$package"
    done <"$work/registry")
    [ -z "$unnoticed" ] || problem "THIRD-PARTY-NOTICES.txt has no entry for: $unnoticed refresh it (and distribution/licenses) before packaging"
else
    problem "distribution/THIRD-PARTY-NOTICES.txt is missing: the dependency notices must be assembled before packaging"
fi

missing=$(crates_of "$list" | while IFS= read -r crate; do
    [ -d "distribution/licenses/$crate" ] || printf '%s ' "$crate"
done)
[ -z "$missing" ] || problem "no licence folder in distribution/licenses for: $missing"

tab=$(printf '\t')
outside=''
unlisted=''
for_folders=$(folders_of "$list")
if [ -n "$for_folders" ]; then
    # (No `case` inside `$( )`: macOS's /bin/sh, bash 3.2, cannot parse one.)
    outside=$(printf '%s\n' "$for_folders" | while IFS= read -r dir; do
        rel=${dir#"$root"/}
        [ "$rel" != "$dir" ] || printf '%s ' "$dir"
    done)
    if [ -f distribution/notices.txt ]; then
        tr -d '\r' <distribution/notices.txt >"$work/listed"
        unlisted=$(printf '%s\n' "$for_folders" | while IFS= read -r dir; do
            rel=${dir#"$root"/}
            [ "$rel" != "$dir" ] || continue
            git ls-files -- "$rel/LICENSE*" "$rel/NOTICE*" "$rel/COPYING*" | while IFS= read -r file; do
                grep -q -e "^$file$tab" "$work/listed" || printf '%s ' "$file"
            done
        done)
    else
        problem "distribution/notices.txt is missing (the notices of the repository's own crates)"
    fi
fi
[ -z "$outside" ] || problem "path crates outside the repository (no rule yet for their licences): $outside"
[ -z "$unlisted" ] || problem "these licence or notice files of the program's crates are not in distribution/notices.txt (add them there and to LEGAL.md): $unlisted"
exit $problems
