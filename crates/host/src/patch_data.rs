//! Patch data: changes the fixes preset makes to records of the player's own game files as they
//! load (docs/FIXES.md, "Patch data"; `crates/host/data/patch_data.tsv`). The maintainer's decision
//! of 2026-11-01 (measure/perf/compat/BUILT-IN-MODS.md, 6.3): a data mod comes as each author's
//! change (the record, the field, the new value) in nv-rs's own format, applied to the player's
//! own files at load, each switchable and credited; no Bethesda-derived file ships.
//!
//! Each row belongs to a fixes row ([`crate::fixes::FIXES`]) by its switch, and is applied only
//! when that row is on: in the vanilla preset nothing here runs. The one row so far sets
//! "No LOD Water" on The Strip's world space, so its form is the world space's load
//! (`TESWorldSpace::Load`, `00583560`, reached only through the form's vtable): the record loads
//! as the translation reads it, then the row's bits are set in its `cFlags` (+0x4c, the `DATA`
//! field). A plugin that overrides the record later loads it again, and the bits are set again.

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_shared::tesworldspace::{fn_00583560, TESWorldSpace};
use std::sync::OnceLock;

/// The table, in the host.
const TABLE: &str = include_str!("../data/patch_data.tsv");

/// `TESWorldSpace::Load` (Xbox PDB), vtable slot +0x20 of the world space form.
pub const WORLD_SPACE_LOAD: u32 = 0x0058_3560;

/// "No LOD Water" on The Strip (`TheStripWorldNew`, FalloutNV.esm `0013b308`).
pub const STRIP_LOD_WATER: Fix = Fix {
    name: "no distant water under the Strip",
    env: "NV_STRIP_LOD_WATER",
    addresses: &[WORLD_SPACE_LOAD],
    summary: "The Strip gets \"No LOD Water\" as it loads (patch data): the distant water under its ground is never made",
    install: install_strip_lod_water,
};

/// One change of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The plugin the record is from.
    pub plugin: String,
    /// The record's form id in that plugin.
    pub form: u32,
    /// The record type and the field (`WRLD`, `DATA`).
    pub record: String,
    pub field: String,
    /// The bits the change sets.
    pub or_bits: u32,
    /// The switch of the fixes row it belongs to.
    pub switch: String,
}

/// The table's rows; an error names the line it can't read.
pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |why: &str| format!("patch_data.tsv line {}: {why}", number + 1);
        let columns: Vec<&str> = line.split('\t').collect();
        let [plugin, form, record, field, change, switch, _what] = columns.as_slice() else {
            return Err(at("seven columns expected"));
        };
        let form = u32::from_str_radix(form, 16).map_err(|_| at("the form id is not hex"))?;
        let or_bits = change
            .strip_prefix("or ")
            .and_then(|bits| u32::from_str_radix(bits, 16).ok())
            .ok_or_else(|| at("the change is not \"or HEX\""))?;
        rows.push(Row {
            plugin: plugin.to_string(),
            form,
            record: record.to_string(),
            field: field.to_string(),
            or_bits,
            switch: switch.to_string(),
        });
    }
    Ok(rows)
}

/// The built-in table's rows (a test holds it readable).
pub fn rows() -> &'static [Row] {
    static ROWS: OnceLock<Vec<Row>> = OnceLock::new();
    ROWS.get_or_init(|| parse(TABLE).unwrap_or_default())
}

/// The form id a row's record has in the game. FalloutNV.esm is always the first file loaded
/// (load index 0), so its ids are the table's; the table holds no other plugin's rows yet (a
/// test holds it so), which would need the plugin's load index.
fn game_form_id(row: &Row) -> Option<u32> {
    (row.plugin.eq_ignore_ascii_case("FalloutNV.esm") && row.form < 0x0100_0000).then_some(row.form)
}

/// The `cFlags` bits the rows of `switch` set on the world space with this form id.
pub fn world_space_flags(switch: &str, form_id: u32) -> u8 {
    rows()
        .iter()
        .filter(|r| r.switch == switch && r.record == "WRLD" && r.field == "DATA")
        .filter(|r| game_form_id(r) == Some(form_id))
        .fold(0, |bits, r| bits | r.or_bits as u8)
}

fn install_strip_lod_water(e: &mut Engine) {
    e.register(WORLD_SPACE_LOAD, strip_world_space_load);
}

fn strip_world_space_load(e: &mut Engine, a: &[u32]) -> Ret {
    world_space_load(e, Ptr::new(a[0]), Ptr::new(a[1]), STRIP_LOD_WATER.env).into_ret()
}

/// `00583560` with the rows of `switch`: the record loads as the original reads it, then, when it
/// was a world space record, the rows' bits for its form id (`TESForm`'s id, +0xc) are set in
/// its flags.
pub fn world_space_load(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr, switch: &str) -> bool {
    let loaded = fn_00583560(e, this, file);
    if loaded {
        set_world_space_flags(e, this, switch);
    }
    loaded
}

/// The bits the rows of `switch` give a loaded world space's form id (`TESForm`'s id, +0xc), set
/// in its flags.
fn set_world_space_flags(e: &mut Engine, this: Ptr<TESWorldSpace>, switch: &str) {
    let form_id = e.mem.u32(this.addr() + 0xc);
    let bits = world_space_flags(switch, form_id);
    if bits != 0 {
        let flags = e.get(this, TESWorldSpace::cFlags);
        e.set(this, TESWorldSpace::cFlags, flags | bits);
    }
}

/// `00583560` when the cell offsets row is on too: both rows replace `TESWorldSpace::Load`, and
/// each alone would replace the other's, so `crate::fixes::install` registers this one after
/// them: cell offsets' load (the translation, then the record's table), then the Strip's flags.
pub fn strip_load_with_cell_offsets(e: &mut Engine, a: &[u32]) -> Ret {
    let this = Ptr::new(a[0]);
    let loaded = crate::cell_offsets::load(e, this, Ptr::new(a[1]));
    if loaded {
        set_world_space_flags(e, this, STRIP_LOD_WATER.env);
    }
    loaded.into_ret()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row is readable, names a fixes row's switch, FalloutNV.esm and a change the code
    /// applies (a world space's `DATA` flags, one byte).
    #[test]
    fn the_table_reads_and_every_row_is_applied() {
        let parsed = parse(TABLE).unwrap();
        assert!(!parsed.is_empty());
        assert_eq!(parsed, rows());
        for row in &parsed {
            assert!(
                crate::fixes::FIXES.iter().any(|f| f.env == row.switch),
                "{row:?}"
            );
            assert!(game_form_id(row).is_some(), "{row:?}");
            assert_eq!((row.record.as_str(), row.field.as_str()), ("WRLD", "DATA"));
            assert!(row.or_bits != 0 && row.or_bits <= 0xff, "{row:?}");
        }
        assert!(parse("FalloutNV.esm\tzz\tWRLD\tDATA\tor 10\tX\tw\n").is_err());
        assert!(parse("FalloutNV.esm\t10\tWRLD\tDATA\tset 10\tX\tw\n").is_err());
        assert!(parse("FalloutNV.esm\t10\tWRLD\n").is_err());
    }

    /// The Strip's row: TheStripWorldNew (`0013b308` in FalloutNV.esm, `DATA` 0x01 there) gets
    /// 0x10, "No LOD Water", the bit `005862c0` reads; no other world space gets anything.
    #[test]
    fn the_strip_gets_no_lod_water() {
        assert_eq!(world_space_flags("NV_STRIP_LOD_WATER", 0x0013_b308), 0x10);
        assert_eq!(world_space_flags("NV_STRIP_LOD_WATER", 0x000d_a726), 0);
        assert_eq!(world_space_flags("NV_OTHER", 0x0013_b308), 0);
    }

    /// A world space record read through the form's load (`00583560`) with the flags the record
    /// gave (0x01, as The Strip's `DATA`): kept in vanilla (nothing registered); with the row the
    /// Strip's form gets 0x10 added, another world space nothing; a record of another type is not
    /// loaded and gets nothing.
    #[test]
    fn the_load_in_both_modes() {
        // The record's reader (the translation's callees): its type, not a master, the form's own
        // load setting the form id, no chunks left.
        let run = |fix: bool, form_id: u32, record_type: u32| {
            let mut e = Engine::new();
            if fix {
                install_strip_lod_water(&mut e);
            }
            e.register_double(0x0047_2660, move |_, _| record_type.into_ret());
            e.register(0x0047_1c20, |_, _| 0u32.into_ret());
            e.register_double(0x0048_5110, move |e, a| {
                e.mem.set_u32(a[0] + 0xc, form_id);
                Ret::default()
            });
            e.register(0x0048_4ab0, |_, _| Ret::default());
            e.register(0x0047_26b0, |_, _| 0u32.into_ret());
            let world: Ptr<TESWorldSpace> = e.new_object();
            // A cell map that is not empty, so the load keeps it.
            let map = e.mem.alloc(0x10);
            e.mem.set_u32(map + 0xc, 3);
            e.set(world, TESWorldSpace::pCellMap, Ptr::new(map));
            e.set(world, TESWorldSpace::cFlags, 0x01);
            let loaded = e.call(WORLD_SPACE_LOAD, &args![world, 0xf11eu32]).bool();
            (loaded, e.get(world, TESWorldSpace::cFlags))
        };
        const STRIP: u32 = 0x0013_b308;
        const WASTELAND: u32 = 0x000d_a726;
        assert_eq!(run(false, STRIP, 0x41), (true, 0x01));
        assert_eq!(run(true, STRIP, 0x41), (true, 0x11));
        assert_eq!(run(true, WASTELAND, 0x41), (true, 0x01));
        assert_eq!(run(true, STRIP, 0x42), (false, 0x01));
    }

    /// With the cell offsets row on as well (the default choice has both), `fixes::install` gives
    /// `TESWorldSpace::Load` one function doing both: cell offsets' load, which asks for the file's
    /// offset data after the translation (here it has none), and then the Strip's flags.
    #[test]
    fn with_cell_offsets_on_the_load_does_both() {
        let mut e = Engine::new();
        let choice = crate::fixes::Choice::from_parts(None, |_| None).unwrap();
        assert!(choice.is_on(&STRIP_LOD_WATER) && choice.is_on(&crate::cell_offsets::FIX));
        crate::fixes::install(&mut e, &choice);
        // The translation's callees, as in `the_load_in_both_modes`, for The Strip's record.
        e.register_double(0x0047_2660, |_, _| 0x41u32.into_ret());
        e.register(0x0047_1c20, |_, _| 0u32.into_ret());
        e.register_double(0x0048_5110, |e, a| {
            e.mem.set_u32(a[0] + 0xc, 0x0013_b308);
            Ret::default()
        });
        e.register(0x0048_4ab0, |_, _| Ret::default());
        e.register(0x0047_26b0, |_, _| 0u32.into_ret());
        // Cell offsets' question after the load: the file's offset data (none).
        let asked = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let count = std::rc::Rc::clone(&asked);
        e.register_double(0x0058_8a90, move |_, _| {
            count.set(count.get() + 1);
            0u32.into_ret()
        });
        let world: Ptr<TESWorldSpace> = e.new_object();
        let map = e.mem.alloc(0x10);
        e.mem.set_u32(map + 0xc, 3);
        e.set(world, TESWorldSpace::pCellMap, Ptr::new(map));
        e.set(world, TESWorldSpace::cFlags, 0x01);
        let file = e.mem.alloc(0x400);
        // The globals cell offsets reads (no data handler yet: no tables are being built).
        e.map(0x011c_0000, 0x6_0000);
        assert!(e.call(WORLD_SPACE_LOAD, &args![world, file]).bool());
        assert_eq!(
            e.get(world, TESWorldSpace::cFlags),
            0x11,
            "the Strip's flags"
        );
        assert_eq!(asked.get(), 1, "cell offsets' load ran");
    }
}
