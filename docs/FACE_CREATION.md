# Face creation: M1 working evidence

## Current research handoff

Active branch `codex/m1-face-menu-research`, based on camera PR #7
(`00876d6`, local checks/live camera verification complete; CI running).
Appearance PR #6 merged as `843926d`; its verified binary is in play/app,
SHA256 `9391A5176CD9A829B99642C9252EE945FC7E6BDB6E255A72E2DAC6750BA9FD98`.
Saves/reports were preserved. Camera publication waits for its own merge.

Parent reads existing `007adce0`/`007af180` click/page exports and installed
XML. Bounded private agent trace owns `007b39e0` and immediate control
construction helpers; no public runtime edits. Do not call the 20 constructed
objects face sliders without confirming their type: setup also treats these
as page/category lists. XML confirms separate list-item and slider templates.
Private sources and forthcoming findings: nv-re/work/m1-overnight-2026-10-04/
face-menu. No original-game menu comparison has succeeded this session.
Next: label the exact native page graph, callback branches and initial state;
then define a complete persistent appearance/preview batch from those facts.

## Race/sex part reconciliation, 2026-10-04

Implemented `appearance::reconcile_parts` and `PartSelection`, as core
groundwork; the viewer face editor remains unimplemented and auto-accepts.
No gameplay integration or original-game fallback comparison is claimed.

Ghidra `007b1ca0` retains hair/eyes when the selected race contains the part
and its sex flags permit it (`005fdfa0`, `005fc5f0`). These predicates do not
require the playable bit. Invalid hair first uses the selected sex's RACE
DNAM slot (`00613870`, race+0x94+sex*4; loader `00610cd0`, setter `00613890`).
Only a zero slot scans the race's HNAM list for the first playable,
sex-compatible hair. This corrects the older note's global-list inference:
assembly shows `004ac110 -> 0045bb80`, the race's list at+0x8c. A nonzero
authored default does not recheck membership, playable or sex restrictions.
Invalid eyes use the first loaded race ENAM entry (`007b1e50`, +0xa8),
without playable/sex filtering. The visible choices retain their stricter
filters and independently unresolved display order.

The loader skips malformed non-multiple-of-four lists, omits unresolved or
wrong-type entries, then deduplicates and appends resolved forms in source
order (`00613810`/`00613910`). Reconciliation respects owner-plugin FormID
mapping and winning/deleted records. Malformed DNAM is explicitly unsupported:
the native fixed-eight-byte reader's malformed-input behavior is not traced.
Unresolved nonzero default hair fails closed; native fixup semantics for
that invalid data are not claimed.

Installed official data gives these authored defaults (male/female hair):
African American `000306BE`/`0005DC78`, Asian `00014B90`/`00022E50`, Hispanic
`000A9D6F`/`0005DC76`, Caucasian `00014B90`/`0005DC6B`. All four have Blue
`00004253` first in ENAM. These are data observations, not a native UI replay.

Six generated regressions cover compatible non-playable retention, sex changes,
unfiltered defaults, HNAM order versus global choice order, first loaded
ENAM, unresolved list entries/defaults, malformed DNAM, and reordered-master
mapping, and last-pair-wins duplicate DNAM. Core919/viewer84 tests, both
clippy and formatting checks, and both release builds pass on
`codex/m1-native-appearance`. Installed-data new-game smoke reached the
situp/bedsit camera idles and Doc's opening speech, then exited after its
inspected screenshot; stderr contained Vulkan present-mode warnings only.
This is a startup regression check, not face-menu acceptance. A private
implementation harness passed all eight official race/sex cases and retained
each returned selection unchanged; `appearance/implementation-check/run.log`.
Private evidence and hashes:
`nv-re/work/m1-overnight-2026-10-04/appearance/{FINDINGS.md,provenance.json}`.
Analysis executable 1.4.0.525 SHA256:
`19406942E48724D797300C4EA6BE9AC69A32F670B8C35DB09279A2258422739F`.
Next: persistent editable appearance and native preview/menu callbacks.

## Earlier evidence

Original-game reference inspected 2026-10-03 after the user opened ShowRaceMenu:
the Reflectron cabinet fills most of the view, oval live head/upper-body preview
on the left, green sex-selection screen on the right (Male selected, Female,
Next), and Sex/Race/Face/Hair labels with indicator lamps below the preview.
The room remains visible and blurred behind it. This confirms the starting
page's presentation only, not the unobserved editing controls or transitions.
Opening animation failures reported by the user take priority before extending
this menu; see OPENING.md.

2026-10-03. Choice reader implemented; no replacement menu published yet.
Executable hash and shared priorities: OPENING.md and MILESTONES.md.
Decompilation stays outside the project in nv-re/decomp/codex-m1.

ShowRaceMenu handler005cedf0 calls00705870 with mode1 outside the script
thread flag, mode0 inside. Barber005cee30 passes2; surgeon005cee50 passes3.
00705870 opens immediately or schedules mode5 with the argument retained.
007ac730 loads Data/Menus/CharGen/race_sex_menu.xml. The original XML
successfully parses through nvinspect (codex-m1/racesex-menu.txt).

RaceSexMenu constructor007ac1f0; vtable01075974. SetTile007ac500 accepts
IDs0..5 into+78. Click007adce0; drag start007ae1b0; update007ae420;
special keys007aecb0. Update checks whether the menu is active before input.
Setup007acb60 loads Terminals/NV_reflectron_UI.NIF for the opening;
BarberInterface01.NIF and PlasticSurgeryInterface01.NIF for modes2/3.
The opening starts page0, barber3, surgeon2. Detailed page-tree/preview
behavior still needs tracing; do not invent controls or a substitute panel.

Choice rules traced so far:
- Setup enumerates races filtered by0059f610: runtime flags at+70 bit1.
  Existing functions.md maps this field to RACE DATA u32 at32.
- Hair page007af300 enumerates the global hair list, requires playable
  (005fdf40, hair+48 bit1) and005fdfa0: membership in the actor's race list,
  then sex restriction (005fdf60: male allowed when bit2 clear;
  005fdf80: female allowed when bit4 clear).
- Eyes page007af450 similarly requires playable005fc4d0 and005fc5f0
  membership/sex restrictions. Eyes flags are at+30; bits1/2/4 have the
  same meanings.005fdcb0 (hair) and005fc220 (eyes) each copy the single
  DATA byte into those fields.00610cd0 copies36-byte RACE DATA to+50;
  HNAM and ENAM are mapped form arrays, skipped unless length is divisible4.
- Changing race/sex validates current hair/eyes in007b1ca0; the defaults,
  ordered race-owned fallback and eye fallback are now traced above.

Existing infrastructure: actor::ActorLook and Face hold assembled models
and symmetric/asymmetric morphs; Game::actor_scene builds their meshes.
Player first_person_look deliberately has no head. GameState/save currently
persist only player_name/player_female for identity, no editable race/hair/
eyes/morphs. Menu creation needs those fields, selection rules, a preview
and native XML callbacks before replacing the current face auto-accept.

## Implemented and checked

`world::chargen::appearance::{races,hair,eyes}` enumerates eligible choices
from winning records, rejects deleted entries, resolves membership IDs via
the race record's owning plugin, and keeps actual FULL names (missing names
stay missing). Native menu ordering is not yet established. No default
selection or appearance mutation is invented here.

Six generated-input regressions cover flags/sex, malformed lists (including
a valid ID followed by a bad tail), wrong/missing records, deleted winning
overrides, master-index remapping, and missing/empty labels. A surviving race
is also checked against deleted hair/eyes, independently of a deleted race.

Real official-data harness outside the project:
`nv-re/work/codex-m1/appearance-observations`; `appearance-real-data.log`.
Observed four playable races: African American, Asian, Hispanic, Caucasian.
Each exposes18 male/23 female hair choices and4 eyes per sex. These are data
observations, not yet a native menu comparison. Full check status: OPENING.md.

Original game process40680 opened via Steam (no capture DLL installed).
Computer Use keys/clicks did not affect its main menu, including after an
explicit focus retry. User was asked to load a save and open the tester with
ShowLoveTesterMenuParams40 for reference inspection. No original saves changed.
