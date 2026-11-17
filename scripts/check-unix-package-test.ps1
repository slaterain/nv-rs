<#
Tests scripts/check-unix-package.ps1 on made-up packages (no game file, no real package): a package that
passes gets its CHECKED.txt and is left byte for byte as it was; each refusal writes nothing.

  ./scripts/check-unix-package-test.ps1 -Checker <built package-check.exe> -Scratch <an empty scratch folder>

Needs git with origin fetched (a commit on origin's main and one that is not) and Windows' tar.exe.
Prints PASS/FAIL a line and "fails: N"; status 0 only when nothing failed.
#>
param(
    [Parameter(Mandatory = $true)][string]$Checker,
    [Parameter(Mandatory = $true)][string]$Scratch
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$checkScript = Join-Path $PSScriptRoot 'check-unix-package.ps1'
$tar = Join-Path $env:SystemRoot 'System32\tar.exe'
New-Item -ItemType Directory -Force -Path $Scratch | Out-Null
$Scratch = (Resolve-Path -LiteralPath $Scratch).Path
$script:fails = 0
function Check([string]$what, [bool]$ok) {
    if ($ok) { Write-Host "PASS $what" } else { Write-Host "FAIL $what"; $script:fails++ }
}
function Bytes([int]$seed, [int]$count) {
    $out = New-Object byte[] $count
    $x = [uint32]$seed
    for ($i = 0; $i -lt $count; $i++) {
        $x = [uint32](([uint64]$x * 1103515245 + 12345) % 4294967296)
        $out[$i] = [byte](($x -shr 16) -band 0xff)
    }
    return ,$out
}
function Sha([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }

$onMain = (& git -C $root rev-parse refs/remotes/origin/main).Trim()
# A commit that is not on origin's main: a made-up one on no branch (a commit object of its own).
$tree = (& git -C $root rev-parse "$onMain^{tree}").Trim()
$offMain = ("not on main" | & git -C $root commit-tree $tree).Trim()
$game = Join-Path $Scratch 'FalloutNV.exe'
[System.IO.File]::WriteAllBytes($game, (Bytes 7 20000))
$run = 0

# A made-up package: CASE's folder with play.sh, app/nv-viewer (PROGRAM's bytes) and BUILD.txt, archived
# with its .sha256; then check-unix-package.ps1 on it. Returns the script's status.
function Package([string]$case, [byte[]]$program, [string]$commit, [string]$built, [string]$programSum, [bool]$badSum) {
    $dir = Join-Path $Scratch $case
    $name = 'nv-rs-preview-linux-x86_64-test'
    $package = Join-Path $dir $name
    New-Item -ItemType Directory -Force -Path (Join-Path $package 'app') | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $package 'play.sh'), "#!/bin/sh`n")
    $exe = Join-Path $package 'app\nv-viewer'
    [System.IO.File]::WriteAllBytes($exe, $program)
    if ($programSum -eq '') { $programSum = Sha $exe }
    $build = @("nv-rs linux-x86_64 preview test", "Source commit: $commit", "app/nv-viewer SHA256: $programSum",
        "Platform: linux-x86_64 (x86_64-unknown-linux-gnu), Cargo profile release", $built,
        "Source: https://github.com/slaterain/nv-rs/tree/$commit")
    [System.IO.File]::WriteAllText((Join-Path $package 'BUILD.txt'), (($build -join "`n") + "`n"))
    $archive = Join-Path $dir "$name.tar.gz"
    & $tar -czf $archive -C $dir $name
    Remove-Item -LiteralPath $package -Recurse -Force
    $sum = Sha $archive
    if ($badSum) { $sum = '0' * 64 }
    [System.IO.File]::WriteAllText("$archive.sha256", "$sum  $name.tar.gz`n")
    $script:before = Sha $archive
    $work = Join-Path $dir 'work'
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    $script:log = Join-Path $dir 'check.log'
    $ErrorActionPreference = 'Continue'
    & powershell -NoProfile -ExecutionPolicy Bypass -File $checkScript -Archive $archive -Game $game -Checker $Checker -WorkIn $work *> $script:log
    $status = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $script:checked = Join-Path $dir "$name.CHECKED.txt"
    $script:archive = $archive
    $script:work = $work
    $status
}
$elf = [byte[]](0x7f, 0x45, 0x4c, 0x46) + (Bytes 99 30000)
$github = 'Built by GitHub Actions run 4242 (https://github.com/slaterain/nv-rs/actions/runs/4242) from refs/heads/main; the maintainers have never started it on this system.'

# P. A package that passes: CHECKED.txt written with the run, the commit and the check's lines; the
# archive unchanged; nothing left of the extraction.
$st = Package 'pass' $elf $onMain $github '' $false
Check "P passes" ($st -eq 0)
$text = if (Test-Path -LiteralPath $checked) { Get-Content -LiteralPath $checked -Raw } else { '' }
Check "P CHECKED.txt names the run" ($text.Contains('Built by GitHub Actions run 4242'))
Check "P CHECKED.txt names the commit" ($text.Contains("Source commit: $onMain"))
Check "P CHECKED.txt has the program's line" ($text -match 'nv-viewer: 30004 bytes, shares 0 runs .* longest 0')
Check "P CHECKED.txt has the 0 hits line" ($text -match '[0-9]+ files, 0 needles: 0 hits')
Check "P CHECKED.txt says the game does not run" ($text.Contains('The game does NOT run on Linux or macOS yet'))
Check "P archive unchanged" ((Sha $archive) -eq $before)
Check "P extraction removed" (@(Get-ChildItem -LiteralPath $work -Force).Count -eq 0)

# Refusals: status 1, no CHECKED.txt.
function Refused([string]$what, [int]$st, [string]$why) {
    $said = Get-Content -LiteralPath $log -Raw
    Check "$what refused" ($st -eq 1 -and -not (Test-Path -LiteralPath $checked) -and $said.Contains($why))
    if (-not ($st -eq 1 -and $said.Contains($why))) { Write-Host $said }
    Check "$what extraction removed" (@(Get-ChildItem -LiteralPath $work -Force).Count -eq 0)
}
Refused 'S a wrong .sha256' (Package 'sum' $elf $onMain $github '' $true) "SHA256 is"
Refused 'H a program BUILD.txt does not name' (Package 'prog' $elf $onMain $github ('1' * 64) $false) "app/nv-viewer's SHA256"
Refused 'M a commit not on main' (Package 'main' $elf $offMain $github '' $false) "is not on origin's main"
$branch = $github.Replace('refs/heads/main', 'refs/heads/main-not')
Refused 'B a run from another branch' (Package 'branch' $elf $onMain $branch '' $false) "built by a GitHub Actions run from refs/heads/main"
$local = 'Built by scripts/package-playtest.sh on Linux x86_64; the maintainers have never started it on this system.'
Refused 'B2 built elsewhere than GitHub' (Package 'local' $elf $onMain $local '' $false) "built by a GitHub Actions run"
# The game's bytes in the program: package-check's run limit (4096) is passed.
$copied = [byte[]](0x7f, 0x45, 0x4c, 0x46) + (Bytes 99 1000) + ([System.IO.File]::ReadAllBytes($game)[100..5299]) + (Bytes 98 1000)
Refused 'L a long run of the game in the program' (Package 'run' $copied $onMain $github '' $false) "package-check found something"
# No program at all (not an ELF file): package-check refuses to compare nothing.
$text = [System.Text.Encoding]::ASCII.GetBytes("just text, no program`n")
Refused 'N no program to compare' (Package 'none' $text $onMain $github '' $false) "package-check found something or could not check (status 2)"

Write-Host "fails: $script:fails"
if ($script:fails -eq 0) { exit 0 } else { exit 1 }
