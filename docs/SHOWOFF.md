# ShowOff NVSE

nv-rs provides selected ShowOff NVSE commands from revision
`f45d069d5357968f01246d87bbdcdee4b030dc90`. The implemented commands cover
temporary ownership, form and actor helpers, equipment sets, cached INI reads
and writes, standard JSON reads, auxiliary string maps, temporary menu timers,
keyboard text, clipboard reads and the perk browser. Auxiliary string maps
retain their ShowOff co-save records across a save and reload.

ShowOff engine events are off in every preset. A mod profile can enable them
with `bShowOffEvents=1` under `[nv-rs Fixes]` in Fallout.ini. Test runs can
use `NV_SHOWOFF_EVENTS=1`. This enables inventory-add, fire-weapon, player
jump, point-of-view, projectile-impact, pre-activation and corner-message
callbacks. Pre-activation handlers can veto the original activation. The
projectile event includes the correction from the referenced ShowOff revision.

The built-in remains unlisted in plugin and version queries. Support covers
the standard JSON reader and temporary menu timer behavior. JSON modes 1 and 2,
JSON writing and permanent timer modes remain unsupported. Layout text depends
on observed key events, and dead keys are unavailable. Gameplay validation and
comparison of equipment set ordering with the original plugin remain open.
