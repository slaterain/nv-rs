# Settings for nv-rs. The choices live in the game's own files in nv-rs's folder (never your real
# Documents): userdata\game and userdata\mods each have a Documents\My Games\FalloutNV with the game's
# Fallout.ini, whose [nv-rs ...] sections hold nv-rs's options, and FalloutPrefs.ini. Save changes only
# the lines of the options changed here, in both, in place: notes, order, other keys and a mod's lines
# stay as they are. Before Play.exe's first start those files don't exist yet: then the choices go into
# userdata\settings.ini, and Play.exe moves them into the files at its next start (and renames it
# settings.ini.migrated).
#
# Test hooks (scripts/launcher's tests): NV_RS_NO_WINDOWS=1 saves without showing the window, and
# NV_RS_SETTINGS_CHOOSE="Name=Value;Name=Value" (settings.ini's names) are the choices made.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$folder = Join-Path $PSScriptRoot 'userdata'
$file = Join-Path $folder 'settings.ini'
$games = @('game', 'mods') | ForEach-Object { Join-Path $folder "$_\Documents\My Games\FalloutNV" }
# The files are read and written one character per byte, so every other byte stays as it was.
$latin1 = [Text.Encoding]::GetEncoding(28591)
$inFiles = (-not (Test-Path -LiteralPath $file)) -and (Test-Path -LiteralPath (Join-Path $games[0] 'Fallout.ini'))

# The value of a key as the game finds it: names without case, spaces trimmed, one pair of quotes
# taken off, the first match; a ; line never matches.
function Get-IniValue($text, $section, $key) {
    $inside = $false
    foreach ($line in ($text -split "`n")) {
        $t = $line.Trim()
        if ($t.StartsWith('[')) { $inside = ($t.Substring(1).Split(']')[0].Trim() -eq $section); continue }
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
            $inside = ($t.Substring(1).Split(']')[0].Trim() -eq $section)
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

# Writes the text back only when it changed, keeping a read-only mark.
function Write-Ini($path, $old, $text) {
    if ($text -ceq $old) { return }
    $readOnly = (Test-Path -LiteralPath $path) -and (Get-Item -LiteralPath $path).IsReadOnly
    if ($readOnly) { (Get-Item -LiteralPath $path).IsReadOnly = $false }
    [IO.File]::WriteAllBytes($path, $latin1.GetBytes($text))
    if ($readOnly) { (Get-Item -LiteralPath $path).IsReadOnly = $true }
}

$own = "Game's setting"
# Today's choices by settings.ini's names: from the files, or from settings.ini before the first start.
$cur = @{}
$kept = New-Object System.Collections.Generic.List[string]
$known = @('Borderless', 'Resolution', 'AntiAliasing', 'Anisotropy', 'VSync', 'Preset', 'FrameLimit', 'MenuFrameLimit', 'NV_THREADS', 'NV_RENDER_THREAD')
# Play.exe makes the game's NV_PRESET, NV_FRAME_LIMIT and NV_MENU_FRAME_LIMIT from Preset, FrameLimit and
# MenuFrameLimit, and NV_FIXES was never one of the game's switches: such lines would only look as if
# they did something.
$dropped = @('NV_PRESET', 'NV_FRAME_LIMIT', 'NV_MENU_FRAME_LIMIT', 'NV_FIXES')
if ($inFiles) {
    $ini = Read-Ini (Join-Path $games[0] 'Fallout.ini')
    $prefs = Read-Ini (Join-Path $games[0] 'FalloutPrefs.ini')
    $times = { param($n) if ($n -eq '0') { 'Off' } elseif ($n -match '^\d+$') { "${n}x" } else { $own } }
    $cur['Borderless'] = Get-IniValue $ini 'nv-rs Display' 'bBorderless'
    $w = Get-IniValue $prefs 'Display' 'iSize W'; $h = Get-IniValue $prefs 'Display' 'iSize H'
    $cur['Resolution'] = if ($w -match '^\d+$' -and $h -match '^\d+$') { "${w}x$h" } else { $own }
    $cur['AntiAliasing'] = & $times (Get-IniValue $prefs 'Display' 'iMultiSample')
    $cur['Anisotropy'] = & $times (Get-IniValue $prefs 'Display' 'iMaxAnisotropy')
    $cur['VSync'] = switch (Get-IniValue $ini 'Display' 'iPresentInterval') { '1' { 'On' } '0' { 'Off' } default { $own } }
    $cur['Preset'] = Get-IniValue $ini 'nv-rs General' 'sPreset'
    $cur['FrameLimit'] = Get-IniValue $ini 'nv-rs Display' 'iFrameLimit'
    $cur['MenuFrameLimit'] = Get-IniValue $ini 'nv-rs Display' 'iMenuFrameLimit'
    $cur['NV_THREADS'] = if ((Get-IniValue $ini 'nv-rs Threads' 'bSteppedThreads') -eq '1') { 'stepped' } else { '' }
    $cur['NV_RENDER_THREAD'] = if ((Get-IniValue $ini 'nv-rs Threads' 'bRenderThread') -eq '0') { '0' } else { '' }
    foreach ($name in @($cur.Keys)) { if ($null -eq $cur[$name]) { $cur.Remove($name) } }
} elseif (Test-Path -LiteralPath $file) {
    foreach ($line in [IO.File]::ReadAllLines($file)) {
        if ($line -match '^\s*([^=#;]+?)\s*=\s*(.*)$') {
            $cur[$Matches[1]] = $Matches[2].Trim()
            if ($known -notcontains $Matches[1] -and $dropped -notcontains $Matches[1]) { $kept.Add($line) }
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
$form.ClientSize = New-Object System.Drawing.Size(360, 470)
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
$save.Location = New-Object System.Drawing.Point(184, 430)
$save.DialogResult = 'OK'
$form.Controls.Add($save)
$form.AcceptButton = $save
$cancel = New-Object System.Windows.Forms.Button
$cancel.Text = 'Cancel'
$cancel.Location = New-Object System.Drawing.Point(268, 430)
$cancel.DialogResult = 'Cancel'
$form.Controls.Add($cancel)
$form.CancelButton = $cancel

# What the window shows, by settings.ini's names and values.
function Get-Choices {
    [ordered]@{
        Borderless       = $(if ($mode.SelectedIndex -eq 0) { '1' } else { '0' })
        Resolution       = "$($res.SelectedItem)"
        AntiAliasing     = "$($aa.SelectedItem)"
        Anisotropy       = "$($af.SelectedItem)"
        VSync            = "$($vs.SelectedItem)"
        Preset           = $(if ($pr.SelectedIndex -eq 1) { 'vanilla' } else { 'fixes' })
        FrameLimit       = $(if ($fl.SelectedIndex -eq 0) { '0' } else { "$($fl.SelectedItem)" })
        MenuFrameLimit   = $(if ($ml.SelectedIndex -eq 0) { '0' } else { "$($ml.SelectedItem)" })
        NV_THREADS       = $(if ($th.SelectedIndex -eq 1) { 'stepped' } else { '' })
        NV_RENDER_THREAD = $(if ($rt.SelectedIndex -eq 1) { '0' } else { '' })
    }
}
$shown = Get-Choices

$noWindow = $env:NV_RS_NO_WINDOWS -eq '1'
if ($noWindow -or $form.ShowDialog() -eq 'OK') {
    $chosen = Get-Choices
    if ($noWindow -and $env:NV_RS_SETTINGS_CHOOSE) {
        foreach ($pair in $env:NV_RS_SETTINGS_CHOOSE.Split(';')) {
            $name, $value = $pair.Split('=', 2)
            if ($chosen.Contains($name)) { $chosen[$name] = $value }
        }
    }
    if ($inFiles) {
        # Only what changed here: each option's own keys, in both ways of playing's files.
        $sizes = { param($v) if ($v -eq 'Off') { '0' } else { $v.TrimEnd('x') } }
        foreach ($dir in $games) {
            $iniPath = Join-Path $dir 'Fallout.ini'
            $prefsPath = Join-Path $dir 'FalloutPrefs.ini'
            if (-not (Test-Path -LiteralPath $iniPath)) { continue }
            $ini = Read-Ini $iniPath; $newIni = $ini
            $prefs = Read-Ini $prefsPath; $newPrefs = $prefs
            foreach ($name in $chosen.Keys) {
                $v = $chosen[$name]
                if ($v -ceq $shown[$name] -or $v -eq $own) { continue }
                switch ($name) {
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
                    'NV_THREADS' { $newIni = Set-IniKey $newIni 'nv-rs Threads' 'bSteppedThreads' $(if ($v -eq 'stepped') { '1' } else { '0' }) }
                    'NV_RENDER_THREAD' { $newIni = Set-IniKey $newIni 'nv-rs Threads' 'bRenderThread' $(if ($v -eq '0') { '0' } else { '1' }) }
                }
            }
            Write-Ini $iniPath $ini $newIni
            if (Test-Path -LiteralPath $prefsPath) { Write-Ini $prefsPath $prefs $newPrefs }
        }
    } else {
        $lines = @('# nv-rs settings (written by Settings.cmd; Play.exe moves them into the game''s files)')
        foreach ($name in @('Borderless', 'Resolution', 'AntiAliasing', 'Anisotropy', 'VSync', 'Preset', 'FrameLimit', 'MenuFrameLimit')) {
            $lines += "$name=$($chosen[$name])"
        }
        if ($chosen['NV_THREADS'] -eq 'stepped') { $lines += 'NV_THREADS=stepped' }
        if ($chosen['NV_RENDER_THREAD'] -eq '0') { $lines += 'NV_RENDER_THREAD=0' }
        $lines += $kept
        [void][IO.Directory]::CreateDirectory($folder)
        [IO.File]::WriteAllLines($file, [string[]]$lines, (New-Object System.Text.UTF8Encoding($false)))
    }
}
