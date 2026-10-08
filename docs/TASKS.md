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
docs/PHYSICS.md. Translate the game's own Havok instead: the exe has
**Havok 7.1.0-r1** (built 2009-12-22) compiled in, and the Xbox
prototype's PDB names its classes and layouts. Only those two sources:
no Havok SDK (proprietary, and a different version). Symptoms: objects
clatter and jitter when they should rest; tumbleweeds don't roll like the
game; dead bodies keep moving or jiggle; repeated impact sounds; physics
is slow. A map of the exe's Havok (frame driver, fixed 0.016 s steps with
4 solver substeps, the integrator, sleeping every 4th step under 0.02
units with 5 passing checks, one contact event per new contact point,
friction √(f₁f₂), Bethesda's overrides) is in the maintainer's private
research notes; it goes into docs/PHYSICS.md with the first PR.
Planned pull requests, in order: (1) world constants and solver
settings, (2) the step driver, (3) the single-body integrator,
(4) sleeping, (5) simulation islands, (6) the contact manager with
per-point events, (7) the contact solver (maybe two PRs), (8) ragdoll
constraints, (9) continuous collision, (10) the character proxy (its
own task). 1–4 should already stop the jitter and the jiggling. Progress: PRs 1–7 merged (#39, #43: constants, step driver, integrator, sleeping, islands, contact manager with per-point events, contact solver; docs/PHYSICS.md). PR 10 (the character proxy: Bethesda's controller, its states and Havok's proxy and simplex solver for every walker; docs/PHYSICS.md, "The character proxy"); left there: pushing bodies (`applySurfaceInteractions`), Havok's collision agents (stand-ins), swimming/flying/climbing. Not started: PR 8 (ragdoll constraints), PR 9 (continuous collision).

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
Traced on `claude/b14-opening` (docs/OPENING.md, B14), not fixed: the
game walks Doc round his chair to its front marker under the 7 s fade;
the viewer's path goes through the chair and his controller is blocked.
The help-up waits on that; the menu camera snap wasn't reproduced.
`claude/b14-navmesh-obstacles`: `NavMeshObstacleManager` traced and ruled
out (only forms with flag 0x02000000 are obstacles, `00564bc0` →
`00401210`; `Chair01F` and every FURN lack it).
`claude/b14-doc-seated`: **Doc seated, fixed.** After a cell load the game
seats every loaded actor in their package's furniture (`00972d30` → the
instant sit `0088d2f0`). The earlier "walked under the fade" reading was
wrong because it took `0088d2f0` for player-only. People sitting down or
seated no longer block others (`00920d00` sets flag 0x08000000, which
`00c711d0` honours). Verified live: Doc is seated from the first frame and
gets up with his special exit at the help-up; Easy Pete is seated on his
porch. Left: the pass over actors already loaded at a later load, the
eat/sleep/patrol branches, the help-up timing against the original, and
the menu camera (not reproduced).
`claude/b14-helpup` (play build 28 report): fixed that Doc's exit waited
for his seated idle to finish (`SitChairRelaxA`, 16 s); the game's stand
update `00921e80` waits only while an idle is starting (`00498f80`), so he
now gets up at `00104BFA`'s `evp` (64.4 s, was 68.2 s). Still about 7 s
after the player's own stand-up begins (end of `00104BF9`); what in the
game, if anything, gets him up earlier isn't established. docs/OPENING.md,
"Help-up timing".

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
**Done** on `claude/b23-b27-first-person`: the game skips its first-person pass only for the dialogue menu (`011d9514`, checked in `00870bd0`), which stays under the barter menu it opens, so the hands stay hidden in barter-from-dialogue as in the game but show behind other menus; the Pip-Boy arm (the game's one first-person model) now holds the view until `Pipboy.kf` ends. Not done: ending a conversation with Tab (the dialogue menu doesn't take it).

### Playtest of builds 23–24 (2026-10-07)

**B24. Easy Pete's face is invisible.** Fixed by #34's texture-cache fix (checked on build 25). Only his beard, moustache and
hat draw (reports 007, 008). #34's texture-cache fix (`ba8e864`: the
shared cache freed textures still in use) fits the symptom; check on
build 25 first. Next suspect if it persists: his `headold.nif` and its
`.tri` disagree on 25 vertices.

**B25. People sink to the waist while moving.** Powder Gangers and other
NPCs sometimes walk waist-deep in the ground. B4's land rule only lifts
feet more than 30 units under the land and its far-from-camera rule puts
people at navmesh height (up to ~21 under the land); this is deeper, so
something else is involved. Reproduce with `NV_GROUND_LOG=1`. Progress (`claude/b1-character-proxy`, B1 PR 10): with the game's own controller nobody near the camera stood more than 10 under the land in the gunfight, Back in the Saddle or the road runs (PHYSICS.md, "The character proxy"); not seen again, so likely the old controller's push-out. Close after a playtest.
Seen again in build 28; re-checked on main 56a3d10 (`claude/b25-sink`, 2026-10-08; logs in
`%USERPROFILE%\nv-re\work\b25`): the vms16 gunfight route, 200 s, `NV_GROUND_LOG=1`, plus a
per-frame trace of ganger 00104C70. Nobody with a controller was more than 10.7 under the
land. The deepest rows are all explained: far from the camera (> 2449.5) people stand at the
navmesh's height (−10 to −24 here, e.g. 00104C68 −23.6; `00697980` interpolates the triangle's
plane as `world::ground::navmesh_height` does); a ganger handed from that rule to his
controller at 2465 units came in 1.93 Havok units (13.5) inside the land triangle under him
(the start collector found it: `Triangle(275418)`, distance −1.93) and rose at the proxy's
penetration recovery (1 Havok unit a second per unit crossed). Frame rate: the same depths at
~200 fps (dt ≈ 0.005) and ~70 fps (dt ≈ 0.013), so the viewer's small dt isn't the cause seen
here. Nothing waist-deep (> 30, which the land rule would lift) was reproduced: not fixed.
Next: a report (F12) with the actor's form ID and place, or compare a far-rule ganger's
height in the original game (if the game shows them on the land at 25–50 m, the navmesh
height or the 2449.5 distance is wrong).

**B26. Ringo greets you as a stranger after the gunfight.** When he comes
over after the Powder Gangers are dead he uses his first-meeting lines.
Trace his greeting's conditions (VMS16 stages, met-before checks) and
why they pass at that point. **Traced (`claude/b26-b30-scripts`): not an
engine fault.** His `GREETING` lines (all `VFreeformGoodsprings`, file
order) put the first meeting, 00104C5F (only `GetIsID`, say once), before
the stage-100 thanks, 00105D0D; the game skips a line only once its
said byte (`INFO+0x22`, say-once flag `+0x25` 0x04, `0061e600`) is set.
The vms16 acceptance route never meets Ringo (it replays the quest's
result scripts from stage 65), so the game would greet him as a stranger
too. Live: met first in `GSGasStation`, then `SetStage VMS16 100`, he
says "I owe you a huge favor" ([GOODSPRINGS_ROUTE.md](GOODSPRINGS_ROUTE.md)).

**B27. The first-person hand looks pale and untextured** (reports 007,
008). Check its texture and skin tint (the player's FaceGen body tint)
and how the first-person model's materials are built.
**Done** on `claude/b23-b27-first-person`: the hands had no body tint; the game makes the player's (and any NPC's without a `bodymods` file) from the race's `UpperBodyHumanMale.egt` and race + NPC `FGTS` (`006149b0`, `00652af0`, `0065b410`, `0064ceb0`): `nif::Egt`, `world::actor::MadeBodyTint`.

**B28. The Prospector Saloon's window is see-through** (report 007). The
game draws it as glass with its window environment map; trace the
shader flags (window environment map, alpha) and draw it the same way.

**B29. The "revise your character" prompt looped.** Fixed in #33: a
message box's answer goes only to the script that showed it (`005b4630`,
`005b4a80`), and references moved by scripts run where they are.

**B30. `GetAV XP` before any XP.** `player.GetAV XP` on a player who has
never earned XP returns nothing instead of 0, which stops script blocks
that test it (VCG04's prompt can't appear before the first XP). Trace the
actor value getter and return what the game returns. **Fixed on
`claude/b26-b30-scripts`:** `GetActorValue` (`0059c4f0`) always sets its
result from the actor's owner `+0xc` (`0093acb0` for the player: base,
`008803a0`, plus modifiers); the base form's getter (`005f0fb0`) gives 0
for every value no record field holds. XP, poison/radiation/fire/
electric/frost/energy/EMP resistance now give 0, speed mult `ACBS`'s
(case 0x15, `008f21d0`). Still none: crit chance, unarmed damage, damage
resistance and threshold (derived or armour-fed, not traced). Live:
`VCG04ActivatorRef.Activate player 1` at 0 XP shows the revise box.

**B31. The blurred room behind menus.** The Vigor Tester (and the
Pip-Boy, pause and other menus) show the world behind them blurred.
Traced: `00871dc0` draws the static menu background and `00718ab0` picks
its image-space modifier from game data: `PopupBackgroundFX` (the Vigor
Tester's), `InterfaceBackgroundFX`, `PipBackgroundFX`,
`PauseBackgroundFX`. Implement it. **Done on `claude/b31-menu-blur`:**
the capture, hold and release (`0086f450`), the modifier choice and its
values, and the blur effect's passes and weights (`00ba4d20`,
`00ba4270`) ([MENU_FADES.md](MENU_FADES.md)); left: depth of field, a
blur under 1, the Pip-Boy's own capture.

**B33. Greetings on their own, every 30 s.** NPCs shouldn't greet
automatically, and not every 30 seconds; check their random lines,
greetings and talk with each other. **Fixed on
`claude/b33-greetings-chatter`:** B10's four left-out conditions decoded
(the player's AI conversation, their target being the player, the ACTION
head-track slot, a package continuing for the player) and the look
`008bc3d0` leaves when a greeting can't be said, which keeps them from
greeting again till they lose sight of the player; conversations with
others also need a package that allows them (`008a78f0(1)`, `00678610`)
(docs/DIALOGUE.md, B33). Live: 9 greetings → 5 on the same route.

**B32. DLC start scripts re-run on a later cell change.** Fixed
(`claude/b32-dlc-rerun`): not a game-state fault. Quest and reference
script variables persist across cell attach/detach in the game
(`00455490` runs quests from the data handler's list; a reference's
variables live in its `ExtraScript`, `00565870`) and here
(`GameState::variables`; three live `MoveTo` cell changes showed each DLC
message once). The repeats seen came from `scripts/acceptance.ps1`
starting each route as a new game; it now says so. Regression test:
`ref_scripts::once_only_guards_survive_cell_changes`.

### In progress

- Nothing; every overnight branch is merged.

Landed (in `main` since 2026-10-07): death into ragdoll, one
radio state and the 2 key, Chazm's PRs #11 and #12 (docs/CONTRIB_CHAZM.md),
B3, B4, B5, B6, B10, B11, B12, B13, B16; B22 (#32), B29 (#33); Chazm's #31 (#34:
casinos, tutorials, terminals, menu fades, companions, weapon classes,
.fos saves, mods, factions and crime).

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