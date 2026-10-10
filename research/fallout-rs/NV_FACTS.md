# New Vegas program facts, checked against nv-rs

[nv-facts.tsv](nv-facts.tsv) holds every New Vegas rule fallout-rs has
read from `FalloutNV.exe` 1.4.0.525: 98 rows after removing duplicates
between its tables. For each row the last column says what nv-rs's `main`
(56a3d10) does with the same rule, read in nv-rs's code by hand:

| Verdict | Rows | Meaning |
| --- | --- | --- |
| AGREES | 73 | nv-rs implements the rule the same way (a few with a minor note). |
| DIFFERS | 8 | nv-rs implements it, and its code gives different answers from the program. |
| PARTLY | 4 | nv-rs implements part of it; the missing part is named. |
| NOTHING | 12 | No implementation found in `crates/` or `viewer/`. |
| (not a rule) | 1 | The settings-defaults check (NV-098). |

Status of the rows: 39 `oracle-verified` (the program's function was
called and agreed on every input, see [ORACLE.md](ORACLE.md)), 59
`exe-read`. "Not found" means a search of nv-rs's code for the settings,
addresses and names involved found nothing; it may live under another
name.

The high agreement is to nv-rs's credit: its readings of the program
have been accurate. Several of them corrected fallout-rs (last section).

## 1. Where nv-rs differs from the program (fix these first)

Ordered by how often they change play. Rows 1 to 5 are oracle-verified:
the program itself gave the answers.

### NV-043 Detection value: five terms missing (`00642ed0`, oracle 1500/1500)

`crates/world/src/detection.rs` `value_with` has the New Vegas sneak-score
terms right but lacks parts of the 25-argument sum the program computes:

- light part: `x trunc((100 - invisibility) / 100)` (whole-number
  division of whole numbers: any invisibility above 0 makes the light
  part 0, none leaves it whole), `x size factor`
  where size factor = `1 + (size - 1) x fDetectionLargeActorSizeMult` when
  size > 1, size from `00642e80` (NV-044: 1, or
  `trunc(height / fDetectionLargeActorThreshold)` when taller than the
  threshold);
- with a stealth device: visible counts as 0, and the light part is then
  `x fSneakStealthBoyMult`;
- sound: the line-of-sight cut `x fSneakSoundLosMult` applies only when
  out of sight AND the ambush type is not 1;
- modifier = `1 + alert x fSneakAlertMod + sleeping x fSneakSleepBonus +
  in_combat x fSneakCombatMod + ambush`, ambush type 1
  `fSneakAmbushTargetMod`, type 2 `fSneakAmbushNonTargetMod`;
- sign check: the program ADDS `fSneakCombatMod` (its built-in default is
  -0.4). nv-rs subtracts the setting and falls back to +0.4. If
  FalloutNV.esm sets -0.4, nv-rs adds 0.4 where the game takes 0.4 off.

Which caller input is "alert" and which is "in combat" was read from the
caller `008a0d10`; confirm against nv-rs's own reading of that function.

### NV-001 NPC level from the player's level (`0047ded0`, oracle 1200/1200)

`crates/world/src/scripting.rs` `Facts::level`:

```rust
// nv-rs now
(f32::from(player_level) * f32::from(level) / 1000.0).round() as u16 // then .max(calc_min).max(1)
// the program
let l = (player_level as f64 * ((record_level as f32 / 1000.0) as f64)).trunc() as u16;
let l = if lowest > 0 && l < lowest { lowest } else if highest > 0 && l > highest { highest } else { l };
```

Truncation, not rounding (a level 25 player and a record level of 880
gives 21; nv-rs gives 22), and the highest level (ACBS u16 at 12) caps it.
There is no `max(1)` in this function. The same level feeds creature
combat skill (NV-014) and NPC health (NV-002).

### NV-002 Auto-calculated NPC health (`006436c0`, `00603fc0`, oracle 1600/1600)

`crates/world/src/combat.rs` `max_health` gives every NPC
`base + fAVDNPCHealthEnduranceMult x (END + offset)`. For an NPC whose
record is auto-calculated the program gives
`max(0, trunc((END + fAVDNPCHealthEnduranceOffset) x fAVDNPCHealthEnduranceMult + (max(level, 1) - 1) x fAVDNPCHealthLevelMult))`
with the level from NV-001, and 0 when such an NPC has no class. NPCs with
their own stats and the player already agree.

### NV-003 Radiation and poison resistance from Endurance (`00643e40`, oracle 1200/1200)

`crates/world/src/scripting.rs` `actor_value` returns 0 for actor values
19 and 20. The program derives them:
`RadResist = (END + fAVDRadResistEnduranceOffset) x fAVDRadResistEnduranceMult`,
`PoisonResist = (END + fAVDPoisonResistEnduranceOffset) x fAVDPoisonResistEnduranceMult`.

### NV-004 Heal rate at Endurance above 10 (`00643a70`, oracle 1000/1000)

`unaffected_actor_value` (value 15) does `min(10)` on Endurance, so 11 and
above get the Endurance 10 bonus. The program's jump table covers 6..10
only; everything else, 11 and above included, gives 0. It reads the
current Endurance (floored), not the permanent one.

### NV-005 Melee damage actor value (`00643b50`, exe-read)

`unaffected_actor_value` (value 17) computes `Offset + Mult x STR`. The
program: `floor(fAVDMeleeDamageStrengthMult x (fAVDMeleeDamageStrengthOffset + STR))`
when above 0, else 0. The Fallout 3 twin of this function (`005816c0`) is
oracle-verified in fallout-rs with the same shape.

### NV-080 GetDeadCount (`0089d900`, exe-read)

nv-rs counts the dead references whose base is the asked form at the
time of asking. The program keeps a list of (form, signed 16-bit count)
pairs on the data handler that `Actor::Kill` increments once per death,
before `OnDeath`, under the leveled base from `ExtraLeveledCreature`
(+0xC) when there is one, else the reference's base; disabled or deleted
corpses still count. `Actor::Kill` returns early when the actor is
already dead (life state 1, 2 or 6, `008844f0`) and, for a non-player,
when the test at `008d40e0` says so.

### NV-022 Two branches of the armour step (`009b5a30`, exe-read)

`crates/world/src/combat.rs` `armour_hit` subtracts the threshold for
every weapon. In the program: (a) when the player's weapon carries
weapon-mod effect 12 (Split Beam) the threshold is divided by the
projectile count first; (b) an automatic melee-type weapon has its hit
multiplied by `max(1 - t / max(weapon damage, hit), 0)` instead of
subtracting. Confirm (b)'s operands in the disassembly before porting.

### NV-010 Experience award rounding (`008d5100`, unsettled)

Not counted as DIFFERS: nv-rs rounds the perk-adjusted amount up,
fallout-rs's notes say truncated. The call just before the float-to-int
conversion (`004e4430`) may be a ceiling, in which case nv-rs is right.
An oracle call settles it.

## 2. Partly implemented

- **NV-051 Fast-travel refusals** (`0093d660`): nv-rs checks hostile,
  script switch, encumbrance (with entry point 51) and the interior cell
  flag, in the right order. Missing: alarm package (procedure 0x15)
  running, a Health-damaging effect, Fatigue or Health below 1, in the
  air, and for exteriors the cell flag 0x04 test reversed plus the
  worldspace flag +0x4c bit 1. Full order in the row.
- **NV-052 Wait and sleep refusals** (`00969fa0`, `005095b0`): the
  order of the tests nv-rs has is right; alarm, under water, in the air,
  radiation and the New Vegas-only process +0x764 test are missing.
- **NV-075 Script event blocks** (`0118e2f0`, `005ac750`): the full table
  of 38 block types and 27 event bits is in the row. nv-rs runs 16 of
  them; OnMurder, OnCombatEnd, OnPackageStart / Done / Change,
  OnMagicEffectHit, OnSell, OnGrab, OnRelease, OnReset, and the New
  Vegas-only OnDestructionStageChange (0x200000), OnFire (0x400000) and
  OnNPCActivate (id 37, no bit) were not found.
- **NV-094 Original saves** (M7): `docs/FOS_SAVES.md` lists MSTT and PCBE
  as not decoded. MSTT is TESForm's LoadGame through its TESForm part at
  +0x14; PCBE (form type 0x69) loads through the projectile path
  (`009b4cf0` -> `009c52f0`) and carries the projectiles' initial data
  (NV-095: the reference test `00564900` accepts 0x3a..0x40 and 0x69).
  With these, fallout-rs decodes 196,228 of 196,228 change records in 13
  New Vegas saves.

## 3. Rules nv-rs does not have yet

| Row | Rule | Address | Status |
| --- | --- | --- | --- |
| NV-014 | Creature combat skill with `fCreatureCalcCombat` | `005f8f70` | oracle |
| NV-036 | Limb sever / explode roll with `iCombatDismemberPartChance` | `00646e50` | exe-read |
| NV-037 | Health difficulty multipliers `fDiffMultHPToPC` / `ByPC` | `00648cb0` | exe-read |
| NV-044 | Detection size input | `00642e80` | oracle |
| NV-048 | Flying speed | `00647fd0` | oracle |
| NV-050 | Swim breath and drowning damage | `00648a10`, `00648a50` | oracle |
| NV-055 | Encounter-zone level with `fLevelScalingMult` | `00526190` | oracle |
| NV-056 | Encounter zone due to reset (`iHoursToRespawnCell`) | `005262a0` | oracle |
| NV-084 | Follow stop / match-speed / resume distances | `00643f20` and two more | oracle |
| NV-085 | Follower catch-up speed | `008855f0` | exe-read |
| NV-092 | Sound group volume and the music-group rule | `00adac30` | exe-read |
| NV-095 | PCBE initial data in saves | `0084e730`, `00564900` | exe-read |

## 4. Where nv-rs was right and fallout-rs was wrong

Before fallout-rs read the New Vegas program itself, an agent checked 38
of nv-rs's documented claims in it: 32 confirmed, 6 partly right, 0 wrong.
fallout-rs then corrected its own rules from those readings. Rows where
nv-rs had it first:

- NV-057 hardcore needs: one point per `rate` real seconds (fallout-rs had
  points per game hour and a guessed thirst rate);
- NV-021 armour order: resistance first, then threshold (fallout-rs had
  threshold first);
- NV-061 companions essential whenever hardcore is off;
- NV-067 kill karma by the victim's band in crime-tracking factions;
- NV-062 to NV-064 reputation levels, thresholds and bumps (fallout-rs had
  guesses);
- NV-008 skill points per level (fallout-rs had the Fallout 3 rule);
- NV-015 weapon condition factor 0.75 / 0.67 constants and NV-072 repair
  without the skill ceiling: the oracle confirmed nv-rs's readings and
  failed fallout-rs's ports (1180 and 877 mismatches of 1200);
- NV-066 karma band limits are inclusive, as nv-rs has them; a fallout-rs
  note said strict, the oracle (270 samples on a limit) says inclusive.

The partly-right readings were about: critical chance (nv-rs
then said no fire-rate divisor; its code now has it), weapon damage (term
placement), the round's effect operations, movement penalties (armour and
two-handed penalties; now in nv-rs), weapon wear (perk entry 68 not
read), and detection beyond 8192 units (a 0.3 s timer not read).

## 5. Settings defaults

fallout-rs ran every Setting registration function of FalloutNV.exe
inside the suspended process (the objects are built at run time; a static
copy of the program holds zeros there) and confirmed 4,459 names and
defaults. Compared with the fallback values in nv-rs's code (`unwrap_or`,
`setting(order, name, default)` and similar), 41 differ. The ones looked
at are FalloutNV.esm's own values, for example `fSneakMaxDistance` 2500
(program 1500), `fMaxArmorRating` 85 (90), `iMaxCharacterLevel` 30 (20),
which is harmless while the plugin sets them; a few nv-rs comments call
such values "the exe's default". The table can be shared if useful.
