# Settings for nv-rs. Writes userdata\settings.ini next to this file; Play.exe applies it at each
# start, to its own copy of FalloutPrefs.ini (never your real Documents). A line this window doesn't
# know stays in the file as it is, and Play.exe hands a line starting with NV_ to the game as a switch,
# so new settings can join without a new window.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$folder = Join-Path $PSScriptRoot 'userdata'
$file = Join-Path $folder 'settings.ini'
$known = @('Borderless', 'Resolution', 'AntiAliasing', 'Anisotropy', 'VSync')
$cur = @{}
$kept = New-Object System.Collections.Generic.List[string]
if (Test-Path -LiteralPath $file) {
    foreach ($line in [IO.File]::ReadAllLines($file)) {
        if ($line -match '^\s*([^=#;]+?)\s*=\s*(.*)$') {
            $cur[$Matches[1]] = $Matches[2]
            if ($known -notcontains $Matches[1]) { $kept.Add($line) }
        }
    }
}
function Get-Setting($name, $default) { if ($cur.ContainsKey($name)) { $cur[$name] } else { $default } }

$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$native = "$($screen.Width)x$($screen.Height)"
$own = "Game's setting"
$resolutions = @($own, $native, '3840x2160', '2560x1440', '1920x1080', '1600x900', '1280x720') | Select-Object -Unique

$form = New-Object System.Windows.Forms.Form
$form.Text = 'nv-rs settings'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.StartPosition = 'CenterScreen'
$form.ClientSize = New-Object System.Drawing.Size(360, 290)
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
function New-Combo($items, $selected) {
    $c = New-Object System.Windows.Forms.ComboBox
    $c.DropDownStyle = 'DropDownList'
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

$note = New-Object System.Windows.Forms.Label
$note.Text = "`"$own`" keeps what the game has now.`nBorderless fullscreen covers the main monitor."
$note.Location = New-Object System.Drawing.Point(16, $y)
$note.Size = New-Object System.Drawing.Size(330, 36)
$form.Controls.Add($note)

$save = New-Object System.Windows.Forms.Button
$save.Text = 'Save'
$save.Location = New-Object System.Drawing.Point(184, 250)
$save.DialogResult = 'OK'
$form.Controls.Add($save)
$form.AcceptButton = $save
$cancel = New-Object System.Windows.Forms.Button
$cancel.Text = 'Cancel'
$cancel.Location = New-Object System.Drawing.Point(268, 250)
$cancel.DialogResult = 'Cancel'
$form.Controls.Add($cancel)
$form.CancelButton = $cancel

if ($form.ShowDialog() -eq 'OK') {
    $lines = @(
        '# nv-rs settings (written by Settings.cmd)'
        "Borderless=$(if ($mode.SelectedIndex -eq 0) { 1 } else { 0 })"
        "Resolution=$($res.SelectedItem)"
        "AntiAliasing=$($aa.SelectedItem)"
        "Anisotropy=$($af.SelectedItem)"
        "VSync=$($vs.SelectedItem)"
    ) + $kept
    [void][IO.Directory]::CreateDirectory($folder)
    [IO.File]::WriteAllLines($file, [string[]]$lines, (New-Object System.Text.UTF8Encoding($false)))
}
