# Stewie Save Manager

With lStewieAl's Tweaks enabled, set `bImprovedAutoSave=1` under `[Tweaks]`
in `Data\NVSE\Plugins\nvse_stewie_tweaks.ini`. The option is off by default.
The existing Tweaks INI loader also reads the `[Save Manager]` options from
the main file and overlays.

The port follows Tweaks 10.05 for timed autosaves, numbered rotation,
manual and incremental save keys, the deferred reload key, and the optional
pause-menu Quicksave button. It also handles the active-save highlight and
autosave triggers for location discovery, quest completion, exit, pre-level-up,
pickpocket and crafting. Successful numbered saves persist their indices
through the portable mod-file layer.

The scripted-save option applies at the original plugin's script-save gate.
Tests cover save timing, event and menu ordering, and index persistence.
Live save and reload with the option enabled still need validation.
