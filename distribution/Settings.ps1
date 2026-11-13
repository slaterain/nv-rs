# Settings for nv-rs. The choices live in the game's own files in nv-rs's folder (never your real
# Documents): userdata\game and userdata\mods each have a Documents\My Games\FalloutNV with the game's
# Fallout.ini, whose [nv-rs ...] sections hold nv-rs's options, and FalloutPrefs.ini. Save changes only
# the lines of the options changed here, in both, in place: notes, order, other keys and a mod's lines
# stay as they are.
#
# The files are edited only once Play.exe has put nv-rs's sections into the game profile's Fallout.ini,
# and only while there is no userdata\settings.ini. Otherwise the choices go into settings.ini: before
# Play.exe's first start, for a Fallout.ini from before the sections (a key added here would leave it
# half made), and while a settings.ini is there (it is the settings then, for Play.exe too). Play.exe
# moves settings.ini into both profiles' files at its next start, its lines over what the files say,
# and renames it settings.ini.migrated.
#
# Test hooks (scripts/launcher's tests): NV_RS_NO_WINDOWS=1 saves without showing the window, and
# NV_RS_SETTINGS_CHOOSE="Name=Value;Name=Value" (settings.ini's names) are the choices made.

# No progress records: with its output redirected, Windows PowerShell writes them to the error stream.
$ProgressPreference = 'SilentlyContinue'
# Any error stops the script, and nothing is written after it: a file it could not read would
# otherwise look like one without the settings, and its defaults would be saved over the
# player's choices. Settings.cmd hides this script's own window, so the error is shown here.
$ErrorActionPreference = 'Stop'
trap {
    $text = "The settings were not saved.`n`n$($_.Exception.Message)`n`nClose the game and any program that has nv-rs's files open, then open Settings again, choose and click Save."
    if ($env:NV_RS_NO_WINDOWS -eq '1') { [Console]::Error.WriteLine($text) }
    else { try { [void][System.Windows.Forms.MessageBox]::Show($text, 'nv-rs settings', 'OK', 'Error') } catch { } }
    exit 1
}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$folder = Join-Path $PSScriptRoot 'userdata'
$file = Join-Path $folder 'settings.ini'
$games = @('game', 'mods') | ForEach-Object { Join-Path $folder "$_\Documents\My Games\FalloutNV" }
# The files are read and written one character per byte, so every other byte stays as it was.
$latin1 = [Text.Encoding]::GetEncoding(28591)
$noWindow = $env:NV_RS_NO_WINDOWS -eq '1'
$failed = $false

# The value of a key as the game finds it: names without case, spaces trimmed, one pair of quotes
# taken off, the first match; a ; line never matches.
function Get-IniValue($text, $section, $key) {
    $inside = $false
    foreach ($line in ($text -split "`n")) {
        $t = $line.Trim()
        # Only the first section of the name counts, as Windows reads a repeated one.
        if ($t.StartsWith('[')) { if ($inside) { return $null }; $inside = ($t.Substring(1).Split(']')[0].Trim() -eq $section.Trim()); continue }
        $i = $t.IndexOf('=')
        if (-not $inside -or $i -lt 0) { continue }
        if ($t.Substring(0, $i).Trim() -eq $key) {
            $v = $t.Substring($i + 1).Trim()
            if ($v.Length -ge 2 -and $v.StartsWith('"') -and $v.EndsWith('"')) { $v = $v.Substring(1, $v.Length - 2) }
            return $v
        }
    }
    $null
}

# Whether a Fallout.ini's text has one of nv-rs's sections, as Play.exe looks for them
# (scripts/launcher game_ini.rs has_sections).
$ours = @('nv-rs General', 'nv-rs Display', 'nv-rs Threads', 'nv-rs Fixes')
function Test-Sections($text) {
    foreach ($line in ($text -split "`n")) {
        $t = $line.Trim()
        if ($t.StartsWith('[') -and $ours -contains $t.Substring(1).Split(']')[0].Trim()) { return $true }
    }
    $false
}

# The text with key=value in section, as Play.exe sets it (scripts/launcher game_ini.rs set_key): the
# key's value replaced where it stands; else a ;key= line of the section turned into the key; else the
# line added under the section's heading; else the section added at the end. Line ends stay.
function Set-IniKey($text, $section, $key, $value) {
    $eol = if ($text.Contains("`r`n") -or -not $text.Contains("`n")) { "`r`n" } else { "`n" }
    $inside = $false; $under = -1; $off = $null; $at = 0
    foreach ($line in [regex]::Split($text, '(?<=\n)')) {
        $start = $at; $at += $line.Length
        $body = $line.TrimEnd([char[]]"`r`n")
        $t = $body.Trim()
        if ($t.StartsWith('[')) {
            # Only the first section of the name, as Windows writes a repeated one.
            if ($inside) { break }
            $inside = ($t.Substring(1).Split(']')[0].Trim() -eq $section.Trim())
            if ($inside -and $under -lt 0) { $under = $at }
            continue
        }
        $i = $body.IndexOf('=')
        if (-not $inside -or $i -lt 0) { continue }
        $name = $body.Substring(0, $i).Trim()
        if ($name -eq $key) { return $text.Substring(0, $start + $i + 1) + $value + $text.Substring($start + $body.Length) }
        if ($null -eq $off -and $name.StartsWith(';') -and $name.Substring(1).Trim() -eq $key) { $off = @($start, ($start + $body.Length)) }
    }
    if ($null -ne $off) { return $text.Substring(0, $off[0]) + "$key=$value" + $text.Substring($off[1]) }
    if ($under -ge 0) {
        $gap = if ($text.Substring(0, $under).EndsWith("`n")) { '' } else { $eol }
        return $text.Substring(0, $under) + $gap + "$key=$value$eol" + $text.Substring($under)
    }
    $gap = if ($text.Length -eq 0 -or $text.EndsWith("`n")) { '' } else { $eol }
    $text + $gap + "[$section]$eol$key=$value$eol"
}

function Read-Ini($path) {
    if (Test-Path -LiteralPath $path) { $latin1.GetString([IO.File]::ReadAllBytes($path)) } else { '' }
}

# A save that fails must be seen: Settings.cmd hides this script's own window.
function Show-NotSaved($path, $why) {
    $script:failed = $true
    $text = "The settings were not saved.`n`n$why`n`nFile:`n$path`n`nClose the game and any program that has the file open, then open Settings again, choose and click Save."
    if ($noWindow) { [Console]::Error.WriteLine($text) }
    else { [void][System.Windows.Forms.MessageBox]::Show($text, 'nv-rs settings', 'OK', 'Error') }
}

# Puts each new text in place of its file ($writes: Path and Bytes). All of them are written whole
# beside their files first (NAME.part); then each takes its file's place in one step. So a full
# disk or a stop mid-write leaves every file whole: the old one or the new one, never a cut one. A
# read-only mark is lifted for that step and put back. Says so when it fails.
function Save-Files($writes) {
    $now = ''
    try {
        foreach ($w in $writes) {
            $now = $w.Path
            [IO.File]::WriteAllBytes("$($w.Path).part", $w.Bytes)
        }
        foreach ($w in $writes) {
            $now = $w.Path
            if (-not (Test-Path -LiteralPath $w.Path)) {
                [IO.File]::Move("$($w.Path).part", $w.Path)
                continue
            }
            $readOnly = (Get-Item -LiteralPath $w.Path -Force -ErrorAction Stop).IsReadOnly
            if ($readOnly) { (Get-Item -LiteralPath $w.Path -Force -ErrorAction Stop).IsReadOnly = $false }
            try { [IO.File]::Replace("$($w.Path).part", $w.Path, [NullString]::Value) }
            finally { if ($readOnly) { (Get-Item -LiteralPath $w.Path -Force -ErrorAction Stop).IsReadOnly = $true } }
        }
    } catch {
        $e = $_.Exception
        if ($e.InnerException) { $e = $e.InnerException }
        Show-NotSaved $now $e.Message
    } finally {
        foreach ($w in $writes) {
            $part = "$($w.Path).part"
            if (Test-Path -LiteralPath $part) { Remove-Item -LiteralPath $part -Force -ErrorAction SilentlyContinue }
        }
    }
}

$own = "Game's setting"
$known = @('Renderer', 'Borderless', 'Resolution', 'AntiAliasing', 'Anisotropy', 'VSync', 'Preset', 'FrameLimit', 'MenuFrameLimit', 'NV_SHOW_FPS', 'NV_THREADS', 'NV_RENDER_THREAD')
# Play.exe makes the game's NV_PRESET, NV_FRAME_LIMIT and NV_MENU_FRAME_LIMIT from Preset, FrameLimit and
# MenuFrameLimit, and NV_FIXES was never one of the game's switches: such lines would only look as if
# they did something.
$dropped = @('NV_PRESET', 'NV_FRAME_LIMIT', 'NV_MENU_FRAME_LIMIT', 'NV_FIXES')

# Where the settings are. Every profile's Fallout.ini has nv-rs's sections ($seeded): Play.exe
# made them ready; one that lacks them would be left with Play.exe's defaults, not the choices
# made here. A settings.ini is there ($pending): it is the settings until Play.exe has moved it.
$ini = Read-Ini (Join-Path $games[0] 'Fallout.ini')
$seeded = $true
foreach ($dir in $games) {
    $path = Join-Path $dir 'Fallout.ini'
    if (-not (Test-Path -LiteralPath $path) -or -not (Test-Sections (Read-Ini $path))) { $seeded = $false }
}
$pending = Test-Path -LiteralPath $file
$inFiles = $seeded -and -not $pending

# Today's choices by settings.ini's names: what the files say, then settings.ini's lines over them
# while it is there, as Play.exe moves it in (a line wins; what no line names stays the files').
$cur = @{}
$kept = New-Object System.Collections.Generic.List[string]
if ($seeded) {
    $prefs = Read-Ini (Join-Path $games[0] 'FalloutPrefs.ini')
    $times = { param($n) if ($n -eq '0') { 'Off' } elseif ($n -match '^\d+$') { "${n}x" } else { $own } }
    $cur['Renderer'] = Get-IniValue $ini 'nv-rs Display' 'sRenderer'
    $cur['Borderless'] = Get-IniValue $ini 'nv-rs Display' 'bBorderless'
    $w = Get-IniValue $prefs 'Display' 'iSize W'; $h = Get-IniValue $prefs 'Display' 'iSize H'
    $cur['Resolution'] = if ($w -match '^\d+$' -and $h -match '^\d+$') { "${w}x$h" } else { $own }
    $cur['AntiAliasing'] = & $times (Get-IniValue $prefs 'Display' 'iMultiSample')
    $cur['Anisotropy'] = & $times (Get-IniValue $prefs 'Display' 'iMaxAnisotropy')
    $cur['VSync'] = switch (Get-IniValue $ini 'Display' 'iPresentInterval') { '1' { 'On' } '0' { 'Off' } default { $own } }
    $cur['Preset'] = Get-IniValue $ini 'nv-rs General' 'sPreset'
    $cur['FrameLimit'] = Get-IniValue $ini 'nv-rs Display' 'iFrameLimit'
    $cur['MenuFrameLimit'] = Get-IniValue $ini 'nv-rs Display' 'iMenuFrameLimit'
    $cur['NV_SHOW_FPS'] = if ((Get-IniValue $ini 'nv-rs Display' 'bShowFrameRate') -eq '1') { '1' } else { '' }
    $cur['NV_THREADS'] = if ((Get-IniValue $ini 'nv-rs Threads' 'bSteppedThreads') -eq '1') { 'stepped' } else { '' }
    $cur['NV_RENDER_THREAD'] = if ((Get-IniValue $ini 'nv-rs Threads' 'bRenderThread') -eq '0') { '0' } else { '' }
    foreach ($name in @($cur.Keys)) { if ($null -eq $cur[$name]) { $cur.Remove($name) } }
}
if ($pending) {
    foreach ($line in [IO.File]::ReadAllLines($file)) {
        if ($line -match '^\s*([^=#;]+?)\s*=\s*(.*)$') {
            $name = $Matches[1]; $value = $Matches[2].Trim()
            if ($known -notcontains $name -and $dropped -notcontains $name) { $kept.Add($line) }
            # As Play.exe reads them: an empty line chooses nothing, and the two thread lines only
            # with the words it knows.
            if ($value -eq '') { continue }
            if ($name -eq 'NV_THREADS' -and @('stepped', 'real') -notcontains $value) { continue }
            if ($name -eq 'NV_RENDER_THREAD' -and @('0', '1') -notcontains $value) { continue }
            if ($name -eq 'NV_SHOW_FPS' -and @('0', '1') -notcontains $value) { continue }
            if ($name -eq 'Renderer' -and @('vulkan', 'd3d9') -notcontains $value.ToLower()) { continue }
            $cur[$name] = $value
        }
    }
}
function Get-Setting($name, $default) { if ($cur.ContainsKey($name)) { $cur[$name] } else { $default } }

$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$native = "$($screen.Width)x$($screen.Height)"
$resolutions = @($own, $native, '3840x2160', '2560x1440', '1920x1080', '1600x900', '1280x720') | Select-Object -Unique

$form = New-Object System.Windows.Forms.Form
$form.Text = 'nv-rs settings'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.StartPosition = 'CenterScreen'
$form.ClientSize = New-Object System.Drawing.Size(360, 542)
$form.Font = New-Object System.Drawing.Font('Segoe UI', 9)

$y = 16
function Add-Row($label, $control) {
    $l = New-Object System.Windows.Forms.Label
    $l.Text = $label
    $l.Location = New-Object System.Drawing.Point(16, ($script:y + 3))
    $l.AutoSize = $true
    $form.Controls.Add($l)
    $control.Location = New-Object System.Drawing.Point(170, $script:y)
    $control.Width = 170
    $form.Controls.Add($control)
    $script:y += 36
}
# A value the list doesn't have (a size or a number written by hand) is kept as a choice of its own.
function New-Combo($items, $selected) {
    $c = New-Object System.Windows.Forms.ComboBox
    $c.DropDownStyle = 'DropDownList'
    if ($selected -and $items -notcontains $selected) { $items = @($items) + $selected }
    [void]$c.Items.AddRange([object[]]$items)
    $i = [Array]::IndexOf([object[]]$items, $selected)
    $c.SelectedIndex = [Math]::Max(0, $i)
    $c
}

# Who draws the game: Vulkan (wgpu; Direct3D 12 on a card without Vulkan), the default, or the original's
# Direct3D 9, the fallback if a game draws wrongly or stops on Vulkan. A file with no sRenderer line (or one that
# chooses nothing) shows the game's default. This 'vulkan' is the host table's default for sRenderer
# (crates/host/data/game_ini.tsv): scripts/launcher's test settings_cmd_shows_the_tables_renderer_default
# compares the two.
$vulkan = 'Vulkan (recommended)'
$d3d9 = 'Direct3D 9 (the original''s)'
$rd = New-Combo @($vulkan, $d3d9) $(if ((Get-Setting 'Renderer' 'vulkan').Trim().ToLower() -eq 'd3d9') { $d3d9 } else { $vulkan })
Add-Row 'Renderer' $rd
# A file with no bBorderless line (or one that chooses nothing) starts as the game's default does,
# borderless, so that is what is shown. This '1' is the host table's default for bBorderless
# (crates/host/data/game_ini.tsv): scripts/launcher's test
# settings_cmd_shows_the_tables_borderless_default compares the two.
$mode = New-Combo @('Borderless fullscreen', 'Window') $(if ((Get-Setting 'Borderless' '1') -eq '0') { 'Window' } else { 'Borderless fullscreen' })
Add-Row 'Display mode' $mode
$res = New-Combo $resolutions (Get-Setting 'Resolution' $own)
Add-Row 'Resolution' $res
$aa = New-Combo @($own, 'Off', '2x', '4x', '8x') (Get-Setting 'AntiAliasing' $own)
Add-Row 'Anti-aliasing' $aa
$af = New-Combo @($own, 'Off', '2x', '4x', '8x', '15x') (Get-Setting 'Anisotropy' $own)
Add-Row 'Anisotropic filtering' $af
$vs = New-Combo @($own, 'On', 'Off') (Get-Setting 'VSync' $own)
Add-Row 'V-sync' $vs
$pr = New-Combo @('Fixes (recommended)', 'Vanilla (the original game)') $(if ((Get-Setting 'Preset' 'fixes') -eq 'vanilla') { 'Vanilla (the original game)' } else { 'Fixes (recommended)' })
Add-Row 'Game fixes' $pr
$limit = Get-Setting 'FrameLimit' '0'
$fl = New-Combo @('No limit', '60', '90', '120', '144', '165', '240') $(if ($limit -eq '0' -or $limit -eq '') { 'No limit' } else { $limit })
Add-Row 'Frame rate limit' $fl
# Menus (main menu, Pip-Boy, dialogue, pause): 120 unless chosen; the frame limit above wins when lower.
$menuLimit = Get-Setting 'MenuFrameLimit' '120'
if ($menuLimit -eq '') { $menuLimit = '120' }
$ml = New-Combo @('No limit', '30', '60', '90', '120', '144', '165', '240') $(if ($menuLimit -eq '0') { 'No limit' } else { $menuLimit })
Add-Row 'Frame rate limit in menus' $ml
# The frame-rate counter in the top-right corner from the start, or hidden (the default); F11 in the
# game shows or hides it either way (Vulkan only). Hidden is the host table's default for bShowFrameRate.
$fpsHidden = 'Hidden (F11 shows it)'
$fpsShown = 'Shown (F11 hides it)'
$fps = New-Combo @($fpsHidden, $fpsShown) $(if ((Get-Setting 'NV_SHOW_FPS' '').Trim() -eq '1') { $fpsShown } else { $fpsHidden })
Add-Row 'Frame rate counter' $fps
# The game's threads side by side on their own cores (the default) or one at a time, slower: the
# fallback if a game misbehaves with them side by side.
$together = 'Side by side (recommended)'
$apart = 'One at a time (slower)'
$th = New-Combo @($together, $apart) $(if ((Get-Setting 'NV_THREADS' '') -eq 'stepped') { $apart } else { $together })
Add-Row 'Game threads' $th
# The game's Direct3D drawing on a thread of its own (the default) or on the game's main thread, as
# the original draws, slower: the fallback if a game draws wrongly or stops with its own thread.
$ownThread = 'Own thread (recommended)'
$mainThread = 'Main thread (slower)'
$rt = New-Combo @($ownThread, $mainThread) $(if ((Get-Setting 'NV_RENDER_THREAD' '').Trim() -eq '0') { $mainThread } else { $ownThread })
Add-Row 'Drawing thread' $rt

$note = New-Object System.Windows.Forms.Label
$note.Text = "`"$own`" keeps what the game has now.`nBorderless fullscreen covers the main monitor."
$note.Location = New-Object System.Drawing.Point(16, $y)
$note.Size = New-Object System.Drawing.Size(330, 36)
$form.Controls.Add($note)

$save = New-Object System.Windows.Forms.Button
$save.Text = 'Save'
$save.Location = New-Object System.Drawing.Point(184, 502)
$save.DialogResult = 'OK'
$form.Controls.Add($save)
$form.AcceptButton = $save
$cancel = New-Object System.Windows.Forms.Button
$cancel.Text = 'Cancel'
$cancel.Location = New-Object System.Drawing.Point(268, 502)
$cancel.DialogResult = 'Cancel'
$form.Controls.Add($cancel)
$form.CancelButton = $cancel

# What the window shows, by settings.ini's names and values.
function Get-Choices {
    [ordered]@{
        Renderer         = $(if ($rd.SelectedIndex -eq 1) { 'd3d9' } else { 'vulkan' })
        Borderless       = $(if ($mode.SelectedIndex -eq 0) { '1' } else { '0' })
        Resolution       = "$($res.SelectedItem)"
        AntiAliasing     = "$($aa.SelectedItem)"
        Anisotropy       = "$($af.SelectedItem)"
        VSync            = "$($vs.SelectedItem)"
        Preset           = $(if ($pr.SelectedIndex -eq 1) { 'vanilla' } else { 'fixes' })
        FrameLimit       = $(if ($fl.SelectedIndex -eq 0) { '0' } else { "$($fl.SelectedItem)" })
        MenuFrameLimit   = $(if ($ml.SelectedIndex -eq 0) { '0' } else { "$($ml.SelectedItem)" })
        NV_SHOW_FPS      = $(if ($fps.SelectedIndex -eq 1) { '1' } else { '' })
        NV_THREADS       = $(if ($th.SelectedIndex -eq 1) { 'stepped' } else { '' })
        NV_RENDER_THREAD = $(if ($rt.SelectedIndex -eq 1) { '0' } else { '' })
    }
}
$shown = Get-Choices

if ($noWindow -or $form.ShowDialog() -eq 'OK') {
    $chosen = Get-Choices
    if ($noWindow -and $env:NV_RS_SETTINGS_CHOOSE) {
        foreach ($pair in $env:NV_RS_SETTINGS_CHOOSE.Split(';')) {
            $name, $value = $pair.Split('=', 2)
            if ($chosen.Contains($name)) { $chosen[$name] = $value }
        }
    }
    $writes = @()
    if ($inFiles) {
        # Only what changed here: each option's own keys, in both ways of playing's files. The game
        # profile's files go last: this window shows what they say, so after a save that failed part
        # of the way it still shows what was not saved, and saving again finishes it.
        $sizes = { param($v) if ($v -eq 'Off') { '0' } else { $v.TrimEnd('x') } }
        foreach ($dir in @($games[1], $games[0])) {
            $iniPath = Join-Path $dir 'Fallout.ini'
            $prefsPath = Join-Path $dir 'FalloutPrefs.ini'
            if (-not (Test-Path -LiteralPath $iniPath)) { continue }
            $ini = Read-Ini $iniPath; $newIni = $ini
            # A file without nv-rs's sections is Play.exe's to make ready at its next start.
            if (-not (Test-Sections $ini)) { continue }
            $prefs = Read-Ini $prefsPath; $newPrefs = $prefs
            foreach ($name in $chosen.Keys) {
                $v = $chosen[$name]
                if ($v -ceq $shown[$name] -or $v -eq $own) { continue }
                switch ($name) {
                    'Renderer' { $newIni = Set-IniKey $newIni 'nv-rs Display' 'sRenderer' $v }
                    'Borderless' { $newIni = Set-IniKey $newIni 'nv-rs Display' 'bBorderless' $v }
                    'Resolution' {
                        $w, $h = $v.Split('x')
                        $newPrefs = Set-IniKey (Set-IniKey $newPrefs 'Display' 'iSize W' $w) 'Display' 'iSize H' $h
                    }
                    'AntiAliasing' { $newPrefs = Set-IniKey $newPrefs 'Display' 'iMultiSample' (& $sizes $v) }
                    'Anisotropy' { $newPrefs = Set-IniKey $newPrefs 'Display' 'iMaxAnisotropy' (& $sizes $v) }
                    'VSync' { $newIni = Set-IniKey $newIni 'Display' 'iPresentInterval' $(if ($v -eq 'On') { '1' } else { '0' }) }
                    'Preset' { $newIni = Set-IniKey $newIni 'nv-rs General' 'sPreset' $v }
                    'FrameLimit' { $newIni = Set-IniKey $newIni 'nv-rs Display' 'iFrameLimit' $v }
                    'MenuFrameLimit' { $newIni = Set-IniKey $newIni 'nv-rs Display' 'iMenuFrameLimit' $v }
                    'NV_SHOW_FPS' { $newIni = Set-IniKey $newIni 'nv-rs Display' 'bShowFrameRate' $(if ($v -eq '1') { '1' } else { '0' }) }
                    'NV_THREADS' { $newIni = Set-IniKey $newIni 'nv-rs Threads' 'bSteppedThreads' $(if ($v -eq 'stepped') { '1' } else { '0' }) }
                    'NV_RENDER_THREAD' { $newIni = Set-IniKey $newIni 'nv-rs Threads' 'bRenderThread' $(if ($v -eq '0') { '0' } else { '1' }) }
                }
            }
            if ([string]::CompareOrdinal($newPrefs, $prefs) -ne 0 -and (Test-Path -LiteralPath $prefsPath)) {
                $writes += @{ Path = $prefsPath; Bytes = $latin1.GetBytes($newPrefs) }
            }
            if ([string]::CompareOrdinal($newIni, $ini) -ne 0) {
                $writes += @{ Path = $iniPath; Bytes = $latin1.GetBytes($newIni) }
            }
        }
    } else {
        # Every option has its line, so each choice shown here wins when Play.exe moves the file in.
        $lines = @('# nv-rs settings (written by Settings.cmd; Play.exe moves them into the game''s files)')
        foreach ($name in @('Renderer', 'Borderless', 'Resolution', 'AntiAliasing', 'Anisotropy', 'VSync', 'Preset', 'FrameLimit', 'MenuFrameLimit')) {
            $lines += "$name=$($chosen[$name])"
        }
        $lines += "NV_SHOW_FPS=$(if ($chosen['NV_SHOW_FPS'] -eq '1') { '1' } else { '0' })"
        $lines += "NV_THREADS=$(if ($chosen['NV_THREADS'] -eq 'stepped') { 'stepped' } else { 'real' })"
        $lines += "NV_RENDER_THREAD=$(if ($chosen['NV_RENDER_THREAD'] -eq '0') { '0' } else { '1' })"
        $lines += $kept
        try { [void][IO.Directory]::CreateDirectory($folder) } catch { Show-NotSaved $folder $_.Exception.Message }
        $text = ($lines -join "`r`n") + "`r`n"
        $writes += @{ Path = $file; Bytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($text) }
    }
    if ($writes.Count -gt 0 -and -not $failed) { Save-Files $writes }
}
if ($failed) { exit 1 }
