//! Cell offsets for every plugin (`NV_CELL_OFFSETS`), a fix of the `fixes` preset
//! ([`crate::fixes`], docs/FIXES.md): **not 1:1 with FalloutNV.exe**; in the `vanilla` preset
//! nothing here runs. Cell Offset Generator's idea (by WallSoGB, GPL-3.0; none of its code).
//!
//! **What the original does.** Each plugin that holds a world space record keeps an
//! `OFFSET_DATA` for it (`TESWorldSpace::OffsetDataMap`): the record's position, the grid its
//! `NAM0` / `NAM9` corners span and a table of the grid's cell record positions (`OFST`). Only a
//! master file gets the position and the corners (`TESWorldSpace::Load`, `00583560`, at
//! `005835bc`, `00583dc0` and `00583ea8`), and `TESWorldSpace::FindCellInFile` (`005854f0`)
//! uses a table only for a master (`00585502`). Every other file, and a master whose table was
//! stripped (YUP's and TTW's are), is searched record by record from the world space record, once
//! for each cell loaded and each such file, on the one loading thread (Cell Offset Generator's
//! author measured up to 20 ms a file a cell in the original; docs/FIXES.md has the cost here).
//!
//! **What this does.** Registered over `Load` and `FindCellInFile`.
//! - `Load` runs the translation, then gives the record's `OFFSET_DATA` the table its search
//!   would give, built from the plugin's records by `esm::cell_offsets` (one pass over its world
//!   space group, on a host thread started at the first world space record of the run for every
//!   plugin of the load order, in load order, waited for when not done yet). A master keeps the
//!   position and corners the translation stored and only gains a table where it has none; any
//!   other file gets the position and corners too, as a master would, and its table replaces an
//!   `OFST` it may carry (which the original never reads for it).
//! - `FindCellInFile` finds a cell of a file that is not a master by that table when the record
//!   has one (its position is set: only this fix sets one for such a file), exactly as the
//!   translation does for a master; anything else goes to the translation.
//!
//! A table is only made where it gives the same record as the search for every cell position
//! (`esm::cell_offsets` declines otherwise, and that world space is searched as before), so the
//! same cells are found without the search. The tables live in the game's heap like the masters'
//! own, freed with the world space (`00588c80`). Nothing is written to disk: building a plugin's
//! tables reads less of it than hashing it for a cache would (YUP's 28 tables: 1.9 ms in a release
//! build, 8.4 ms in the play profile).

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::fallout_shared::tesworldspace::{
    fn_00583560, tes_world_space_find_cell_in_file, OffsetData, TESWorldSpace,
};
use esm::cell_offsets::{world_offsets, Decline, WorldOffsets};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// `TESWorldSpace::Load` and `TESWorldSpace::FindCellInFile` (Xbox PDB).
pub const LOAD: u32 = 0x0058_3560;
pub const FIND_CELL_IN_FILE: u32 = 0x0058_54f0;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "cell offsets for every plugin",
    env: "NV_CELL_OFFSETS",
    addresses: &[LOAD, FIND_CELL_IN_FILE],
    summary: "a cell of a plugin (or of a master without its cell offsets, as YUP's and TTW's are) is found by a table built from the plugin at load, not by searching the file each time a cell loads",
    install,
};

/// `TESFile::GetMaster` (`00471c20`), `TESFile::SetOffset` (`004723a0`), the `OFFSET_DATA` of
/// a file (`00588a90`), `TESWorldSpace::CreateOffsetData` (`00588b00`) and a cell's index in
/// a file's table (`00584530`, -1 outside its grid).
const FILE_IS_MASTER: u32 = 0x0047_1c20;
const FILE_SET_OFFSET: u32 = 0x0047_23a0;
const OFFSET_DATA_OF_FILE: u32 = 0x0058_8a90;
const CREATE_OFFSET_DATA: u32 = 0x0058_8b00;
const CELL_INDEX: u32 = 0x0058_4530;
/// `MemoryManager::Allocate` and `Deallocate` (`00401000`, `00401030`).
const MEMORY_ALLOC: u32 = 0x0040_1000;
const MEMORY_FREE: u32 = 0x0040_1030;
/// `TESFile`: `m_Filename` (+0x20), `m_Path` (+0x124) and `m_fileoffset` (+0x264, the current
/// record's position, `00467bb0`).
const FILE_NAME: u32 = 0x20;
const FILE_PATH: u32 = 0x124;
const FILE_OFFSET: u32 = 0x264;
/// The data handler (`011c3f2c`): `iNumCompile` (+0x218) and `pFileIndex` (+0x21c).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const COMPILE_COUNT: u32 = 0x218;
const FILE_INDEX: u32 = 0x21c;

/// One plugin's world space records and their tables (`None`: the file could not be read).
type Tables = Option<Arc<Vec<WorldOffsets>>>;
type Slot = Arc<OnceLock<Tables>>;

/// A plugin file as it is now: its path, size and time written, so a changed file is read anew.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    len: u64,
    written: Option<std::time::SystemTime>,
}

fn key_of(path: PathBuf) -> Option<Key> {
    let meta = std::fs::metadata(&path).ok()?;
    Some(Key {
        len: meta.len(),
        written: meta.modified().ok(),
        path,
    })
}

/// Every plugin's tables, each built once by whoever asks first (the host thread or the game).
fn slots() -> std::sync::MutexGuard<'static, HashMap<Key, Slot>> {
    static SLOTS: OnceLock<Mutex<HashMap<Key, Slot>>> = OnceLock::new();
    SLOTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The tables of the plugin at `path`, built now unless another thread has or is building them
/// (then waited for).
pub fn tables_of(path: PathBuf) -> Tables {
    let key = key_of(path)?;
    let slot = slots().entry(key.clone()).or_default().clone();
    slot.get_or_init(|| build(&key)).clone()
}

fn build(key: &Key) -> Tables {
    let start = std::time::Instant::now();
    let file = std::fs::File::open(&key.path).ok()?;
    match world_offsets(file, false) {
        Ok(worlds) => {
            let built = worlds.iter().filter(|w| w.built.is_ok()).count();
            let searched: Vec<String> = worlds
                .iter()
                .filter_map(|w| match w.built {
                    Err(Decline::NotAsked) | Ok(_) => None,
                    Err(why) => Some(format!("{:08x} {why:?}", w.form_id)),
                })
                .collect();
            if built > 0 || !searched.is_empty() {
                eprintln!(
                    "cell offsets: {}: {built} of {} world spaces given a table in {:.1} ms{}",
                    key.path.file_name().unwrap_or_default().to_string_lossy(),
                    worlds.len(),
                    start.elapsed().as_secs_f64() * 1000.0,
                    if searched.is_empty() {
                        String::new()
                    } else {
                        format!("; searched as before: {}", searched.join(", "))
                    }
                );
            }
            Some(Arc::new(worlds))
        }
        Err(why) => {
            eprintln!("cell offsets: {}: not read ({why})", key.path.display());
            None
        }
    }
}

/// The host path of a `TESFile`: its `m_Path` and `m_Filename` as the game opens them.
fn plugin_path(e: &Engine, file: Ptr) -> PathBuf {
    let mut name = e.mem.cstr(file.addr() + FILE_PATH);
    if !name.is_empty() && !name.ends_with(b"\\") && !name.ends_with(b"/") {
        name.push(b'\\');
    }
    name.extend(e.mem.cstr(file.addr() + FILE_NAME));
    let full = engine_platform::files::full_path(e, &name);
    PathBuf::from(String::from_utf8_lossy(&full).into_owned())
}

/// Starts building every plugin of the load order on a host thread, once per run (the first
/// world space record loaded): by the time a later plugin's records load, its tables are made.
fn start_building(e: &Engine) {
    static STARTED: Mutex<Option<usize>> = Mutex::new(None);
    {
        let mut started = STARTED.lock().unwrap_or_else(|p| p.into_inner());
        if *started == Some(e.kernel.id()) {
            return;
        }
        *started = Some(e.kernel.id());
    }
    let handler = e.mem.u32(DATA_HANDLER);
    if handler == 0 {
        return;
    }
    let count = e.mem.u32(handler + COMPILE_COUNT).min(0xff);
    let paths: Vec<PathBuf> = (0..count)
        .map(|i| e.mem.u32(handler + FILE_INDEX + 4 * i))
        .filter(|&file| file != 0)
        .map(|file| plugin_path(e, Ptr::new(file)))
        .collect();
    let _ = std::thread::Builder::new()
        .name("cell offsets".into())
        .spawn(move || {
            for path in paths {
                tables_of(path);
            }
        });
}

/// `TESWorldSpace::Load` with the row: the translation, then the record's table.
pub fn load(e: &mut Engine, this: Ptr<TESWorldSpace>, file: Ptr) -> bool {
    let offset = e.mem.u32(file.addr() + FILE_OFFSET);
    let loaded = fn_00583560(e, this, file);
    if loaded && !file.is_null() {
        start_building(e);
        let master = e.call(FILE_IS_MASTER, &args![file]).bool();
        let data: Ptr<OffsetData> = e.call(OFFSET_DATA_OF_FILE, &args![this, file]).ptr();
        // A master with its own table keeps it: the translation reads that one.
        let has_table = !data.is_null() && !e.get(data, OffsetData::pCellFileOffsets).is_null();
        if !(master && has_table) {
            if let Some(worlds) = tables_of(plugin_path(e, file)) {
                if let Some(world) = worlds.iter().find(|w| w.record_offset == offset) {
                    give_table(e, this, file, master, world);
                }
            }
        }
    }
    loaded
}

/// Gives the world space record `world` of `file` its built table (module documentation).
pub fn give_table(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    file: Ptr,
    master: bool,
    world: &WorldOffsets,
) {
    let Ok(table) = &world.built else { return };
    let (Some(min), Some(max)) = (world.min_corner, world.max_corner) else {
        return;
    };
    let corners = [min[0], min[1], max[0], max[1]];
    let data: Ptr<OffsetData> = if master {
        // The translation stored this record's position and corners: they must be the ones the
        // table was built for.
        let data: Ptr<OffsetData> = e.call(OFFSET_DATA_OF_FILE, &args![this, file]).ptr();
        if data.is_null()
            || !e.get(data, OffsetData::pCellFileOffsets).is_null()
            || e.get(data, OffsetData::iFileOffset) != world.record_offset
            || (0..4).any(|i| e.mem.u32(data.addr() + 4 + 4 * i) != corners[i as usize])
        {
            return;
        }
        data
    } else {
        let data: Ptr<OffsetData> = e.call(CREATE_OFFSET_DATA, &args![this, file]).ptr();
        if data.is_null() {
            return;
        }
        let old = e.get(data, OffsetData::pCellFileOffsets);
        if !old.is_null() {
            e.call(MEMORY_FREE, &args![old]);
            e.set(data, OffsetData::pCellFileOffsets, Ptr::NULL);
        }
        for (i, bits) in corners.iter().enumerate() {
            e.mem.set_u32(data.addr() + 4 + 4 * i as u32, *bits);
        }
        e.set(data, OffsetData::iFileOffset, world.record_offset);
        data
    };
    let bytes: Vec<u8> = table.iter().flat_map(|o| o.to_le_bytes()).collect();
    let block = e.call(MEMORY_ALLOC, &args![bytes.len() as u32]).ptr();
    if block.is_null() {
        return;
    }
    e.mem.write(block.addr(), &bytes);
    e.set(data, OffsetData::pCellFileOffsets, block);
}

/// `TESWorldSpace::FindCellInFile` with the row: a file that is not a master is found by its
/// table when this fix gave it one, as the translation finds a master's; else the translation.
pub fn find_cell_in_file(
    e: &mut Engine,
    this: Ptr<TESWorldSpace>,
    file: Ptr,
    x: i32,
    y: i32,
) -> bool {
    if !file.is_null() && !e.call(FILE_IS_MASTER, &args![file]).bool() {
        let data: Ptr<OffsetData> = e.call(OFFSET_DATA_OF_FILE, &args![this, file]).ptr();
        if !data.is_null()
            && !e.get(data, OffsetData::pCellFileOffsets).is_null()
            && e.get(data, OffsetData::iFileOffset) != 0
        {
            let index = e.call(CELL_INDEX, &args![this, file, x, y]).i32();
            if index == -1 {
                return false;
            }
            let table = e.get(data, OffsetData::pCellFileOffsets).addr();
            let entry = e
                .mem
                .u32(table.wrapping_add((index as u32).wrapping_mul(4)));
            if entry == 0 {
                return false;
            }
            let base = e.get(data, OffsetData::iFileOffset);
            e.call(FILE_SET_OFFSET, &args![file, entry.wrapping_add(base)]);
            return true;
        }
    }
    tes_world_space_find_cell_in_file(e, this, file, x, y)
}

/// Registers both.
pub fn install(e: &mut Engine) {
    e.register(LOAD, |e, a| {
        load(e, Ptr::new(a[0]), Ptr::new(a[1])).into_ret()
    });
    e.register(FIND_CELL_IN_FILE, |e, a| {
        find_cell_in_file(e, Ptr::new(a[0]), Ptr::new(a[1]), a[2] as i32, a[3] as i32).into_ret()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The test's own words, in the mapped data area: the `OFFSET_DATA` the world space has for the
    /// file (0: none), the block `CreateOffsetData` makes it in, the last `SetOffset` and their
    /// count, the last block freed and the searches started (`FindForm`).
    const DATA: u32 = 0x011f_f000;
    const DATA_BLOCK: u32 = 0x011f_f004;
    const SET_OFFSET: u32 = 0x011f_f008;
    const SET_OFFSET_COUNT: u32 = 0x011f_f00c;
    const FREED: u32 = 0x011f_f010;
    const SEARCHES: u32 = 0x011f_f014;
    /// `TESFile::FindForm` (`004734d0`): where the translation's search starts.
    const FILE_FIND_FORM: u32 = 0x0047_34d0;
    /// `TESFile::m_Flags` (bit 0: master).
    const FILE_FLAGS: u32 = 0x3e8;
    const RECORD: u32 = 0x9402ce;
    /// A 4 x 4 grid from (-2, -2) to (1, 1): the corners' bits.
    const MIN: [u32; 2] = [0xc600_0000, 0xc600_0000];
    const MAX: [u32; 2] = [0x45ff_f800, 0x45ff_f800];

    struct World {
        e: Box<Engine>,
        world: Ptr<TESWorldSpace>,
        file: Ptr,
    }

    /// A world space, a file (a master when `master`), and stand-ins for the offset data map, the
    /// game's heap, `SetOffset` and `FindForm` (which finds nothing). `fix` installs the row.
    fn world(master: bool, fix: bool) -> World {
        let mut e = Box::new(Engine::new());
        e.map(0x011c_0000, 0x6_0000);
        let world = Ptr::new(e.mem.alloc(0x100));
        let file = Ptr::new(e.mem.alloc(0x430));
        e.mem.set_u32(file.addr() + FILE_FLAGS, u32::from(master));
        let block = e.mem.alloc(0x18);
        e.mem.set_u32(DATA_BLOCK, block);
        e.register(FILE_IS_MASTER, |e, a| {
            (e.mem.u32(a[0] + FILE_FLAGS) & 1 != 0).into_ret()
        });
        e.register(OFFSET_DATA_OF_FILE, |e, _| e.mem.u32(DATA).into_ret());
        e.register(CREATE_OFFSET_DATA, |e, _| {
            if e.mem.u32(DATA) == 0 {
                let data = e.mem.u32(DATA_BLOCK);
                e.mem.set_u32(data, 0);
                for (i, bits) in [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX]
                    .iter()
                    .enumerate()
                {
                    e.mem.set_u32(data + 4 + 4 * i as u32, bits.to_bits());
                }
                e.mem.set_u32(data + 0x14, 0);
                e.mem.set_u32(DATA, data);
            }
            e.mem.u32(DATA).into_ret()
        });
        e.register(MEMORY_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMORY_FREE, |e, a| {
            e.mem.set_u32(FREED, a[0]);
            Ret::default()
        });
        e.register(FILE_SET_OFFSET, |e, a| {
            e.mem.set_u32(SET_OFFSET, a[1]);
            let count = e.mem.u32(SET_OFFSET_COUNT) + 1;
            e.mem.set_u32(SET_OFFSET_COUNT, count);
            true.into_ret()
        });
        e.register(FILE_FIND_FORM, |e, _| {
            let count = e.mem.u32(SEARCHES) + 1;
            e.mem.set_u32(SEARCHES, count);
            false.into_ret()
        });
        if fix {
            install(&mut e);
        }
        World { e, world, file }
    }

    /// A world space record at [`RECORD`] over the 4 x 4 grid with the cell (0, 0) 0x40 bytes on
    /// and (-2, 1) 0x80 bytes on.
    fn built() -> WorldOffsets {
        let mut table = vec![0; 16];
        table[esm::cell_offsets::grid_index((-2, -2, 1, 1), 0, 0).unwrap()] = 0x40;
        table[esm::cell_offsets::grid_index((-2, -2, 1, 1), -2, 1).unwrap()] = 0x80;
        WorldOffsets {
            record_offset: RECORD,
            form_id: 0xda726,
            flags: 0,
            min_corner: Some(MIN),
            max_corner: Some(MAX),
            stored: None,
            built: Ok(table),
        }
    }

    fn find(w: &mut World, x: i32, y: i32) -> bool {
        w.e.call(FIND_CELL_IN_FILE, &args![w.world, w.file, x, y])
            .bool()
    }

    fn data(w: &World) -> Ptr<OffsetData> {
        Ptr::new(w.e.mem.u32(DATA))
    }

    /// The master's `OFFSET_DATA` as `Load` leaves it for a record without `OFST`.
    fn master_data(w: &mut World, corners: [u32; 4]) {
        let data = w.e.call(CREATE_OFFSET_DATA, &args![w.world, w.file]).u32();
        for (i, bits) in corners.iter().enumerate() {
            w.e.mem.set_u32(data + 4 + 4 * i as u32, *bits);
        }
        w.e.mem.set_u32(data + 0x14, RECORD);
    }

    #[test]
    fn a_plugin_cell_is_found_by_the_table_the_row_gave() {
        let mut w = world(false, true);
        give_table(&mut w.e, w.world, w.file, false, &built());
        let data = data(&w);
        assert_eq!(w.e.get(data, OffsetData::iFileOffset), RECORD);
        let corners: Vec<u32> = (0..4)
            .map(|i| w.e.mem.u32(data.addr() + 4 + 4 * i))
            .collect();
        assert_eq!(corners, [MIN[0], MIN[1], MAX[0], MAX[1]]);
        assert!(find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SET_OFFSET), RECORD + 0x40);
        assert!(find(&mut w, -2, 1));
        assert_eq!(w.e.mem.u32(SET_OFFSET), RECORD + 0x80);
        // No cell there, and outside the grid: not found, and the file is not searched.
        assert!(!find(&mut w, 1, 1));
        assert!(!find(&mut w, 5, 0));
        assert_eq!(w.e.mem.u32(SET_OFFSET_COUNT), 2);
        assert_eq!(w.e.mem.u32(SEARCHES), 0);
    }

    #[test]
    fn vanilla_searches_a_plugin_whatever_its_offset_data_holds() {
        // The same offset data, the row not installed: the translation searches the file.
        let mut w = world(false, false);
        give_table(&mut w.e, w.world, w.file, false, &built());
        assert!(!find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SEARCHES), 1);
        assert_eq!(w.e.mem.u32(SET_OFFSET_COUNT), 0);
    }

    #[test]
    fn a_plugin_table_the_row_did_not_give_is_not_used() {
        // An `OFST` a plugin carries is read into its offset data with no position: the original
        // never uses it, and neither does the row.
        let mut w = world(false, true);
        let data = w.e.call(CREATE_OFFSET_DATA, &args![w.world, w.file]).u32();
        let stale = w.e.mem.alloc(64);
        w.e.mem.set_u32(stale + 4 * 10, 0x40);
        w.e.mem.set_u32(data, stale);
        assert!(!find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SEARCHES), 1);
        // Given the row's table, the stale one is freed and replaced.
        give_table(&mut w.e, w.world, w.file, false, &built());
        assert_eq!(w.e.mem.u32(FREED), stale);
        assert_ne!(
            w.e.get(Ptr::<OffsetData>::new(data), OffsetData::pCellFileOffsets)
                .addr(),
            stale
        );
        assert!(find(&mut w, 0, 0));
    }

    #[test]
    fn a_master_without_a_table_gains_one_and_the_translation_uses_it() {
        let mut w = world(true, true);
        master_data(&mut w, [MIN[0], MIN[1], MAX[0], MAX[1]]);
        give_table(&mut w.e, w.world, w.file, true, &built());
        assert!(!w.e.get(data(&w), OffsetData::pCellFileOffsets).is_null());
        assert!(find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SET_OFFSET), RECORD + 0x40);
        assert_eq!(w.e.mem.u32(SEARCHES), 0);
    }

    #[test]
    fn a_master_keeps_its_own_table_and_one_built_for_other_corners_is_not_given() {
        let mut w = world(true, true);
        master_data(&mut w, [MIN[0], MIN[1], MAX[0], MAX[1]]);
        let own = w.e.mem.alloc(64);
        w.e.set(data(&w), OffsetData::pCellFileOffsets, Ptr::new(own));
        give_table(&mut w.e, w.world, w.file, true, &built());
        assert_eq!(w.e.get(data(&w), OffsetData::pCellFileOffsets).addr(), own);

        let mut w = world(true, true);
        master_data(&mut w, [MIN[0], MIN[1], MAX[0], 0x4600_0000]);
        give_table(&mut w.e, w.world, w.file, true, &built());
        assert!(w.e.get(data(&w), OffsetData::pCellFileOffsets).is_null());
        assert!(!find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SEARCHES), 1);
    }

    #[test]
    fn a_world_space_without_a_built_table_is_left_as_it_was() {
        let mut w = world(false, true);
        let mut declined = built();
        declined.built = Err(Decline::CellOutsideGrid);
        give_table(&mut w.e, w.world, w.file, false, &declined);
        assert_eq!(w.e.mem.u32(DATA), 0);
        assert!(!find(&mut w, 0, 0));
        assert_eq!(w.e.mem.u32(SEARCHES), 1);
    }

    fn sub(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        [&tag[..], &(data.len() as u16).to_le_bytes(), data].concat()
    }

    fn record(tag: &[u8; 4], flags: u32, form_id: u32, data: &[u8]) -> Vec<u8> {
        let size = (data.len() as u32).to_le_bytes();
        [
            &tag[..],
            &size,
            &flags.to_le_bytes(),
            &form_id.to_le_bytes(),
            &[0; 8],
            data,
        ]
        .concat()
    }

    fn group(label: u32, kind: u32, content: &[u8]) -> Vec<u8> {
        let size = (24 + content.len() as u32).to_le_bytes();
        [
            &b"GRUP"[..],
            &size,
            &label.to_le_bytes(),
            &kind.to_le_bytes(),
            &[0; 8],
            content,
        ]
        .concat()
    }

    fn corners(c: [u32; 2]) -> Vec<u8> {
        [c[0].to_le_bytes(), c[1].to_le_bytes()].concat()
    }

    /// An exterior cell record at (`x`, `y`).
    fn cell_at(form_id: u32, x: i32, y: i32) -> Vec<u8> {
        let grid: Vec<u8> = [x, y, 0].iter().flat_map(|v| v.to_le_bytes()).collect();
        record(b"CELL", 0, form_id, &sub(b"XCLC", &grid))
    }

    /// Where `needle` starts in `bytes`.
    fn find_in(bytes: &[u8], needle: &[u8]) -> u32 {
        bytes
            .windows(needle.len())
            .position(|x| x == needle)
            .unwrap() as u32
    }

    /// Writes `bytes` as `Data\name` in a folder of the test's own (`tag`), and points `file`'s
    /// `m_Path` and `m_Filename` at it as the game keeps them. Returns the folder.
    fn plugin_file(e: &mut Engine, file: Ptr, tag: &str, name: &str, bytes: &[u8]) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nv-cell-offsets-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(dir.join("Data")).unwrap();
        std::fs::write(dir.join("Data").join(name), bytes).unwrap();
        let folder = format!("{}\\Data\\", dir.display());
        e.mem.set_cstr(file.addr() + FILE_PATH, folder.as_bytes());
        e.mem.set_cstr(file.addr() + FILE_NAME, name.as_bytes());
        dir
    }

    #[test]
    fn a_plugins_tables_are_read_from_its_file() {
        // A plugin with one world space record (no `OFST`) and one cell, in a Data folder of its
        // own; the file's path as the game keeps it in the `TESFile`.
        let wrld = record(
            b"WRLD",
            0,
            0x50,
            &[sub(b"NAM0", &corners(MIN)), sub(b"NAM9", &corners(MAX))].concat(),
        );
        let cell = cell_at(0x60, 0, 0);
        let children = group(0x50, 1, &cell);
        let bytes = [
            record(b"TES4", 0, 0, &sub(b"HEDR", &[0; 12])),
            group(
                u32::from_le_bytes(*b"WRLD"),
                0,
                &[wrld.clone(), children].concat(),
            ),
        ]
        .concat();
        let mut w = world(false, true);
        let dir = plugin_file(&mut w.e, w.file, "read", "Test.esp", &bytes);
        let path = plugin_path(&w.e, w.file);
        let worlds = tables_of(path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(worlds.len(), 1);
        let world = &worlds[0];
        assert_eq!(world.record_offset, find_in(&bytes, &wrld));
        let table = world.built.as_ref().unwrap();
        let index = esm::cell_offsets::grid_index((-2, -2, 1, 1), 0, 0).unwrap();
        assert_eq!(
            table[index],
            find_in(&bytes, &cell) - find_in(&bytes, &wrld)
        );
    }

    // ---- `Load` through its exe address, with the translation ----------------------------

    /// `TESFile::GetRecordType` (`00472660`) and the chunk functions `Load` reads its record
    /// with: the current chunk's tag (`004726b0`), the step to the next (`004726f0`), its size
    /// (`00401660`), its data (`00472890`), whether the file is byte-swapped (`00401680`).
    const FILE_RECORD_TYPE: u32 = 0x0047_2660;
    const FILE_NEXT_CHUNK: u32 = 0x0047_26b0;
    const FILE_ADVANCE_CHUNK: u32 = 0x0047_26f0;
    const FILE_CHUNK_SIZE: u32 = 0x0040_1660;
    const FILE_READ_CHUNK: u32 = 0x0047_2890;
    const FILE_NEEDS_SWAP: u32 = 0x0040_1680;
    /// `TESForm::Load` and the form's state setter (`00485110`, `00484ab0`).
    const FORM_LOAD: u32 = 0x0048_5110;
    const FORM_SET_STATE: u32 = 0x0048_4ab0;

    /// A record's chunks as `TESFile`'s chunk functions serve them, and the one they are on.
    #[derive(Default)]
    struct Chunks {
        list: Vec<([u8; 4], Vec<u8>)>,
        at: usize,
    }

    /// An engine for `Load` with the doubles of [`world`] (the file a master when `master`, the
    /// row installed when `fix`), but an `OFFSET_DATA` for each world space (`CreateOffsetData`
    /// makes one with the defaults, the map finds it or gives 0), the record's chunks from
    /// [`Chunks`], and `Load`'s other callees doing nothing. `TESFile::m_fileoffset` (+0x264) is
    /// set for each record loaded; the translation's own `FindForm` and `SetOffset` are counted.
    struct Loader {
        e: Box<Engine>,
        file: Ptr,
        chunks: Rc<RefCell<Chunks>>,
        made: Rc<RefCell<HashMap<u32, u32>>>,
    }

    fn loader(master: bool, fix: bool) -> Loader {
        let World { mut e, file, .. } = world(master, fix);
        let made: Rc<RefCell<HashMap<u32, u32>>> = Rc::default();
        let m = made.clone();
        e.register_double(OFFSET_DATA_OF_FILE, move |_, a| {
            m.borrow().get(&a[0]).copied().unwrap_or(0).into_ret()
        });
        let m = made.clone();
        e.register_double(CREATE_OFFSET_DATA, move |e, a| {
            let mut m = m.borrow_mut();
            let data = *m.entry(a[0]).or_insert_with(|| {
                let data = e.mem.alloc(0x18);
                for (i, bits) in [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX]
                    .iter()
                    .enumerate()
                {
                    e.mem.set_u32(data + 4 + 4 * i as u32, bits.to_bits());
                }
                data
            });
            data.into_ret()
        });
        let chunks: Rc<RefCell<Chunks>> = Rc::default();
        let c = chunks.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let c = c.borrow();
            c.list
                .get(c.at)
                .map_or(0, |(tag, _)| u32::from_le_bytes(*tag))
                .into_ret()
        });
        let c = chunks.clone();
        e.register_double(FILE_ADVANCE_CHUNK, move |_, _| {
            let mut c = c.borrow_mut();
            c.at += 1;
            (c.at < c.list.len()).into_ret()
        });
        let c = chunks.clone();
        e.register_double(FILE_CHUNK_SIZE, move |_, _| {
            let c = c.borrow();
            (c.list[c.at].1.len() as u32).into_ret()
        });
        let c = chunks.clone();
        e.register_double(FILE_READ_CHUNK, move |e, a| {
            let c = c.borrow();
            let data = &c.list[c.at].1;
            // A size of 0: the whole chunk.
            let n = if a[2] == 0 {
                data.len()
            } else {
                (a[2] as usize).min(data.len())
            };
            e.mem.write(a[1], &data[..n]);
            (n as u32).into_ret()
        });
        e.register(FILE_NEEDS_SWAP, |_, _| false.into_ret());
        e.register(FILE_RECORD_TYPE, |_, _| 0x41u32.into_ret());
        e.register(FORM_LOAD, |_, _| Ret::default());
        e.register(FORM_SET_STATE, |_, _| Ret::default());
        Loader {
            e,
            file,
            chunks,
            made,
        }
    }

    impl Loader {
        /// A world space object whose cell map holds a cell (so `Load` keeps the map).
        fn world_space(&mut self) -> Ptr<TESWorldSpace> {
            let world: Ptr<TESWorldSpace> = self.e.new_object();
            let map = self.e.mem.alloc(0x10);
            self.e.mem.set_u32(map + 0xc, 1);
            self.e.set(world, TESWorldSpace::pCellMap, Ptr::new(map));
            world
        }

        /// `Load` called at its exe address, as the game calls it, on the record at `offset`
        /// (the file's `m_fileoffset`) with its `chunks`, into `world`.
        fn load(&mut self, world: Ptr<TESWorldSpace>, offset: u32, chunks: &[Chunk]) -> bool {
            self.e.mem.set_u32(self.file.addr() + FILE_OFFSET, offset);
            *self.chunks.borrow_mut() = Chunks {
                list: chunks.to_vec(),
                at: 0,
            };
            self.e.call(LOAD, &args![world, self.file]).bool()
        }

        /// `world`'s `OFFSET_DATA` for the file (null: none).
        fn data(&self, world: Ptr<TESWorldSpace>) -> Ptr<OffsetData> {
            Ptr::new(self.made.borrow().get(&world.addr()).copied().unwrap_or(0))
        }

        /// The position, the corners' bits and the first `len` table entries of `world`'s
        /// `OFFSET_DATA` (no entries without a table).
        fn offsets(&self, world: Ptr<TESWorldSpace>, len: u32) -> (u32, Vec<u32>, Vec<u32>) {
            let data = self.data(world);
            assert!(!data.is_null(), "an OFFSET_DATA");
            let corners = (0..4)
                .map(|i| self.e.mem.u32(data.addr() + 4 + 4 * i))
                .collect();
            let table = self.e.get(data, OffsetData::pCellFileOffsets);
            let entries = if table.is_null() {
                Vec::new()
            } else {
                (0..len)
                    .map(|i| self.e.mem.u32(table.addr() + 4 * i))
                    .collect()
            };
            (self.e.get(data, OffsetData::iFileOffset), corners, entries)
        }

        fn find(&mut self, world: Ptr<TESWorldSpace>, x: i32, y: i32) -> bool {
            self.e
                .call(FIND_CELL_IN_FILE, &args![world, self.file, x, y])
                .bool()
        }
    }

    type Chunk = ([u8; 4], Vec<u8>);

    /// The second world space's corners: the cells (0, 0) to (3, 3).
    fn second_corners() -> ([u32; 2], [u32; 2]) {
        let at = |c: f32| (c * 4096.0).to_bits();
        ([at(0.0), at(0.0)], [at(3.0), at(3.0)])
    }

    /// A file (a master when `master`) with two world space records over different grids, each
    /// followed by its cells: 0x50 over [`MIN`] to [`MAX`] with cells at (0, 0) and (-2, 1),
    /// 0x51 over [`second_corners`] with a cell at (1, 1) and `own` as its `OFST` when given.
    struct TwoWorlds {
        bytes: Vec<u8>,
        records: [Vec<u8>; 2],
        cells: [Vec<u8>; 3],
        chunks: [Vec<Chunk>; 2],
    }

    impl TwoWorlds {
        fn new(master: bool, own: Option<&[u8]>) -> TwoWorlds {
            let (second_min, second_max) = second_corners();
            let mut chunks = [
                vec![(*b"NAM0", corners(MIN)), (*b"NAM9", corners(MAX))],
                vec![
                    (*b"NAM0", corners(second_min)),
                    (*b"NAM9", corners(second_max)),
                ],
            ];
            if let Some(own) = own {
                chunks[1].push((*b"OFST", own.to_vec()));
            }
            let records = [0, 1].map(|i| {
                let data: Vec<u8> = chunks[i]
                    .iter()
                    .flat_map(|(tag, data)| sub(tag, data))
                    .collect();
                record(b"WRLD", 0, 0x50 + i as u32, &data)
            });
            let cells = [
                cell_at(0x60, 0, 0),
                cell_at(0x61, -2, 1),
                cell_at(0x62, 1, 1),
            ];
            let content = [
                records[0].clone(),
                group(0x50, 1, &[cells[0].clone(), cells[1].clone()].concat()),
                records[1].clone(),
                group(0x51, 1, &cells[2]),
            ]
            .concat();
            let bytes = [
                record(b"TES4", u32::from(master), 0, &sub(b"HEDR", &[0; 12])),
                group(u32::from_le_bytes(*b"WRLD"), 0, &content),
            ]
            .concat();
            TwoWorlds {
                bytes,
                records,
                cells,
                chunks,
            }
        }

        fn record(&self, i: usize) -> u32 {
            find_in(&self.bytes, &self.records[i])
        }

        fn cell(&self, i: usize) -> u32 {
            find_in(&self.bytes, &self.cells[i])
        }

        /// The first world space's table: its two cells, from its record.
        fn first_table(&self) -> Vec<u32> {
            let mut table = vec![0; 16];
            let slot = |x, y| esm::cell_offsets::grid_index((-2, -2, 1, 1), x, y).unwrap();
            table[slot(0, 0)] = self.cell(0) - self.record(0);
            table[slot(-2, 1)] = self.cell(1) - self.record(0);
            table
        }

        /// The second's: its one cell.
        fn second_table(&self) -> Vec<u32> {
            let mut table = vec![0; 16];
            let slot = esm::cell_offsets::grid_index((0, 0, 3, 3), 1, 1).unwrap();
            table[slot] = self.cell(2) - self.record(1);
            table
        }
    }

    #[test]
    fn load_gives_each_world_space_of_a_plugin_the_table_of_the_record_it_reads() {
        // The second record carries an `OFST`, which the translation reads for a plugin too but
        // never uses: the row's table replaces it.
        let stale: Vec<u8> = (0..16u32)
            .flat_map(|i| (0x2000 + i).to_le_bytes())
            .collect();
        let two = TwoWorlds::new(false, Some(&stale));
        let mut l = loader(false, true);
        let dir = plugin_file(&mut l.e, l.file, "load-plugin", "Two.esp", &two.bytes);
        let (first, second) = (l.world_space(), l.world_space());
        // The second record first: each world space gets the table of the record the file is on.
        assert!(l.load(second, two.record(1), &two.chunks[1]));
        assert_ne!(l.e.mem.u32(FREED), 0, "the stale table freed");
        assert!(l.load(first, two.record(0), &two.chunks[0]));
        std::fs::remove_dir_all(&dir).unwrap();

        let (second_min, second_max) = second_corners();
        assert_eq!(
            l.offsets(first, 16),
            (
                two.record(0),
                vec![MIN[0], MIN[1], MAX[0], MAX[1]],
                two.first_table()
            )
        );
        assert_eq!(
            l.offsets(second, 16),
            (
                two.record(1),
                vec![second_min[0], second_min[1], second_max[0], second_max[1]],
                two.second_table()
            )
        );
        // Each cell found by its world space's table; the file never searched.
        assert!(l.find(first, -2, 1));
        assert_eq!(l.e.mem.u32(SET_OFFSET), two.cell(1));
        assert!(l.find(second, 1, 1));
        assert_eq!(l.e.mem.u32(SET_OFFSET), two.cell(2));
        assert!(!l.find(first, 1, 1));
        assert_eq!(l.e.mem.u32(SET_OFFSET_COUNT), 2);
        assert_eq!(l.e.mem.u32(SEARCHES), 0);

        // Vanilla: the same load gives a plugin no offset data, and its cells are searched for.
        let mut l = loader(false, false);
        let first = l.world_space();
        assert!(l.load(first, two.record(0), &two.chunks[0]));
        assert!(l.data(first).is_null());
        assert!(!l.find(first, 0, 0));
        assert_eq!(l.e.mem.u32(SEARCHES), 1);
    }

    #[test]
    fn load_gives_a_masters_record_without_offsets_its_table_and_keeps_a_masters_own() {
        let own: Vec<u8> = (0..16u32)
            .flat_map(|i| (0x1000 + i).to_le_bytes())
            .collect();
        let two = TwoWorlds::new(true, Some(&own));
        let mut l = loader(true, true);
        let dir = plugin_file(&mut l.e, l.file, "load-master", "Two.esm", &two.bytes);
        let (first, second) = (l.world_space(), l.world_space());
        assert!(l.load(first, two.record(0), &two.chunks[0]));
        assert!(l.load(second, two.record(1), &two.chunks[1]));
        std::fs::remove_dir_all(&dir).unwrap();

        // The first: the position and corners the translation stored, and the built table.
        assert_eq!(
            l.offsets(first, 16),
            (
                two.record(0),
                vec![MIN[0], MIN[1], MAX[0], MAX[1]],
                two.first_table()
            )
        );
        // The second: the master's own table, as the translation read it.
        let (position, _, table) = l.offsets(second, 16);
        assert_eq!(position, two.record(1));
        assert_eq!(table, (0..16).map(|i| 0x1000 + i).collect::<Vec<u32>>());
        // The translation finds the first's cell by the table it was given.
        assert!(l.find(first, -2, 1));
        assert_eq!(l.e.mem.u32(SET_OFFSET), two.cell(1));
        assert_eq!(l.e.mem.u32(SEARCHES), 0);
    }
}
