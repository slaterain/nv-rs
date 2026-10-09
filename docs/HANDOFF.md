# Handoff: sessions of 2026-10-08/09

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[CONTRIBUTING.md](../CONTRIBUTING.md), [MILESTONES.md](MILESTONES.md) and
[TASKS.md](TASKS.md) first. This session went past the 95% usage stop rule
at the maintainer's request and stopped at about 96%.

## State

- `main` has everything merged (PRs only; squash; `gh pr merge --admin`
  once `check (core)` and `check (viewer)` pass). A CI job failing with
  "A device which does not exist was specified" is a runner fault: rerun
  it.
- Play copy (`Desktop\nv-rs-play`): build 34 (main `73de649`); build 35
  follows #69 (Pip-Boy equip sounds, container model sounds) once merged.
- Landed in these sessions: #51 Havok on the game's frame timer, contact
  sounds at their point · #52/#60 Doc's chair exit, seated package checks
  · #58 B1 parts 8–10 (proxy push and GSK cast, ragdoll constraints,
  simplified TOI) · #64 help-up timing (from the maintainer's
  recordings), SayTo speaker idles, daylight opening, thin bodies vs
  floors, weather log spam · #65 opening step table; nested stage scripts
  keep the quest's variables · #67 bodies touching something move again;
  turns in place end · #68 Ko-fi Sponsor button (`.github/FUNDING.yml`)
  · #69 Pip-Boy equip sounds, arm blink, containers' model sounds.

## Everything still open (not resolved, or not traced to the exe)

### The opening (B14; docs/OPENING.md, "The whole opening, step by step")

1. **Not run or compared after the walk to the tester.** The tag-skill
   menu needs input the automation can't give, so traits (stages
   98–102), Doc's farewell transition (105), his walk to the exit (110),
   the farewell conversation (115) and stage 200 (`GameHour` 8,
   `StartQuest VMQ01`, `VCG04`) were never driven or compared with the
   original. Drive them (menu keys or `--run` stages) and compare.
2. **Help-up trigger comes from the recording, not the exe.** Doc's
   packages are re-checked when the face menu closes
   (`GameState::evaluate_everyone`). Ruled out as the engine's trigger:
   `SetStage` (`005c7140` → `0060d510`), `StartQuest` (`0060c9c0`), the
   line's end (`00935f60`), the menu's finish (`007ada40`), the 20 s
   timer and the hour (both pause in menu mode: `0086e650` sets
   `011dea2b`). `EvaluatePackage` (`008a6ce0`) sets actor +0x145 and the
   process's last hour to hour − 1 (`00693d50`); find which caller of
   `00693d50` runs when the face menu closes.
3. **Daylight is a maintainer decision, not traced.** `VCG00` stage 0
   ("DEMO ONLY") sets `GameHour` 23, and the game does run it (only its
   cleanup sets `VCG01` stage 0); the viewer keeps the hour through that
   stage. If the engine resets the time somewhere, find it.
4. **The face menu (B15) isn't built** (auto-accepts; its `MenuMode`
   block runs once).
5. **Re-adding the same script package** (VCG01 stage 7,
   `VCG01PlayerSection1`): whether its begin action starts again is
   untraced (the viewer avoids a second sit-up only because the bed-sit
   idle is still blending in).
6. **Not compared with the original today:** the Vigor Tester and
   SPECIAL steps (verified in an earlier batch only).

### Physics (B1, B2; docs/PHYSICS.md, TASKS.md B1)

7. Under the character proxy's GSK cast, the GJK (`00daad40`) and the
   penetration depth (`00daa7e0`) are stand-ins (which of several equally
   close points becomes the contact is ours); the MOPP/world-caster
   culling; `IsStep`'s convex branch.
8. A pushed body can overlap the player's hull (no keep-out traced);
   moving platforms (`00c6ca30`, property 0x1300) untranslated;
   swimming, flying and climbing states.
9. Continuous collision: the event time comes from conservative
   advancement, not the agents' predictive linear cast (separating plane,
   fractions of each body's allowed penetration depth; the NIF's
   `penetration_depth` is read but unused); full TOI (`00d100a0`) for
   moving/critical bodies; body–body events. #67 skips points already
   touching at the step's start (labelled).
10. Ragdolls: motors, the constraint runtime data's constructor,
    malleable strength, the actor-scale friction clone (`00cbd9e0`);
    creatures' ragdolls (the dog) not re-run; contact points come from
    our generator.
11. Bodies rolling off the viewer's loaded terrain (tumbleweed 00178A82
    at y −4096): what the game does with a body leaving its loaded grid.
12. B2 grab on the new solver: the "complex" spring for actors and
    ragdolls, letting go when standing on the held body, the
    Activate-hold grab.
13. Ravens turn in place at about 1.25°/s (`NVCrRaven` `TNAM` 1.0):
    check in the original whether they really turn that slowly.

### Rendering

14. **White blobs (B17).** Soft white glows, ~30–60 px, flash for a
    moment at a fixed spot in the world while looking or moving (F12
    report 009 and the maintainer's screenshots). Ruled out: NaN/Inf, any
    pre-grade value above 2 in 450 frames at report 009's spot (burst
    capture with a probe; scripts and frames in
    `%USERPROFILE%\nv-re\work\wf`), particles without textures. Most
    likely a specular glint (window, metal, glass) at one exact view
    angle, swelled by bloom: sweep the view in tiny steps across that
    spot, find the draw and the value, compare with the game's shader for
    that material. Also: F9 can't be sent headless (`--key-at` takes
    letters and digits only), so saved states can't be loaded in
    automated runs; add it.
15. Single-pixel sun-coloured specular glints on foliage edges: they
    follow the traced formula; confirm against the original.

### Pip-Boy and menus

16. The Pip-Boy "bugs out when changing armour or weapons": only the arm
    blink was found and fixed (#69); nothing else reproduced (15 weapons,
    6 armours). Ask the maintainer for an F12 report or a description.
17. Containers' lid/door open animation isn't played (only its sounds);
    the close sound wasn't heard live; reading a book's sound is traced
    only.

### People

18. **B25, NPCs sinking to the waist:** not reproduced (nobody with a
    controller deeper than 10.7 under the land; people far from the
    camera start 10–24 under when handed back to their controller). Needs
    an F12 report on a sunk NPC.

### The rest of the backlog (TASKS.md)

B2 grab, B7 player damage feedback, B8 animation blending, B9 jumping,
B10/B33 greetings (check), B11 voice after skipping lines, B12 Doc at
the door, B13 barter menu fades, B16 local map scale, B18 frame time and
hitches, B19 repositioning when the line of fire is blocked, B20 aim
height, B21 linked lines (INFC), B26 Ringo's greeting (data, not
engine), B28 the saloon window, B31's depth of field, the B14 menu
camera snap.

## Tips

- Use `--walk` for any log about the opening camera (fly mode never
  advances the first-person idles).
- The maintainer's recordings can be aligned with Doc's voice files by
  loudness envelopes (WebAudio in the built-in browser, served by a small
  local HTTP server with Range support); it pinned the help-up exactly.
- The Ghidra server has `read_memory` and `search_instructions`
  endpoints (vtable slots, writers of a field offset).
- Build caches fill the disk (~8 GB each): delete finished worktrees'
  `viewer\target` and `target-*` folders.
- Temporary per-walker debug logs (turn state, stuck turns, dropped
  bodies) found three bugs today; add them, then remove before
  committing.
