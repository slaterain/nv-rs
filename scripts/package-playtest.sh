#!/bin/sh
# Builds a Linux or macOS preview package: one .tar.gz a player can extract and start with
# ./play.sh, without building anything. The file list below is the whole package: nothing else can
# get in, and it holds no game files, no saves, no reports, no machine tables, no debug files. The
# same rules as the Windows package (scripts/package-playtest.ps1).
#
#   scripts/package-playtest.sh --version 2026-11-16.1 --system linux-x86_64
#       --system linux-x86_64 | macos-arm64   which package (built on that system: Linux x86-64 or
#                        an Apple silicon Mac; nothing here cross-compiles)
#       [--test-build]   a package for testing (CI's builds): BUILD.txt says "test build, not for
#                        release". Without it the script refuses unless HEAD is on origin's main,
#                        the licence notices match Cargo.lock and every dependency's licence is in
#                        distribution/licenses.
#       [--profile release|play]   the Cargo profile (default release, as the Windows package)
#       [--use-built]    use the programs built before with the same profile, under --target
#       [--target DIR]   the Cargo target folder (default: target)
#       [--out DIR]      where the package goes (default: dist)
#       [--game FILE]    the game's FalloutNV.exe, only read: the check that no data of it is in
#                        the program (the Windows package always makes it; CI has no game, so
#                        there it is not made, and BUILD.txt says so)
#
# Makes DIR/nv-rs-preview-<system>.tar.gz and its .sha256.
set -eu

fail() {
    printf 'package-playtest.sh: %s\n' "$*" >&2
    exit 1
}

version=''
system=''
test_build=0
profile=release
use_built=0
target=''
out=''
game=''
while [ $# -gt 0 ]; do
    case $1 in
    --version) version=${2-}; shift ;;
    --system) system=${2-}; shift ;;
    --test-build) test_build=1 ;;
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
release | play) ;;
*) fail "--profile must be release or play" ;;
esac
[ -z "$game" ] || [ -f "$game" ] || fail "--game must be the game's FalloutNV.exe, and this file is not there: $game"

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
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

# The licence notices were made for this Cargo.lock (its hash with LF line ends).
lock=$(tr -d '\r' <Cargo.lock | sha256 | cut -d ' ' -f 1)
want=$(tr -d '\r \n' <distribution/DEPENDENCY-LOCK.sha256)
if [ "$lock" != "$want" ]; then
    [ "$test_build" = 1 ] || fail "the dependency lockfile changed: refresh the licence notices and their lockfile hash (distribution/DEPENDENCY-LOCK.sha256) before packaging"
    note "THIRD-PARTY-NOTICES.txt was made for another Cargo.lock: refresh it before a release."
fi
[ -f distribution/THIRD-PARTY-NOTICES.txt ] || fail "the dependency notices must be assembled before packaging"

# Every crates.io crate in this system's program has its licence folder (licenses/<name>-<version>).
# The workspace's own crates and the vendored ones are listed with their folder and left out here.
missing=$(cargo tree --locked --target "$triple" -p nv-viewer -e normal --prefix none --format '{p}' |
    sed -e 's/ (\*)$//' | grep -v ' (' | sed -e 's/ v/-/' | sort -u |
    while IFS= read -r crate; do
        [ -d "distribution/licenses/$crate" ] || printf '%s ' "$crate"
    done) || fail "cargo tree did not list the program's crates"
if [ -n "$missing" ]; then
    [ "$test_build" = 1 ] || fail "no licence folder in distribution/licenses for: $missing"
    note "No licence folder yet in this package for: $missing"
fi

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
    cargo build --locked --profile "$profile" -p nv-viewer || fail "the game program did not build"
    unset RUSTFLAGS
    cargo build --release --locked --manifest-path scripts/package-check/Cargo.toml || fail "package-check did not build"
fi
[ -f "$checker" ] || checker=$checker.exe
[ -f "$programs/nv-viewer" ] || fail "no program at $programs/nv-viewer (build it, or leave out --use-built)"
[ -f "$checker" ] || fail "no package-check at $target/release (build it, or leave out --use-built)"

name=nv-rs-preview-$system
stage=$out/$name
archive=$stage.tar.gz
if [ -e "$stage" ] || [ -e "$archive" ]; then
    fail "$stage or its archive already exists: move it away first"
fi
mkdir -p -- "$stage/app" "$stage/licenses"

# Our own program and the start script.
cp -- "$programs/nv-viewer" "$stage/app/nv-viewer"
cp -- distribution/unix/play.sh "$stage/play.sh"
# The READ ME of this system, the licences and notices LEGAL.md asks for.
case $system in
linux-*) cp -- "distribution/linux/READ ME.txt" "$stage/READ ME.txt" ;;
macos-*) cp -- "distribution/macos/READ ME.txt" "$stage/READ ME.txt" ;;
esac
cp -- distribution/THIRD-PARTY-NOTICES.txt LICENSE LEGAL.md "$stage/"
cp -R -- distribution/licenses/. "$stage/licenses/"
cp -- crates/vorbis/LICENSE-libvorbis "$stage/licenses/LICENSE-libvorbis"
cp -- crates/nvse/LICENSE-msvc-stl "$stage/licenses/LICENSE-msvc-stl"
# Text with LF line ends whatever this checkout has.
for file in "$stage/READ ME.txt" "$stage/play.sh"; do
    tr -d '\r' <"$file" >"$file.lf" && mv -f -- "$file.lf" "$file"
done
find "$stage" -type d -exec chmod 755 {} +
find "$stage" -type f -exec chmod 644 {} +
chmod 755 "$stage/play.sh" "$stage/app/nv-viewer"

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
if [ "$test_build" = 1 ]; then
    source='test build, not for release'
else
    source="Source: https://github.com/slaterain/nv-rs/tree/$commit"
fi
[ -n "$game" ] || note "Not checked: that no data of the game's FalloutNV.exe is in the program (packaged without the game, --game)."
{
    printf '%s\n' "nv-rs $system preview $version" "Source commit: $commit" \
        "app/nv-viewer SHA256: $sha" "Platform: $system ($triple), Cargo profile $profile" "$needs" \
        "Built by CI or a script on that system; never started on it by the maintainers." "$source"
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
# (LEGAL.md names fnv_decomp, so that one only counts in the program.) FUN_, DAT_ and LAB_ are the
# names a disassembler gives code and data nobody has named.
for needle in nv-re build43 fnv_decomp /target/release /target/play FUN_ DAT_ LAB_; do
    set -- "$@" --program-needle "$needle"
done
if [ -n "$game" ]; then
    set -- "$@" --game "$game" --max-run 4096
fi
"$checker" "$@" || fail "package-check found something that must not be in the package"

# The archive: entries under the package's name, owned by nobody in particular.
# macOS's tar is bsdtar; elsewhere GNU tar.
if [ "$(uname -s)" = Darwin ]; then
    owner='--uid 0 --gid 0'
else
    owner='--owner=0 --group=0 --numeric-owner'
fi
# shellcheck disable=SC2086
(cd -- "$out" && tar $owner -cf - "$name" | gzip -9 >"$archive") || fail "the archive was not made"
(cd -- "$out" && sha256 "$name.tar.gz" >"$name.tar.gz.sha256")
printf '%s\n' "Package: $archive" "Bytes: $(wc -c <"$archive" | tr -d ' ')" "Checksum: $archive.sha256" "Commit: $commit" "Program SHA256: $sha"
