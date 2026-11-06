# Builds the Windows preview package: one ZIP a player can extract and play without building
# anything. The file list below is the whole package: nothing else can get in, and it holds no game
# files, no saves, no reports, no machine tables, no debug files.
#
#   ./scripts/package-playtest.ps1 -Version 2026-11-06.1
#       [-Game 'path of the game's FalloutNV.exe, only read, to check no data of it is in the programs']
#       [-UseBuilt]      use programs built before (with the flags below), under -Target
#       [-Target DIR]    the Cargo target folder (default: target)
#       [-Toolchain32 stable]   the Rust toolchain that has i686-pc-windows-msvc, for the 32-bit tools
#
# The 32-bit tools need `rustup target add i686-pc-windows-msvc` on that toolchain once.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9._-]+$')][string]$Version,
    [string]$Game,
    [switch]$UseBuilt,
    [string]$Target,
    [string]$Toolchain32 = 'stable',
    [string]$Out
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
if (!$Target) { $Target = Join-Path $root 'target' }
if (!$Out) { $Out = Join-Path $root 'dist' }
Push-Location $root
try {
    $commit = git rev-parse HEAD
    if ($LASTEXITCODE) { throw 'Package from a committed Git checkout.' }
    $dirty = git status --porcelain --untracked-files=normal
    if ($dirty) { throw 'Commit or remove untracked changes before packaging.' }
    $licenseLock = (Get-Content -LiteralPath (Join-Path $root 'distribution/DEPENDENCY-LOCK.sha256') -Raw).Trim()
    # The hash of the file with LF line ends, whichever way this checkout has them.
    $lockText = [IO.File]::ReadAllText((Join-Path $root 'Cargo.lock')) -replace "`r`n", "`n"
    $lockHash = [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($lockText))).Replace('-', '').ToLowerInvariant()
    if ($licenseLock -ne $lockHash) {
        throw 'Dependency lockfile changed. Refresh license notices and their lockfile hash before packaging.'
    }
    $notice = Join-Path $root 'distribution/THIRD-PARTY-NOTICES.txt'
    if (!(Test-Path -LiteralPath $notice)) { throw 'Dependency notices must be assembled before packaging.' }

    $x64 = Join-Path $Target 'release'
    $x86 = Join-Path $Target 'i686-pc-windows-msvc/release'
    $tool = Join-Path $x64 'package-check.exe'
    if (!$UseBuilt) {
        # A static C runtime, because a new PC may not have the Visual C++ runtime. The build
        # machine's folders stay out of the programs.
        $registry = Join-Path $(if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }) 'registry\src'
        $rustup = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
        $env:CARGO_ENCODED_RUSTFLAGS = @('-Ctarget-feature=+crt-static', "--remap-path-prefix=$root=nv-rs", "--remap-path-prefix=$registry=cargo", "--remap-path-prefix=$rustup=rustup") -join [char]0x1f
        $env:CARGO_TARGET_DIR = $Target
        cargo build --release --locked -p nv-viewer
        if ($LASTEXITCODE) { throw 'The game program did not build.' }
        foreach ($manifest in @('launcher', 'd3d9-capture', 'package-check')) {
            cargo build --release --locked --manifest-path "scripts/$manifest/Cargo.toml"
            if ($LASTEXITCODE) { throw "$manifest did not build." }
        }
        foreach ($manifest in @('nvapi-capture', 'dsound-capture')) {
            cargo "+$Toolchain32" build --release --locked --target i686-pc-windows-msvc --manifest-path "scripts/$manifest/Cargo.toml"
            if ($LASTEXITCODE) { throw "$manifest did not build." }
        }
        Remove-Item Env:CARGO_ENCODED_RUSTFLAGS
    }

    $name = "nv-rs-preview-$Version-windows-x64"
    $stage = Join-Path $Out $name
    $zip = "$stage.zip"
    if ((Test-Path -LiteralPath $stage) -or (Test-Path -LiteralPath $zip)) { throw 'This package version already exists. Use a new version.' }
    New-Item -ItemType Directory -Path (Join-Path $stage 'app/tools') -Force | Out-Null

    # Our own programs. app\FalloutNV.exe is nv-viewer under that name: NVIDIA's driver fixes and
    # Windows' per-program sound go by the name (docs/SOUND.md), and no link is needed.
    Copy-Item -LiteralPath (Join-Path $x64 'Play.exe') -Destination (Join-Path $stage 'Play.exe')
    Copy-Item -LiteralPath (Join-Path $x64 'nv-viewer.exe') -Destination (Join-Path $stage 'app/FalloutNV.exe')
    Copy-Item -LiteralPath (Join-Path $x64 'd3d9-capture.exe') -Destination (Join-Path $stage 'app/tools/d3d9-capture.exe')
    Copy-Item -LiteralPath (Join-Path $x86 'nvapi-capture.exe') -Destination (Join-Path $stage 'app/tools/nvapi-capture.exe')
    Copy-Item -LiteralPath (Join-Path $x86 'dsound-capture.exe') -Destination (Join-Path $stage 'app/tools/dsound-capture.exe')
    # The launchers, the settings tool and the READ MEs.
    foreach ($file in @('Play with mods.cmd', 'Import my saves.cmd', 'Settings.cmd', 'Settings.ps1', 'READ ME.txt', 'THIRD-PARTY-NOTICES.txt')) {
        Copy-Item -LiteralPath (Join-Path $root "distribution/$file") -Destination $stage
    }
    New-Item -ItemType Directory -Path (Join-Path $stage 'mods') | Out-Null
    Copy-Item -LiteralPath (Join-Path $root 'distribution/mods/READ ME.txt') -Destination (Join-Path $stage 'mods')
    # The licences and notices LEGAL.md asks for.
    Copy-Item -LiteralPath (Join-Path $root 'distribution/licenses') -Destination $stage -Recurse
    foreach ($file in @('LICENSE', 'LEGAL.md')) { Copy-Item -LiteralPath (Join-Path $root $file) -Destination $stage }
    Copy-Item -LiteralPath (Join-Path $root 'crates/vorbis/LICENSE-libvorbis') -Destination (Join-Path $stage 'licenses/LICENSE-libvorbis')
    # Text files for Notepad.
    foreach ($file in @('READ ME.txt', 'mods/READ ME.txt')) {
        $path = Join-Path $stage $file
        $text = [IO.File]::ReadAllText($path) -replace "`r?`n", "`r`n"
        [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding($false)))
    }
    $sha = (Get-FileHash -LiteralPath (Join-Path $stage 'app/FalloutNV.exe') -Algorithm SHA256).Hash
    @("nv-rs Windows preview $Version", "Source commit: $commit", "app\FalloutNV.exe SHA256: $sha", 'Platform: Windows x64', "Source: https://github.com/slaterain/nv-rs/tree/$commit") |
        Set-Content -LiteralPath (Join-Path $stage 'BUILD.txt') -Encoding ascii

    # What must not be in it: the file types first, then the build machine's traces and game data.
    $forbidden = '\.(esm|esp|bsa|nif|kf|dds|ogg|mp3|wav|lip|bik|fos|sdp|pdb|tsv|ini|log|dll|sym)$'
    $bad = Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object { $_.Name -match $forbidden }
    if ($bad) { throw "These must not be in the package: $($bad.FullName -join ', ')" }
    # (LEGAL.md names fnv_decomp, so that one only counts in the programs.)
    $needles = @($env:USERNAME, $env:USERPROFILE, $root) | Where-Object { $_ }
    $programNeedles = @('nv-re', 'build43', 'fnv_decomp', '\target\release')
    $checkArguments = @($stage) + ($needles | ForEach-Object { '--needle'; $_ }) + ($programNeedles | ForEach-Object { '--program-needle'; $_ })
    if ($Game) { $checkArguments += @('--game', $Game) }
    & (Join-Path $x64 'package-check.exe') @checkArguments
    if ($LASTEXITCODE) { throw 'package-check found something that must not be in the package.' }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::CreateFromDirectory($stage, $zip, [IO.Compression.CompressionLevel]::Optimal, $true)
    $zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    "$zipHash  $name.zip" | Set-Content -LiteralPath "$zip.sha256" -Encoding ascii
    [pscustomobject]@{ Zip = $zip; Bytes = (Get-Item -LiteralPath $zip).Length; Checksum = "$zip.sha256"; Commit = $commit; ExecutableSHA256 = $sha }
} finally { Pop-Location }
