#!/bin/sh
# Tests scripts/package-licences.sh (the licence checks of the Linux and macOS packages) on a
# made-up repository: cargo tree's list read the way the packages read it (its "(*)" and
# "(proc-macro)" marks), and each check failing on what it is for.
#
#   sh scripts/test-package-licences.sh
#
# Needs git and a POSIX shell; no Rust build. Run by the play.sh workflow on Linux and macOS.
set -eu

script=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)/package-licences.sh
[ -f "$script" ] || { echo "test-package-licences.sh: no $script" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
repo=$(CDPATH='' cd -- "$tmp" && pwd -P)/repo
mkdir -p -- "$repo"

passed=0
failed=0
ok() {
    passed=$((passed + 1))
    printf 'ok   %s\n' "$1"
}
bad() {
    failed=$((failed + 1))
    printf 'FAIL %s\n' "$1"
}

# The made-up repository: two crates.io crates (one a macro crate), the program's own crates (one a
# macro crate), a vendored crate, and another crate's LICENSE that is not the program's.
make_repo() {
    rm -rf -- "$repo"
    mkdir -p -- "$repo/distribution/licenses/a-1.0" "$repo/distribution/licenses/b-2.0" \
        "$repo/app" "$repo/crates/own" "$repo/crates/pm" "$repo/crates/other" "$repo/vendor/v"
    printf 'MIT\n' >"$repo/distribution/licenses/a-1.0/LICENSE-MIT"
    printf 'MIT\n' >"$repo/distribution/licenses/b-2.0/LICENSE-MIT"
    printf 'own notice\n' >"$repo/crates/own/NOTICE"
    printf 'vendored\n' >"$repo/vendor/v/LICENSE.MIT"
    printf 'not the program\n' >"$repo/crates/other/LICENSE"
    cat >"$repo/Cargo.lock" <<'EOF'
version = 4

[[package]]
name = "a"
version = "1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "00"

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "a",
]

[[package]]
name = "b"
version = "2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "00"
EOF
    printf 'NOTICES\n\na 1.0\n  License expression: MIT\n\nb 2.0\n  License expression: MIT\n' \
        >"$repo/distribution/THIRD-PARTY-NOTICES.txt"
    printf '# the notices\ncrates/own/NOTICE\tNOTICE-own\r\nvendor/v/LICENSE.MIT\tLICENSE-v.MIT\n' \
        >"$repo/distribution/notices.txt"
    cat >"$repo/crates.txt" <<EOF
app v0.1.0 ($repo/app)
a v1.0
own v0.1.0 ($repo/crates/own)
b v2.0 (proc-macro)
a v1.0 (*)
own v0.1.0 ($repo/crates/own) (*)
pm v0.1.0 ($repo/crates/pm) (proc-macro)
v v30.0.1 ($repo/vendor/v)
EOF
    git -C "$repo" init -q
    git -C "$repo" -c core.autocrlf=false -c core.safecrlf=false add -A
}

# Runs the checks in the repository: status and stderr.
run() {
    status=0
    (cd -- "$repo" && sh "$script" crates.txt) >"$tmp/out" 2>"$tmp/err" || status=$?
}

expect_fail() { # NAME TEXT: the checks fail, naming TEXT
    run
    if [ "$status" = 1 ] && grep -qF -e "$2" "$tmp/err"; then
        ok "$1"
    else
        bad "$1 (status $status; stderr: $(cat "$tmp/err"))"
    fi
}

# The two lists, read from cargo tree's lines.
make_repo
got=$(cd -- "$repo" && sh "$script" --crates crates.txt | tr '\n' ' ')
[ "$got" = "a-1.0 b-2.0 " ] && ok "crates.io crates: a-1.0 b-2.0 (marks taken off)" || bad "crates.io crates: got '$got'"
got=$(cd -- "$repo" && sh "$script" --folders crates.txt | tr '\n' ' ')
want="$repo/app $repo/crates/own $repo/crates/pm $repo/vendor/v "
[ "$got" = "$want" ] && ok "path crates' folders: app, crates/own, crates/pm, vendor/v (no \"*\", no \"proc-macro\")" ||
    bad "path crates' folders: got '$got', want '$want'"

# Complete: passes, says nothing (crates/other/LICENSE is not the program's, so not asked for: a
# "*" taken for a folder would ask for it).
make_repo
run
if [ "$status" = 0 ] && [ ! -s "$tmp/err" ]; then ok "complete licences pass"; else bad "complete licences: status $status; $(cat "$tmp/err")"; fi

make_repo
rm -rf -- "$repo/distribution/licenses/b-2.0"
expect_fail "a crates.io crate's licence folder removed fails" "no licence folder in distribution/licenses for: b-2.0"

make_repo
printf 'vendor/v/LICENSE.MIT\tLICENSE-v.MIT\n' >"$repo/distribution/notices.txt"
expect_fail "a path crate's NOTICE not in notices.txt fails" "not in distribution/notices.txt (add them there and to LEGAL.md): crates/own/NOTICE"

make_repo
printf 'NOTICES\n\na 1.0\n' >"$repo/distribution/THIRD-PARTY-NOTICES.txt"
expect_fail "a Cargo.lock package without its THIRD-PARTY-NOTICES.txt entry fails" "THIRD-PARTY-NOTICES.txt has no entry for: b 2.0"

make_repo
cat >>"$repo/Cargo.lock" <<'EOF'

[[package]]
name = "g"
version = "0.3.0"
source = "git+https://example.invalid/g?rev=1#0123"
EOF
printf 'g 0.3.0\n' >>"$repo/distribution/THIRD-PARTY-NOTICES.txt"
expect_fail "a crate from Git in Cargo.lock fails" "not from crates.io (from Git or another source)"

make_repo
printf 'far v0.1.0 (%s/elsewhere/far)\n' "$tmp" >>"$repo/crates.txt"
expect_fail "a path crate outside the repository fails" "path crates outside the repository"

make_repo
: >"$repo/crates.txt"
expect_fail "an empty list of crates fails" "the list of the program's crates is empty or missing"

printf 'test-package-licences.sh: %s passed, %s failed\n' "$passed" "$failed"
[ "$failed" = 0 ]
