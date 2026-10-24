# Playtesting

Playtesting uses the tester's own Fallout: New Vegas installation. Never include game data in a test package or copy, modify or redistribute the installation. The repository's generated test fixtures are designed to run without game files.

## Local package launch

The Windows package launchers are `Play.ps1` and `Play.cmd`. The PowerShell launcher accepts a data path and an optional place; without a place it starts at `GSDocMitchellHouse`. Use `-NewGame` to begin with a fresh game state. If `-DataPath` is omitted, `NV_RS_DATA` supplies the fallback path.

For example:

```powershell
.\Play.ps1 -DataPath 'C:\Games\Steam\steamapps\common\Fallout New Vegas\Data'
.\Play.ps1 -DataPath 'C:\Games\Steam\steamapps\common\Fallout New Vegas\Data' -Place GSDocMitchellHouse -NewGame
```

Download the experimental ZIP from GitHub Releases and extract the entire folder. Official plugins are loaded by default; -UseActivePlugins opts into your active plugin list. Read BUILD.txt and the release notes before testing. Local saves and reports belong under the package's `userdata` directory.

## Current live route

The verified route is a copied stage-55 opening save moved to the vigor tester in Doc Mitchell's house. Triggering the tester advances to stage 60, Doc gives the instruction, the objective appears, and pressing E opens the original SPECIAL scene. Same-cell F9 reload at the tester also works after script occupancy is reset. This verifies that segment of the route only; it does not demonstrate completion of the opening or campaign.

The opening still needs a full acceptance run. In particular, the reported brief camera turn still needs replay verification during the lying-to-sitting transition, Doc's seated head/eye gaze is unresolved, and the exact opening choreography needs comparison with the original. See [docs/OPENING.md](OPENING.md) and [docs/OPENING_LOOK_IK.md](OPENING_LOOK_IK.md) for evidence and handoff.

A screenshot or a successful launch alone does not establish feature completion. Record the starting state, steps taken, observed result, logs or report, and comparison with the original game. Do not skip progression when checking an acceptance route.

## Acceptance rerun (2026-10-20)

Using the local game `Data` folder on macOS, the default acceptance set
(`doc,vcg02,vms16`) passed on the PR #29 source with a temporary overlay of PR
#108's Metal renderer changes. The recorded route times were 54 s, 455 s, and
340 s respectively. This confirms the scripted quest routes under the Mac
renderer fix; it does not validate a PR #29-only renderer build or a Windows
run. The VCG02 route logged stale-entity `despawn` warnings while streaming
exterior cells after scripted moves, but it reached its completion marker and
awarded 50 XP without a panic. The exterior streaming sources are unchanged
from upstream `main`.
