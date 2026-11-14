//! `functions_jip/jip_fn_ccc.h` (JIP LN NVSE): the commands JIP keeps for its companion mod, JIP
//! CC&C. 11 of its 21; the core track built `CCCSetFollowState` and `LockEquipment`
//! (`actor_hooks.rs`).
//!
//! Left for later: `CCCOnLoad` reads `Data\Config\ccc_avatars.ini` and the avatar textures' folder
//! (the file layer keeps no such reads), looks up the CC&C plugin's forms and wires 119 of the
//! HUD's tile values (`g_HUDMainMenu`'s `JIPCCCHUDDisplay` component, whose XML the mod injects);
//! `CCCSetFloat`, `CCCSetString` and `CCCSetTrait` write those tile values, and `CCCSavedForm` and
//! `CCCSMS` read the forms and the mod index `CCCOnLoad` sets, so they come with it.
//! `MoveToPosStr` moves the reference through JIP's own position marker (`s_tempPosMarker`, a
//! reference JIP creates at its start, and the game's `MoveToMarker`): JIP's start-up objects are
//! not built. `CCCSetEquipped` is the core track's (step 3.5d: the container menu, the combat
//! style's restrictions and JIP's always-on `GetCannotWear` patch).

use super::{assign_string, extra, Handlers};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2701, ccc_get_distance),
    (0x2702, ccc_task_package_flags),
    (0x2704, ccc_in_faction),
    (0x2705, ccc_set_nccs),
    (0x2707, ccc_load_nccs),
    (0x2709, ccc_location_name),
    (0x270a, ccc_get_reputation),
    (0x2752, ccc_say_to),
    (0x2753, ccc_run_result_scripts),
    (0x2762, ref_to_pos_str),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x2706, get_encumbrance_rate, get_encumbrance_rate_eval),
];

/// `TESObjectREFR::baseForm` +0x20.
const BASE: u32 = 0x20;
/// `TESActorBase::baseData.factionList` (+0x5C: `TESActorBaseData` +0x30, its list +0x2C, read by
/// `005D8A70` and the faction list's copy `0047CA20`; batch 11's audit).
const FACTIONS: u32 = 0x5c;
/// `TESActorBase::scriptable` (+0xF4: the actor base's constructor `005F75E0` builds the
/// `TESScriptableForm` there, `005F76BC`), its script +4.
const ACTOR_BASE_SCRIPT: u32 = 0xf8;
/// `TESFaction::reputation` (+0x38, forms.rs: the faction loader `005FCB70`).
const FACTION_REPUTATION: u32 = 0x38;
/// `kXData_ExtraFactionChanges` (its list pointer +0xC), `kXData_ExtraContainerChanges` (its
/// data +0xC), `kXData_ExtraSayToTopicInfo` (its topic info +0xC).
const EXTRA_FACTION_CHANGES: u32 = 0x5e;
const EXTRA_SAY_TO_TOPIC_INFO: u32 = 0x75;
const EXTRA_WORD: u32 = 0x0c;
/// `Actor::GetMaxCarryWeight` as JIP calls it (`008A0C20`, the actor in `ecx`: its carry weight
/// actor value through `avOwner` +0xA4's virtual +0xC, then the entry point 0x16).
const MAX_CARRY_WEIGHT: u32 = 0x008a_0c20;
/// The game's `SayTo` execute (`005C9100`, `cdecl`, the command's eight words): JIP calls it
/// with its own arguments.
const SAY_TO: u32 = 0x005c_9100;
/// `TESTopicInfo::RunResultScript` (`0061F170`, `thiscall`: the info, 0 begin / 1 end, the actor).
const RUN_RESULT_SCRIPT: u32 = 0x0061_f170;
/// `GetTopicInfo` (`0061A7D0`, `cdecl`: the topic, where the topic found goes, the speaker, the
/// target, 1, 0, 0).
const GET_TOPIC_INFO: u32 = 0x0061_a7d0;
/// `g_thePlayer`.
const PLAYER: u32 = 0x011d_ea3c;
/// `TESPackage::packageFlags` (+0x1C, forms.rs: the package loader `00673A40`) and the flags
/// (`kPackageFlag_*` (JIP LN NVSE)).
const PACKAGE_FLAGS: u32 = 0x1c;
const ALWAYS_RUN: u32 = 1 << 13;
const ALWAYS_SNEAK: u32 = 1 << 17;
const WEAPONS_DRAWN: u32 = 1 << 23;
const NO_IDLE_ANIMS: u32 = 1 << 24;
const CONTINUE_DURING_COMBAT: u32 = 1 << 26;
const NO_COMBAT_ALERT: u32 = 1 << 27;
const ALWAYS_WALK: u32 = 1 << 29;
/// `TESObjectCELL`: `fullName` +0x18 (its text +0x1C, length +0x20), `worldSpace` +0xC0,
/// `objectList` +0xAC.
const CELL_NAME: u32 = 0x1c;
const CELL_WORLD: u32 = 0xc0;
const CELL_OBJECTS: u32 = 0xac;
/// `TESWorldSpace`: `fullName` +0x18 (text +0x1C, length +0x20), `worldMap` (a `TESTexture` at
/// +0x24: its path's length +0x2C), `cell` +0x34, `parent` +0x70.
const WS_MAP_PATH_LEN: u32 = 0x2c;
const WS_CELL: u32 = 0x34;
const WS_PARENT: u32 = 0x70;
/// A map marker's name (`MapMarkerData::fullName`: text +4, length +8).
const MARKER_NAME: u32 = 0x04;

/// What the CC&C commands keep: `s_NCCSActors` (the actor bases set to NCCS, JIP's sorted
/// `Set`) and `s_worldspaceMap` (each worldspace's map markers by their 4096-unit grid cell,
/// built at the first `CCCLocationName` in that worldspace and never rebuilt). Never saved,
/// never cleared (JIP's `TempObject`s).
#[derive(Default)]
pub struct State {
    pub nccs_actors: BTreeSet<u32>,
    pub marker_grids: BTreeMap<u32, HashMap<u32, u32>>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().ccc
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCGetDistance_Execute.
/// The distance to the reference in x and y (`Point2Distance`), or with axis 4 the absolute
/// difference in z (a float). Axis defaults to 3.
fn ccc_get_distance(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let target = read.form(0);
    if !read.ok || a.this_obj == 0 || target == 0 {
        return handled(true);
    }
    let axis = read.int(1, 3);
    let me = super::ln_fn_math::position(e, a.this_obj);
    let it = super::ln_fn_math::position(e, target);
    let value = if axis == 4 {
        (me[2] - it[2]).abs()
    } else {
        super::ln_fn_math::point2_distance(me, it)
    };
    a.set_result(e, f64::from(value));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameBSExtraData.cpp:
// BaseExtraList::GetExtraFactionRank (asm).
/// The reference's own rank in the faction (its `ExtraFactionChanges` list: the first entry of
/// the faction, empty entries skipped), -1 for none.
fn extra_faction_rank(e: &mut Engine, refr: u32, faction: u32) -> i8 {
    let x = extra::refr_extra(e, refr, EXTRA_FACTION_CHANGES);
    if x == 0 {
        return -1;
    }
    let mut node = e.mem.u32(x + EXTRA_WORD);
    while node != 0 {
        let data = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if data != 0 && e.mem.u32(data) == faction {
            return e.mem.u8(data + 4) as i8;
        }
    }
    -1
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCInFaction_Execute.
/// 1 when the actor is in the faction: in its base's list (raw base form) when the second
/// argument is not 0, or in its own changes.
fn ccc_in_faction(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let faction = read.form(0);
    let in_base = read.int(1, 0) != 0;
    let base = e.mem.u32(a.this_obj + BASE);
    if (in_base && super::ln_fn_actor::faction_rank(e, base, faction) >= 0)
        || extra_faction_rank(e, a.this_obj, faction) >= 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h: SetNCCS.
/// An actor base with a script: the script disabled or enabled (`SetScriptDisabled`) and its
/// activation disabled or enabled (`SetScriptEventDisabled` with no mask, "OnActivate").
fn set_nccs(e: &mut Engine, base: u32, set: bool) {
    let script = e.mem.u32(base + ACTOR_BASE_SCRIPT);
    if script != 0 {
        crate::ai_hooks::set_one_script_disabled(e, script, set);
        crate::world_hooks::set_events_disabled(e, base, 0, true, set);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetNCCS_Execute.
/// The actor's base (raw) set to NCCS (default) or back: only a change of the set acts.
fn ccc_set_nccs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let set = read.int(0, 1) != 0;
    let base = e.mem.u32(a.this_obj + BASE);
    let changed = if set {
        state(e).nccs_actors.insert(base)
    } else {
        state(e).nccs_actors.remove(&base)
    };
    if changed {
        set_nccs(e, base, set);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCLoadNCCS_Execute.
/// Every NCCS actor base set back (in the set's order), the set emptied.
fn ccc_load_nccs(e: &mut Engine, _: &[u32]) -> Ret {
    let bases = std::mem::take(&mut state(e).nccs_actors);
    for base in bases {
        set_nccs(e, base, false);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h: GetEncumbranceRate.
/// An actor with container changes data: `100 * weight / max carry weight` in doubles when the
/// maximum is above 0 (the weight as `GetInventoryWeight` gives it); else 0.
fn encumbrance_rate(e: &mut Engine, refr: u32) -> f64 {
    if !super::is_actor(e, refr) {
        return 0.0;
    }
    let x = extra::refr_extra(e, refr, extra::kind::CONTAINER_CHANGES);
    if x == 0 {
        return 0.0;
    }
    let data = e.mem.u32(x + EXTRA_WORD);
    if data == 0 {
        return 0.0;
    }
    let max = e.call(MAX_CARRY_WEIGHT, &args![refr]).st0;
    if max > 0.0 {
        let weight = super::ln_fn_misc_ref::inventory_weight(e, data);
        return 100.0 * weight / max;
    }
    0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_GetEncumbranceRate_Execute, Cmd_GetEncumbranceRate_Eval.
fn get_encumbrance_rate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let rate = encumbrance_rate(e, a.this_obj);
    a.set_result(e, rate);
    handled(true)
}

fn get_encumbrance_rate_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let rate = encumbrance_rate(e, a.this_obj);
    e.mem.set_f64(a.result, rate);
    handled(true)
}

/// A game string's text (`m_data`, `m_dataLen` + 1 bytes as JIP's `COPY_BYTES` copies them, its
/// 0 dropped); no buffer: none.
fn string_bytes(e: &Engine, text_at: u32) -> Vec<u8> {
    let data = e.mem.u32(text_at);
    if data == 0 {
        return Vec::new();
    }
    let len = u32::from(e.mem.u16(text_at + 4));
    let mut v = e.mem.bytes(data, len);
    if let Some(end) = v.iter().position(|&b| b == 0) {
        v.truncate(end);
    }
    v
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: Coordinate::operator=
// (asm).
/// A position's 4096-unit grid cell: x and y truncated to integers (`cvttps2dq`, the indefinite
/// for NaN and past the range), shifted right by 12 (arithmetic), x in the high 16 bits, y in the
/// low.
pub fn grid_cell(x: f32, y: f32) -> u32 {
    let t = |v: f32| -> i32 {
        if v.is_nan() || v >= 2_147_483_648.0 || v < -2_147_483_648.0 {
            i32::MIN
        } else {
            v as i32
        }
    };
    let gx = (t(x) >> 12) as u16;
    let gy = (t(y) >> 12) as u16;
    (u32::from(gx) << 16) | u32::from(gy)
}

/// A grid cell moved by `dx`, `dy` (16-bit halves that wrap, as `Coordinate`'s `SInt16`s).
fn grid_add(cell: u32, dx: i32, dy: i32) -> u32 {
    let x = ((cell >> 16) as u16 as i16 as i32).wrapping_add(dx) as u16;
    let y = (cell as u16 as i16 as i32).wrapping_add(dy) as u16;
    (u32::from(x) << 16) | u32::from(y)
}

/// The worldspace's grid of named map markers (`s_worldspaceMap`): made at its first use, from
/// the persistent cell's references when the worldspace has a world map texture or a parent;
/// a later marker in the list takes a cell's place.
fn marker_grid(e: &mut Engine, world: u32) -> HashMap<u32, u32> {
    if let Some(g) = state(e).marker_grids.get(&world) {
        return g.clone();
    }
    let mut grid = HashMap::new();
    let cell = e.mem.u32(world + WS_CELL);
    if cell != 0 && (e.mem.u16(world + WS_MAP_PATH_LEN) != 0 || e.mem.u32(world + WS_PARENT) != 0)
    {
        let head = Ptr::<super::forms::ListNode>::new(cell + CELL_OBJECTS);
        for refr in super::list_items(e, head) {
            if refr == 0 {
                continue;
            }
            let Some(marker) = super::ln_fn_map_marker::marker_data(e, refr) else {
                continue;
            };
            if marker.is_null() || e.mem.u16(marker.addr() + MARKER_NAME + 4) == 0 {
                continue;
            }
            let p = super::ln_fn_math::position(e, refr);
            grid.insert(grid_cell(p[0], p[1]), refr);
        }
    }
    state(e).marker_grids.insert(world, grid.clone());
    grid
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCLocationName_Execute.
/// Where the reference is, in words: "at an unknown location" with no cell; "in " and the
/// cell's name in an interior; else the nearest named map marker of its worldspace's grid,
/// searched ring by ring out to 8 cells from its own (the first ring with one wins; the nearest
/// in 3D in it): "at " it within 4096 units, else the direction ("East of ", "West of ", "North
/// of ", "South of ", "NE of " ...) and its name; with none, "at " and the worldspace's name.
///
/// JIP builds the text in a 0x80-byte buffer on its stack (`COPY_BYTES`, `StrCat` unchecked);
/// the port builds it whole.
fn ccc_location_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let Some(cell) = super::ln_fn_math::cell_or_persistent(e, a.this_obj).filter(|&c| c != 0)
    else {
        assign_string(e, &a, Some(b"at an unknown location".to_vec()));
        return handled(true);
    };
    let world = e.mem.u32(cell + CELL_WORLD);
    if world == 0 {
        let mut text = b"in ".to_vec();
        text.extend(string_bytes(e, cell + CELL_NAME));
        assign_string(e, &a, Some(text));
        return handled(true);
    }
    let grid = marker_grid(e, world);
    if !grid.is_empty() {
        let me = super::ln_fn_math::position(e, a.this_obj);
        let home = grid_cell(me[0], me[1]);
        let mut found = 0u32;
        let mut dist_min = 40000.0f32;
        for radius in 0i32..=8 {
            let mut coord = grid_add(home, -radius, -radius);
            let (mut mod_x, mut mod_y) = (0i32, 1i32);
            let mut count = 1i32;
            loop {
                if let Some(&marker) = grid.get(&coord) {
                    let it = super::ln_fn_math::position(e, marker);
                    let d = super::ln_fn_math::point3_distance(me, it);
                    if dist_min > d {
                        found = marker;
                        dist_min = d;
                    }
                }
                if radius == 0 {
                    break;
                }
                if mod_x != 0 {
                    coord = grid_add(coord, mod_x, 0);
                } else {
                    coord = grid_add(coord, 0, mod_y);
                }
                if count % (radius * 2) == 0 {
                    mod_x = mod_y;
                    mod_y = if mod_y == 0 { -1 } else { 0 };
                }
                count += 1;
                if count > radius * 8 {
                    break;
                }
            }
            if found != 0 {
                break;
            }
        }
        if found != 0 {
            if let Some(marker) = super::ln_fn_map_marker::marker_data(e, found) {
                if !marker.is_null() {
                    let it = super::ln_fn_math::position(e, found);
                    let mut text = if dist_min > 4096.0 {
                        let dx = (me[0] - it[0]) / dist_min;
                        let dy = me[1] - it[1];
                        if (0.97..=1.0).contains(&dx) {
                            b"East of ".to_vec()
                        } else if (-1.0..=-0.97).contains(&dx) {
                            b"West of ".to_vec()
                        } else if (-0.26..=0.26).contains(&dx) {
                            if dy > 0.0 {
                                b"North of ".to_vec()
                            } else {
                                b"South of ".to_vec()
                            }
                        } else if dx > 0.0 {
                            if dy > 0.0 {
                                b"NE of ".to_vec()
                            } else {
                                b"SE of ".to_vec()
                            }
                        } else if dy > 0.0 {
                            b"NW of ".to_vec()
                        } else {
                            b"SW of ".to_vec()
                        }
                    } else {
                        b"at ".to_vec()
                    };
                    let name = e.mem.u32(marker.addr() + MARKER_NAME);
                    if name != 0 {
                        text.extend(nvse::table::guest_text(e, name));
                    }
                    assign_string(e, &a, Some(text));
                    return handled(true);
                }
            }
        }
    }
    let mut text = b"at ".to_vec();
    text.extend(string_bytes(e, world + CELL_NAME));
    assign_string(e, &a, Some(text));
    handled(true)
}

/// A faction entry's faction's reputation (`FactionListData`: the faction +0, the rank +4). JIP
/// reads through the faction unchecked; the port answers none for an entry with no faction.
fn reputation(e: &Engine, data: u32) -> u32 {
    let faction = e.mem.u32(data);
    if faction == 0 {
        0
    } else {
        e.mem.u32(faction + FACTION_REPUTATION)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCGetReputation_Execute.
/// The reputation of the first faction with one: the actor's base's list (raw base form) first,
/// then its own changes; empty entries skipped.
fn ccc_get_reputation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let base = e.mem.u32(a.this_obj + BASE);
    let head = Ptr::<super::forms::ListNode>::new(base + FACTIONS);
    for data in super::list_items(e, head) {
        if data == 0 {
            continue;
        }
        let rep = reputation(e, data);
        if rep != 0 {
            super::set_form_result(e, &a, rep);
            return handled(true);
        }
    }
    let x = extra::refr_extra(e, a.this_obj, EXTRA_FACTION_CHANGES);
    if x == 0 {
        return handled(true);
    }
    let list = e.mem.u32(x + EXTRA_WORD);
    if list == 0 {
        return handled(true);
    }
    for data in super::list_items(e, Ptr::new(list)) {
        if data == 0 {
            continue;
        }
        let rep = reputation(e, data);
        if rep != 0 {
            super::set_form_result(e, &a, rep);
            return handled(true);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSayTo_Execute.
/// The game's `SayTo` with this command's arguments, then the topic info it left on the actor
/// (`ExtraSayToTopicInfo`) has its end result script run at once, and the extra goes.
fn ccc_say_to(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let words: Vec<u32> = (0..8).map(|i| w.get(i).copied().unwrap_or(0)).collect();
    e.call(SAY_TO, &words);
    let x = extra::refr_extra(e, a.this_obj, EXTRA_SAY_TO_TOPIC_INFO);
    if x != 0 {
        let info = e.mem.u32(x + EXTRA_WORD);
        if info != 0 {
            e.call(RUN_RESULT_SCRIPT, &args![info, 1u32, a.this_obj]);
        }
        extra::remove_by_type(
            e,
            a.this_obj + extra::REFR_EXTRA_LIST,
            EXTRA_SAY_TO_TOPIC_INFO,
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCRunResultScripts_Execute; internal/jip_core.h: GetTopicInfo.
/// The topic's info the actor would say to the player (the game's `GetTopicInfo`, the topic's
/// variable as the place for the topic it settles on): its begin and end result scripts run.
fn ccc_run_result_scripts(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let topic = read.form(0);
    let actor = a.this_obj;
    let player = e.mem.u32(PLAYER);
    let info = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), topic);
        e.call(
            GET_TOPIC_INFO,
            &args![topic, slot.addr(), actor, player, 1u32, 0u32, 0u32],
        )
        .u32()
    });
    if info != 0 {
        e.call(RUN_RESULT_SCRIPT, &args![info, 0u32, actor]);
        e.call(RUN_RESULT_SCRIPT, &args![info, 1u32, actor]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_RefToPosStr_Execute.
/// "<cell or worldspace id in hex> <x> <y> <z+1>" (the positions truncated to integers), or ""
/// for a reference with no cell.
fn ref_to_pos_str(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let text = match super::ln_fn_math::cell_or_persistent(e, a.this_obj).filter(|&c| c != 0) {
        None => Vec::new(),
        Some(cell) => {
            let world = e.mem.u32(cell + CELL_WORLD);
            let id = super::ref_id(e, if world != 0 { world } else { cell });
            let p = super::ln_fn_math::position(e, a.this_obj);
            let mut s = crate::console::uint_to_hex(id);
            s.extend(
                format!(
                    " {} {} {}",
                    super::cvttss2si(p[0]),
                    super::cvttss2si(p[1]),
                    super::cvttss2si(p[2]).wrapping_add(1)
                )
                .bytes(),
            );
            s
        }
    };
    assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCTaskPackageFlags_Execute.
/// The package's flags for a CC&C task: by task type (0 or not), the first variable, and
/// "always sneak" from the second.
fn ccc_task_package_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let package = read.form(0);
    if !read.ok || package == 0 {
        return handled(true);
    }
    let (task, var1, var2) = (read.int(1, 0), read.int(2, 0), read.int(3, 0));
    let mut flags = e.mem.u32(package + PACKAGE_FLAGS);
    let (set, clear) = if task != 0 {
        if var1 != 0 {
            (
                NO_IDLE_ANIMS | NO_COMBAT_ALERT | ALWAYS_WALK,
                ALWAYS_RUN | WEAPONS_DRAWN | CONTINUE_DURING_COMBAT,
            )
        } else {
            (
                ALWAYS_RUN | NO_IDLE_ANIMS | NO_COMBAT_ALERT,
                WEAPONS_DRAWN | CONTINUE_DURING_COMBAT | ALWAYS_WALK,
            )
        }
    } else if var1 != 0 {
        (
            ALWAYS_RUN | WEAPONS_DRAWN | CONTINUE_DURING_COMBAT,
            NO_IDLE_ANIMS | NO_COMBAT_ALERT | ALWAYS_WALK,
        )
    } else {
        (
            ALWAYS_RUN | WEAPONS_DRAWN,
            NO_IDLE_ANIMS | CONTINUE_DURING_COMBAT | NO_COMBAT_ALERT | ALWAYS_WALK,
        )
    };
    flags |= set;
    flags &= !clear;
    if var2 != 0 {
        flags |= ALWAYS_SNEAK;
    } else {
        flags &= !ALWAYS_SNEAK;
    }
    e.mem.set_u32(package + PACKAGE_FLAGS, flags);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ccc_tests.rs"]
mod tests;
