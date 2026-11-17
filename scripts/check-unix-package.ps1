<#
Checks a Linux or macOS preview package that GitHub Actions built (.github/workflows/preview-packages.yml)
on the maintainer's Windows PC, before it is published, and writes the text published beside it.
The archive itself is never changed: it is published byte for byte as GitHub built it, so the Unix
permissions inside it (play.sh and app/nv-viewer executable) stay exactly as they were made on Linux or
macOS, and its .sha256 stays valid. (Re-packing on Windows would lose them: NTFS keeps no Unix modes.)

  ./scripts/check-unix-package.ps1 -Archive <downloaded>\nv-rs-preview-linux-x86_64-test.tar.gz -Game '<game folder>\FalloutNV.exe'

What it checks, each one a refusal (status 1, nothing written) when it fails:
1. The archive's SHA256 is the one in the .sha256 file beside it (as the workflow uploaded both).
2. The archive's own list (read before extracting): only plain files and folders (no links), every
   name under "<base>/", no "..", no two names equal when case is ignored.
3. It holds one package folder, <base>, with play.sh, app/nv-viewer and BUILD.txt; app/nv-viewer's
   SHA256 is the one BUILD.txt names.
4. BUILD.txt names a run of slaterain/nv-rs from refs/heads/main, and GitHub itself (gh, signed in)
   says that run is the package workflow, started by hand, on main, at BUILD.txt's commit, and that
   it succeeded; this checkout's origin is slaterain/nv-rs and the commit is on its main (git fetch,
   then git merge-base --is-ancestor), so the published source link holds.
5. package-check --game <FalloutNV.exe> --max-run 4096 on the extracted folder ends with status 0,
   read as many files as the archive lists, and names app/nv-viewer with its longest run shared with
   the game (no data of the game's program in it).
Then it writes <archive name without .tar.gz>.CHECKED.txt beside the archive: which run built it, from
which commit, the check's date and result lines. That file is published beside the archive and its
.sha256, and its text goes into the release notes (docs/MAINTAINING.md).

-Checker: a built package-check.exe (default: built here with cargo, release profile).
The game's file is only read; nothing of it is written anywhere.
#>
param(
    [Parameter(Mandatory = $true)][string]$Archive,
    [Parameter(Mandatory = $true)][string]$Game,
    [string]$Checker = '',
    # Where the archive is extracted for the check (a fresh folder inside it, removed after);
    # default the system's temporary folder.
    [string]$WorkIn = '',
    # GitHub's command line, signed in (gh auth login): asked what the run really was.
    [string]$Gh = 'gh'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Refuse([string]$why) {
    Write-Host "check-unix-package: REFUSED: $why"
    exit 1
}

if (-not (Test-Path -LiteralPath $Archive -PathType Leaf)) { Refuse "no archive at $Archive" }
if (-not (Test-Path -LiteralPath $Game -PathType Leaf)) { Refuse "no game file at $Game (the game's FalloutNV.exe)" }
$Archive = (Resolve-Path -LiteralPath $Archive).Path
$Game = (Resolve-Path -LiteralPath $Game).Path
$name = Split-Path -Leaf $Archive
if (-not $name.EndsWith('.tar.gz')) { Refuse "$name is not a .tar.gz" }
$base = $name.Substring(0, $name.Length - '.tar.gz'.Length)
$folderOfArchive = Split-Path -Parent $Archive
$checked = Join-Path $folderOfArchive "$base.CHECKED.txt"
if (Test-Path -LiteralPath $checked) { Refuse "$checked already exists: move it away first" }

# 1. The archive as GitHub uploaded it.
$sumFile = "$Archive.sha256"
if (-not (Test-Path -LiteralPath $sumFile -PathType Leaf)) { Refuse "no $sumFile beside the archive" }
$sumLine = (Get-Content -LiteralPath $sumFile -TotalCount 1)
$expected = ($sumLine -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expected -ne $actual) { Refuse "the archive's SHA256 is $actual, its .sha256 file says $expected" }

# 2. The archive's own list, before anything is extracted: only plain files and folders, every
#    name under "<base>/", no `..`, and no two names that differ only in case (Windows' folders
#    would hold only one of them, and the check would then not see what is published).
$tar = Join-Path $env:SystemRoot 'System32\tar.exe'
if (-not (Test-Path -LiteralPath $tar)) { $tar = 'tar' }
$ErrorActionPreference = 'Continue'
$names = @(& $tar -tzf $Archive 2>$null | ForEach-Object { "$_" })
$namesStatus = $LASTEXITCODE
$long = @(& $tar -tvzf $Archive 2>$null | ForEach-Object { "$_" })
$longStatus = $LASTEXITCODE
$ErrorActionPreference = 'Stop'
if ($namesStatus -ne 0 -or $longStatus -ne 0) { Refuse "tar could not list $name" }
if ($names.Count -eq 0 -or $names.Count -ne $long.Count) { Refuse "tar's two listings of $name do not agree" }
$seen = New-Object 'System.Collections.Generic.HashSet[string]'
$listedFiles = 0
for ($i = 0; $i -lt $names.Count; $i++) {
    $entry = $names[$i]
    $kind = $long[$i].Substring(0, 1)
    if ($kind -ne '-' -and $kind -ne 'd') { Refuse "the archive holds $entry, which is not a plain file or folder (a link or a device)" }
    if ($long[$i].Contains(' link to ')) { Refuse "the archive holds $entry as a hard link" }
    if ($entry -ne "$base/" -and -not $entry.StartsWith("$base/")) { Refuse "the archive holds $entry, outside $base/" }
    if (($entry -split '/') -contains '..') { Refuse "the archive holds $entry, with a .. in it" }
    # A name Windows cannot hold as it is (a trailing dot or space, a reserved character) would be
    # checked under another name, or not removed after.
    foreach ($part in ($entry.TrimEnd('/') -split '/')) {
        if ($part -match '[. ]$' -or $part -match '[\\:*?"<>|]') { Refuse "the archive holds $entry, a name Windows cannot hold as it is" }
    }
    if (-not $seen.Add($entry.TrimEnd('/').ToLowerInvariant())) { Refuse "the archive holds $entry twice when case is ignored" }
    if ($kind -eq '-') { $listedFiles++ }
}

# 3. Extracted into a fresh folder of its own (Windows' own tar reads .tar.gz).
if ($WorkIn -eq '') { $WorkIn = [System.IO.Path]::GetTempPath() }
$work = Join-Path $WorkIn ("nv-rs-check-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    & $tar -xzf $Archive -C $work
    if ($LASTEXITCODE -ne 0) { Refuse "tar could not extract $name" }
    $tops = @(Get-ChildItem -LiteralPath $work -Force)
    if ($tops.Count -ne 1 -or -not $tops[0].PSIsContainer -or $tops[0].Name -ne $base) { Refuse "the archive must hold exactly one package folder, $base" }
    $package = $tops[0].FullName
    foreach ($file in @('play.sh', 'app\nv-viewer', 'BUILD.txt')) {
        if (-not (Test-Path -LiteralPath (Join-Path $package $file) -PathType Leaf)) { Refuse "the package has no $file" }
    }
    $build = Get-Content -LiteralPath (Join-Path $package 'BUILD.txt')
    $title = $build[0]
    $commit = ''
    $programSum = ''
    $run = ''
    $runUrl = ''
    $runRef = ''
    foreach ($line in $build) {
        if ($line -match '^Source commit: ([0-9a-f]{40})$') { $commit = $Matches[1] }
        if ($line -match '^app/nv-viewer SHA256: ([0-9a-f]{64})$') { $programSum = $Matches[1] }
        if ($line -match '^Built by GitHub Actions, run (\S+), from (\S+); ') { $run = $line; $runUrl = $Matches[1]; $runRef = $Matches[2] }
    }
    if ($commit -eq '') { Refuse "BUILD.txt names no source commit" }
    $programActual = (Get-FileHash -LiteralPath (Join-Path $package 'app\nv-viewer') -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($programSum -ne $programActual) { Refuse "app/nv-viewer's SHA256 is $programActual, BUILD.txt says '$programSum'" }

    # 4. Built by the package workflow from main, as GitHub itself says (not only BUILD.txt), at a
    #    commit that is on origin's main.
    if ($run -eq '') { Refuse "BUILD.txt does not say it was built by a GitHub Actions run" }
    if ($runUrl -notmatch '^https://github\.com/slaterain/nv-rs/actions/runs/([0-9]+)$') { Refuse "BUILD.txt's run $runUrl is not a run of slaterain/nv-rs" }
    $runId = $Matches[1]
    if ($runRef -ne 'refs/heads/main') { Refuse "BUILD.txt says the run was from $runRef, not refs/heads/main" }
    $origin = (& git -C $root remote get-url origin)
    if ("$origin" -notmatch 'github\.com[:/]slaterain/nv-rs(\.git)?/?$') { Refuse "this checkout's origin ($origin) is not slaterain/nv-rs" }
    if (-not (Get-Command $Gh -ErrorAction SilentlyContinue)) { Refuse "GitHub's command line (gh) is not installed: install it and sign in (gh auth login). What the run was is asked of GitHub, never taken from BUILD.txt alone." }
    $ErrorActionPreference = 'Continue'
    & $Gh auth status *> $null
    $signedIn = $LASTEXITCODE
    $answer = (& $Gh run view $runId --repo slaterain/nv-rs --json headBranch,headSha,event,workflowName,conclusion 2>$null) -join "`n"
    $answered = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($signedIn -ne 0) { Refuse "gh is not signed in to GitHub (gh auth login): what the run was is asked of GitHub" }
    if ($answered -ne 0 -or $answer.Trim() -eq '') { Refuse "GitHub gave no answer for run $runId of slaterain/nv-rs" }
    $facts = $answer | ConvertFrom-Json
    if ($facts.workflowName -ne 'Preview packages (Linux, macOS)') { Refuse "run $runId is GitHub's '$($facts.workflowName)', not the package workflow" }
    if ($facts.event -ne 'workflow_dispatch') { Refuse "run $runId was started by '$($facts.event)', not by hand (workflow_dispatch)" }
    if ($facts.headBranch -ne 'main') { Refuse "GitHub says run $runId was on branch '$($facts.headBranch)', not main" }
    if ($facts.headSha -ne $commit) { Refuse "GitHub says run $runId built $($facts.headSha), BUILD.txt says $commit" }
    if ($facts.conclusion -ne 'success') { Refuse "GitHub says run $runId ended '$($facts.conclusion)', not success" }
    # (GitHub's digest of the artifact is of the zip it keeps, not of the archive inside it, so it
    # cannot be compared with the archive's SHA256: the .sha256 file came from the same run.)
    & git -C $root fetch --quiet origin main
    if ($LASTEXITCODE -ne 0) { Refuse "could not ask origin for its main branch (git fetch)" }
    & git -C $root merge-base --is-ancestor $commit refs/remotes/origin/main
    if ($LASTEXITCODE -ne 0) { Refuse "the source commit $commit is not on origin's main" }

    # 5. The check the Windows package always makes, against the game's own program.
    if ($Checker -eq '') {
        & cargo build --release --locked --quiet --manifest-path (Join-Path $root 'scripts\package-check\Cargo.toml')
        if ($LASTEXITCODE -ne 0) { Refuse "package-check did not build" }
        $targetDir = $env:CARGO_TARGET_DIR
        if (-not $targetDir) { $targetDir = Join-Path $root 'scripts\package-check\target' }
        $Checker = Join-Path $targetDir 'release\package-check.exe'
    }
    if (-not (Test-Path -LiteralPath $Checker -PathType Leaf)) { Refuse "no package-check at $Checker" }
    # (Its error output read as text: Windows PowerShell would stop on it as an error record.)
    $ErrorActionPreference = 'Continue'
    $output = @(& $Checker $package --game $Game --max-run 4096 2>&1 | ForEach-Object { "$_" })
    $status = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $output | ForEach-Object { Write-Host $_ }
    if ($status -ne 0) { Refuse "package-check found something or could not check (status $status)" }
    $programPattern = 'app[\\/]nv-viewer: [0-9]+ bytes, shares .* longest ([0-9]+)$'
    $programLine = $output | Where-Object { $_ -match $programPattern } | Select-Object -First 1
    if (-not $programLine) { Refuse "package-check's output names no app/nv-viewer: the program was not compared with the game" }
    $null = $programLine -match $programPattern
    if ([int]$Matches[1] -gt 4096) { Refuse "the longest run shared with the game is over 4096 bytes" }
    $filesLine = $output | Where-Object { $_ -match '^[0-9]+ files, [0-9]+ needles: 0 hits$' } | Select-Object -First 1
    if (-not $filesLine) { Refuse "package-check's output has no '0 hits' line" }
    $null = $filesLine -match '^([0-9]+) files'
    if ([int]$Matches[1] -ne $listedFiles) { Refuse "package-check read $($Matches[1]) files, the archive lists $listedFiles" }
    $programLine = $programLine.Replace($package, $base)
}
finally {
    Remove-Item -LiteralPath $work -Recurse -Force
}

$date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
$text = @(
    "$($title): checked before publishing",
    "",
    "Archive: $name, SHA256 $actual (published exactly as GitHub Actions built it)",
    "$run",
    "GitHub confirms: run $runId is the package workflow (Preview packages (Linux, macOS)), started by hand on main at the commit below, and it succeeded.",
    "Source commit: $commit (on the project's main branch: https://github.com/slaterain/nv-rs/tree/$commit)",
    "",
    "Checked on $date on the maintainer's Windows PC with package-check --game --max-run 4096 against",
    "the game's own FalloutNV.exe (only read, not in the package): no data of the game's program is in",
    "the package's program. Its result:",
    "  $filesLine",
    "  $programLine",
    "",
    "The game does NOT run on Linux or macOS yet: this preview finds your game, keeps nv-rs's saves",
    "apart from yours, and says plainly that it cannot start the game yet."
)
[System.IO.File]::WriteAllText($checked, (($text -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))
Write-Host "check-unix-package: PASSED. Wrote $checked"
exit 0
