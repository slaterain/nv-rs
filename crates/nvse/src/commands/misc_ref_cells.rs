//! xNVSE's `Commands_MiscRef.cpp` (xNVSE), the cell scans: `GetFirstRef`, `GetNextRef`,
//! `GetNumRefs`, `GetRefs`, their `InCell` forms and `GetInGrid`, `GetInGridInCell`. A scan walks
//! the cell's reference list, and with a depth the cells around it in its worldspace, as xNVSE's
//! `CellScanInfo` does; `GetFirstRef` keeps the walk per script for `GetNextRef`.
//!
//! The walk's position is read from the shipped DLL (xNVSE 6.4.9 `nvse_1_4.dll`, the scan record's
//! constructor and `NextCell`): two signed bytes, the depth one unsigned byte, the end test
//! comparing the byte widened to 32 bits, minus the centre's 32-bit coordinate, with the depth;
//! the cell map's key is made from the 32-bit sums (`(x << 16) + y`), so a step from 127 looks up
//! 128 while the byte it keeps wraps to -128.

// A pointer xNVSE reads through and the script reference checks row answers for instead
// (`misc_ref.rs`'s rule): here a cell reference without a base form (the type filters read its
// type) and a cell without its coordinates.
use super::misc_ref::passed_over;
use super::{console_print, dynamic_cast, is_console_mode, Read};
use crate::invref;
use crate::table::{Command, Param, Returns};
use crate::vars::{self, Key, Value};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;
use std::collections::HashMap;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Float`, `_ObjectRef`,
/// `_Cell`, `_AnyForm`.
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT_REF: u32 = 0x04;
const CELL: u32 = 0x09;
const ANY_FORM: u32 = 0x3d;
/// `kParams_GetFirstRef`, `kParams_GetRefs`, `kParams_GetInGrid`, `kParams_GetFirstRefInCell`,
/// `kParams_GetRefsInCell`, `kParams_GetInGridInCell` (xNVSE `Commands_MiscRef.h`).
const GET_FIRST_REF: &[Param] = &[
    p("form type", INT, true),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
];
const GET_REFS: &[Param] = &[
    p("form type", INT, true),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
    p("max distance", FLOAT, true),
    p("max angle", FLOAT, true),
    p("baseForm", ANY_FORM, true),
];
const GET_IN_GRID: &[Param] = &[
    p("refr", OBJECT_REF, true),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
];
const GET_FIRST_REF_IN_CELL: &[Param] = &[
    p("cell", CELL, false),
    p("form type", INT, true),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
];
const GET_REFS_IN_CELL: &[Param] = &[
    p("cell", CELL, false),
    p("form type", INT, true),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
    p("max distance", FLOAT, true),
    p("max angle", FLOAT, true),
    p("baseForm", ANY_FORM, true),
];
const GET_IN_GRID_IN_CELL: &[Param] = &[
    p("cell", CELL, false),
    p("refr", OBJECT_REF, false),
    p("cell depth", INT, true),
    p("include taken refs", INT, true),
];

/// `TESForm`: `typeID` (`+4`), `flags` (`+8`: `kFlags_Taken` is `kFlags_Deleted | 2`), `refID`
/// (`+0xC`). `TESObjectREFR`: `baseForm` (`+0x20`), `rotZ` (`+0x2C`), `posX` (`+0x30`),
/// `parentCell` (`+0x40`). `TESObjectCELL`: `cellData` (`+0x48`, its x and y the first two words),
/// `objectList` (`+0xAC`, a `tList` whose head node is in the cell), `worldSpace` (`+0xC0`).
/// `TESWorldSpace::cellMap` (`+0x30`, a `NiTPointerMap`: buckets' count `+4`, buckets `+8`; an
/// entry's next `+0`, key `+4`, cell `+8`). `TESObjectLIGH::lightFlags` (`+0xA8`, 2 can be carried).
const FORM_TYPE: u32 = 4;
const FORM_FLAGS: u32 = 8;
const FORM_ID: u32 = 0xc;
const TAKEN: u32 = 0x22;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_ROT_Z: u32 = 0x2c;
const REFR_POS: u32 = 0x30;
const REFR_PARENT_CELL: u32 = 0x40;
const CELL_DATA: u32 = 0x48;
const CELL_OBJECTS: u32 = 0xac;
const CELL_WORLD: u32 = 0xc0;
const WORLD_CELL_MAP: u32 = 0x30;
const LIGHT_FLAGS: u32 = 0xa8;
const LIGHT_CAN_CARRY: u32 = 2;
/// `PlayerCharacter::GetSingleton` (`*0x011DEA3C`).
const PLAYER: u32 = 0x011d_ea3c;
/// The player's base form id, which the type filters pass over (`kFormType_TESNPC` would find it).
const PLAYER_BASE_ID: u32 = 7;
/// `IsInventoryObjectType` (`0x00481F30`, `TESForm::IsInventoryObject` (xNVSE)).
const IS_INVENTORY_OBJECT_TYPE: u32 = 0x0048_1f30;

/// Form types (`kFormType_*` (xNVSE)) the actor and item filters name.
mod ty {
    pub const ARMO: u8 = 0x18;
    pub const BOOK: u8 = 0x19;
    pub const CLOT: u8 = 0x1a;
    pub const INGREDIENT: u8 = 0x1d;
    pub const LIGH: u8 = 0x1e;
    pub const MISC: u8 = 0x1f;
    pub const WEAP: u8 = 0x28;
    pub const AMMO: u8 = 0x29;
    pub const NPC: u8 = 0x2a;
    pub const CREATURE: u8 = 0x2b;
    pub const KEY: u8 = 0x2e;
    pub const ALCHEMY: u8 = 0x2f;
    pub const NOTE: u8 = 0x31;
    pub const ARMA: u8 = 0x60;
    pub const IMOD: u8 = 0x67;
    pub const CASINO_CHIPS: u8 = 0x6c;
    pub const CARAVAN_CARD: u8 = 0x73;
    pub const CARAVAN_MONEY: u8 = 0x74;
}

/// `CustomFormTypeFilters` (xNVSE): any type, actors, inventory items, a given reference
/// (`GetInGrid`); 202 (projectiles) is listed but unused, so it filters by type and finds nothing.
const ANY_TYPE: u32 = 0;
const ACTORS: u32 = 200;
const ITEMS: u32 = 201;
const SPECIFIC_REFERENCE: u32 = 203;

/// `GetFirstRef`'s depth when none is given (`-127`, read as 0), and the one that asks for
/// `uGridsToLoad:General`.
const NO_DEPTH: i32 = -127;
const GRID_DEPTH: i32 = -1;
/// The console's scan key: xNVSE's console makes a new script for every line (`0xFFFEED`).
const CONSOLE_KEY: u32 = 0x00ff_feed;
/// `ShowRuntimeError`'s text for a `GetNextRef` with no scan.
const NO_SCAN: &str = "GetFirstRef MUST be called before GetNextRef";

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

pub static COMMANDS: &[Command] = &[
    cmd(
        0x1471,
        "GetFirstRef",
        "returns the first reference of the specified type in the current cell",
        GET_FIRST_REF,
        get_first_ref,
        Returns::Form,
    ),
    cmd(
        0x1472,
        "GetNextRef",
        "returns the next reference of a given type in the current cell",
        &[],
        get_next_ref,
        Returns::Form,
    ),
    cmd(
        0x1473,
        "GetNumRefs",
        "returns the number of references of a given type in the current cell",
        GET_REFS,
        get_num_refs,
        Returns::Default,
    ),
    cmd(
        0x1474,
        "GetFirstRefInCell",
        "returns the first reference of the specified type in the specified cell",
        GET_FIRST_REF_IN_CELL,
        get_first_ref_in_cell,
        Returns::Form,
    ),
    cmd(
        0x1475,
        "GetNumRefsInCell",
        "returns the number of references of a given type in the specified cell",
        GET_REFS_IN_CELL,
        get_num_refs_in_cell,
        Returns::Default,
    ),
    cmd(
        0x15c7,
        "GetRefs",
        "returns an array of references of a given type in the current cell",
        GET_REFS,
        get_refs,
        Returns::Array,
    ),
    cmd(
        0x15c8,
        "GetRefsInCell",
        "returns an array of references of a given type in the specified cell",
        GET_REFS_IN_CELL,
        get_refs_in_cell,
        Returns::Array,
    ),
    cmd(
        0x15ed,
        "GetInGrid",
        "returns if a specific reference is in the current cell",
        GET_IN_GRID,
        get_in_grid,
        Returns::Default,
    ),
    cmd(
        0x15ee,
        "GetInGridInCell",
        "returns if a specific reference is in the grid centered on the specified cell",
        GET_IN_GRID_IN_CELL,
        get_in_grid_in_cell,
        Returns::Default,
    ),
];

/// The scans `GetFirstRef` started, by script (`s_scanScripts` (xNVSE)); `None` for a record
/// xNVSE made without building it (a `GetNextRef` with no scan: its iterator zeroed, the rest
/// whatever its memory held).
#[derive(Default)]
pub struct State {
    scans: HashMap<u32, Option<Scan>>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<crate::Nvse>().cell_scans
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: CellScanInfo, read
// against the shipped DLL's layout (iterator `+0`, current cell `+4`, cell `+8`, world `+0xC`,
// x and y bytes `+0x10`, `+0x11`, type `+0x12`, depth `+0x13`, taken `+0x14`).
/// A walk over a cell and the cells around it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Scan {
    /// The node of the last reference answered (`TESObjectCELL::RefList::Iterator`), 0 for none.
    prev: u32,
    /// The cell being scanned, 0 once the walk is over.
    cur_cell: u32,
    /// The centre cell and its worldspace (0 for an interior).
    cell: u32,
    world: u32,
    cur_x: i8,
    cur_y: i8,
    form_type: u8,
    depth: u8,
    include_taken: bool,
    /// Where the walk started: a walk that comes back there can never end (the list guard).
    start: (i8, i8),
    /// Stepped (`GetFirstRef`, `GetNextRef`): each call walks only until it finds a reference, so
    /// coming back to the start is a hang only when one call has gone the whole way round from
    /// where it began without finding one ([`Scan::begin_call`]). An area walk (`GetNumRefs`,
    /// `GetRefs`) goes round in one call: back at the start it can never end.
    stepped: bool,
    /// A stepped call has passed its starting position once.
    lapped: bool,
    /// The list guard ended the walk (no cell is current: the scan answers 0).
    guard_ended: bool,
}

/// The cell's coordinates (`cellData`'s two words); a cell without that data is read through, as
/// in xNVSE, unless [`passed_over`].
fn coordinates(e: &mut Engine, cell: u32) -> Option<(u32, u32)> {
    let data = e.mem.u32(cell + CELL_DATA);
    if passed_over(e, data) {
        return None;
    }
    Some((e.mem.u32(data), e.mem.u32(data + 4)))
}

/// The cell map's key for a position (`(x << 16) + y` in 32 bits).
fn key(x: i32, y: i32) -> u32 {
    (x << 16).wrapping_add(y) as u32
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), NiTypes.h: NiTPointerMap::Lookup.
fn lookup(e: &mut Engine, world: u32, key: u32) -> u32 {
    let map = e.mem.u32(world + WORLD_CELL_MAP);
    let count = e.mem.u32(map + 4);
    let mut entry = e.mem.u32(e.mem.u32(map + 8) + 4 * (key % count));
    while entry != 0 {
        if e.mem.u32(entry + 4) == key {
            return e.mem.u32(entry + 8);
        }
        entry = e.mem.u32(entry);
    }
    0
}

impl Scan {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
    // CellScanInfo::CellScanInfo (the DLL's `100368F0`).
    /// The walk's start: an exterior with a depth starts at the corner (the centre minus the
    /// depth, in bytes), anything else at the cell itself with no depth. `None` when the cell's
    /// coordinates are missing and [`passed_over`].
    fn new(
        e: &mut Engine,
        depth: u8,
        form_type: u8,
        include_taken: bool,
        cell: u32,
    ) -> Option<Scan> {
        let world = e.mem.u32(cell + CELL_WORLD);
        let (x, y) = coordinates(e, cell)?;
        let mut s = Scan {
            prev: 0,
            cur_cell: 0,
            cell,
            world,
            cur_x: 0,
            cur_y: 0,
            form_type,
            depth,
            include_taken,
            start: (0, 0),
            stepped: false,
            lapped: false,
            guard_ended: false,
        };
        if world != 0 && depth != 0 {
            s.cur_x = (x as u8).wrapping_sub(depth) as i8;
            s.cur_y = (y as u8).wrapping_sub(depth) as i8;
            s.cur_cell = lookup(e, world, key(i32::from(s.cur_x), i32::from(s.cur_y)));
        } else {
            s.depth = 0;
            s.cur_cell = cell;
            s.cur_x = x as u8 as i8;
            s.cur_y = y as u8 as i8;
        }
        s.start = (s.cur_x, s.cur_y);
        Some(s)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
    // CellScanInfo::NextCell (the DLL's `100369C0`).
    /// The next cell of the area that is in the map, row by row; false (and no current cell) at
    /// the end, or at once for an interior or no depth. A depth that takes the area's far edge
    /// past -128..127 is never met by the end test (the position is two bytes): xNVSE walks on for
    /// ever and the game hangs. Once the walk is back where it started it has been through every
    /// position it can reach: with the list guard row (`NV_NVSE_LIST_GUARD`) it ends there as if
    /// it had reached its end; without the row the port stops the game there instead.
    fn next_cell(&mut self, e: &mut Engine) -> bool {
        if self.world == 0 || self.depth == 0 {
            self.cur_cell = 0;
            return false;
        }
        let depth = i32::from(self.depth);
        loop {
            let Some((x, y)) = coordinates(e, self.cell) else {
                self.cur_cell = 0;
                return false;
            };
            let (old_x, old_y) = (i32::from(self.cur_x), i32::from(self.cur_y));
            let at = if old_x.wrapping_sub(x as i32) == depth {
                if old_y.wrapping_sub(y as i32) == depth {
                    self.cur_cell = 0;
                    return false;
                }
                self.cur_y = self.cur_y.wrapping_add(1);
                self.cur_x = (self.cur_x as u8).wrapping_sub(self.depth.wrapping_mul(2)) as i8;
                key(i32::from(self.cur_x), old_y + 1)
            } else {
                self.cur_x = self.cur_x.wrapping_add(1);
                key(old_x + 1, i32::from(self.cur_y))
            };
            if (self.cur_x, self.cur_y) == self.start && self.stepped && !self.lapped {
                // A stepped call back where it began: its own cell is searched again from the
                // top (xNVSE answers the same references again, round and round); a second time
                // round with nothing found is the hang.
                self.lapped = true;
            } else if (self.cur_x, self.cur_y) == self.start {
                if !super::array::fix_on(e, super::array::LIST_GUARD) {
                    panic!(
                        "a cell scan whose depth takes it past the cell grid's 8-bit positions never \
                         ends: xNVSE walks on for ever and the game hangs (NV_NVSE_LIST_GUARD ends it)"
                    );
                }
                self.cur_cell = 0;
                self.guard_ended = true;
                return false;
            }
            self.cur_cell = lookup(e, self.world, at);
            if self.cur_cell != 0 {
                return true;
            }
        }
    }

    /// A stepped call begins here: the loop check counts from this position.
    fn begin_call(&mut self) {
        self.stepped = true;
        self.lapped = false;
        self.start = (self.cur_x, self.cur_y);
    }

    /// `CellScanInfo::FirstCell`.
    fn first_cell(&mut self, e: &mut Engine) {
        if self.cur_cell == 0 {
            self.next_cell(e);
        }
    }
}

// ---- the reference filters ----

/// What a scan accepts (xNVSE's `RefMatcher*`): the filter (`form_type`, one of the custom
/// filters or a form type), the taken references, the distance and angle from a reference
/// (`DistanceAngleMatcher`, none without one), a base form, the reference `GetInGrid` looks for.
#[derive(Clone, Copy, Default)]
struct Matcher {
    form_type: u32,
    include_taken: bool,
    distance_ref: u32,
    max_distance: f32,
    max_angle: f32,
    base_form: u32,
    refr: u32,
    /// `GetNumRefs` and `GetRefs` have no case for a given reference: it filters by type.
    by_reference: bool,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameObjects.cpp: GetDistance3D.
/// The distance between two references' positions in `float`s, summed as xNVSE's SSE code does
/// (z, then x, then y).
fn distance_3d(e: &Engine, a: u32, b: u32) -> f32 {
    let d = |i: u32| e.mem.f32(a + REFR_POS + 4 * i) - e.mem.f32(b + REFR_POS + 4 * i);
    let (x, y, z) = (d(0), d(1), d(2));
    ((z * z + x * x) + y * y).sqrt()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameObjects.cpp:
// TESObjectREFR::GetHeadingAngle.
/// The angle in degrees from `from`'s heading to `to`, from -180 to 180, as the shipped DLL makes it
/// (inlined in the distance and angle filter at `10036B2C`): the `float` differences widened to
/// `double` for the C runtime's `atan2`, its answer rounded to `float`, the rotation taken off in
/// `float`, then widened and multiplied by `57.29577951308232` in `double`.
pub(crate) fn heading_angle(e: &Engine, from: u32, to: u32) -> f64 {
    let dx = e.mem.f32(to + REFR_POS) - e.mem.f32(from + REFR_POS);
    let dy = e.mem.f32(to + REFR_POS + 4) - e.mem.f32(from + REFR_POS + 4);
    let rot = e.mem.f32(from + REFR_ROT_Z);
    let angle = f64::from(dx).atan2(f64::from(dy)) as f32 - rot;
    let mut result = f64::from(angle) * 57.295_779_513_082_32;
    if result < -180.0 {
        result += 360.0;
    } else if result > 180.0 {
        result -= 360.0;
    }
    result
}

impl Matcher {
    /// `IncludeTakenMatcher::MatchTakenItems`.
    fn taken_ok(&self, e: &Engine, refr: u32) -> bool {
        self.include_taken || e.mem.u32(refr + FORM_FLAGS) & TAKEN == 0
    }
    /// `BaseFormMatcher::MatchBaseForms`.
    fn base_ok(&self, e: &Engine, refr: u32) -> bool {
        self.base_form == 0 || e.mem.u32(refr + REFR_BASE_FORM) == self.base_form
    }
    /// `DistanceAngleMatcher::MatchDistanceAndAngle`.
    fn distance_ok(&self, e: &Engine, refr: u32) -> bool {
        if self.distance_ref == 0 || refr == self.distance_ref {
            return true;
        }
        if self.max_distance > 0.0 && distance_3d(e, self.distance_ref, refr) > self.max_distance {
            return false;
        }
        if self.max_angle >= 0.0
            && heading_angle(e, self.distance_ref, refr).abs() > f64::from(self.max_angle)
        {
            return false;
        }
        true
    }
    /// The base form's type, read through a missing base form as xNVSE does unless
    /// [`passed_over`] (`None`: not accepted).
    fn base_type(e: &mut Engine, refr: u32) -> Option<(u8, u32)> {
        let base = e.mem.u32(refr + REFR_BASE_FORM);
        if passed_over(e, base) {
            return None;
        }
        Some((e.mem.u8(base + FORM_TYPE), base))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: RefMatcherARefr,
    // RefMatcherAnyForm, RefMatcherFormType, RefMatcherActor, RefMatcherItem (Accept each).
    fn accept(&self, e: &mut Engine, refr: u32) -> bool {
        match self.form_type {
            SPECIFIC_REFERENCE if self.by_reference => {
                if !self.include_taken {
                    let t = u32::from(e.mem.u8(refr + FORM_TYPE));
                    let inventory = e.call(IS_INVENTORY_OBJECT_TYPE, &args![t]).u32() & 0xff != 0;
                    if inventory && e.mem.u32(refr + FORM_FLAGS) & TAKEN != 0 {
                        return false;
                    }
                }
                refr == self.refr
            }
            ANY_TYPE => {
                self.taken_ok(e, refr) && self.base_ok(e, refr) && self.distance_ok(e, refr)
            }
            ACTORS => {
                if !self.base_ok(e, refr) {
                    return false;
                }
                let Some((t, base)) = Self::base_type(e, refr) else {
                    return false;
                };
                if t != ty::CREATURE
                    && (t != ty::NPC || e.mem.u32(base + FORM_ID) == PLAYER_BASE_ID)
                {
                    return false;
                }
                self.distance_ok(e, refr)
            }
            ITEMS => {
                if !self.taken_ok(e, refr) || !self.base_ok(e, refr) {
                    return false;
                }
                let Some((t, base)) = Self::base_type(e, refr) else {
                    return false;
                };
                let item = match t {
                    ty::ARMO
                    | ty::BOOK
                    | ty::CLOT
                    | ty::INGREDIENT
                    | ty::MISC
                    | ty::WEAP
                    | ty::AMMO
                    | ty::KEY
                    | ty::ALCHEMY
                    | ty::NOTE
                    | ty::ARMA
                    | ty::IMOD
                    | ty::CASINO_CHIPS
                    | ty::CARAVAN_CARD
                    | ty::CARAVAN_MONEY => true,
                    ty::LIGH => {
                        let light = dynamic_cast(
                            e,
                            base,
                            super::rtti::TES_FORM,
                            super::rtti::TES_OBJECT_LIGH,
                        );
                        light != 0 && e.mem.u32(light + LIGHT_FLAGS) & LIGHT_CAN_CARRY != 0
                    }
                    _ => false,
                };
                item && self.distance_ok(e, refr)
            }
            _ => {
                if !self.taken_ok(e, refr) || !self.base_ok(e, refr) {
                    return false;
                }
                let Some((t, base)) = Self::base_type(e, refr) else {
                    return false;
                };
                if u32::from(t) != self.form_type || e.mem.u32(base + FORM_ID) == PLAYER_BASE_ID {
                    return false;
                }
                self.distance_ok(e, refr)
            }
        }
    }
}

// ---- the game's reference lists (`tList`, GameTypes.h (xNVSE)) ----

/// `tList::Iterator::End`: no node, or an empty last node.
fn at_end(e: &Engine, node: u32) -> bool {
    node == 0 || (e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::Find(op, prev).
/// The node of the first reference after `prev` (from the list's head when `prev` is at its
/// end) the matcher accepts; a node at the end when there is none.
fn find(e: &mut Engine, head: u32, prev: u32, m: &Matcher) -> u32 {
    let mut cur = if at_end(e, prev) {
        head
    } else {
        e.mem.u32(prev + 4)
    };
    while !at_end(e, cur) {
        let refr = e.mem.u32(cur);
        if refr != 0 && m.accept(e, refr) {
            break;
        }
        cur = e.mem.u32(cur + 4);
    }
    cur
}

/// The references of a cell's list the matcher accepts, in order (`tList::CountIf` and the
/// `GetRefs` loop).
fn accepted(e: &mut Engine, cell: u32, m: &Matcher) -> Vec<u32> {
    let mut found = Vec::new();
    let mut node = cell + CELL_OBJECTS;
    while node != 0 {
        let refr = e.mem.u32(node);
        if refr != 0 && m.accept(e, refr) {
            found.push(refr);
        }
        node = e.mem.u32(node + 4);
    }
    found
}

// ---- the commands ----

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

/// A depth as the commands take it: -127 (none given) is 0, -1 is `uGridsToLoad:General` (0
/// without it), converted as xNVSE converts a double to an `SInt32`.
fn depth_of(e: &mut Engine, depth: i32) -> i32 {
    match depth {
        NO_DEPTH => 0,
        GRID_DEPTH => super::string::numeric_ini_setting(e, b"uGridsToLoad:General")
            .map_or(0, crate::args::f64_to_i32),
        d => d,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: GetCellRefEntry.
fn entry_matcher(form_type: u32, include_taken: bool, refr: u32) -> Matcher {
    Matcher {
        form_type,
        include_taken,
        refr,
        by_reference: true,
        max_angle: -1.0,
        ..Matcher::default()
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: CellScan (read
// against the DLL's `10037350`: a new record's iterator is zeroed and nothing else).
/// The next reference of the script's scan, starting a new one with `start` (`GetFirstRef`):
/// the cell, the filter, the depth (as a byte), the taken references, the reference
/// `GetInGrid` looks for. The scan is kept by the script's id, or the console's key for a
/// console line, and dropped when it ends: by the script's own id, as xNVSE drops it, so a
/// console scan stays after its end.
///
/// A `GetNextRef` with no scan reports the error and leaves an unbuilt record, a second one reads
/// that record; a console scan that has ended is read again through its missing cell: xNVSE reads
/// through both and the game stops. With the script reference checks row (`NV_NVSE_INVREF_CHECK`)
/// both answer as the first call did (the error, 0); without it the port stops the game there.
fn cell_scan(e: &mut Engine, script: u32, start: Option<(u32, u32, u32, bool, u32)>) -> u32 {
    let script_id = form_id(e, script);
    let key = if script_id >> 24 == 0xff && is_console_mode(e) {
        CONSOLE_KEY
    } else {
        script_id
    };
    let new = !state(e).scans.contains_key(&key);
    if new {
        state(e).scans.insert(key, None);
    }
    let (mut s, refr) = if new || start.is_some() {
        let Some((cell, form_type, depth, include_taken, refr)) = start.filter(|s| s.0 != 0) else {
            crate::functions::runtime_error(e, script, NO_SCAN);
            return 0;
        };
        let mut s = Scan::new(e, depth as u8, form_type as u8, include_taken, cell);
        if let Some(s) = s.as_mut() {
            s.begin_call();
            s.first_cell(e);
        }
        state(e).scans.insert(key, s);
        let Some(s) = s else {
            return 0;
        };
        (s, refr)
    } else {
        let s = state(e).scans[&key];
        match s {
            Some(mut s) if s.cur_cell != 0 => {
                s.begin_call();
                (s, 0)
            }
            _ => {
                if !invref::checks(e) {
                    panic!(
                        "GetNextRef reads a cell scan xNVSE never built or has ended through its missing \
                         cell, and the game stops (NV_NVSE_INVREF_CHECK answers as the first call)"
                    );
                }
                crate::functions::runtime_error(e, script, NO_SCAN);
                return 0;
            }
        }
    };
    let found = loop {
        if s.cur_cell == 0 {
            // A walk that found no cell at all: xNVSE reads the missing cell's list. A walk the
            // list guard ended answers 0 (the row's own end, not that read).
            if !s.guard_ended && !invref::checks(e) {
                panic!(
                    "a cell scan found no cell, and xNVSE reads through the missing cell: the game \
                     stops (NV_NVSE_INVREF_CHECK ends the scan)"
                );
            }
            break 0;
        }
        let m = entry_matcher(u32::from(s.form_type), s.include_taken, refr);
        s.prev = if m.form_type == SPECIFIC_REFERENCE && refr == 0 {
            0
        } else {
            find(e, s.cur_cell + CELL_OBJECTS, s.prev, &m)
        };
        if at_end(e, s.prev) || e.mem.u32(s.prev) == 0 {
            if !s.next_cell(e) {
                break 0;
            }
        } else {
            break e.mem.u32(s.prev);
        }
    };
    state(e).scans.insert(key, Some(s));
    if found == 0 {
        state(e).scans.remove(&script_id);
    }
    found
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: GetFirstRef_Execute.
/// The first reference of a new scan (the form id); in the player's cell or a given one, by
/// filter or for a given reference. Answers whether a result was made (for `GetInGrid`).
fn first_ref(e: &mut Engine, a: &CommandArgs, player_cell: bool, by_ref: bool) -> u32 {
    a.set_form_result(e, 0);
    let pc = e.global::<u32>(PLAYER);
    if pc == 0 {
        return 0;
    }
    let read = Read::of(e, a);
    if !read.ok {
        return 0;
    }
    let first = usize::from(!player_cell);
    let (cell, form_type, refr) = match (player_cell, by_ref) {
        (true, true) => (
            e.mem.u32(pc + REFR_PARENT_CELL),
            SPECIFIC_REFERENCE,
            read.form(0),
        ),
        (true, false) => (e.mem.u32(pc + REFR_PARENT_CELL), read.int(0, ANY_TYPE), 0),
        (false, true) => (read.form(0), SPECIFIC_REFERENCE, read.form(1)),
        (false, false) => (read.form(0), read.int(1, ANY_TYPE), 0),
    };
    let depth = read.int(first + 1, NO_DEPTH as u32) as i32;
    let include = read.int(first + 2, 0) != 0;
    if cell == 0 {
        return 0;
    }
    let depth = depth_of(e, depth);
    let found = cell_scan(
        e,
        a.script,
        Some((cell, form_type, depth as u32, include, refr)),
    );
    let id = if found != 0 { form_id(e, found) } else { 0 };
    a.set_form_result(e, id);
    if is_console_mode(e) {
        console_print(e, "GetFirstRef >> %08x", &[id]);
    }
    id
}

fn get_first_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    first_ref(e, &a, true, false);
    handled(true)
}

fn get_first_ref_in_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    first_ref(e, &a, false, false);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: Cmd_GetInGrid_Execute,
// Cmd_GetInGridInCell_Execute.
/// 1 when the reference is in the cell's area (a scan for it; the scan is kept as
/// `GetFirstRef`'s, and the console line printed is `GetFirstRef`'s).
fn get_in_grid(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = first_ref(e, &a, true, true);
    a.set_result(e, if id != 0 { 1.0 } else { 0.0 });
    handled(true)
}

fn get_in_grid_in_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = first_ref(e, &a, false, true);
    a.set_result(e, if id != 0 { 1.0 } else { 0.0 });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: Cmd_GetNextRef_Execute.
/// The script's scan's next reference; nothing at all (the result untouched) while the player is
/// in no cell.
fn get_next_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let pc = e.global::<u32>(PLAYER);
    if pc == 0 || e.mem.u32(pc + REFR_PARENT_CELL) == 0 {
        return handled(true);
    }
    a.set_form_result(e, 0);
    let found = cell_scan(e, a.script, None);
    let id = if found != 0 { form_id(e, found) } else { 0 };
    a.set_form_result(e, id);
    if is_console_mode(e) {
        console_print(e, "GetNextRef >> %08x", &[id]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: GetNumRefs_Execute,
// GetRefs_Execute.
/// The references of the area the filters accept, cell by cell (each cell's list in order); the
/// distance and angle are from the calling reference. `None` when the command answers nothing.
fn area_refs(e: &mut Engine, a: &CommandArgs, player_cell: bool) -> Option<Vec<u32>> {
    let pc = e.global::<u32>(PLAYER);
    if pc == 0 || e.mem.u32(pc + REFR_PARENT_CELL) == 0 {
        return None;
    }
    let read = Read::of(e, a);
    if !read.ok {
        return None;
    }
    let first = usize::from(!player_cell);
    let cell = if player_cell {
        e.mem.u32(pc + REFR_PARENT_CELL)
    } else {
        read.form(0)
    };
    if cell == 0 {
        return None;
    }
    let form_type = read.int(first, ANY_TYPE);
    let include = read.int(first + 2, 0) != 0;
    let depth = depth_of(e, read.int(first + 1, NO_DEPTH as u32) as i32);
    let m = Matcher {
        form_type,
        include_taken: include,
        distance_ref: a.this_obj,
        max_distance: read.float(first + 3, 0.0),
        max_angle: read.float(first + 4, -1.0),
        base_form: read.form(first + 5),
        refr: 0,
        by_reference: false,
    };
    let mut s = Scan::new(e, depth as u8, form_type as u8, include, cell)?;
    s.first_cell(e);
    let mut found = Vec::new();
    while s.cur_cell != 0 {
        found.extend(accepted(e, s.cur_cell, &m));
        s.next_cell(e);
    }
    Some(found)
}

fn num_refs(e: &mut Engine, w: &[u32], player_cell: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if let Some(found) = area_refs(e, &a, player_cell) {
        a.set_result(e, found.len() as f64);
    }
    handled(true)
}

fn get_num_refs(e: &mut Engine, w: &[u32]) -> Ret {
    num_refs(e, w, true)
}

fn get_num_refs_in_cell(e: &mut Engine, w: &[u32]) -> Ret {
    num_refs(e, w, false)
}

/// A new array of the references' ids (made first, so a command that answers nothing still
/// answers an empty array).
fn refs(e: &mut Engine, w: &[u32], player_cell: bool) -> Ret {
    let a = CommandArgs::new(w);
    let modi = vars::script_mod_index(e, a.script);
    let array = vars::create_array(e, vars::KEY_NUMBER, true, modi);
    a.set_result(e, f64::from(array));
    if let Some(found) = area_refs(e, &a, player_cell) {
        for (i, refr) in found.into_iter().enumerate() {
            let id = form_id(e, refr);
            vars::set_element(e, array, &Key::Number(i as f64), Value::Form(id));
        }
    }
    handled(true)
}

fn get_refs(e: &mut Engine, w: &[u32]) -> Ret {
    refs(e, w, true)
}

fn get_refs_in_cell(e: &mut Engine, w: &[u32]) -> Ret {
    refs(e, w, false)
}

#[cfg(test)]
#[path = "misc_ref_cells_tests.rs"]
mod tests;
