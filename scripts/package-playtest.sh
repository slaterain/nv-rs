#!/bin/sh
# Builds a Linux or macOS preview package: one .tar.gz a player can extract and start with ./nv-rs
# (the launcher) or ./play.sh, without building anything. The file list below is the whole package: nothing else can
# get in, and it holds no game files, no saves, no reports, no machine tables, no debug files. The
# same rules as the Windows package (scripts/package-playtest.ps1).
#
#   scripts/package-playtest.sh --version 2026-11-16.1 --system linux-x86_64
#       --system linux-x86_64 | macos-arm64   which package (built on that system: Linux x86-64 or
#                        an Apple silicon Mac; nothing here cross-compiles)
#       [--test-build]   a package built without the game (every CI build): the archive is named
#                        nv-rs-preview-<system>-test.tar.gz and BUILD.txt says the program was not
#                        checked against the game where it was built, and that a published package
#                        is checked first, its result in CHECKED.txt beside it (made on Windows by
#                        scripts/check-unix-package.ps1). Without it (a release package) the
#                        script refuses unless --game (or --game-check-later) is given and HEAD
#                        is on origin's main. Every package, a test build too, is refused unless the
#                        licence notices match Cargo.lock and every dependency's licence is in
#                        distribution/licenses (scripts/package-licences.sh).
#       [--game-check-later]   a release package built where the game is not (GitHub's runners):
#                        named and filled as a release package (crash-reports.txt in it), HEAD on
#                        origin's main, and BUILD.txt says the check against the game was not made
#                        where it was built: scripts/check-unix-package.ps1 makes it on Windows
#                        before the package is published, and its CHECKED.txt is published beside
#                        it. Not with --test-build or --game.
#       [--profile release|play-opt|play]   the Cargo profile (default release: the optimised
#                        build; play-opt the same settings under its own name, the build the
#                        project plays and measures; play only for quick tests)
#       [--source DIR]   the checkout to package (default: this script's own), so the script of
#                        one commit can package another (the package workflow runs main's script
#                        on the commit it is asked for)
#       [--use-built]    use the programs built before with the same profile, under --target
#       [--target DIR]   the Cargo target folder (default: target)
#       [--out DIR]      where the package goes (default: dist)
#       [--game FILE]    the game's FalloutNV.exe, only read: the check that no data of it is in
#                        the program (package-check --game --max-run 4096, as the Windows package
#                        always makes it). A release package needs it: a machine without the game
#                        makes test builds only, whose program must get this check before any
#                        release.
#
# Makes DIR/nv-rs-preview-<system>.tar.gz (a test build: nv-rs-preview-<system>-test.tar.gz) and its
# .sha256.
set -eu

fail() {
    printf 'package-playtest.sh: %s\n' "$*" >&2
    exit 1
}

version=''
system=''
test_build=0
game_later=0
profile=release
use_built=0
target=''
out=''
game=''
source=''
while [ $# -gt 0 ]; do
    case $1 in
    --version) version=${2-}; shift ;;
    --system) system=${2-}; shift ;;
    --test-build) test_build=1 ;;
    --game-check-later) game_later=1 ;;
    --source) source=${2-}; shift ;;
    --profile) profile=${2-}; shift ;;
    --use-built) use_built=1 ;;
    --target) target=${2-}; shift ;;
    --out) out=${2-}; shift ;;
    --game) game=${2-}; shift ;;
    *) fail "unknown argument $1 (the script's first lines list them)" ;;
    esac
    shift
done
case $version in
'' | [!a-zA-Z0-9]* | *[!a-zA-Z0-9._-]*) fail "--version must be letters, digits, '.', '_' or '-' (for example 2026-11-16.1)" ;;
esac
case $system in
linux-x86_64) triple=x86_64-unknown-linux-gnu ;;
macos-arm64) triple=aarch64-apple-darwin ;;
*) fail "--system must be linux-x86_64 or macos-arm64" ;;
esac
case $profile in
release | play-opt | play) ;;
*) fail "--profile must be release, play-opt or play" ;;
esac
[ -z "$game" ] || [ -f "$game" ] || fail "--game must be the game's FalloutNV.exe, and this file is not there: $game"
if [ "$game_later" = 1 ] && [ "$test_build" = 1 ]; then
    fail "--game-check-later makes a release package, and --test-build a test one: give one of them"
fi
if [ "$game_later" = 1 ] && [ -n "$game" ]; then
    fail "--game-check-later is for a machine without the game: with --game the check is made here"
fi
# A release package is checked against the game, as the Windows package always is: here, or (with
# --game-check-later) on Windows before it is published.
if [ "$test_build" = 0 ] && [ -z "$game" ] && [ "$game_later" = 0 ]; then
    fail "a release package needs --game FILE (the game's FalloutNV.exe, only read): the check that no data of it is in the program, which the Windows package always makes. Without the game make a test build (--test-build), or a release package checked on Windows before it is published (--game-check-later)."
fi

if [ -n "$source" ]; then
    [ -d "$source" ] || fail "--source must be a checkout's folder, and this one is not there: $source"
    root=$(CDPATH='' cd -- "$source" && pwd -P)
else
    root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
fi
cd -- "$root"
[ -n "$target" ] || target=$root/target
[ -n "$out" ] || out=$root/dist
mkdir -p -- "$out"
out=$(CDPATH='' cd -- "$out" && pwd -P)

commit=$(git rev-parse HEAD) || fail "package from a committed Git checkout"
[ -z "$(git status --porcelain --untracked-files=normal)" ] || fail "commit or remove untracked changes before packaging"
if [ "$test_build" = 0 ]; then
    git fetch --quiet origin || fail "could not ask origin which commits it has; use --test-build for a package that is not for release"
    git merge-base --is-ancestor "$commit" refs/remotes/origin/main ||
        fail "HEAD ($commit) is not on origin's main, so BUILD.txt's source link may lead nowhere; use --test-build for a test package"
fi

# Notes BUILD.txt carries about what was not checked (test builds only).
notes=''
note() { notes="$notes$1
"; }

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$@"
    else
        shasum -a 256 "$@"
    fi
}

# The licences: every crates.io package of Cargo.lock has its THIRD-PARTY-NOTICES.txt entry, every
# crates.io crate of this system's program its folder distribution/licenses/<name>-<version>, and
# every licence or notice file of its path crates (the workspace's, vendor/) is in
# distribution/notices.txt; a crate from Git or a path crate outside the repository is refused
# (scripts/package-licences.sh). Test builds too: a test archive goes to players as well.
# (The Windows script keeps its hash of Cargo.lock, distribution/DEPENDENCY-LOCK.sha256.)
# (Through a file: a failing cargo tree must stop the script, which a pipe would hide.)
crates=$(mktemp) || fail "no temporary file"
cargo tree --locked --target "$triple" -p nv-viewer -e normal --prefix none --format '{p}' >"$crates" ||
    fail "cargo tree did not list the program's crates"
[ -s "$crates" ] || fail "cargo tree listed no crates for the program"
# The launcher's crates too (nv-rs, crates/nv-launcher).
cargo tree --locked --target "$triple" -p nv-launcher -e normal --prefix none --format '{p}' >>"$crates" ||
    fail "cargo tree did not list the launcher's crates"
licences=0
sh scripts/package-licences.sh "$crates" || licences=1
rm -f -- "$crates"
[ "$licences" = 0 ] || fail "the licences are not complete (the lines above say which); no package is made without them"

programs=$target/$profile
checker=$target/release/package-check
if [ "$use_built" = 0 ]; then
    # The build machine's folders stay out of the program, and so does its symbol table (the
    # Windows program has none either: its names are in a PDB that is not shipped). macOS keeps
    # the names of every function in it otherwise.
    registry=${CARGO_HOME:-$HOME/.cargo}/registry/src
    rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
    RUSTFLAGS="--remap-path-prefix=$root=nv-rs --remap-path-prefix=$registry=cargo --remap-path-prefix=$rustup_home=rustup -C strip=symbols"
    export RUSTFLAGS
    CARGO_TARGET_DIR=$target
    export CARGO_TARGET_DIR
    cargo build --locked --profile "$profile" -p nv-viewer -p nv-launcher || fail "the game program or the launcher did not build"
    unset RUSTFLAGS
    cargo build --release --locked --manifest-path scripts/package-check/Cargo.toml || fail "package-check did not build"
fi
[ -f "$checker" ] || checker=$checker.exe
[ -f "$programs/nv-viewer" ] || fail "no program at $programs/nv-viewer (build it, or leave out --use-built)"
[ -f "$programs/nv-rs" ] || fail "no launcher at $programs/nv-rs (build it, or leave out --use-built)"
[ -f "$checker" ] || fail "no package-check at $target/release (build it, or leave out --use-built)"

name=nv-rs-preview-$system
# A test build says so in its name too.
[ "$test_build" = 0 ] || name=$name-test
stage=$out/$name
archive=$stage.tar.gz
if [ -e "$stage" ] || [ -e "$archive" ]; then
    fail "$stage or its archive already exists: move it away first"
fi
mkdir -p -- "$stage/app" "$stage/licenses"

# Our own programs and the start script: the launcher (nv-rs, the terminal window; it runs play.sh)
# beside play.sh (the plain-terminal way in), the game program in app.
cp -- "$programs/nv-viewer" "$stage/app/nv-viewer"
cp -- "$programs/nv-rs" "$stage/nv-rs"
cp -- distribution/unix/play.sh "$stage/play.sh"
# The READ ME play.sh --mods puts in the mods folder it makes in nv-rs's own folder.
cp -- "distribution/unix/mods READ ME.txt" "$stage/app/mods READ ME.txt"
# The READ ME of this system, the licences and notices LEGAL.md asks for.
case $system in
linux-*) cp -- "distribution/linux/READ ME.txt" "$stage/READ ME.txt" ;;
macos-*) cp -- "distribution/macos/READ ME.txt" "$stage/READ ME.txt" ;;
esac
cp -- distribution/THIRD-PARTY-NOTICES.txt LICENSE LEGAL.md "$stage/"
# The crash service's address beside the launcher, in a release package only (crash reports are
# sent only when the player agrees; a test build and a build from source have no such file and
# send none).
if [ "$test_build" = 0 ]; then
    cp -- distribution/crash-reports.txt "$stage/crash-reports.txt"
fi
# Only the files Git tracks: a .DS_Store or other ignored file left in the folder stays out.
git ls-files -- distribution/licenses | while IFS= read -r file; do
    rel=${file#distribution/licenses/}
    mkdir -p -- "$stage/licenses/$(dirname -- "$rel")"
    cp -- "$file" "$stage/licenses/$rel"
done
# The notices of code that is not a crates.io crate (distribution/notices.txt: a file, a tab, its
# name in licenses/). scripts/test-package-playtest.sh runs this whole script on stand-in programs.
tab=$(printf '\t')
grep -v '^#' distribution/notices.txt | while IFS="$tab" read -r file named; do
    [ -n "$file" ] || continue
    cp -- "$file" "$stage/licenses/$named" || fail "the notice $file is missing"
done
# The system libraries play.sh checks for before a start (Linux: ALSA's libasound today).
case $system in
linux-*) cp -- distribution/unix/libraries.txt "$stage/app/libraries.txt" ;;
esac
# Text with LF line ends whatever this checkout has.
for file in "$stage/READ ME.txt" "$stage/play.sh" "$stage/app/libraries.txt" "$stage/app/mods READ ME.txt"; do
    [ -f "$file" ] || continue
    tr -d '\r' <"$file" >"$file.lf" && mv -f -- "$file.lf" "$file"
done
find "$stage" -type d -exec chmod 755 {} +
find "$stage" -type f -exec chmod 644 {} +
chmod 755 "$stage/play.sh" "$stage/nv-rs" "$stage/app/nv-viewer"

# What the program needs of the system it starts on.
case $system in
linux-*)
    glibc=$(objdump -T "$stage/app/nv-viewer" 2>/dev/null | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/GLIBC_//' | sort -t . -k 1,1n -k 2,2n -k 3,3n | tail -n 1)
    needs="Needs: Linux on x86-64 with glibc ${glibc:-(not read: objdump missing)} or newer, and a Vulkan driver"
    ;;
macos-*)
    minos=$(otool -l "$stage/app/nv-viewer" 2>/dev/null | awk '/minos/ { print $2; exit }')
    needs="Needs: macOS ${minos:-(not read: otool missing)} or newer on Apple silicon (arm64)"
    ;;
esac
sha=$(sha256 "$stage/app/nv-viewer" | cut -d ' ' -f 1)
launcher_sha=$(sha256 "$stage/nv-rs" | cut -d ' ' -f 1)
# The source link in every mode: any archive that gets out names its source.
source="Source: https://github.com/slaterain/nv-rs/tree/$commit"
# Without the game (every GitHub build): said as it is, with what happens before publishing.
[ -n "$game" ] || note "Checked against the game: not where it was built (no game there). A package the project publishes is checked first against the game's own FalloutNV.exe (package-check --game --max-run 4096: no data of the game's program in this program), and that check's date and result are published beside the archive (its name with .CHECKED.txt for .tar.gz)."
[ "$game_later" = 0 ] || note "A release package: it is published only with that CHECKED.txt beside it."
# Who built it: GitHub's run (its address and branch) or the packager's own machine. A run that
# packaged another commit than its own (the workflow's input) says both.
if [ "${GITHUB_ACTIONS:-}" = true ] && [ -n "${GITHUB_RUN_ID:-}" ]; then
    other=''
    if [ -n "${GITHUB_SHA:-}" ] && [ "$GITHUB_SHA" != "$commit" ]; then
        other=" the workflow at $GITHUB_SHA built the source commit below;"
    fi
    built="Built by GitHub Actions, run ${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-slaterain/nv-rs}/actions/runs/$GITHUB_RUN_ID, from ${GITHUB_REF:-an unknown branch};$other the maintainers have never started it on this system."
else
    built="Built by scripts/package-playtest.sh on the packager's own machine ($(uname -sm)); the maintainers have never started it on this system."
fi
# What was checked: this script makes no archive unless all of it passed (the steps below).
checked="Checked when packaged (no archive is made otherwise): only the package's own file list and file types; no trace of the build machine (its folders, user name) and no disassembler names in the program (package-check); every licence notice of the program's own and vendored crates in licenses/; one package folder in the archive holding every packaged file"
if [ -n "$game" ]; then
    checked="$checked; no data of the game's FalloutNV.exe in the program (package-check --game --max-run 4096)"
fi
{
    printf '%s\n' "nv-rs $system preview $version" "Source commit: $commit" \
        "app/nv-viewer SHA256: $sha" "nv-rs SHA256: $launcher_sha" \
        "Platform: $system ($triple), Cargo profile $profile" "$needs" \
        "$built" "$checked." "$source"
    [ -z "$notes" ] || printf '\n%s' "$notes"
} >"$stage/BUILD.txt"

# What must not be in it: the file types first, then the build machine's traces and game data.
bad=$(find "$stage" -type f | grep -Ei '\.(esm|esp|bsa|nif|kf|dds|ogg|mp3|wav|lip|bik|fos|sdp|pdb|tsv|ini|log|dll|sym|exe|dylib|so|debug|dwarf)$' || true)
[ -z "$bad" ] || fail "these must not be in the package: $bad"
set -- "$stage"
for needle in "$HOME" "$root"; do
    [ -z "$needle" ] || set -- "$@" --needle "$needle"
done
# The user name too, but CI's ("runner") is a common word that names no person.
if [ -z "${GITHUB_ACTIONS:-}" ] && [ -n "${USER:-}" ]; then
    set -- "$@" --needle "$USER"
fi
# The research folders and any Windows user folder, in every file: scripts/package-needles.txt,
# which the Windows script reads too.
[ -f scripts/package-needles.txt ] || fail "scripts/package-needles.txt is missing"
shared=0
while IFS= read -r needle || [ -n "$needle" ]; do
    needle=$(printf '%s' "$needle" | tr -d '\r')
    case $needle in
    '' | '#'*) continue ;;
    esac
    set -- "$@" --needle "$needle"
    shared=$((shared + 1))
done <scripts/package-needles.txt
[ "$shared" -gt 0 ] || fail "scripts/package-needles.txt lists no needle"
# (LEGAL.md names fnv_decomp, so that one only counts in the program.)
for needle in fnv_decomp /target/release /target/play; do
    set -- "$@" --program-needle "$needle"
done
# FUN_, DAT_ and LAB_ are the names a disassembler gives code and data nobody has named, always in
# upper case: searched in that case only, since macOS's program holds `fun_` inside other bytes (a
# table of our own function names, a name's tail after a byte that reads as `f`; CI, 2026-11-16).
for needle in FUN_ DAT_ LAB_; do
    set -- "$@" --program-needle-exact "$needle"
done
if [ -n "$game" ]; then
    set -- "$@" --game "$game" --max-run 4096
fi
set -- "$@" --notices distribution/notices.txt
# A release package carries the crash service's file as the repository has it; a test build none.
if [ "$test_build" = 0 ]; then
    set -- "$@" --crash-reports distribution/crash-reports.txt
fi
"$checker" "$@" || fail "package-check found something that must not be in the package"

# The archive: entries under the package's name, owned by nobody in particular.
# macOS's tar is bsdtar (no extended attributes or AppleDouble ._ entries); elsewhere GNU tar.
if [ "$(uname -s)" = Darwin ]; then
    owner='--uid 0 --gid 0 --no-mac-metadata --no-xattrs'
    COPYFILE_DISABLE=1
    export COPYFILE_DISABLE
else
    owner='--owner=0 --group=0 --numeric-owner'
fi
# The tar first and then gzip, each checked (a tar failing inside a pipe would go unseen).
[ ! -e "$stage.tar" ] || fail "$stage.tar already exists: move it away first"
# A failure from here on leaves no half-made archive behind (the next run would refuse it).
fail_archive() {
    rm -f -- "$stage.tar" "$archive"
    fail "$@"
}
# shellcheck disable=SC2086
(cd -- "$out" && tar $owner -cf "$name.tar" "$name") || fail_archive "the archive was not made (tar)"
gzip -9 -n -- "$stage.tar" || fail_archive "the archive was not compressed (gzip)"
[ -s "$archive" ] || fail_archive "no archive at $archive"
# Every entry under the package's folder, none of Apple's metadata, no `..`.
entries=$(tar -tzf "$archive") || fail_archive "the archive cannot be listed"
odd=$(printf '%s\n' "$entries" | grep -v -e "^$name/" -e "^$name\$" || true)
odd="$odd$(printf '%s\n' "$entries" | grep -e '/\._' -e '\.\./' || true)"
[ -z "$odd" ] || fail_archive "the archive holds entries it must not: $odd"
# And every staged file is in it: play.sh and the program first of all.
# (grep reads the list from a file: fed through a pipe, grep -q stops reading at its match and
# the writer fails with "Broken pipe".)
listing=$(mktemp) || fail_archive "no temporary file"
printf '%s\n' "$entries" >"$listing"
unlisted=$(cd -- "$out" && find "$name" -type f | while IFS= read -r file; do
    grep -qxF -e "$file" "$listing" || printf '%s ' "$file"
done)
[ -z "$unlisted" ] || { rm -f -- "$listing"; fail_archive "the archive lacks: $unlisted"; }
for file in "$name/play.sh" "$name/nv-rs" "$name/app/nv-viewer"; do
    grep -qxF -e "$file" "$listing" || { rm -f -- "$listing"; fail_archive "the archive has no $file"; }
done
rm -f -- "$listing"
(cd -- "$out" && sha256 "$name.tar.gz" >"$name.tar.gz.sha256")
printf '%s\n' "Package: $archive" "Bytes: $(wc -c <"$archive" | tr -d ' ')" "Checksum: $archive.sha256" "Commit: $commit" "Program SHA256: $sha"
