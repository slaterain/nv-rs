# High-resolution local maps and screens

Both options are off in every preset and can be enabled independently in
Fallout.ini under `[nv-rs Fixes]`.

- `bHighResLocalMaps=1` keeps the game's high-resolution local-map flag set.
  The optional `[Main] iResScale` setting in
  `Data\NVSE\Plugins\HighResLocalMaps.ini` selects 256 pixels for 0,
  1024 pixels for 2, and 512 pixels for other values or a missing setting.
- `bHighResScreens=1` starts the Pip-Boy, terminal and character-creation
  render-target calculations at the game display width. It also preserves
  the original plugin's rendered-menu and mouse-picking changes.

Test runs can set `NV_HIGH_RES_LOCAL_MAPS=1` or `NV_HIGH_RES_SCREENS=1`.
Tests exercise render-target descriptors, input rectangles, map flag lifetime
and profile settings. In-game appearance and input still need validation.

The Rust changes follow the behavior of the 1.2 releases alongside the
FalloutNV.exe 1.4.0.525 translations. The original authors are Ungeziefi
and WallSoGB for local maps, and lStewieAl and Wall_SoGB for screens.
[High Res Local Maps](https://www.nexusmods.com/newvegas/mods/77963)
[High Resolution Screens](https://www.nexusmods.com/newvegas/mods/77989)
