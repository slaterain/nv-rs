# Open tasks

Two lists. **Major systems** are open to contributors: each is a GitHub
issue on `slaterain/nv-rs` titled `[task] <name>`, and you claim it on the
issue before starting (see [CONTRIBUTING.md](../CONTRIBUTING.md), "Claiming
a task"). The **maintainer's list** (playtest bugs and polish in the
systems the maintainer owns) is not open for claiming; report new bugs as
issues instead.

Every task follows [AGENTS.md](../AGENTS.md): trace the behaviour in the
original FalloutNV.exe 1.4.0.525 (or record it from the original game), no
stand-ins, provenance comments, tests, and pull requests that pass
`scripts/acceptance.ps1` (Doc's walk, Back in the Saddle, Ghost Town
Gunfight). Say what was verified by playing and what only by tests. Big
tasks are split into reviewable pull requests.

## Major systems (claim on GitHub)

**M1. Primm quest routes.** Make the Primm quests playable end to end in
the viewer, the way docs/GOODSPRINGS_ROUTE.md did for Goodsprings: drive
each quest through its own scripts, fix what blocks it (tracing each
fix), and add the route to `scripts/acceptance.ps1`. Start with the
Primm deputy/sheriff quest; one quest per pull request.

**M2. Main quest to Novac.** "They Went That-a-Way" from Goodsprings
through Primm, Nipton and Novac: the travel, the scripted scenes (Nipton,
the Legion), the follow-up quests started on the way. Same method as M1.

**M3. Base-game quest coverage matrix.** A generated table of every
base-game quest (like docs/DEAD_MONEY_COVERAGE.md): played, partial, not
played, with the blocker for each, and the generator script in the
repository so it can be rerun. This tells everyone where the gaps are.

**M4. Weapon class coverage.** Verify and complete each class against
the original: energy weapons (beams, plasma), launchers and explosive
projectiles, thrown weapons and mines, unarmed and melee specials (power
attacks, VATS specials), scopes. One class per pull request, each with a
short route or test scene.

**M5. Creature AI coverage.** Geckos, coyotes, radscorpions, cazadors,
deathclaws, mantises, robots and turrets: their combat styles, special
attacks, sounds and behaviour packages, each compared with the original.
One creature family per pull request.

**M6. World map, fast travel and the whole Mojave.** Exterior streaming
and LOD across the whole worldspace (not just Goodsprings), the world map
travel rules and timing, and worldspace changes (the Strip, Freeside).

**M7. Original save files.** Research and implement reading the original
game's `.fos` saves (form changes, quest and actor state) into nv-rs
state. Research first: write down the format and scope before code.

**M8. Mods and plugins (milestone M5).** Plugin and archive load order,
loose-file overrides, archive invalidation, and a coverage list of
script-extender functions people's mods use. Reproducible test plugins.

**M9. Other DLCs.** Honest Hearts, Old World Blues, Lonesome Road, Gun
Runners' Arsenal: one DLC per claim, following the Dead Money
contributor's method (docs/HANDOFF_DEAD_MONEY.md).

**M10. Comparison harness with the original game.** Tooling to record
the original game's state along a route (positions, quest stages, AI
packages, health) and diff it against an nv-rs replay of the same route,
so "1:1" can be measured. See docs/METHODOLOGY.md and `research/`.

**M11. Factions, crime, karma and disguises.** Verify and finish
reputation changes, crime detection and bounties, karma, and faction
armour disguises against the original.

**M12. VR (milestone M6).** Shared simulation with action inputs and
headset rendering; docs/VR.md has the architecture.

Areas already owned (see CONTRIBUTING.md): Chazm (terminals, item
scripts, repair, weapon mods, companions, Caravan, casinos, performance),
the Dead Money contributor (Dead Money, crafting).

## Maintainer's list (not open for claiming)

Playtest bugs from build 11 (2026-10-06) and polish in the maintainer's
systems. Report anything new as an issue.
### Physics

**B1. Havok world 1:1 (solver, integration, sleeping, contacts).**
`crates/physics` uses its own solver and sleep rules, labelled as such in
docs/PHYSICS.md. Translate the game's Havok 2010 world step instead:
integration, contact solver and friction/restitution, deactivation
(sleeping) rules, collision filter and layers (already traced), contact
points and callbacks. Symptoms: objects clatter and jitter when they
should rest; tumbleweeds don't roll like the game; dead bodies keep
moving or jiggle; repeated impact sounds. Big task: split it into
sub-PRs (step and integration, then contacts and solver, then sleeping).

**B2. Grab (Z) 1:1.** Carried objects flail, spasm and pass through
things. Trace the game's grab spring (`0095f930`, `00960520`, the
`fZKey…` settings; docs/PHYSICS.md) and the held body's collision. Depends
on B1 for the solver, but the spring itself can land first.

**B3. The crosshair pick misses objects.** Done on
`claude/b3-crosshair-pick` (docs/PLAYER_ACTIONS.md, "The crosshair's
pick"); left: drawn-triangle picking, placeable water, comparison with the
game.

**B4. NPCs fall through or sink into the ground.** Done on
`claude/b4-npc-ground` (docs/PHYSICS.md, "People on the ground"):
`MobileObject::Move`'s land and far-from-camera rules; left: a ragdoll
sunk into the land (B1's solver), comparison with the game.

### Combat effects and damage

**B5. Weapon effects: muzzle flash, projectiles, firing sound position.**
NPC guns show no muzzle flash and their shots aren't heard from where
they are. The player's shots show no projectile or tracer effects; melee
swings and hits show none either. Trace the weapon's fire path (muzzle
flash node, projectile spawn and its effects, the 3D sound attached to
the shooter) and implement it for NPCs and the player. Firing sounds and
muzzle flashes done on `claude/b5-weapon-effects` (docs/WEAPON_EFFECTS.md);
left: projectiles and tracers in flight, the flash's light and particles,
stereo panning, `FireWeapon` objects.

**B6. Bullet impacts: decals, particles and sounds.** Shots hitting the
world or bodies need the game's impact data set (IPDS/IPCT) effects:
decals, particles and sounds by material. `hiteffects` logs the choice
already; make it render and play. Decals, blood spatter and impact sounds done on
`claude/b6-impacts` (docs/WEAPON_EFFECTS.md, Impacts); left: the effect
models draw but aren't visible (billboard mode 1 untraced), decals on
people, parallax.

**B7. Player damage feedback.** Missing: blood on the player, the
hit/damage screen effect, limb crippling with its effects and crippled
animations (limping, shaking arm aim). Trace and implement.

### Animation

**B8. Animation blending and snapping.** Animations snap or break
between groups; Cheyenne's (dog) run looks wrong; creature attacks have
no sound. Trace the game's blend times and transitions
(`TESAnimGroup` blend values, the anim sequence switch) and the
attack-sound text keys.

**B9. Jumping.** Player jump animations (start, loop, land) and jump
physics are missing or wrong. Trace the controller's jump and fall
states and their animation groups.

### Dialogue

**B10. NPC greetings.** NPCs greet too often, sometimes at the same
moment the player starts a conversation. Some NPCs, when activated,
should only say a line (no conversation menu). Trace the hello timers
(`fAIMinGreetingDistance`, greeting cooldowns), how activation decides
between "say a line" and a conversation, and fix both. Done on
`claude/b10-greetings` (docs/DIALOGUE.md, B10); left: four greeting
conditions not decoded (listed there).

**B11. Voice stops after skipping lines.** After skipping some lines,
the next lines show text but play no voice. Done on
`claude/b11-voice-skip`: the voice file name rule (`006172c0`), not the
skip; left: check every line's file name against the voice archives.

**B12. Doc Mitchell traps you in dialogue at the door.** After Doc walks
the player to the door and they talk, leaving the conversation starts
another one with him at once, forever. Done on `claude/b12-doc-dialogue`
(docs/DIALOGUE.md, B12); left: Doc's sandbox walk sticks in the doorway
after the farewell.

**B13. Barter menu over the dialogue.** Opening barter from dialogue
draws the dialogue box in front of the barter menu. Done on
`claude/b13-barter-menu` (docs/DIALOGUE.md, B13); left: the `BarterExit`
line, other menus' fades.

### Opening

**B14. New game opening.** Doc isn't sitting in his chair at the start
(he walks in place); the camera snaps around during the name prompt and
other menus; the help-up sequence isn't right. Compare with the opening
movie/recordings and fix. docs/OPENING.md, docs/FURNITURE.md.

**B15. Character creator (race menu).** The face editor/race menu is
not implemented (it auto-accepts). docs/FACE_CREATION.md and
docs/RACE_SEX_MENU.md have the traced parts (sliders, SI.CTL reader).
Build the real menu.

### Interface and rendering

**B16. Local map.** Too zoomed in, and panning/looking around doesn't
work like the game. Trace the local map's scale, zoom steps and drag.
Done on `claude/b16-local-map` (2026-10-07, [PIPBOY.md](PIPBOY.md)): the
drag no longer runs the map off to its limits, the map is centred again
when the tab is shown, the pad's sticks pan and zoom. The scale (0.9 at the
start, 1.1 a wheel step, 0.1 .. 0.9) matches the exe's code; if it still
feels too close next to the original, compare screenshots of the same place.

**B17. White ball flashing outdoors.** A white sphere sometimes flashes
outdoors. Find which object or effect it is (likely an untextured
particle, light or sun glare) and fix it.

### Performance

**B18. Frame time and hitches.** Measure frame time, loading and
streaming stalls on Goodsprings routes; publish the numbers and remove
the stalls without changing behaviour (M4 in MILESTONES). First numbers
with Chazm's PR #12 in docs/CONTRIB_CHAZM.md (measured on a busy machine;
a clean measurement on an idle one is still to do).

### Smaller follow-ups

**B19. NPCs reposition when the line of fire is blocked.** The game
re-plans with `COMBAT_ACTION_ACQUIRE_LINE_OF_SIGHT` (`00997cf0`,
`CombatState::bTargetBlocked` +0x72); the viewer only holds fire.
docs/NPC_COMBAT.md.

**B20. Aim height follows the target's animation.** Gangers hit in the
leg drop low but shots aim at standing height. Trace whether the game's
aim point follows the pose. docs/NPC_COMBAT.md.

**B21. Linked dialogue lines (INFC) rule.** Since the Dead Money merge,
lines linked from another topic are picked like the topic's own (an
untraced guess): Ringo offers two extra replies. Trace or gate it.

### Playtest of build 23 (2026-10-07)

**B22. The Vigor Tester's screen is invisible.** The menu opened and
worked (sounds, stats changing) but wasn't drawn, since build 11. Cause:
the Caravan table's code reset the HUD camera's blending every frame
whenever the Caravan wasn't open, undoing the Vigor Tester's. Fixed on
`claude/menu-scene-hud`: one system (`game_menus::compose_hud_over_scene`)
decides the HUD camera's output for every menu with a 3D scene.

**B23. Hands and the weapon in menus.** The first-person hands and the
equipped weapon disappear in menus such as barter, and come back too
early when the Pip-Boy is put away. Trace when the game hides and shows
the first-person model around menus and the Pip-Boy's lowering
animation, and do the same.

### In progress

- Nothing; every overnight branch is merged.

Landed (in `main` since 2026-10-07): death into ragdoll, one
radio state and the 2 key, Chazm's PRs #11 and #12 (docs/CONTRIB_CHAZM.md),
B3, B4, B5, B6, B10, B11, B12, B13, B16.

Follow-ups found: creatures without a ragdoll tip onto their side (a
stand-in, `ai::fallen_transform`, from the original baseline) instead of
playing `Death`; ragdolls don't come to rest (B1); receivers (NPC radios)
play without distance falloff; thrown weapons play `SNAM`, which the game
doesn't.

### Work in forks (checked 2026-10-07)

Most forks only hold copies of the maintainer's `claude/*` branches,
which are all in `main` now. Work found that isn't in `main`:

- `abusager13`: `codex/macos-metal-support`, a native macOS / Metal port
  (2 commits). Welcome as a pull request against `main` once rebased;
  claim it on GitHub first.
- `Playcon`: `rebase/*` (INFC links, line of sight, radio functions,
  `--use`). Already in `main`: the first and last unchanged, the other
  two through the Dead Money merge (radio since unified with the Pip-Boy's).
  Nothing left to merge; check against `main` before reopening.
- `suzeclaw-coder`: one commit on `main` (view bob and recoil springs,
  landing dip, heartbeat vignette, flanking "anti-conga" steering, a
  procedural audio synthesizer for tests). These aren't traced from the
  game, so they can't be merged as they are; anything traced from them is
  welcome as its own pull request.