<#
.SYNOPSIS
Replays the shared acceptance routes in the release viewer and reports
pass/fail for each. Every pull request that touches the game must pass
them (CONTRIBUTING.md).

.DESCRIPTION
Routes (commands and success lines documented in docs/GOODSPRINGS_ROUTE.md
and docs/PATHING.md):
  doc    Doc Mitchell walks the west rooms to his chair spot and talks
         (GSDocMitchellHouse, VCG01 stage 110).
  vcg02  Back in the Saddle: Sunny's walks, wells, reward (the quest's own
         scripts; quest start, bottle hits, some gecko kills and the
         player's following are console lines).
  vms16  Ghost Town Gunfight with Trudy's help: the gangers come in and
         die, stage 100.
  ede    ED-E My Love's logs, radio, holotape, Followers upgrade and return:
         ED-E's greeting and both log INFOs run through their packages; the
         game-day advance and finished-package cleanup are forced before the
         radio INFO; the holotape item and travel to Hidden Valley are forced,
         but their item and marker-discovery scripts run; the Followers
         handover, three-day wait, Pip-Boy return step and completion package run
         (docs/ED_E_MY_LOVE_ROUTE.md).
A route passes when every one of its success lines appears in the
viewer's output and no panic does.
Every route runs with the viewer's --answer-boxes test aid, which answers
the prompts the game opens as a player would, by rule, as soon as each is
shown: a message box with one button (OK) with it; a box with more buttons
with the next of --box-answers (button numbers in the box's own order, 0
the first); a tutorial box closed; the name entry accepted with Enter (the
name in it). The routes give --box-answers 2: VCG01's "Before you venture
deeper into the wasteland, you may revise your character." box answered
"Finished - Travel Onward" (buttons: Edit Name, Rebuild Character,
Finished - Travel Onward). With the DLCs installed their start-up quests
also open message boxes on a new game (Dead Money's signal, the pre-order
packs' "items added", the level-cap notices, ...), as the game does; left
open, a prompt holds the AI and the input and the route never finishes.
Each answer is logged as "--answer-boxes: ..."; a box with several
buttons and no choice left is left open and logged as waiting. Outputs (logs, screenshots) go to
-Out; they are private, never commit them.

.EXAMPLE
powershell -File scripts\acceptance.ps1 -Data "D:\Steam\steamapps\common\Fallout New Vegas\Data"
powershell -File scripts\acceptance.ps1 -Routes vms16 -Build
powershell -File scripts\acceptance.ps1 -Background
#>
param(
    # The game's Data folder (or set NV_DATA).
    [string]$Data = $env:NV_DATA,
    # Which routes to run, comma-separated (doc, vcg02, vms16, ede).
    [string]$Routes = 'doc,vcg02,vms16,ede',
    # Where logs and screenshots go (default: %USERPROFILE%\nv-re\acceptance\<time>).
    [string]$Out,
    # Build the release viewer first.
    [switch]$Build,
    # Keep the runs out of the way (the viewer's --background: its window
    # behind the others, without the focus, the mouse left alone).
    [switch]$Background
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Data) {
    $default = 'C:\Games\Steam\steamapps\common\Fallout New Vegas\Data'
    if (Test-Path $default) { $Data = $default } else { throw 'Give -Data (the game''s Data folder) or set NV_DATA.' }
}
if (-not $Out) {
    $Out = Join-Path $env:USERPROFILE ("nv-re\acceptance\" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
}
New-Item -ItemType Directory -Force $Out | Out-Null

if ($Build) {
    Push-Location (Join-Path $root 'viewer')
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'viewer release build failed' }
    } finally { Pop-Location }
}
$viewer = Join-Path $root 'viewer\target\release\nv-viewer.exe'
if (-not (Test-Path $viewer)) { throw "No release viewer at $viewer (run with -Build)." }

function Run-Line([string[]]$lines) {
    $a = @()
    foreach ($l in $lines) { $a += '--run'; $a += $l }
    $a
}
function Run-At([int]$at, [string]$line) { @('--run-at', "$at", $line) }

$routeArgs = @{
    doc   = @{
        Args    = @('GSDocMitchellHouse', '--stage', 'VCG01', '110', '--wait', '50')
        Success = @('Dialogue menu: talking to Doc Mitchell')
    }
    vcg02 = @{
        Args    = @('WastelandNV', '--at', '-68250,5800,8480,180') +
            (Run-Line @('SetStage VCG02 5', 'set VFreeformGoodsprings.bMetSunny to 1', 'StartQuest VCG02',
                'SetObjectiveCompleted VCG02 3 1', 'SetObjectiveDisplayed VCG02 5 1', 'SetStage VCG02 10')) +
            (Run-At 40 'set VCG02.nTargetCount to 3') + (Run-At 40 'SetObjectiveCompleted VCG02 10 1') +
            (Run-At 40 'SunnyREF.evp') +
            (90, 110, 130, 150, 170, 190, 210 | ForEach-Object { Run-At $_ 'player.MoveTo SunnyREF' }) +
            (Run-At 230 'player.MoveTo VCG02SunnySneakMarkerREF') +
            (Run-At 240 'VCG02Gecko1REF.Kill') + (Run-At 241 'VCG02Gecko2REF.Kill') +
            (270, 290, 310 | ForEach-Object { Run-At $_ 'player.MoveTo SunnyREF' }) +
            # Well 3's geckos (and well 2's if still alive).
            (Run-At 320 '0010A210.Kill') + (Run-At 321 '0010ABB0.Kill') + (Run-At 322 '0010A20F.Kill') +
            (Run-At 323 '0010A211.Kill') + (Run-At 324 '0010A212.Kill') + (Run-At 325 '0010A213.Kill') +
            (Run-At 340 'SunnyREF.StartConversation player') +
            @('--say', "Okay, I'm in.", '--say', "Sure, I'll come with you.", '--say', "Couldn't hurt.", '--wait', '370')
        Success = @('Completed: Talk to Sunny about your reward', 'XP +50')
    }
    vms16 = @{
        Args    = @('WastelandNV', '--at', '-67845,3000,8400,180') +
            (Run-Line @('set VMS16.bTrudyHelp to 1', 'SetStage VMS16 65', 'SunnyRef.ResetHealth',
                'SunnyRef.MoveTo SunnySpawnMarker', 'SunnyRef.AddScriptPackage SunnyTriggerGunfightDialoguePackage',
                'SunnyRef.RemoveScriptPackage', 'GoodspringsPowderGangMarker.Enable',
                'SunnyRef.AddScriptPackage SunnyTravelPackage', 'RingoRef.AddScriptPackage RingoTravelPackage',
                'RingoRef.AddToFaction GoodspringsFaction 1', 'SetStage VMS16 70', 'set VMS16.bGunFightStart to 1',
                'player.ModAV Health 5000')) +
            @('--wait', '330', '--walk', '--weapon', 'WeapNV9mmPistol')
        Success = @('XP +50')
    }
    ede   = @{
        # ED-E My Love (issue #13): the two stage-10 dispatches and day advance
        # are explicit test lines, but each log INFO and the radio INFO run
        # through the actor's dialogue packages. RemoveScriptPackage is a
        # forced viewer handoff after log two. AddItem runs the holotape's
        # OnAdd script; MoveTo the marker exercises proximity discovery.
        # April's handover INFO, the quest's 3-day timer and MenuMode return
        # run through their scripts; item spawn, travel and dialogue starts are
        # test inputs. Package cleanup after log two remains forced.
        Args    = @('WastelandNV', '--at', '-67845,3000,8400,180') +
            (Run-Line @('StartQuest vDialogueEDE', 'set vDialogueEDE.iLogsPlayed to 0',
                'set vDialogueEDE.iEDEDaysPassed to 5',
                'set VNPCFollowers.bEDEHired to 1', 'SetObjectiveDisplayed vDialogueEDE 10 1',
                'EDE1Ref.Enable', 'EDE1Ref.MoveTo player')) +
            (Run-At 1 'SetStage vDialogueEDE 10') +
            (Run-At 20 'set vDialogueEDE.iEDEDaysPassed to 5') +
            (Run-At 21 'SetStage vDialogueEDE 10') +
            (Run-At 45 'set GameDaysPassed to 10') +
            (Run-At 46 'EDE1Ref.RemoveScriptPackage EDEDialoguePackage') +
            (Run-At 60 'player.AddItem HVMissionDisc01 1') +
            (Run-At 75 'player.MoveTo HiddenValleyMarkerREF') +
            (Run-At 77 'player.MoveTo EDEHomeMarker') +
            (Run-At 100 'player.MoveTo VFSEDEScientistRef') +
            # Moving to April's cell can be deferred until the radio dialogue
            # closes. Give the cell a few seconds to load before starting her
            # handover conversation; later lines leave room for both talks.
            (Run-At 116 'VFSEDEScientistRef.StartConversation player') +
            # Let the quest's delayed upgrade tick record the day ED-E was
            # taken before advancing three days for the return branch.
            (Run-At 145 'set GameDaysPassed to 13') +
            (Run-At 185 'player.MoveTo EDEHomeMarker') +
            (Run-At 200 'EDE2Ref.StartConversation player') +
            @('--say-id', '001579CE', '--say', 'End', '--say', 'End',
                '--say', 'Ok, you can take it for a little while', '--say', 'Log Off',
                '--key-at', '160', 'tab', '--key-at', '164', 'tab', '--wait', '240')
        Success = @('00157F17', '00158829', '0015FF67', '0015FF65', '00160411',
            'Speak to Knight Lorenzo in Hidden Valley about ED-E.',
            'ED-E has returned to Primm.',
            'EDEQuestCompleteDialogue End action', 'Quest completed: ED-E My Love', 'XP +100')
    }
}

$chosen = @($Routes.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
foreach ($r in $chosen) {
    if (-not $routeArgs.ContainsKey($r)) { throw "Unknown route '$r' (doc, vcg02, vms16, ede)." }
}

$results = @()
foreach ($r in $chosen) {
    $spec = $routeArgs[$r]
    $log = Join-Path $Out "$r.log"
    $shot = Join-Path $Out "$r.png"
    # --answer-boxes: prompts answered as they appear, as a player would
    # (see the header); --box-answers 2: VCG01's "revise your character"
    # box answered "Finished - Travel Onward" (its third button).
    $a = @($Data) + $spec.Args + @('--answer-boxes', '--box-answers', '2', '--screenshot', $shot)
    if ($Background) { $a += '--background' }
    # Each route is its own viewer run: a new game, so the DLC start
    # messages show again in every route that starts outside Doc's house.
    Write-Host "== $r (new game)"
    $start = Get-Date
    # Windows PowerShell turns a native program's stderr lines into errors.
    $ErrorActionPreference = 'Continue'
    & $viewer @a > $log 2>&1
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $secs = [int]((Get-Date) - $start).TotalSeconds
    $text = Get-Content -Raw $log
    $missing = @($spec.Success | Where-Object { $text -notmatch [regex]::Escape($_) })
    $panic = $text -match 'panicked at'
    $pass = ($missing.Count -eq 0) -and -not $panic -and $code -eq 0
    $why = if ($panic) { 'panic' } elseif ($code -ne 0) { ('exit code 0x{0:X8}' -f $code) } elseif ($missing.Count) { 'missing: ' + ($missing -join '; ') } else { '' }
    $results += [pscustomobject]@{ Route = $r; Pass = $pass; Seconds = $secs; Detail = $why; Log = $log }
}
$results | Format-Table -AutoSize | Out-String | Write-Host
if ($results | Where-Object { -not $_.Pass }) { exit 1 }
exit 0
