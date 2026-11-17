<#
Tests scripts/check-unix-package.ps1 on made-up packages (no game file, no real package): a package that
passes gets its CHECKED.txt and is left byte for byte as it was; each refusal writes nothing.

  ./scripts/check-unix-package-test.ps1 -Checker <built package-check.exe> -Scratch <an empty scratch folder>

Needs git with origin fetched (a commit on origin's main and one that is not) and Windows' tar.exe.
GitHub's command line is stood in for by a made-up gh whose answers each case chooses: nothing is
asked of GitHub. Prints PASS/FAIL a line and "fails: N"; status 0 only when nothing failed.
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
$name = 'nv-rs-preview-linux-x86_64-test'
# The stand-in for gh: `gh auth status` ends with FAKE_GH_AUTH; anything else prints FAKE_GH_JSON's file.
$fakeGh = Join-Path $Scratch 'fake-gh.cmd'
[System.IO.File]::WriteAllText($fakeGh, "@echo off`r`nif `"%1`"==`"auth`" exit /b %FAKE_GH_AUTH%`r`ntype `"%FAKE_GH_JSON%`"`r`nexit /b 0`r`n")

# What each case changes; reset after every case.
function Defaults {
    $script:gh = $fakeGh
    $script:ghAuth = 0
    $script:answer = @{ headBranch = 'main'; event = 'workflow_dispatch'; workflowName = 'Preview packages (Linux, macOS)'; conclusion = 'success' }
    $script:extra = @()
}
Defaults

# A made-up package: CASE's folder with play.sh, app/nv-viewer (PROGRAM's bytes) and BUILD.txt, archived
# (with $extra's entries too) with its .sha256; then check-unix-package.ps1 on it, the stand-in gh
# answering $answer (headSha: the commit unless the case sets it). Returns the script's status.
function Package([string]$case, [byte[]]$program, [string]$commit, [string]$built, [string]$programSum, [bool]$badSum) {
    $dir = Join-Path $Scratch $case
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
    $ErrorActionPreference = 'Continue'
    & $tar -czf $archive -C $dir $name $script:extra 2>$null
    $ErrorActionPreference = 'Stop'
    Remove-Item -LiteralPath $package -Recurse -Force
    $sum = Sha $archive
    if ($badSum) { $sum = '0' * 64 }
    [System.IO.File]::WriteAllText("$archive.sha256", "$sum  $name.tar.gz`n")
    $answer = $script:answer.Clone()
    if (-not $answer.ContainsKey('headSha')) { $answer.headSha = $commit }
    $json = Join-Path $dir 'gh.json'
    [System.IO.File]::WriteAllText($json, ($answer | ConvertTo-Json))
    $env:FAKE_GH_JSON = $json
    $env:FAKE_GH_AUTH = "$($script:ghAuth)"
    $script:before = Sha $archive
    $work = Join-Path $dir 'work'
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    $script:log = Join-Path $dir 'check.log'
    $ErrorActionPreference = 'Continue'
    & powershell -NoProfile -ExecutionPolicy Bypass -File $checkScript -Archive $archive -Game $game -Checker $Checker -WorkIn $work -Gh $script:gh *> $script:log
    $status = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $script:checked = Join-Path $dir "$name.CHECKED.txt"
    $script:archive = $archive
    $script:work = $work
    Defaults
    $status
}
$elf = [byte[]](0x7f, 0x45, 0x4c, 0x46) + (Bytes 99 30000)
$github = 'Built by GitHub Actions, run https://github.com/slaterain/nv-rs/actions/runs/4242, from refs/heads/main; the maintainers have never started it on this system.'

# P. A package that passes: CHECKED.txt written with the run, the commit and the check's lines; the
# archive unchanged; nothing left of the extraction.
$st = Package 'pass' $elf $onMain $github '' $false
Check "P passes" ($st -eq 0)
$text = if (Test-Path -LiteralPath $checked) { Get-Content -LiteralPath $checked -Raw } else { Get-Content -LiteralPath $log -Raw }
Check "P CHECKED.txt names the run" ($text.Contains('Built by GitHub Actions, run https://github.com/slaterain/nv-rs/actions/runs/4242'))
Check "P CHECKED.txt says GitHub confirmed it" ($text.Contains('GitHub confirms: run 4242'))
Check "P CHECKED.txt names the commit" ($text.Contains("Source commit: $onMain"))
Check "P CHECKED.txt has the program's line" ($text -match 'nv-viewer: 30004 bytes, shares 0 runs .* longest 0')
Check "P CHECKED.txt has the 0 hits line" ($text -match '3 files, 0 needles: 0 hits')
Check "P CHECKED.txt says the game does not run" ($text.Contains('The game does NOT run on Linux or macOS yet'))
Check "P archive unchanged" ((Sha $archive) -eq $before)
Check "P extraction removed" (@(Get-ChildItem -LiteralPath $work -Force).Count -eq 0)
if ($st -ne 0) { Write-Host $text }

# Refusals: status 1, no CHECKED.txt, the extraction gone.
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
Refused 'B a run from another branch (BUILD.txt)' (Package 'branch' $elf $onMain $branch '' $false) "not refs/heads/main"
$local = "Built by scripts/package-playtest.sh on the packager's own machine (Linux x86_64); the maintainers have never started it on this system."
Refused 'B2 built elsewhere than GitHub' (Package 'local' $elf $onMain $local '' $false) "built by a GitHub Actions run"
$elsewhere = $github.Replace('github.com/slaterain/nv-rs/', 'github.com/someone/nv-rs/')
Refused 'R a run address in another repository' (Package 'repo' $elf $onMain $elsewhere '' $false) "is not a run of slaterain/nv-rs"
# What GitHub says, not BUILD.txt: each fact of the run checked.
$script:answer.headBranch = 'feature'
Refused 'G1 GitHub says another branch' (Package 'gh-branch' $elf $onMain $github '' $false) "was on branch 'feature'"
$script:answer.headSha = $offMain
Refused 'G2 GitHub says another commit' (Package 'gh-sha' $elf $onMain $github '' $false) "built $offMain"
$script:answer.workflowName = 'CI'
Refused 'G3 GitHub says another workflow' (Package 'gh-workflow' $elf $onMain $github '' $false) "not the package workflow"
$script:answer.event = 'push'
Refused 'G4 GitHub says not started by hand' (Package 'gh-event' $elf $onMain $github '' $false) "not by hand"
$script:answer.conclusion = 'failure'
Refused 'G5 GitHub says the run failed' (Package 'gh-failed' $elf $onMain $github '' $false) "ended 'failure'"
$script:gh = Join-Path $Scratch 'no-such-gh.exe'
Refused 'G6 no gh' (Package 'gh-missing' $elf $onMain $github '' $false) "(gh) is not installed"
$script:ghAuth = 1
Refused 'G7 gh not signed in' (Package 'gh-auth' $elf $onMain $github '' $false) "not signed in"
# The archive's own list: a second name that differs only in case, and a link.
$dupDir = Join-Path $Scratch 'dup-src'
New-Item -ItemType Directory -Force -Path (Join-Path $dupDir $name) | Out-Null
[System.IO.File]::WriteAllText((Join-Path $dupDir "$name\PLAY.SH"), "not the same file`n")
$dupTar = Join-Path $Scratch 'dup.tar'
$ErrorActionPreference = 'Continue'
& $tar -cf $dupTar -C $dupDir "$name/PLAY.SH" 2>$null
$ErrorActionPreference = 'Stop'
$script:extra = @("@$dupTar")
Refused 'T1 a name twice when case is ignored' (Package 'tar-case' $elf $onMain $github '' $false) "twice when case is ignored"
$linkSpec = Join-Path $Scratch 'link.mtree'
[System.IO.File]::WriteAllText($linkSpec, "#mtree`n$name/link type=link link=/etc/passwd mode=0777`n")
$script:extra = @("@$linkSpec")
Refused 'T2 a link in the archive' (Package 'tar-link' $elf $onMain $github '' $false) "not a plain file or folder"
# A name Windows cannot hold as it is (`notes.`: a trailing dot), beside `notes`.
$dotDir = Join-Path $Scratch 'dot-src'
New-Item -ItemType Directory -Force -Path (Join-Path $dotDir $name) | Out-Null
[System.IO.File]::WriteAllText((Join-Path $dotDir "$name\notes"), "one`n")
$dotSpec = Join-Path $Scratch 'dot.mtree'
[System.IO.File]::WriteAllText($dotSpec, "#mtree`n$name/notes. type=file mode=0644 contents=$((Join-Path $dotDir "$name\notes").Replace('\', '/'))`n$name/notes type=file mode=0644 contents=$((Join-Path $dotDir "$name\notes").Replace('\', '/'))`n")
$script:extra = @("@$dotSpec")
Refused 'T3 a name with a trailing dot' (Package 'tar-dot' $elf $onMain $github '' $false) "a name Windows cannot hold as it is"
# The game's bytes in the program: package-check's run limit (4096) is passed.
$copied = [byte[]](0x7f, 0x45, 0x4c, 0x46) + (Bytes 99 1000) + ([System.IO.File]::ReadAllBytes($game)[100..5299]) + (Bytes 98 1000)
Refused 'L a long run of the game in the program' (Package 'run' $copied $onMain $github '' $false) "package-check found something"
# No program at all (not an ELF file): package-check refuses to compare nothing.
$text = [System.Text.Encoding]::ASCII.GetBytes("just text, no program`n")
Refused 'N no program to compare' (Package 'none' $text $onMain $github '' $false) "package-check found something or could not check (status 2)"

Write-Host "fails: $script:fails"
if ($script:fails -eq 0) { exit 0 } else { exit 1 }
