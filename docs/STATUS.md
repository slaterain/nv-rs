# Where each system stands

The front page ([README.md](../README.md)) gives one line per system. This page has the detail
behind each line. The per-fix detail (what each fix changes and how to turn it off) is in
[FIXES.md](FIXES.md); every system's internal state is in [SYSTEMS.md](SYSTEMS.md); the plan is in
[MILESTONES.md](MILESTONES.md).

Marks: ✅ working (not free of bugs), 🔨 started, ⬜ planned.

- [The whole game](#the-whole-game)
- [Fixes](#fixes)
- [Windows preview](#windows-preview)
- [Native 64-bit code](#native-64-bit-code)
- [xNVSE built in](#xnvse-built-in)
- [JIP LN built in](#jip-ln-built-in)
- [Mod Configuration Menu (MCM) built in](#mod-configuration-menu-mcm-built-in)
- [UI Organizer (UIO) built in](#ui-organizer-uio-built-in)
- [lStewieAl's Tweaks built in](#lstewieals-tweaks-built-in)
- [Mod installers (FOMOD)](#mod-installers-fomod)
- [The launcher](#the-launcher)
- [Modern graphics (wgpu)](#modern-graphics-wgpu)
- [Game pads](#game-pads)
- [More platforms, co-op, VR](#more-platforms-co-op-vr)

## The whole game

✅ Movement, physics, animation, graphics, sound, combat, AI, quests, dialogue, menus, saves, the
open world and the DLCs: translated from the original's program, checked against the original and
playable. The whole game runs on its own translated code, with the original graphics and sound.
Bugs remain, and it is slower than the original in busy places for now.

## Fixes

✅ Bug fixes are on by default; the original's behaviour is one setting away. Each fix can be set
in the game's own `Fallout.ini`. The first fixes in, each described in [FIXES.md](FIXES.md):

- Frame rate, physics and scripts.
- No shader-compile stutter the first time something is drawn.
- Shorter waits for loading at cell borders.
- Cell offsets for every plugin: a plugin's cells are found through a table built at load, as the
  game finds a master's.
- Files read without the file thread, and without the limit of 512 open files (about 140 plugins)
  or of 2 GB for one file or archive (TTW's LOD archives).
- No crash when running fast across cells.
- The game's threads are safe where they share a scrap heap, the console, face generation and
  loaded models.
- V.A.T.S. chances without the 2-second wait.
- Smaller saves (no witness lists).
- Radio songs with their own length.
- A modded race's own beards (kept under its head's folder) are offered only to that race in the
  character menu.
- No hidden distant water under the Strip.

## Windows preview

✅ [Released](https://github.com/slaterain/nv-rs/releases). It needs Fallout: New Vegas 1.4.0.525;
only the Steam version has been tested so far, and so far it has only been run on one PC. The
download page lists what works and the known issues; [PLAYTESTING.md](PLAYTESTING.md) covers
building and running from source.

## Native 64-bit code

🔨 Started. This is what brings the speed, lifts the 4 GB memory limit and puts every core to work.
In so far:

- The game's threads run on their own cores and signal each other without one shared lock.
- Its drawing runs on a thread of its own.
- Saves reach the disk from a thread of their own.
- Its compressed archives unpack in native code and are read without reopening them.
- Its textures, vertex and index buffers and sound samples are kept outside its 32-bit memory.
- The first native families are on: among them the script expression evaluator (every condition and
  `set ... to` line of a quest or mod script), the world draw's device calls (made directly, one lock
  per half of the draw, repeated states not sent) and the per-actor loops after each frame.
- On the drawing thread, a draw reuses what the draw before it resolved while nothing changed.
- Finding what is on screen, the water's reflection included, is shared out over several cores.

## xNVSE built in

🔨 xNVSE's script commands, with nothing to install. All 654 of its commands are in: the 20
console commands it re-exports are copies of the console's own.

- **The language:** variables, loops, functions, events, the co-save, inventory references, string
  and array commands, mod-local data, format strings made exactly as the C runtime makes them.
- **Commands:** keyboard and mouse input; all of its inventory, miscellaneous form, form list,
  faction and quest commands; all of its reference commands; its game commands (settings, loaded
  files, messages); its UI, console and print commands; its package, dialogue and script commands;
  its version and plugin checks.
- **Script decompiler:** `DecompileScript`, `HasScriptCommand`, and `Assert`'s message naming the
  failed line.
- **Run-time compiler for the old script syntax:** console lines and scripts compiled in the game
  take `Let`, `eval`, loops, `Call`, lambdas and every expression argument, read as xNVSE reads
  them:
  - a line ends at a line feed; a lambda or an expression in parentheses may run over several
    lines;
  - `x = 5`, `x += 1` and `int a, b, c` are understood, and `string_var` and `array_var` can be
    declared;
  - any command's argument can be an expression in parentheses, inside `set` and `if` too. For the
    game's own commands this holds with the fixes preset; under vanilla those lines compile as the
    game compiles them;
  - compile errors go to the console, `nvse.log` and nv-rs's own log.
- **Function files** in `data/nvse/user_defined_functions` are compiled at the first frame
  (`CompileScript`).
- **Documentation:** its documentation dumps to `nvse.log` and its console `help` listing.
- **The new script language** (scripts that start with `name`: lambdas, `for` and `match`, string
  interpolation) compiles in the console, in plugins' compiles and from function files as in
  xNVSE. A call of one of the game's own commands with arguments from it runs with the fixes
  preset; under vanilla it is refused when it compiles.
- **Left out:** the compiled-script cache file, as it only saves compile time.

## JIP LN built in

🔨 JIP LN's script commands and game patches, with nothing to install. All are built, and JIP
answers "installed" in the default fixes preset, together with MCM's plugin and UI Organizer (vanilla
leaves all three off; `bJIPInstalled=0` in `Fallout.ini` turns JIP off). The per-patch detail is in
[FIXES.md, "JIP LN NVSE's game patches"](FIXES.md#jip-ln-nvses-game-patches) and
[FIXES.md, "JIP LN NVSE switched on"](FIXES.md#jip-ln-nvse-switched-on).

JIP LN's command table (its 1,150 slots at their own numbers) and its co-save are in, and all of
its commands: 1,116, plus its two version commands, 1,118 in all. The command count on the front
page is checked by a test (`the_readme_counts_the_built_commands` in `crates/jip`).

### Script commands, by area

- **Forms and game data:** climate, weather, body part and destruction data, ammo and its effects,
  classes, grass, lights, water, texture sets, armour, explosions, impact data, land textures,
  combat styles, globals, factions, perks, packages, worldspaces, image spaces, casinos,
  challenges, map markers, races, weapons' impulse, VATS and critical data, perk entries, levels
  and ranks, activators, weapon sounds, mods and models, quests and their objectives, recipes,
  leveled lists, projectiles, magic items' effects, object and unarmed effects, sound forms, voice
  types and creature models, base forms, loaded forms by type, game settings, forms' descriptions,
  ash pile activators.
- **Effects:** actors' spells, addiction and poison, active and timed effects and the spell
  target conditions, the running image space modifiers.
- **Script tools:** auxiliary variables and ref maps, the maths and distance commands, angles
  between points, form ids as text, one array added to another, the load order and plugin checks,
  mods' own settings files (INI values and sections read, written and removed, and whether a file
  is there), the session time, the game hour.
- **Places:** load screens, terminals and locks, cells (water, climate, lighting, flags, owners),
  radios, buffered and exterior cells, nav meshes, the moon's textures, the physics world's
  gravity, a reference's place on the world map.
- **Actor bases and actors:** aggro, services, templates, faction ranks, karma, body part data,
  actor bases' spells, skeletons, death items, impact materials and sound templates, hotkeys and
  base containers; equipped item data, the last hit's data and node, killers and kill XP, actors'
  factions, packages, idles and equipped slots, processes, combat targets and allies, detection,
  leveling data, an actor's radiation level, whether a hit got through armour, killable teammates,
  idles by condition and face copies, VATS targetability, ash piles, backing away, pushes away
  from a point, dismembering, dead actors cleared, the player made to sneak.
- **The interface:** interface tiles by path (loaded, strings, clicks, removal, gradual floats,
  XML files, templates, menu XML through the built-in UI Organizer), the cursor, the top menu and
  the active tile, menu buttons, messages disabled and their flags and display times, item
  filters, barter caps, prices and offers, the recipe menu, VATS targets, font files, text input
  boxes (a name or a number typed in), the level-up menu, menu item lists as inventory
  references, terminal models, the character creation check, HUD messages with icons and sounds
  and the message queue cleared, system colours, the Pip-Boy light, text sizes in a font, a menu's
  target and the top menu's highlighted item as references.
- **Events:** menus, keys, controls, triggers, discovered locations and crafting; for actors:
  animations, damage, weapons fired and reloaded, hits, critical hits, ragdolls, aid items used and
  projectile impacts; fast travel, cell change, target change, note, dialogue topic and quest
  stage events.
- **AI switches:** combat disabled or a forced target, teammates' health resets, follow and wait
  orders, locked equipment, forced detection, disabled scripts, and the getters of all the
  switches.
- **World switches:** descriptions, reference names and model paths a script sets, items made
  unique for repairs, references and form types the player may not activate, script events and
  activations disabled, teammates passing locked doors, the weather's transition time, swapped LOD
  blocks, scripts that wait, pushes without a ragdoll, companions equipping their own gear or the
  companion inventory's selected item; script blocks disabled and enabled, quest script delays,
  faked script events.
- **Input:** the gamepad (sticks, triggers, dead zones, buttons disabled, held and tapped),
  controls held and the mouse wheel.
- **CC&C companion commands:** distances, factions, NCCS, encumbrance, location names,
  reputation, result scripts, positions as text, task packages.
- **Weapons:** the equipped weapon's silencer, scope and mods, shots a second, the attack queue,
  fired projectiles, the weapon and armour condition penalties, the armour wear cap and
  no-movement combat.
- **References:** the item check, type, teammates, enable and linked children, teleport doors,
  purge and persistent flags, reference flags and radii, primitives (read, set and added),
  crosshair water, fade and container moves, playing animation groups, position, angles, turned
  offsets and editor place into script variables, map markers on references, a sound at a node,
  JIP's own extra data on references and inventory stacks.
- **Moves and turns:** placing at a cell through JIP's position marker; moves to a cell, to the
  editor place, to a position text, to another reference's node, to the crosshair and by offsets
  (`SetPosEx`); turns (`SetAngleEx`, `RotateAroundPoint`) through JIP's own position, angle and
  cell moves; linked references set and restored at loads.
- **Game state:** difficulty, detection and combat, scope and iron sights, kill cam, the Pip-Boy
  radio, time frozen, container respawns, idle loops, days passed, hardcore stages, exact XP
  rewards, actor values by name and keywords, the music state and volumes, the large address
  check, the camera's movement.
- **Script variables:** variables added to scripts, made again from the co-save at a load.
- **Models:** the blocks of a reference's model (translation, rotation, scale, flags, parents and
  children, collision nodes, materials and textures) read and written, renamed and removed;
  texture transform keys; animation sequences played and their speed; a model's dimensions and
  centre.
- **LN miscellaneous:** form, ingestible and zone flags, water traits, biped model lists, pickup
  and container sounds, names, the player's regions and keys.
- **Inventory:** values, all items, base items, no-unequip marks, poison, inventory references for
  stacks, worn items, hotkeys and a scripted item, item health, armour DT and DR, items moved to a
  target, items added, equipped, unequipped and dropped, and an inventory reference's mod flags,
  health and hotkey set.
- **Physics:** references' velocities, pushes, collision properties and layers, the no-collision
  flag, mass, contacts, phantoms, mobile and grabbable; the ray casts (reticle and point to point),
  the hit material and the crosshair pick; actors' character controllers (fall time, velocity,
  tilt, gravity, air state, ground material, the body under the feet).
- **JIP MiniMap:** its own commands: its start, and the local map drawn as JIP draws it, with its
  markers and fog of war.

### Game patches

JIP's game patches change the game's own code. 174 of its 183 are built, inert until
JIP answers "installed"; three more are folded into other rows and six do nothing on this port
(below; that list also names Enhanced Camera's patch, which is not one of the 183). They include:

- **Framework and the player:** the patch framework, the first two fixes, the player's update 3D
  (both views, weapon and queued references), the detection patch (forced detection values and the
  detection fix), the player kept at a reference's node while a script synchronizes them, a busy
  script runner asked to run again skipping its names and log line.
- **Projectiles and combat:** fired projectiles' ammo, count and split beams; the hit and combat
  fixes (equip notices, life state saved, kill challenges, impact data and hit blood, weapon wear,
  on-hit weapon effects, creature spread, the player's killer kept); JIP's hit damage (damage
  threshold and resistance, ammo effects, blocking, armour by body part); ammo scripts on impact;
  a combat controller's excluded combat actions; silenced shots for everyone; V.A.T.S. showing the
  damage threshold JIP's hit damage used; touch spells a script casts at once; JIP's muzzle flash
  for shots a script fires.
- **Survival and rules:** the fatigue floor, hardcore needs and immortal mode (with
  `ToggleHardcoreTracking` and `ToggleImmortalMode`), the weapon condition step for every item a
  script adds, JIP's weapon and armour condition formulas, the armour wear cap and no-movement
  combat, crafting values, weights and ammo effects, Fallout 3 style repair (`bEnableFO3Repair`),
  the repair skill cap setting, Big Guns as a skill (`bEnableBigGunsSkill`), doors stolen by their
  own rules, Medicine or Survival scaling of aid items and chem durations, NPCs keeping perks of
  their own (`uNPCPerks`).
- **Sound and music:** music markers, a place's faction music, a plugin armour's sounds, the
  acoustic space fix, sound pitch settings, shuffled region music, `.ogg` sounds in sound folders.
- **World:** override archives, flagged references' moves not saved, new NPCs and creatures without
  leftover JIP flags, the sun glare texture and high-noon light, the wind speed setting, forced
  climates, `GameDaysPassed` without drift (`bGameDaysPassedFix`), `FalloutCustom.ini`, immobile
  creatures staying up, fixed bodies following their objects, jump height, the sneak collision
  shape, `SetScale` clamped, the weapon node cleared, clouds after leaving a building, the water
  lookup, save and global fixes, fresh mod logs, markers by id, magic effect fixes, loading screens
  made for a place (`bLoadScreenFix`), JIP's light data (flicker and pulse, light editor ids,
  lights tagged in mod meshes), the process lists' time refresh made at JIP's deferred start
  instead of at the game's start-up.
- **Weapons and models:** a modded weapon's "+" in the crosshair, modded weapons' "Name+" in every
  item list, NPCs showing their weapon mods and random NPC weapon mods (`bNPCWeaponMods`), the gun
  wobble as JIP works it out (and the actors a script spares it), body slots a script hides in
  first person, the nodes and models a script adds to worn items and weapons (`InsertNode`,
  `AttachModel`).
- **Interface and input:** the map menu keeping its place once, the interface's two timed fades,
  the corner messages' default time, the barter tab title and vendor caps, the quantity menu's
  mouse wheel, Ctrl taking one and Shift taking all without the quantity menu, the rumble setting,
  the mouse movement switch, the companion wheel a script turns off for an actor, the open item
  list refreshed when a scripted item gives itself to the player, the mouse over the HUD with
  JIP's HUD cursor, message boxes a script shows with `MessageBoxExAlt` (up to 1200 wide), and
  mods' own fonts (89 fonts instead of 8).
- **Guards of nv-rs's own** (fixes preset): text in the empty font 9 measures as nothing where the
  game would crash, and a font file is read no further than the game's font data block; a muzzle
  flash or an explosion whose light record a script took away keeps the game's brightness (the
  missing-data guard row) where JIP reads through nothing.
- **JIP's null checks:** two are in (a null object given to the game's "first word" getter, a
  light without its data in a lit object's light count); the third place JIP lists changes
  nothing, as the game's own loop already stops there.
- **JIP's lights in combat:** shots whose projectile record names a light glow and light what
  they pass (`SetProjectileLight`'s light included), and a muzzle flash's and an explosion's
  light shine at the light record's own brightness. Each lit shot adds a light while it flies,
  with no cap, as in JIP.
- **Loading:** JIP's fixed-string pool in place of the game's slow store of name strings (a hash
  lookup for every model, node and animation name a load makes; unused names freed at the same
  clean-ups as the game's).

Not built, because they do nothing on this port:

- four tiny functions JIP shortens: the port's code for them is already just the answer;
- the expression buffer JIP grows: the built-in xNVSE writes the same bytes;
- JIP's own start-up hook: it hands back the same camera and only fills JIP's own copies;
- the editor's two patches: the port does not run the editor (GECK);
- NVAC's console alerts and Enhanced Camera's ground fix: they patch those programs' DLLs, which the
  port does not load (Enhanced Camera's fix is not one of the 183).

## Mod Configuration Menu (MCM) built in

🔨 The script commands MCM's menu needs, with nothing to install: its nine commands (MCM 1.5's
`MCM.dll`, built from its machine code), its menu traits and its mods' INI settings, kept between
sessions beside the saves. On in the fixes preset with JIP, off in vanilla (`bMCM`, `NV_MCM`).

## UI Organizer (UIO) built in

🔨 UI mods' HUD and menu additions, with nothing to install. On in the fixes preset with JIP,
off in vanilla (`bUIO`, `NV_UIO`). In so far:

- Its lists: UIO's own `supported.txt` of the UI mods it knows (35 entries) is built in, and a
  copy in `Data\uio\` takes its place; the UI mods' `Data\uio\public` lists, with their
  conditions, are read too. The mods' additions go into the menus as UIO puts them, in memory.
- Menu files are read through its reader and its own XML parser (game setting values, user
  traits, its parse cache), and JIP's `InjectUIComponent` goes through it.
- Its tile operators (17 new ones) and text zoom.
- The play log says which lists it read, what it added and why it left anything out.

More in [FIXES.md, "UI Organizer"](FIXES.md#ui-organizer).

## lStewieAl's Tweaks built in

🔨 Its engine fixes and tweaks, with nothing to install. 184 of its changes are built:

- 72 of its crash and leak fixes;
- Engine Optimizations' first change (a save's changed-forms map);
- 111 other fixes and tweaks: 16 to the engines and the console; 14 to forms, cells, scripts, saves
  and sound; 32 to AI, actors and the player; 49 to menus and the HUD.

- Each change is on with the fixes preset and has its own off key in `Fallout.ini`'s
  `[nv-rs Stewie]`.
- Its settings are VNV's, with the player's `nvse_stewie_tweaks.ini` read over them.
- One of the crash fixes, a leak, the port had already fixed.
- The engines and the console (seven fixes): the sun's light comes from the side of the sky its disc
  is on; blood and impact marks on bodies show the variant picked for them; things that are not
  doors no longer count as doors for NPCs' paths; and four crashes are gone (a tree whose distant
  LOD a plugin disabled, with Tweaks' console line naming it; a place whose navigation mesh cannot
  be worked out; a path through a door with no way back; a switch or LOD node that updates only its
  shown part). One Tweaks slip is not copied, under the missing-data guard row.
- Stairs: collision marked as stairs (123 of the game's own meshes, among them the Hoover Dam
  towers' stairs and the vault, utility and basement stairs) counts every riser as a step, so the
  player and NPCs walk up and down them without catching on a riser or being pushed back. Two
  Tweaks slips are not copied, under the missing-data guard row.
- Menus' input: the inventory keeps its scroll place when the Pip-Boy closes; typed text uses
  shift as it is now; arrow keys keep a menu's selection from the mouse; the pad's key repeat no
  longer carries over; the left thumbstick moves the console's pointer; menus get L3 and R3; the
  Pip-Boy light fades when it should; the console shows the selected reference's model path.
- The remaining crash fixes and tweaks come next.

More in [FIXES.md, "lStewieAl's Tweaks"](FIXES.md#lstewieals-tweaks).

## Mod installers (FOMOD)

🔨 Mods that come with an installer, installed by nv-rs itself into its own folder.

- The installer reader (`crates/fomod`) reads FOMOD installers of every version and works out which
  files the player's choices install.
- The mod's archive is opened and unpacked by nv-rs itself (`crates/mod-archive`): .zip and .7z,
  and .fomod files that are either. RAR is not read: the player is told to save it again as .7z or
  .zip.
- Each installed mod gets its own folder with a record of what it installed, in order. "Play with
  mods" lays them over the game, then the hand-filled `mods` folder (`crates/mod-install`, which
  Play.exe uses). Mod archives in the `mods` folder are no longer copied into the game.
- On Linux and macOS, `./play.sh --mods` plays with them the same way, in a profile of its own.
- The text chooser (`crates/mod-chooser`, the same on all three systems: the launcher's `> Mods`,
  which runs Play.exe's or play.sh's `--install-mods` in its window) lists the archives and installed mods,
  asks each installer's questions in plain words, and installs and removes mods.

## The launcher

🔨 One window in the look of the game's own terminals, where the player plays, changes the
settings and the renderer, installs mods and imports saves. `Play.exe` opens it (`crates/nv-launcher`).

- The terminal's font, colour, header lines and intro are read from the player's game at every
  start (nothing of the game ships); before the game is found it has the port's own plain look.
- `> Play` and `> Play with mods` do what Play.exe did, with every message and question in the
  window instead of Windows' message boxes.
- It draws with Direct3D 12 first, so it opens where Vulkan does not.
- On Linux and macOS, `nv-rs` runs play.sh in its window: it compiles and its tests pass there, but
  it has not been tried on those systems yet.
- `> Settings` has the twelve settings Settings.cmd had (saved at once into both profiles, with
  and without mods) and a Testing list (the new lighting, JIP LN built in). Settings.cmd is gone.
- `> Mods` installs or removes mods from archives, and `> Import saves` copies the player's saves
  over (shown when there are saves to import); the package's `.cmd` files are gone (#907). Help
  comes next; play.sh still works.

## Modern graphics (wgpu)

🔨 The game's own shaders and frames draw on wgpu, on Vulkan (Direct3D 12 where a card has no
Vulkan driver). It is the default renderer in play; the launcher's Settings > Renderer chooses the
original's Direct3D 9 instead, which also draws by itself where Vulkan and Direct3D 12 cannot.

- On wgpu alone the game plays from the main menu into Doc Mitchell's house and out onto the
  Goodsprings street.
- There, in the race menu and in the Pip-Boy, its frames match Direct3D 9's to within 1% of
  pixels on Direct3D 12. On Vulkan the graphics driver filters distant textures a shade
  differently; the main menu is identical. Measured on one NVIDIA card with the game at 4x
  antialiasing.
- At 8x antialiasing it runs (its pictures not yet compared), about 8% slower than Direct3D 9 on
  the Goodsprings street.
- What the recorded places draw is prepared at the start behind a loading screen; elsewhere it
  still pauses briefly the first time something new is drawn.
- Other vendors' pictures are not yet compared, though every card, AMD and Intel too, is given
  NVIDIA's shader path and alpha to coverage switch, so foliage under antialiasing draws as on
  NVIDIA.
- F11 in the game shows a frame-rate counter (frames a second and the slowest recent frame) in the
  corner.

## Game pads

🔨 The game shows Xbox buttons whatever the pad.

- **Windows, Xbox-style pads:** play as in the original (the game's own controller code, through
  Windows' XInput, with rumble). The game's Controls settings turn the pad and its vibration off.
- **Windows, PlayStation pads** (DualSense, DualSense Edge, DualShock 4), plugged in or over
  Bluetooth: play the same way with rumble and a steady light bar, read by nv-rs itself (no
  DS4Windows or Steam needed). Not yet tried with a real pad.
- **Windows, other pads** that SDL's controller database knows (764 models, such as 8BitDo and
  Logitech pads in their DirectInput modes): play the same way, without rumble. Each line is
  checked against the pad's own inputs and is not used where it does not fit, and a player can add
  a line for a pad it lacks. Not yet tried with a real pad.
- **Linux, the Steam Deck among them:** pads are read from the system's input devices, with
  rumble: Steam's own pad first (the one a game started from Steam reads; on the Deck its built-in
  controls), then Xbox-style, PlayStation and Nintendo pads. Tried with virtual pads only, not yet
  with a real one.
- Other pads on Linux, and pads on macOS, come next.

## More platforms, co-op, VR

⬜ Planned. Texture loading already needs no Windows DLL (D3DX's, translated byte for byte). Co-op
is planned, with no design yet.
