//! UI Organizer's two places in the game (its patches at `0070A049` and `00A01B87`) and the game's
//! calls it makes: the files read through the game's own Windows calls (KERNEL32's import slots,
//! the platform layer every file the game reads goes through, as the built-in xNVSE's file
//! commands read them), a menu file's text through `Menu::CompressAndStoreXML`, the parse through
//! `Tile::ParseFile`.

use crate::lists::{self, Files};
use crate::patch;
use crate::reader::Reader;
use crate::Uio;
use engine_core::prelude::*;

/// KERNEL32's import slots the game's own file code calls.
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const FIND_FIRST_FILE_A: u32 = 0x00fd_f070;
const FIND_NEXT_FILE_A: u32 = 0x00fd_f068;
const FIND_CLOSE: u32 = 0x00fd_f064;
const GET_FILE_ATTRIBUTES_A: u32 = 0x00fd_f160;
const GENERIC_READ: u32 = 0x8000_0000;
/// `fopen`'s share mode (`_SH_DENYNO`: read and write).
const SHARE_READ_WRITE: u32 = 3;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID: u32 = 0xffff_ffff;
/// `WIN32_FIND_DATAA`: its size and where `cFileName` is.
const FIND_DATA_SIZE: u32 = 0x140;
const FIND_DATA_NAME: u32 = 0x2c;
/// The game's folder for `plugins.txt` (a string the game fills at start-up), which UIO reads.
const PLUGINS_FOLDER: u32 = 0x0120_2e98;

/// `Menu::CompressAndStoreXML(path, text)` (`cdecl`), UIO's `ADDR_GetXMLFileData`.
pub const COMPRESS_AND_STORE_XML: u32 = 0x00a1_ce70;
/// `Tile::ParseFile(path, buffer, length)` (`cdecl`), the call UIO's hook replaces.
pub const TILE_PARSE_FILE: u32 = 0x00a0_1e20;
/// The game's heap and its allocate and free (`thiscall`): UIO's `GAME_HEAP_ALLOC`/`FREE`.
const GAME_HEAP: u32 = 0x011f_6238;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;

/// The file system as UIO sees it, through the game's Windows calls.
pub struct GameFiles<'a>(pub &'a mut Engine);

impl Files for GameFiles<'_> {
    fn exists(&mut self, path: &[u8]) -> bool {
        let attributes = nvse::events::with_guest_text(self.0, path, |e, p| {
            e.call(GET_FILE_ATTRIBUTES_A, &args![p]).u32()
        });
        attributes != INVALID && attributes & FILE_ATTRIBUTE_DIRECTORY == 0
    }

    fn read(&mut self, path: &[u8], limit: usize) -> Option<Vec<u8>> {
        let e = &mut *self.0;
        let handle = nvse::events::with_guest_text(e, path, |e, p| {
            e.call(
                CREATE_FILE_A,
                &args![
                    p,
                    GENERIC_READ,
                    SHARE_READ_WRITE,
                    0u32,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    0u32
                ],
            )
            .u32()
        });
        if handle == INVALID {
            return None;
        }
        let mut out = Vec::new();
        let mut fits = true;
        e.with_stack(0x1004, |e, frame| {
            let (buf, read) = (frame.addr(), frame.addr() + 0x1000);
            loop {
                e.mem.set_u32(read, 0);
                let ok = e
                    .call(READ_FILE, &args![handle, buf, 0x1000u32, read, 0u32])
                    .u32();
                let n = e.mem.u32(read).min(0x1000);
                if ok == 0 || n == 0 {
                    break;
                }
                if out.len() + n as usize > limit {
                    fits = false;
                    break;
                }
                out.extend(e.mem.bytes(buf, n));
            }
        });
        e.call(CLOSE_HANDLE, &args![handle]);
        fits.then_some(out)
    }

    fn list(&mut self, pattern: &[u8]) -> Vec<Vec<u8>> {
        let e = &mut *self.0;
        let mut out = Vec::new();
        e.with_stack(FIND_DATA_SIZE, |e, data| {
            let data = data.addr();
            let handle = nvse::events::with_guest_text(e, pattern, |e, p| {
                e.call(FIND_FIRST_FILE_A, &args![p, data]).u32()
            });
            if handle == INVALID {
                return;
            }
            loop {
                out.push(nvse::table::guest_text(e, data + FIND_DATA_NAME));
                if e.call(FIND_NEXT_FILE_A, &args![handle, data]).u32() == 0 {
                    break;
                }
            }
            e.call(FIND_CLOSE, &args![handle]);
        });
        out
    }

    fn plugins_folder(&mut self) -> Vec<u8> {
        nvse::table::guest_text(self.0, PLUGINS_FOLDER)
    }
}

/// `XMLFileData::Get(path)` (UIO): the game's text of a menu file (`CompressAndStoreXML(path,
/// NULL)`: the file from the archives or loose, its insignificant characters removed), freed as
/// UIO frees it (`XMLFileData::Destroy`: the text, then the record, on the game's heap); `None`
/// when the game finds no file.
pub fn game_text(e: &mut Engine, path: &[u8]) -> Option<Vec<u8>> {
    let storage = nvse::events::with_guest_text(e, path, |e, p| {
        e.call(COMPRESS_AND_STORE_XML, &args![p, 0u32]).u32()
    });
    if storage == 0 {
        return None;
    }
    let (length, data) = (e.mem.u32(storage), e.mem.u32(storage + 4));
    let text = if data == 0 {
        Vec::new()
    } else {
        e.mem.bytes(data, length)
    };
    e.call(HEAP_FREE, &args![GAME_HEAP, data]);
    e.call(HEAP_FREE, &args![GAME_HEAP, storage]);
    Some(text)
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: UIOLoad (its lists and patching; the trait
// names and code patches it also makes are UIO's later parts, not built yet).
/// The seam at `0070A049` (the new interface manager stored): UIO's lists read, and each file they
/// target read (`Menu::CompressAndStoreXML`) and patched in memory; a target the game cannot read
/// is passed over. Nothing is written to a file. The exe's path goes on.
pub fn uio_load(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let lists = lists::scan(&mut GameFiles(e));
    let mut patched = std::mem::take(&mut e.ext::<Uio>().patched);
    for (file, tiles) in lists.iter() {
        if let Some(text) = game_text(e, file) {
            patched.insert(file, patch::patch(&text, tiles.clone()));
        }
    }
    e.ext::<Uio>().patched = patched;
    e.ext::<Uio>().loaded = true;
    None
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: ResolveXMLFileHook (its reading; the parse
// is the game's `Tile::ParseFile` on UIO's text until UIO's own parser is built).
/// The seam at `00A01B87` (`Tile::ReadFile`'s call of `Tile::ParseFile`): the file's text from
/// UIO's reader ([`Reader::file`]) parsed by `Tile::ParseFile` from a buffer on the game's heap
/// (freed after). A file the game cannot find answers nothing, so the exe's call runs (UIO
/// answers no storage there, which `ReadFile` then reads through: a deliberate difference).
pub fn read_file(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let path_at = a[0];
    let path = nvse::table::guest_text(e, path_at);
    let mut patched = std::mem::take(&mut e.ext::<Uio>().patched);
    let text = {
        let mut raw = |p: &[u8]| game_text(e, p);
        Reader::new(&mut patched, &mut raw).file(&path)
    };
    e.ext::<Uio>().patched = patched;
    let text = text?;
    let length = u32::try_from(text.len()).ok()?;
    let buffer = e
        .call(HEAP_ALLOCATE, &args![GAME_HEAP, length.checked_add(1)?])
        .u32();
    if buffer == 0 {
        return None;
    }
    e.mem.write(buffer, &text);
    e.mem.set_u8(buffer + length, 0);
    let storage = e
        .call(TILE_PARSE_FILE, &args![path_at, buffer, length])
        .u32();
    e.call(HEAP_FREE, &args![GAME_HEAP, buffer]);
    Some(storage)
}
