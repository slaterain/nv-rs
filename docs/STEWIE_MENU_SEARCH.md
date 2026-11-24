# Stewie Menu Search

With lStewieAl's Tweaks enabled, set `bMenuSearch=1` under `[Tweaks]` in
`Data\NVSE\Plugins\nvse_stewie_tweaks.ini`. The option is off by default.
The search fields use the original XML prefabs under
`Data\Menus\Prefabs\lStewieAl\MenuSearch` from the user's Tweaks installation.

The port follows Tweaks 10.05 for Inventory, Stats, Map, Container, Level Up,
Recipe, Barter and the save/load browser. The `[Menu Search]` options control
per-menu availability, filter removal on close and input clearing when search
reopens. Clipboard text is read on paste through the platform's clipboard and
the game's ANSI conversion.

Map refresh shares the centering hook with JIP G87. Recipe open and close
share the native lifecycle hooks with Save Manager. Both features have
independent INI settings. Tests cover the shared hooks, filtering and editor
state, and preserving inventory scroll on Pip-Boy close. Live validation in
each menu remains open.
