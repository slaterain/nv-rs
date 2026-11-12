//! Direct file access (`NV_DIRECT_FILES`), a row of the `fixes` preset ([`crate::fixes`],
//! docs/FIXES.md, "Direct file access"): the file layer of Mod Limit Fix 7 and Cell Offset
//! Generator 1.08 (WallSoGB, GPL-3.0), their hook set built here from the exe, and JohnnyGuitar
//! NVSE's full archive offset range (LGPL-2.1; its `bFullOffsetRange`, on by default, which needs
//! Mod Limit Fix's 64-bit seeks), its patch sites built the same way; no code is taken from them.
//! In the `vanilla` preset nothing here runs.
//!
//! **The original.** The game reads its plugins, archives and loose files through `NiFile` (its
//! open `00aa14d0` and the functions after it) and `NiFile`'s subclass `BSFile` (`BSFile::Open`
//! `00aff300`, its size `00b004e0`). Both keep a C runtime `FILE` at `NiFile`'s +0x24 and reach
//! it with `fopen_s`, `fseek`, `ftell`, `fread`, `fwrite` and `fclose`, and every read, write and
//! seek goes through Obsidian's "Serialized IO Thread" (`OEI::IO`, started by `00aa7eb0`; the
//! wrappers `00aa85c0`, `00aa8610`, `00aa8660`): the caller queues a request, wakes that one
//! thread and waits on a semaphore until it has run the C function. So:
//! - the C runtime's 512 streams (`__initstdio`, `00ecdf4a`) are every file the game can hold
//!   open; the game keeps one on each plugin for each of its form-loading threads, and about 140
//!   plugins fill the table (Mod Limit Fix's author);
//! - every read, on any thread, waits for that one thread: two wake-ups a read, one at a time;
//! - `fseek`'s offset is a signed 32-bit number and `ftell` refuses a position past 2 GB, so a
//!   file larger than 2 GB is not read past that point and its size reads 0;
//! - an archive's file entry keeps the file's offset in the low 31 bits of its word +0xc and a
//!   mark in bit 31 (unused on PC: the check that a file has data compares it with the archive's
//!   headers-only bit, `00afb2a0`), so a file stored at 2 GB or more in an archive (TTW's and
//!   VNV's generated LOD archives grow past 2 GB) has no data.
//!
//! **The row.**
//! - `NiFile`'s open and `BSFile::Open` (its binary modes) open the file with the platform's
//!   `CreateFileA` as the C runtime's `fopen` of the same mode opens it (`__tsopen_nolock`,
//!   `00ee311d`: the same access and creation; `BSFile`'s mode 1 makes a missing file first, as
//!   the original does), and keep the platform's handle in `m_pFile`, tagged (below). No C
//!   runtime stream is taken.
//! - The IO thread's three wrappers run on the calling thread: a tagged handle is read, written
//!   and moved with `ReadFile`, `WriteFile` and `SetFilePointer`; a C runtime `FILE` goes straight
//!   to `fread`, `fwrite` and `fseek`, the calls the IO thread made for it. The IO thread is not
//!   started (`00aa7eb0`), and the game's stop of it at its end (`00aa81d0`) has nothing to stop.
//!   Nothing else waits on that thread: its semaphores, lock and lists (`011f6bf0`..`011f6c08`)
//!   are used by these wrappers and that stop only (the exe's references to them).
//! - `NiFile`'s seek (`00aa16d0`) moves a tagged handle with 64-bit positions and keeps the new
//!   one (its low 32 bits: the game's fields are 32-bit). A relative seek by 2 GB or more lands
//!   where the game's own 32-bit sum puts it (`seek_relative`): an archive's reads (`00afb7a0`)
//!   move by the difference of two 32-bit positions, so they reach every position of an archive
//!   of up to 4 GB, as JohnnyGuitar NVSE's stepped seeks at `00afb99b` do (with Mod Limit Fix's
//!   seek, which reads the word as signed, it splits the move instead). `NiFile`'s size
//!   (`00aa15f0`) and `BSFile`'s (`00b004e0`) come from `GetFileSizeEx`; `fclose` (`00ec9907`)
//!   closes a tagged handle with `CloseHandle`.
//! - The archive functions that read a file entry's offset word take all 32 bits of it, at
//!   JohnnyGuitar NVSE's sites: the check that a file has data (`00afb2a0`: the word nonzero), the
//!   getters of a file's object (`00afa550`, `00afa750`), the invalidations of a file that is
//!   loose on disk (`00afb190`, `00afac20`, `00afac90`, `00afad00`: the whole word cleared, since 0
//!   is what marks a file with no data), and the readers for a log line, a queued file's
//!   description and its queue key (`0043c3b0`, `00c3d0e0`, `00c3d440`). The variants live beside
//!   the translations (`engine-bs`, `engine-fallout-cell`), whose callers reach them by address.
//!
//! The platform's calls go through the exe's import slots, as the C runtime's own file layer
//! makes them, so with real threads they run under the kernel lock as before; each thread now
//! makes its own instead of handing it to the IO thread. An archive read by several threads takes
//! the archive's own lock around its seek and read (`00afb7a0`, the archive's +0x19c), as before.
//!
//! **The tag.** The platform's handles are multiples of 4, as Windows' are, and a C runtime
//! `FILE` is 4-aligned (an `_iob` entry or a heap block), so bit 0 of `m_pFile` tells them apart:
//! set, the rest is the platform's handle; bit 1 set too, every write goes to the file's end (the
//! append modes, the C runtime's `FAPPEND`). A `BSFile` opened in text mode (the game's own dumps:
//! the C runtime turns their line ends) and a `FILE` handed to a `BSFile` (its vtable +0x24) keep
//! the C runtime, through the same functions.
//!
//! **What differs from the original**, besides the time a read takes and the IO thread:
//! - no cap on the files the game holds open;
//! - a seek from the start reads its 32-bit position as unsigned (0 to 4 GB), where the
//!   original's `fseek` refuses one past 2 GB; positions and sizes past 2 GB are kept (the
//!   original's `ftell` answers -1 and its size 0); a file of 4 GB or more answers size 0, as the
//!   original answers for any file past 2 GB;
//! - a relative seek of 2 GB or more lands at the game's 32-bit sum (`seek_relative`), where the
//!   original's went the other way or was refused; one before the start of a file is refused as
//!   before, unless the sum is a position inside the file (only a file of over 2 GB has one);
//! - an archive's files stored at 2 GB or more have data and are read; bit 31 of an entry's
//!   offset word is no longer a mark (on PC no archive under 2 GB sets it);
//! - the C runtime's 4 KB buffer between `NiFile`'s own buffer and the file is gone: the same bytes
//!   reach the file and the reads, `NiFile`'s writes reach the platform at each of its flushes
//!   instead of every 4 KB;
//! - the C runtime's undefined cases come out as Windows' calls answer them, as with Mod Limit
//!   Fix 7: a read of a file opened only for writing answers 0 (the C runtime answered 0, or copied
//!   bytes out of its own write buffer), and a write right after a read through one file opened for
//!   update is written (the C runtime kept it in its read buffer and dropped it, or refused it).

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::bscore::oei_io_thread::{self, IO_THREAD};
use engine::units::bsmain::queuedfiles;
use engine::units::bssystem::archive::{self, Archive};
use engine::units::bssystem::bsfile::{self, BSFile};
use engine::units::crt;
use engine::units::fallout_shared::modelloader;
use engine::units::nisystem::run_00aa1380::{self as ni_file, NiFile};

/// `NiFile`'s open (its constructor with a path), size and seek (`NiSystem/run_00aa1380`).
pub const NI_FILE_OPEN: u32 = 0x00aa_14d0;
pub const NI_FILE_SIZE: u32 = 0x00aa_15f0;
pub const NI_FILE_SEEK: u32 = 0x00aa_16d0;
/// The IO thread's seek, read and write (`OEI::IO`, `BSCore/oei_io_thread`).
pub const IO_SEEK: u32 = 0x00aa_85c0;
pub const IO_READ: u32 = 0x00aa_8610;
pub const IO_WRITE: u32 = 0x00aa_8660;
/// The IO thread's start (`OEI::IO::initialize`) and stop.
pub const IO_START: u32 = 0x00aa_7eb0;
pub const IO_STOP: u32 = 0x00aa_81d0;
/// The C runtime's `fclose`.
pub const FCLOSE: u32 = 0x00ec_9907;
/// `BSFile::Open` and `BSFile`'s size (its vtable's +0x20 and +0x28).
pub const BS_FILE_OPEN: u32 = 0x00af_f300;
pub const BS_FILE_SIZE: u32 = 0x00b0_04e0;
/// The functions JohnnyGuitar NVSE's full offset range patches, which read an archive file
/// entry's offset word (+0xc) or clear it (`engine-bs`'s `bssystem::archive` and
/// `bsmain::queuedfiles`, `engine-fallout-cell`'s model loader): the check that a file has data,
/// the getters of a file object by its indices and by its entry, the three invalidations by name
/// (on disk, by hashes, in a directory) and the one at an archive's open, the offset's getter, a
/// queued file's description and its queue key.
pub const ARCHIVE_VALID_FILE: u32 = 0x00af_b2a0;
pub const ARCHIVE_FILE_BY_INDICES: u32 = 0x00af_a550;
pub const ARCHIVE_FILE_BY_ENTRY: u32 = 0x00af_a750;
pub const ARCHIVE_INVALIDATE_ON_DISK: u32 = 0x00af_b190;
pub const ARCHIVE_INVALIDATE_BY_HASHES: u32 = 0x00af_ac20;
pub const ARCHIVE_INVALIDATE_IN_DIRECTORY: u32 = 0x00af_ac90;
pub const ARCHIVE_INVALIDATE_AT_OPEN: u32 = 0x00af_ad00;
pub const FILE_ENTRY_OFFSET: u32 = 0x0043_c3b0;
pub const QUEUED_ENTRY_DESCRIPTION: u32 = 0x00c3_d0e0;
pub const QUEUED_ENTRY_KEY: u32 = 0x00c3_d440;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "direct file access",
    env: "NV_DIRECT_FILES",
    addresses: &[
        NI_FILE_OPEN,
        NI_FILE_SIZE,
        NI_FILE_SEEK,
        IO_SEEK,
        IO_READ,
        IO_WRITE,
        IO_START,
        IO_STOP,
        FCLOSE,
        BS_FILE_OPEN,
        BS_FILE_SIZE,
        ARCHIVE_VALID_FILE,
        ARCHIVE_FILE_BY_INDICES,
        ARCHIVE_FILE_BY_ENTRY,
        ARCHIVE_INVALIDATE_ON_DISK,
        ARCHIVE_INVALIDATE_BY_HASHES,
        ARCHIVE_INVALIDATE_IN_DIRECTORY,
        ARCHIVE_INVALIDATE_AT_OPEN,
        FILE_ENTRY_OFFSET,
        QUEUED_ENTRY_DESCRIPTION,
        QUEUED_ENTRY_KEY,
    ],
    summary: "the game's files are opened, read and written by the thread that asks, without the \
              file thread, the limit of 512 open files (about 140 plugins) or the 2 GB limit on a \
              file or an archive",
    install,
};

/// Registers the row's twenty-one functions.
pub fn install(e: &mut Engine) {
    let entries = [
        engine::entry!(
            0x00aa_14d0,
            ni_file_open(Ptr<NiFile>, Ptr, u32, u32) -> Ptr<NiFile>
        ),
        engine::entry!(0x00aa_15f0, ni_file_size(Ptr<NiFile>) -> u32),
        engine::entry!(0x00aa_16d0, ni_file_seek(Ptr<NiFile>, i32, i32)),
        engine::entry!(0x00aa_85c0, file_seek(u32, u32, u32) -> u32),
        engine::entry!(0x00aa_8610, file_read(u32, u32, u32, u32) -> u32),
        engine::entry!(0x00aa_8660, file_write(u32, u32, u32, u32) -> u32),
        engine::entry!(0x00aa_7eb0, io_start_skipped()),
        engine::entry!(0x00aa_81d0, io_stop_skipped()),
        engine::entry!(0x00ec_9907, fclose_direct(Ptr) -> i32),
        engine::entry!(
            0x00af_f300,
            bs_file_open_direct(Ptr<BSFile>, u32, bool) -> bool
        ),
        engine::entry!(0x00b0_04e0, bs_file_size_direct(Ptr<BSFile>) -> u32),
        engine::entry!(
            0x00af_b2a0,
            archive::fn_00afb2a0_full_offsets(Ptr<Archive>, Ptr, bool) -> bool
        ),
        engine::entry!(
            0x00af_a550,
            archive::fn_00afa550_full_offsets(Ptr<Archive>, u32, u32, u32, u32) -> u32
        ),
        engine::entry!(
            0x00af_a750,
            archive::fn_00afa750_full_offsets(Ptr<Archive>, Ptr, u32, u32) -> u32
        ),
        engine::entry!(
            0x00af_b190,
            archive::fn_00afb190_full_offsets(Ptr<Archive>, Ptr, Ptr) -> bool
        ),
        engine::entry!(
            0x00af_ac20,
            archive::fn_00afac20_full_offsets(Ptr<Archive>, Ptr, Ptr, u32) -> bool
        ),
        engine::entry!(
            0x00af_ac90,
            archive::fn_00afac90_full_offsets(Ptr<Archive>, u32, Ptr, u32) -> bool
        ),
        engine::entry!(
            0x00af_ad00,
            archive::fn_00afad00_full_offsets(Ptr<Archive>) -> u32
        ),
        engine::entry!(
            0x0043_c3b0,
            modelloader::fn_0043c3b0_full_offsets(Ptr) -> u32
        ),
        engine::entry!(
            0x00c3_d0e0,
            queuedfiles::queued_file_entry_get_description_full_offsets(u32, u32, u32, u32) -> bool
        ),
        engine::entry!(
            0x00c3_d440,
            queuedfiles::queued_file_entry_generate_key_full_offsets(u32)
        ),
    ];
    for (at, f) in entries {
        e.register(at, f);
    }
}

/// Bit 0 of `m_pFile`: the rest is the platform's handle of a file the row opened.
const HOST: u32 = 1;
/// Bit 1: every write goes to the file's end (`"ab"`, `"a+b"`).
const APPEND: u32 = 2;

/// The platform's handle in `m_pFile`, and whether its writes append; `None` for a C runtime
/// `FILE`, or none.
fn tagged(file: u32) -> Option<(u32, bool)> {
    (file & HOST != 0).then_some((file & !3, file & APPEND != 0))
}

/// The exe's import slots of the KERNEL32 calls the row makes (`stdcall`), as the C runtime's
/// file layer calls them.
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const WRITE_FILE: u32 = 0x00fd_f0dc;
const SET_FILE_POINTER: u32 = 0x00fd_f240;
const GET_FILE_SIZE_EX: u32 = 0x00fd_f13c;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const GET_LAST_ERROR: u32 = 0x00fd_f094;
/// `GENERIC_READ`, `GENERIC_WRITE` and both; the creations; `FILE_SHARE_READ`;
/// `FILE_ATTRIBUTE_NORMAL`; `INVALID_HANDLE_VALUE` and `INVALID_SET_FILE_POINTER`; the seek
/// origins.
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const READ_WRITE: u32 = GENERIC_READ | GENERIC_WRITE;
const CREATE_ALWAYS: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const OPEN_ALWAYS: u32 = 4;
const FILE_SHARE_READ: u32 = 1;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const INVALID_HANDLE_VALUE: u32 = 0xffff_ffff;
const INVALID_SET_FILE_POINTER: u32 = 0xffff_ffff;
const FILE_BEGIN: u32 = 0;
const FILE_CURRENT: u32 = 1;
const FILE_END: u32 = 2;

/// The C runtime's `fseek`, `fread`, `fwrite` and `ftell` (`cdecl`), the calls the IO thread made.
const FSEEK: u32 = 0x00ec_b65a;
const FREAD: u32 = 0x00ec_b3a8;
const FWRITE: u32 = 0x00ec_b086;
const FTELL: u32 = 0x00ec_b562;
/// What `NiFile`'s open calls around its `fopen_s` (`00aa14d0`): the base constructor
/// (`00aa1d10`), the `NiFile` vtable, the stream functions' setter (`00aa1b30`) and the buffer's
/// allocation (`00aa1070`).
const BINARY_STREAM_CONSTRUCTOR: u32 = 0x00aa_1d10;
const NI_FILE_VTABLE: u32 = 0x010a_2490;
const BINARY_STREAM_SET_FUNCTIONS: u32 = 0x00aa_1b30;
const ALLOCATE_BUFFER: u32 = 0x00aa_1070;
/// `NiFile`'s buffer flush (`00aa15a0`, `thiscall`).
const NI_FILE_FLUSH: u32 = 0x00aa_15a0;
/// `BSFile::CheckIsGood` (`00aff490`), which `BSFile::Open` ends with, and the `Open` slot.
const CHECK_IS_GOOD: u32 = 0x00af_f490;
const OPEN_SLOT: u32 = 0x20;

/// Opens the file at `path` (a C string) as the C runtime's `fopen` in a binary mode opens it
/// (`__tsopen_nolock`: `access` and `disposition`; reading alone shares reading, as `fopen_s`'s
/// `_SH_SECURE` asks; `FILE_ATTRIBUTE_NORMAL`). Answers `m_pFile`'s tagged handle, or 0 when the
/// file can't be opened (`fopen_s` leaves a null `FILE` then).
fn open(e: &mut Engine, path: u32, access: u32, disposition: u32, append: bool) -> u32 {
    let share = if access == GENERIC_READ {
        FILE_SHARE_READ
    } else {
        0
    };
    let handle = e
        .call(
            CREATE_FILE_A,
            &args![
                path,
                access,
                share,
                0u32,
                disposition,
                FILE_ATTRIBUTE_NORMAL,
                0u32
            ],
        )
        .u32();
    if handle == INVALID_HANDLE_VALUE || handle == 0 {
        return 0;
    }
    assert_eq!(
        handle & 3,
        0,
        "the platform's file handle {handle:#x} is not a multiple of 4: m_pFile's tag needs its low bits"
    );
    handle | HOST | if append { APPEND } else { 0 }
}

/// Moves `handle` by `offset` from the start (`origin` 0: the game's 32-bit position, read as
/// unsigned), the position (1: [`seek_relative`]) or the end (2: a signed distance). Answers the
/// new position, or `None` when the move is refused (a position before the start, an origin that
/// is none of the three).
fn host_seek(e: &mut Engine, handle: u32, offset: u32, origin: u32) -> Option<u64> {
    match origin {
        FILE_BEGIN => set_pointer(e, handle, i64::from(offset), FILE_BEGIN),
        FILE_CURRENT => seek_relative(e, handle, offset),
        _ => set_pointer(e, handle, i64::from(offset as i32), origin),
    }
}

/// `SetFilePointer`'s 64-bit form: `handle` moved by `distance` from `origin`. The new position,
/// or `None` when refused (the position then stays where it was).
fn set_pointer(e: &mut Engine, handle: u32, distance: i64, origin: u32) -> Option<u64> {
    e.with_stack(4, |e, high| {
        e.mem.set_u32(high.addr(), (distance >> 32) as u32);
        let low = e
            .call(
                SET_FILE_POINTER,
                &args![handle, distance as u32, high, origin],
            )
            .u32();
        if low == INVALID_SET_FILE_POINTER && e.call(GET_LAST_ERROR, &args![]).u32() != 0 {
            return None;
        }
        Some((u64::from(e.mem.u32(high.addr())) << 32) | u64::from(low))
    })
}

/// A move from the position by the game's 32-bit distance `offset`. The game's positions are
/// 32-bit and it works a relative seek out as a 32-bit difference (an archive's read, `00afb7a0`:
/// the position it wants less the one its file is at), so a distance of 2 GB or more one way is
/// the same word as one of under 2 GB the other way. The word is moved by as signed, as the C
/// runtime's `fseek` reads it, and where that leaves 0..4 GB the game's own 32-bit sum is the
/// target: past 4 GB, the sum less 4 GB (a move back); before the start, the sum, a move on of 2 GB
/// or more, when the file is that long. Otherwise a move before the start is refused, as the C
/// runtime refuses it (in a file of under 2 GB, every one).
fn seek_relative(e: &mut Engine, handle: u32, offset: u32) -> Option<u64> {
    let distance = i64::from(offset as i32);
    match set_pointer(e, handle, distance, FILE_CURRENT) {
        Some(position) if position >> 32 == 0 => Some(position),
        Some(position) => {
            let from = position.wrapping_sub(distance as u64);
            if from >> 32 != 0 {
                // From past 4 GB, where the game's fields hold no position: as asked.
                return Some(position);
            }
            set_pointer(e, handle, (position - (1 << 32)) as i64, FILE_BEGIN)
        }
        None => {
            let from = set_pointer(e, handle, 0, FILE_CURRENT)?;
            let target = from.checked_add(u64::from(offset))?;
            if target >> 32 != 0 || target > host_size_64(e, handle)? {
                return None;
            }
            set_pointer(e, handle, target as i64, FILE_BEGIN)
        }
    }
}

/// `ReadFile` or `WriteFile` (the import slot `call`) of `count` bytes at `buffer` on `handle`:
/// the bytes moved, 0 when the call fails. No bytes asked, or no buffer, makes no call and answers
/// 0, as `fread` and `fwrite` answer them.
fn host_transfer(e: &mut Engine, call: u32, handle: u32, buffer: u32, count: u32) -> u32 {
    if count == 0 || buffer == 0 {
        return 0;
    }
    e.with_stack(4, |e, done| {
        let moved = e
            .call(call, &args![handle, buffer, count, done, 0u32])
            .u32();
        if moved == 0 {
            0
        } else {
            e.mem.u32(done.addr())
        }
    })
}

/// The size of the file behind `handle` (`GetFileSizeEx`) as the game's 32-bit sizes hold it: 0
/// when it can't be read, or for 4 GB or more (the original answers 0 for any size past 2 GB).
fn host_size(e: &mut Engine, handle: u32) -> u32 {
    match host_size_64(e, handle) {
        Some(size) if size >> 32 == 0 => size as u32,
        _ => 0,
    }
}

/// The size of the file behind `handle` (`GetFileSizeEx`), `None` when it can't be read.
fn host_size_64(e: &mut Engine, handle: u32) -> Option<u64> {
    e.with_stack(8, |e, size| {
        if e.call(GET_FILE_SIZE_EX, &args![handle, size]).u32() == 0 {
            return None;
        }
        Some(e.mem.u64(size.addr()))
    })
}

/// `NiFile`'s open (`00aa14d0`, `thiscall`: the path, the mode, the buffer size; translated as
/// `fn_00aa14d0`) with the file opened by [`open`] instead of `fopen_s`: mode 0 reads an existing
/// file (`"rb"`), mode 1 empties or makes it to write (`"wb"`), any other writes at its end, made
/// when missing (`"ab"`). The rest as the original: the base constructor, the vtable, the stream
/// functions, the fields, `m_bGood` for an open file and its buffer. Answers `this`.
pub fn ni_file_open(
    e: &mut Engine,
    this: Ptr<NiFile>,
    path: Ptr,
    mode: u32,
    buffer_size: u32,
) -> Ptr<NiFile> {
    e.call(BINARY_STREAM_CONSTRUCTOR, &args![this]);
    e.mem.set_u32(this.addr(), NI_FILE_VTABLE);
    e.call(BINARY_STREAM_SET_FUNCTIONS, &args![this, 0u32]);
    e.set(this, NiFile::m_uiCurrentFilePos, 0);
    e.set(this, NiFile::m_eMode, mode);
    let file = match mode {
        0 => open(e, path.addr(), GENERIC_READ, OPEN_EXISTING, false),
        1 => open(e, path.addr(), GENERIC_WRITE, CREATE_ALWAYS, false),
        _ => open(e, path.addr(), GENERIC_WRITE, OPEN_ALWAYS, true),
    };
    e.set(this, NiFile::m_pFile, file);
    let good = file != 0;
    e.set(this, NiFile::m_bGood, u8::from(good));
    e.set(this, NiFile::m_uiBufferAllocSize, buffer_size);
    e.set(this, NiFile::m_uiBufferReadSize, 0);
    e.set(this, NiFile::m_uiPos, 0);
    let buffer = if good && buffer_size != 0 {
        e.call(ALLOCATE_BUFFER, &args![buffer_size]).u32()
    } else {
        0
    };
    e.set(this, NiFile::m_pBuffer, buffer);
    this
}

/// `NiFile`'s size (`00aa15f0`): [`host_size`] for a tagged handle; the original's (`ftell`,
/// `fseek` to the end and back) for a C runtime `FILE`.
pub fn ni_file_size(e: &mut Engine, this: Ptr<NiFile>) -> u32 {
    match tagged(e.get(this, NiFile::m_pFile)) {
        Some((handle, _)) => host_size(e, handle),
        None => ni_file::fn_00aa15f0(e, this),
    }
}

/// `NiFile`'s seek (`00aa16d0`, `thiscall`: offset, origin) for a tagged handle: as the
/// original (nothing unless the file is good; a relative seek inside the read buffer only moves
/// `m_uiPos` and `m_uiAbsoluteCurrentPos`; in read mode one outside it is made relative to the
/// file's position; the buffer flushed), then [`host_seek`] in place of the IO thread's `fseek`
/// and `ftell`: `m_bGood` is whether it moved, and the new position's low 32 bits go to
/// `m_uiCurrentFilePos` and `m_uiAbsoluteCurrentPos`. A C runtime `FILE` takes the original.
pub fn ni_file_seek(e: &mut Engine, this: Ptr<NiFile>, offset: i32, origin: i32) {
    let Some((handle, _)) = tagged(e.get(this, NiFile::m_pFile)) else {
        return ni_file::fn_00aa16d0(e, this, offset, origin);
    };
    if e.get(this, NiFile::m_bGood) == 0 {
        return;
    }
    let mut offset = offset;
    if origin == 1 {
        let position = e.get(this, NiFile::m_uiPos);
        let target = position.wrapping_add(offset as u32) as i32;
        if target >= 0 && target < e.get(this, NiFile::m_uiBufferReadSize) as i32 {
            let absolute = e.get(this, NiFile::m_uiAbsoluteCurrentPos);
            e.set(
                this,
                NiFile::m_uiAbsoluteCurrentPos,
                absolute.wrapping_add(offset as u32),
            );
            e.set(this, NiFile::m_uiPos, target as u32);
            return;
        }
        if e.get(this, NiFile::m_eMode) == 0 {
            let read_size = e.get(this, NiFile::m_uiBufferReadSize);
            offset = offset.wrapping_add(position.wrapping_sub(read_size) as i32);
        }
    }
    e.call(NI_FILE_FLUSH, &args![this]);
    match host_seek(e, handle, offset as u32, origin as u32) {
        Some(position) => {
            e.set(this, NiFile::m_bGood, 1);
            e.set(this, NiFile::m_uiCurrentFilePos, position as u32);
            e.set(this, NiFile::m_uiAbsoluteCurrentPos, position as u32);
        }
        None => e.set(this, NiFile::m_bGood, 0),
    }
}

/// The IO thread's seek (`00aa85c0`, `cdecl`: the file, offset, origin) on the calling thread:
/// [`host_seek`] for a tagged handle (0, or -1 when refused, as `fseek` answers); `fseek` for a C
/// runtime `FILE`.
pub fn file_seek(e: &mut Engine, file: u32, offset: u32, origin: u32) -> u32 {
    match tagged(file) {
        Some((handle, _)) => match host_seek(e, handle, offset, origin) {
            Some(_) => 0,
            None => u32::MAX,
        },
        None => e.call(FSEEK, &args![file, offset, origin]).u32(),
    }
}

/// The IO thread's read (`00aa8610`, `cdecl`: the buffer, a word it does not read, the count,
/// the file) on the calling thread: `ReadFile` for a tagged handle ([`host_transfer`]: the bytes
/// read); `fread(buffer, 1, count, file)` for a C runtime `FILE`, as the IO thread called it.
pub fn file_read(e: &mut Engine, buffer: u32, _unused: u32, count: u32, file: u32) -> u32 {
    match tagged(file) {
        Some((handle, _)) => host_transfer(e, READ_FILE, handle, buffer, count),
        None => e.call(FREAD, &args![buffer, 1u32, count, file]).u32(),
    }
}

/// The IO thread's write (`00aa8660`, the read's words) on the calling thread: `WriteFile` for a
/// tagged handle, at the file's end first when it appends (as the C runtime's `_write` does for
/// `FAPPEND`); `fwrite(buffer, 1, count, file)` for a C runtime `FILE`.
pub fn file_write(e: &mut Engine, buffer: u32, _unused: u32, count: u32, file: u32) -> u32 {
    match tagged(file) {
        Some((handle, append)) => {
            if append && count != 0 {
                host_seek(e, handle, 0, FILE_END);
            }
            host_transfer(e, WRITE_FILE, handle, buffer, count)
        }
        None => e.call(FWRITE, &args![buffer, 1u32, count, file]).u32(),
    }
}

/// `OEI::IO::initialize` (`00aa7eb0`, run once by the Gamebryo start-up `00aa3050`): the IO thread
/// is not made or started, and its globals stay null; the wrappers above do its work.
pub fn io_start_skipped(_e: &mut Engine) {}

/// The IO thread's stop (`00aa81d0`, run by the Gamebryo shutdown `00aa3070`): nothing to stop
/// when the thread was never started. One started before the row was installed is stopped as the
/// original stops it.
pub fn io_stop_skipped(e: &mut Engine) {
    if e.global::<u32>(IO_THREAD) != 0 {
        oei_io_thread::fn_00aa81d0(e);
    }
}

/// `fclose` (`00ec9907`, `cdecl`: the file): `CloseHandle` for a tagged handle (0; -1 when the
/// handle was not open, as `fclose` answers a failure); the C runtime's `fclose` for any other.
pub fn fclose_direct(e: &mut Engine, file: Ptr) -> i32 {
    match tagged(file.addr()) {
        Some((handle, _)) => {
            if e.call(CLOSE_HANDLE, &args![handle]).u32() != 0 {
                0
            } else {
                -1
            }
        }
        None => crt::fclose(e, file),
    }
}

/// `BSFile::Open` (`00aff300`, `thiscall`: a word it does not read, the text flag; translated as
/// `bs_file_open`) with the binary modes opened by [`open`]: true at once when a file is open;
/// otherwise `m_uiPos`, `m_uiBufferReadSize` and `m_pBuffer` cleared and `pFileName` opened by
/// `m_eMode`: 0 reads an existing file (`"rb"`), 1 reads and writes an existing one (`"r+b"`; a
/// missing one is made first, `"wb"` closed at once, and opened again, as the original does), any
/// other reads and writes at its end, made when missing (`"a+b"`). Ends with
/// `BSFile::CheckIsGood` and answers `m_bGood`. The text modes keep the C runtime (the original).
pub fn bs_file_open_direct(e: &mut Engine, this: Ptr<BSFile>, unused: u32, text: bool) -> bool {
    if text {
        return bsfile::bs_file_open(e, this, unused, text);
    }
    let file: Ptr<NiFile> = this.cast();
    if e.get(file, NiFile::m_pFile) != 0 {
        return true;
    }
    e.set(file, NiFile::m_uiPos, 0);
    e.set(file, NiFile::m_uiBufferReadSize, 0);
    e.set(file, NiFile::m_pBuffer, 0);
    let name = this.addr() + BSFile::pFileName.off;
    e.set(file, NiFile::m_pFile, 0);
    let opened = match e.get(file, NiFile::m_eMode) {
        0 => open(e, name, GENERIC_READ, OPEN_EXISTING, false),
        1 => match open(e, name, READ_WRITE, OPEN_EXISTING, false) {
            0 => {
                if let Some((made, _)) = tagged(open(e, name, GENERIC_WRITE, CREATE_ALWAYS, false))
                {
                    e.call(CLOSE_HANDLE, &args![made]);
                }
                open(e, name, READ_WRITE, OPEN_EXISTING, false)
            }
            opened => opened,
        },
        _ => open(e, name, READ_WRITE, OPEN_ALWAYS, true),
    };
    e.set(file, NiFile::m_pFile, opened);
    e.call(CHECK_IS_GOOD, &args![this]);
    e.get(file, NiFile::m_bGood) != 0
}

/// `BSFile`'s size (`00b004e0`; translated as `fn_00b004e0`): for a file open through the C
/// runtime, the original. Otherwise as the original (`iFileSize` cleared, the buffer flushed in
/// mode 1, the file opened through the vtable's `Open` with `(0, 0)`), then [`host_size`] into
/// `iFileSize` for a tagged handle, or the original's measure (`ftell`, `fseek` to the end,
/// `ftell`, `fseek` back) for a C runtime `FILE` its class's `Open` gave. Answers `iFileSize`.
pub fn bs_file_size_direct(e: &mut Engine, this: Ptr<BSFile>) -> u32 {
    let file: Ptr<NiFile> = this.cast();
    let held = e.get(file, NiFile::m_pFile);
    if held != 0 && tagged(held).is_none() {
        return bsfile::fn_00b004e0(e, this);
    }
    e.set(this, BSFile::iFileSize, 0);
    if e.get(file, NiFile::m_eMode) == 1 {
        e.call(NI_FILE_FLUSH, &args![this]);
    }
    e.vcall(this.addr(), OPEN_SLOT, &args![0u32, 0u32]);
    let opened = e.get(file, NiFile::m_pFile);
    if let Some((handle, _)) = tagged(opened) {
        let size = host_size(e, handle);
        e.set(this, BSFile::iFileSize, size);
    } else if opened != 0 {
        let here = e.call(FTELL, &args![opened]).u32();
        e.call(FSEEK, &args![opened, 0u32, FILE_END]);
        let size = e.call(FTELL, &args![opened]).u32();
        e.set(this, BSFile::iFileSize, size);
        e.call(FSEEK, &args![opened, here, 0u32]);
    }
    e.get(this, BSFile::iFileSize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::exe::Import;

    // The C runtime's data and functions its stream calls reach (`crates/engine-core/src/units/
    // crt.rs`), as `engine-platform`'s `crt_stdio_tests` set them up.
    const LOCKS_READY: u32 = 0x0127_05bc;
    const LOCK_TABLE: u32 = 0x011c_28a8;
    const PIOINFO: u32 = 0x0127_0960;
    const NHANDLE: u32 = 0x0127_094c;
    const PIOB: u32 = 0x0127_0a70;
    const NSTREAM: u32 = 0x0127_1a80;
    const GETPTD_NOEXIT: u32 = 0x00ed_1165;
    const DECODE_POINTER: u32 = 0x00ed_0f92;
    const GET_FMODE: u32 = 0x00ee_6236;
    const GET_FILE_TYPE: u32 = 0x00fd_f200;
    const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
    const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
    const INIT_CRITICAL_SECTION_SPIN: u32 = 0x00fd_f130;
    /// The invalid-parameter handler the doubled `__decode_pointer` answers.
    const HANDLER: u32 = 0x0e30_0000;
    /// The buffers' frees and the memory manager's allocate and free, doubled on the engine's heap.
    const FREE_BUFFER: u32 = 0x00aa_10f0;
    const MEMORY_MANAGER_ALLOCATE: u32 = 0x00aa_3e40;
    const MEMORY_MANAGER_DEALLOCATE: u32 = 0x00aa_4060;
    /// `NiFile`'s buffered read and write, its direct read, its scalar deleting destructor;
    /// `BSFile`'s `ReadF`, `WriteF`, seek and `Close`.
    const NI_FILE_READ: u32 = 0x00aa_1750;
    const NI_FILE_WRITE: u32 = 0x00aa_1810;
    const NI_FILE_READ_DIRECT: u32 = 0x00aa_1570;
    const NI_FILE_DELETE: u32 = 0x00aa_18c0;
    const BS_FILE_READ_F: u32 = 0x00b0_0650;
    const BS_FILE_WRITE_F: u32 = 0x00af_f950;
    const BS_FILE_SEEK: u32 = 0x00b0_0590;
    const BS_FILE_CLOSE: u32 = 0x00af_fd10;
    /// Two `BSFile` vtables of the test: the game's functions (the row's where it registers), and
    /// the original's `Open` and size (registered at two spare addresses) for a `BSFile` that keeps
    /// the C runtime.
    const VTABLE: u32 = 0x0e40_0000;
    const ORIGINAL_VTABLE: u32 = 0x0e40_1000;
    const ORIGINAL_OPEN: u32 = 0x0e50_0000;
    const ORIGINAL_SIZE: u32 = 0x0e50_0010;
    /// An `Archive`'s vtable for the test: [`VTABLE`]'s `BSFile` slots and the name (+0x18, which
    /// the archive's read `00afb7a0` asks first and does not read), a double.
    const ARCHIVE_VTABLE: u32 = 0x0e40_2000;
    const ARCHIVE_NAME: u32 = 0x0e50_0020;
    /// The archive's read (`00afb7a0`: the archive file, buffer, count, a skip); the archive
    /// files' buffer size (a global its constructor reads); the platform's interlocked calls the
    /// archive's references make.
    const ARCHIVE_READ: u32 = 0x00af_b7a0;
    const ARCHIVE_FILE_BUFFER_SIZE: u32 = 0x011f_8164;
    const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
    const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;

    /// An engine as the game's start leaves it for these calls: the C runtime's stream table of
    /// `streams` slots with `busy` of them taken, its locks and one block of 32 handles; the
    /// platform's calls at the exe's import slots; the buffers on the engine's heap; the open
    /// modes and seek origins in `.rdata`; the two `BSFile` vtables and the archive's.
    fn engine(streams: u32, busy: u32) -> Engine {
        let mut e = Engine::new();
        e.map(0x0126_f000, 0x2000);
        e.map(0x011c_1000, 0x2000);
        e.map(0x0127_0000, 0x2000);
        e.map(0x011f_6000, 0x1000);
        e.map(0x011f_8000, 0x1000);
        e.map(0x0106_e000, 0x1000);
        e.map(0x010a_2000, 0x3000);
        e.mem.set_u32(ARCHIVE_FILE_BUFFER_SIZE, 0x1000);
        for (at, mode) in [
            (0x0106_e79c, "rb"),
            (0x0106_e798, "wb"),
            (0x010a_24b0, "ab"),
            (0x010a_46cc, "rt"),
            (0x010a_46d0, "rb"),
            (0x010a_46d4, "r+t"),
            (0x010a_46d8, "r+b"),
            (0x010a_46dc, "a+t"),
            (0x010a_46e0, "a+b"),
            (0x010a_46e4, "wt"),
            (0x010a_46e8, "wb"),
        ] {
            e.mem.set_cstr(at, mode.as_bytes());
        }
        for origin in 0..3 {
            e.mem.set_u32(0x010a_2480 + 4 * origin, origin);
        }
        let block = e.mem.alloc(0x40);
        e.register_double(GETPTD_NOEXIT, move |_, _| block.into_ret());
        e.register(DECODE_POINTER, |_, _| HANDLER.into_ret());
        e.register(HANDLER, |_, _| Ret::default());
        e.mem.set_u32(LOCKS_READY, 1);
        for number in [1u32, 0xa, 0xb] {
            let section = e.mem.alloc(0x18);
            e.mem.set_u32(LOCK_TABLE + 8 * number, section);
        }
        let entries = e.mem.alloc(0x800);
        for index in 0..0x20 {
            e.mem.set_u32(entries + 0x40 * index, 0xffff_ffff);
        }
        e.mem.set_u32(PIOINFO, entries);
        e.mem.set_u32(NHANDLE, 0x20);
        let piob = e.mem.alloc(4 * streams);
        e.mem.zero(piob, 4 * streams);
        for index in 0..busy {
            let stream = e.mem.alloc(0x38);
            e.mem.set_u32(stream + 0xc, 1);
            e.mem.set_u32(piob + 4 * index, stream);
        }
        e.mem.set_u32(PIOB, piob);
        e.mem.set_u32(NSTREAM, streams);
        e.register(GET_FMODE, |e, a| {
            e.mem.set_u32(a[0], 0);
            0u32.into_ret()
        });
        let slots = [
            (CREATE_FILE_A, "CreateFileA"),
            (READ_FILE, "ReadFile"),
            (WRITE_FILE, "WriteFile"),
            (SET_FILE_POINTER, "SetFilePointer"),
            (GET_FILE_SIZE_EX, "GetFileSizeEx"),
            (CLOSE_HANDLE, "CloseHandle"),
            (GET_LAST_ERROR, "GetLastError"),
            (GET_FILE_TYPE, "GetFileType"),
            (ENTER_CRITICAL_SECTION, "EnterCriticalSection"),
            (LEAVE_CRITICAL_SECTION, "LeaveCriticalSection"),
            (
                INIT_CRITICAL_SECTION_SPIN,
                "InitializeCriticalSectionAndSpinCount",
            ),
            (INTERLOCKED_INCREMENT, "InterlockedIncrement"),
            (INTERLOCKED_DECREMENT, "InterlockedDecrement"),
        ];
        let imports: Vec<Import> = slots
            .iter()
            .map(|&(slot, name)| Import {
                slot,
                dll: "KERNEL32.dll".into(),
                name: name.into(),
            })
            .collect();
        assert_eq!(e.register_imports(&imports), slots.len());
        e.register(ALLOCATE_BUFFER, |e, a| e.mem.alloc(a[0].max(1)).into_ret());
        e.register(FREE_BUFFER, |e, a| {
            if a[0] != 0 {
                e.mem.free(a[0]);
            }
            Ret::default()
        });
        e.register(MEMORY_MANAGER_ALLOCATE, |e, a| {
            e.mem.alloc(a[1].max(1)).into_ret()
        });
        e.register(MEMORY_MANAGER_DEALLOCATE, |e, a| {
            if a[1] != 0 {
                e.mem.free(a[1]);
            }
            Ret::default()
        });
        let (_, original_open) =
            engine::entry!(0, bsfile::bs_file_open(Ptr<BSFile>, u32, bool) -> bool);
        let (_, original_size) = engine::entry!(0, bsfile::fn_00b004e0(Ptr<BSFile>) -> u32);
        e.register(ORIGINAL_OPEN, original_open);
        e.register(ORIGINAL_SIZE, original_size);
        let mut slots = vec![0u32; 0x13];
        slots[0x1c / 4] = 0x00af_f610;
        slots[0x20 / 4] = BS_FILE_OPEN;
        slots[0x28 / 4] = BS_FILE_SIZE;
        slots[0x44 / 4] = BS_FILE_READ_F;
        slots[0x48 / 4] = BS_FILE_WRITE_F;
        e.put_vtable(VTABLE, &slots);
        let mut archive_slots = slots.clone();
        archive_slots[0x18 / 4] = ARCHIVE_NAME;
        e.put_vtable(ARCHIVE_VTABLE, &archive_slots);
        e.register(ARCHIVE_NAME, |_, _| 0u32.into_ret());
        slots[0x20 / 4] = ORIGINAL_OPEN;
        slots[0x28 / 4] = ORIGINAL_SIZE;
        e.put_vtable(ORIGINAL_VTABLE, &slots);
        e
    }

    /// The platform's open files (`CreateFileA`'s handles not yet closed).
    fn open_files(e: &Engine) -> usize {
        e.kernel
            .objects
            .values()
            .filter(|o| matches!(o, engine::units::platform::KernelObject::File(_)))
            .count()
    }

    /// `text` as a C string in the engine's heap.
    fn c_string(e: &mut Engine, text: &str) -> u32 {
        let at = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(at, text.as_bytes());
        at
    }

    /// The run's own file at `path` (the platform's disk, kept by the lower-case path).
    fn put(e: &mut Engine, path: &str, data: &[u8]) {
        let disk = &mut engine_platform::state_mut(&mut e.kernel.platform).disk;
        disk.files
            .insert(path.to_ascii_lowercase().into_bytes(), data.to_vec());
    }

    fn disk_file(e: &Engine, path: &str) -> Option<Vec<u8>> {
        engine_platform::state(&e.kernel.platform)
            .disk
            .files
            .get(path.to_ascii_lowercase().as_bytes())
            .cloned()
    }

    /// A `NiFile` on `path` opened by the row (`row`, through its address) or by the original's
    /// translation (a C runtime `FILE`).
    fn ni_open(e: &mut Engine, path: &str, mode: u32, buffer: u32, row: bool) -> Ptr<NiFile> {
        let this: Ptr<NiFile> = Ptr::new(e.mem.alloc(0x30));
        let path = Ptr::new(c_string(e, path));
        if row {
            e.call(NI_FILE_OPEN, &args![this, path, mode, buffer]);
            this
        } else {
            ni_file::fn_00aa14d0(e, this, path, mode, buffer)
        }
    }

    /// A `BSFile` on `path` (mode, buffer size) as `BSFile::BSFile` leaves one before `Open`,
    /// with the test's vtable for the row or the original's.
    fn bs_file(e: &mut Engine, path: &str, mode: u32, buffer: u32, row: bool) -> Ptr<BSFile> {
        let vtable = if row { VTABLE } else { ORIGINAL_VTABLE };
        new_bs_file(e, 0x158, vtable, path, mode, buffer)
    }

    /// A `BSFile` (or a subclass `size` bytes long) with `vtable`, as [`bs_file`] makes one.
    fn new_bs_file(
        e: &mut Engine,
        size: u32,
        vtable: u32,
        path: &str,
        mode: u32,
        buffer: u32,
    ) -> Ptr<BSFile> {
        let this = e.mem.alloc(size);
        e.mem.zero(this, size);
        e.call(0x00aa_14b0, &args![this]);
        e.mem.set_u32(this, vtable);
        e.call(0x00af_f990, &args![this, 0u32]);
        e.mem.set_u32(this + 0x28, mode);
        e.mem.set_u32(this + 0x10, buffer);
        e.mem.set_cstr(this + 0x44, path.as_bytes());
        Ptr::new(this)
    }

    /// An `Archive` on the file at `path` as its constructor leaves one for reading: its `BSFile`
    /// part opened through its vtable (the row's `Open` once installed, the original's before),
    /// one directory holding `files` (offset word, size word), no flags, its lock made.
    fn archive_on(e: &mut Engine, path: &str, files: &[(u32, u32)]) -> Ptr<Archive> {
        let file = new_bs_file(e, 0x1d0, ARCHIVE_VTABLE, path, 0, 0x4000);
        assert!(e.vcall(file.addr(), OPEN_SLOT, &args![0u32, 0u32]).bool());
        let this: Ptr<Archive> = file.cast();
        let entries = e.mem.alloc(16 * files.len() as u32);
        for (i, &(offset, size)) in files.iter().enumerate() {
            let at = entries + 16 * i as u32;
            e.mem.set_u64(at, i as u64 + 1);
            e.mem.set_u32(at + 8, size);
            e.mem.set_u32(at + 0xc, offset);
        }
        let directory = e.mem.alloc(16);
        e.mem.set_u64(directory, 1);
        e.mem.set_u32(directory + 8, files.len() as u32);
        e.mem.set_u32(directory + 0xc, entries);
        e.set(this, Archive::pDirectories, directory);
        e.set(this, Archive::iDirectories, 1);
        let lock = this.addr() + Archive::ArchiveCriticalSection.off;
        e.call(INIT_CRITICAL_SECTION_SPIN, &args![lock, 0u32]);
        this
    }

    /// What a `NiFile` shows its callers: its positions, its buffer's state and `m_bGood`.
    fn fields(e: &mut Engine, file: Ptr<NiFile>) -> [u32; 6] {
        [
            e.get(file, NiFile::m_uiAbsoluteCurrentPos),
            e.get(file, NiFile::m_uiBufferAllocSize),
            e.get(file, NiFile::m_uiBufferReadSize),
            e.get(file, NiFile::m_uiPos),
            e.get(file, NiFile::m_uiCurrentFilePos),
            u32::from(e.get(file, NiFile::m_bGood)),
        ]
    }

    /// One call on a file and what it answered.
    #[derive(Debug, Clone, Copy)]
    enum Step {
        /// The buffered read of so many bytes (`00aa1750`).
        Read(u32),
        /// The direct read (`00aa1570`).
        Direct(u32),
        /// The seek (`00aa16d0`): offset, origin.
        Seek(i32, i32),
        /// The size (`00aa15f0`).
        Size,
        /// The buffered write of so many bytes of a pattern (`00aa1810`).
        Write(u32),
        /// The flush (`00aa15a0`).
        Flush,
    }

    /// Runs `step` on `file`: its answer, the bytes it read and the fields after it.
    fn run(e: &mut Engine, file: Ptr<NiFile>, step: Step) -> (u32, Vec<u8>, [u32; 6]) {
        let buffer = e.mem.alloc(0x8000);
        let (answer, bytes) = match step {
            Step::Read(n) | Step::Direct(n) => {
                let at = if matches!(step, Step::Read(_)) {
                    NI_FILE_READ
                } else {
                    NI_FILE_READ_DIRECT
                };
                let got = e.call(at, &args![file, buffer, n]).u32();
                (got, e.mem.bytes(buffer, got))
            }
            Step::Seek(offset, origin) => {
                e.call(NI_FILE_SEEK, &args![file, offset, origin]);
                (0, vec![])
            }
            Step::Size => (e.call(NI_FILE_SIZE, &args![file]).u32(), vec![]),
            Step::Write(n) => {
                let pattern: Vec<u8> = (0..n).map(|i| (i * 13 + n) as u8).collect();
                e.mem.write(buffer, &pattern);
                (e.call(NI_FILE_WRITE, &args![file, buffer, n]).u32(), vec![])
            }
            Step::Flush => (e.call(NI_FILE_FLUSH, &args![file]).u32() & 0xff, vec![]),
        };
        e.mem.free(buffer);
        (answer, bytes, fields(e, file))
    }

    /// The same steps on a file the original opened and one the row opened: the same answers,
    /// bytes and fields at every step.
    fn same_steps(e: &mut Engine, original: Ptr<NiFile>, row: Ptr<NiFile>, steps: &[Step]) {
        assert!(tagged(e.get(original, NiFile::m_pFile)).is_none());
        assert!(tagged(e.get(row, NiFile::m_pFile)).is_some());
        for (i, step) in steps.iter().enumerate() {
            let old = run(e, original, *step);
            let new = run(e, row, *step);
            assert_eq!(old, new, "step {i}: {step:?}");
        }
    }

    /// A file of `n` bytes no two neighbours of which are the same.
    fn pattern(n: u32) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + i / 251) as u8).collect()
    }

    /// Reading: every answer, byte and field as the C runtime gives them (the IO thread's calls),
    /// through the buffer and around it, inside it and outside it, at and past the end.
    #[test]
    fn reads_and_seeks_answer_as_the_c_runtime() {
        let mut e = engine(20, 3);
        install(&mut e);
        let data = pattern(0x5123);
        put(&mut e, r"C:\Test\data.bin", &data);
        let original = ni_open(&mut e, r"C:\Test\data.bin", 0, 0x1000, false);
        let row = ni_open(&mut e, r"C:\Test\data.bin", 0, 0x1000, true);
        use Step::*;
        // Logical positions after each step in the comments; the buffer holds 0x1000 bytes.
        same_steps(
            &mut e,
            original,
            row,
            &[
                Size,
                Read(1),         // 1
                Read(7),         // 8
                Read(0x1003),    // 0x100b: past the buffer, refilled
                Seek(5, 1),      // 0x1010: inside the buffer
                Read(3),         // 0x1013
                Seek(-0x20, 1),  // 0xff3: before the buffer
                Read(0x40),      // 0x1033
                Seek(-0x800, 1), // 0x833
                Read(0x10),      // 0x843
                Seek(0x2000, 0),
                Read(0x10), // 0x2010
                Size,
                Read(0x2500),   // 0x4510: more than the buffer holds, read straight
                Seek(-0x10, 2), // 0x5113
                Read(0x40),     // the last 0x10 bytes
                Read(5),        // none
                Seek(0, 0),
                Direct(0x3000),
                Read(0x18),
                Seek(0x10, 2), // past the end
                Read(4),       // none
                Seek(0x6000, 0),
                Read(4), // none
                Seek(0x10, 0),
                Read(0x10),
                Seek(-0x8000, 1), // before the start: refused, m_bGood cleared
                Read(4),
                Seek(0x10, 0), // a file that is not good does not move
                Read(4),
                Size,
            ],
        );
        // The destructor closes only a good file, in both: the refused seek left the handle open,
        // as it leaves the original's FILE.
        let handle = e.get(row, NiFile::m_pFile) & !3;
        e.call(NI_FILE_DELETE, &args![row, 0u32]);
        e.call(NI_FILE_DELETE, &args![original, 0u32]);
        assert!(e.kernel.object(handle).is_some());
        assert_eq!(e.call(FCLOSE, &args![handle | HOST]).i32(), 0);
        assert!(e.kernel.object(handle).is_none());
        assert_eq!(e.call(FCLOSE, &args![handle | HOST]).i32(), -1);
    }

    /// Writing, in the two write modes: the same answers and fields, and the same files.
    #[test]
    fn writes_make_the_same_files_as_the_c_runtime() {
        let mut e = engine(20, 3);
        install(&mut e);
        use Step::*;
        // The buffer holds 0x100 bytes. No reads: these files are opened for writing only.
        let steps = [
            Write(0x10),
            Write(0x300), // more than the buffer holds: the rest written straight
            Flush,
            Write(0x80),
            Seek(4, 0),
            Write(3),
            Size,
            Write(0x1000),
            Seek(-2, 2),
            Write(0x20),
            Flush,
            Size,
        ];
        for mode in [1u32, 2] {
            // Longer than all the writes reach, so a "wb" that did not empty it would show.
            let before = pattern(0x2123);
            let (old_path, new_path) = (
                format!(r"C:\Test\old-{mode}.bin"),
                format!(r"C:\Test\new-{mode}.bin"),
            );
            put(&mut e, &old_path, &before);
            put(&mut e, &new_path, &before);
            let original = ni_open(&mut e, &old_path, mode, 0x100, false);
            let row = ni_open(&mut e, &new_path, mode, 0x100, true);
            assert_eq!(
                tagged(e.get(row, NiFile::m_pFile)),
                Some((e.get(row, NiFile::m_pFile) & !3, mode == 2))
            );
            same_steps(&mut e, original, row, &steps);
            e.call(NI_FILE_DELETE, &args![original, 0u32]);
            e.call(NI_FILE_DELETE, &args![row, 0u32]);
            let old = disk_file(&e, &old_path).unwrap();
            assert_eq!(disk_file(&e, &new_path).unwrap(), old, "mode {mode}");
            // "wb" emptied it and wrote where the seeks put it; "ab" wrote every byte after the
            // old ones.
            if mode == 1 {
                assert_eq!(old.len(), 0x1025);
            } else {
                assert_eq!(
                    old.len(),
                    before.len() + 0x10 + 0x300 + 0x80 + 3 + 0x1000 + 0x20
                );
                assert_eq!(old[..before.len()], before[..]);
            }
        }
        // A missing file: both fail in mode 0, both make it in modes 1 and 2.
        for mode in 0..3u32 {
            let old_path = format!(r"C:\Test\missing-old-{mode}.bin");
            let new_path = format!(r"C:\Test\missing-new-{mode}.bin");
            let original = ni_open(&mut e, &old_path, mode, 0x100, false);
            let row = ni_open(&mut e, &new_path, mode, 0x100, true);
            assert_eq!(fields(&mut e, original), fields(&mut e, row), "mode {mode}");
            assert_eq!(e.get(row, NiFile::m_bGood), u8::from(mode != 0));
            assert_eq!(e.get(row, NiFile::m_pFile) == 0, mode == 0);
            assert_eq!(
                disk_file(&e, &old_path).is_some(),
                disk_file(&e, &new_path).is_some()
            );
            e.call(NI_FILE_DELETE, &args![original, 0u32]);
            e.call(NI_FILE_DELETE, &args![row, 0u32]);
        }
    }

    /// `BSFile` in its three binary modes, a missing file in mode 1 made first: the same answers,
    /// sizes, positions and files as the original's C runtime path; text mode keeps the C runtime.
    #[test]
    fn bs_files_answer_as_the_c_runtime() {
        let mut e = engine(20, 3);
        install(&mut e);
        let data = pattern(0x2345);
        for (mode, exists) in [(0u32, true), (1, true), (1, false), (2, true), (2, false)] {
            let paths = [
                format!(r"C:\Test\bs-old-{mode}-{exists}.bin"),
                format!(r"C:\Test\bs-new-{mode}-{exists}.bin"),
            ];
            let mut seen = Vec::new();
            for (path, row) in paths.iter().zip([false, true]) {
                if exists {
                    put(&mut e, path, &data);
                }
                // Opened through the vtable: the row's Open, or the original's.
                let before = open_files(&e);
                let file = bs_file(&mut e, path, mode, 0x400, row);
                let mut out =
                    vec![e.vcall(file.addr(), OPEN_SLOT, &args![0u32, 0u32]).u32() & 0xff];
                assert_eq!(
                    tagged(e.get(file.cast::<NiFile>(), NiFile::m_pFile)).is_some(),
                    row
                );
                // Only the file's own handle is left open (mode 1 closes the one that made a
                // missing file).
                assert_eq!(
                    open_files(&e),
                    before + 1,
                    "mode {mode}, exists {exists}, row {row}"
                );
                let buffer = e.mem.alloc(0x4000);
                out.push(e.vcall(file.addr(), 0x28, &args![]).u32());
                if mode == 0 {
                    let read = e.call(BS_FILE_READ_F, &args![file, buffer, 0x123u32]).u32();
                    out.push(read);
                    out.extend(e.mem.bytes(buffer, read).iter().map(|&b| u32::from(b)));
                    e.call(BS_FILE_SEEK, &args![file, 0x2000u32, 0u32]);
                    let read = e.call(BS_FILE_READ_F, &args![file, buffer, 0x400u32]).u32();
                    out.push(read);
                    out.extend(e.mem.bytes(buffer, read).iter().map(|&b| u32::from(b)));
                } else {
                    // The update modes write without reading first: a read and then a write
                    // through one file is the C runtime's undefined case (module comment).
                    e.mem.write(buffer, &[0x5a; 0x30]);
                    out.push(e.call(BS_FILE_WRITE_F, &args![file, buffer, 0x30u32]).u32());
                    out.push(e.vcall(file.addr(), 0x28, &args![]).u32());
                    e.call(BS_FILE_SEEK, &args![file, 0x10u32, 0u32]);
                    e.mem.write(buffer, &[0xa5; 0x500]);
                    out.push(
                        e.call(BS_FILE_WRITE_F, &args![file, buffer, 0x500u32])
                            .u32(),
                    );
                    out.push(e.vcall(file.addr(), 0x28, &args![]).u32());
                }
                out.extend(fields(&mut e, file.cast()));
                out.push(e.mem.u32(file.addr() + 0x150));
                e.call(BS_FILE_CLOSE, &args![file]);
                assert_eq!(e.get(file.cast::<NiFile>(), NiFile::m_pFile), 0);
                assert_eq!(open_files(&e), before);
                seen.push((out, disk_file(&e, path)));
            }
            assert_eq!(seen[0], seen[1], "mode {mode}, exists {exists}");
            assert!(seen[1].1.is_some(), "mode {mode}, exists {exists}");
        }
        // Asked its size before any open (`00b004e0` opens it through the vtable with (0, 0)),
        // in each binary mode: the same size, fields and file as the original's.
        for mode in 0..3u32 {
            let mut seen = Vec::new();
            for row in [false, true] {
                let path = format!(r"C:\Test\bs-size-{mode}-{row}.bin");
                put(&mut e, &path, &data);
                let file = bs_file(&mut e, &path, mode, 0x400, row);
                let mut out = vec![e.vcall(file.addr(), 0x28, &args![]).u32()];
                assert_eq!(
                    tagged(e.get(file.cast::<NiFile>(), NiFile::m_pFile)).is_some(),
                    row,
                    "mode {mode}"
                );
                out.extend(fields(&mut e, file.cast()));
                out.push(e.mem.u32(file.addr() + 0x150));
                e.call(BS_FILE_CLOSE, &args![file]);
                seen.push((out, disk_file(&e, &path)));
            }
            assert_eq!(seen[0], seen[1], "mode {mode}");
            assert_eq!(seen[1].0[0], data.len() as u32, "mode {mode}");
        }
        // Text mode: the C runtime's FILE, as the original.
        put(&mut e, r"C:\Test\text.txt", b"one\r\ntwo\r\n");
        let file = bs_file(&mut e, r"C:\Test\text.txt", 0, 0x100, true);
        assert!(e.call(BS_FILE_OPEN, &args![file, 0u32, 1u32]).bool());
        let held = e.get(file.cast::<NiFile>(), NiFile::m_pFile);
        assert!(held != 0 && tagged(held).is_none());
        let buffer = e.mem.alloc(0x40);
        assert_eq!(
            e.call(BS_FILE_READ_F, &args![file, buffer, 0x40u32]).u32(),
            8
        );
        assert_eq!(e.mem.bytes(buffer, 8), b"one\ntwo\n");
        assert_eq!(e.call(BS_FILE_SIZE, &args![file]).u32(), 10);
        e.call(BS_FILE_CLOSE, &args![file]);
    }

    /// The original can't open a file once the C runtime's 512 streams are taken (as with about 140
    /// plugins); the row holds 600 open at once and reads each.
    #[test]
    fn holds_more_than_512_files_open() {
        let dir = std::env::temp_dir().join(format!("nv-direct-files-{}", std::process::id()));
        let _gone = Remove(dir.clone());
        std::fs::create_dir_all(&dir).unwrap();
        let path = |i: u32| dir.join(format!("{i}.bin")).to_string_lossy().into_owned();
        for i in 0..600u32 {
            let bytes = [i.to_le_bytes(), (!i).to_le_bytes()].concat();
            std::fs::write(path(i), bytes).unwrap();
        }
        let mut e = engine(512, 512);
        let refused = ni_open(&mut e, &path(0), 0, 0x100, false);
        assert_eq!(e.get(refused, NiFile::m_bGood), 0);
        assert_eq!(e.get(refused, NiFile::m_pFile), 0);
        install(&mut e);
        let files: Vec<Ptr<NiFile>> = (0..600)
            .map(|i| ni_open(&mut e, &path(i), 0, 0x100, true))
            .collect();
        let buffer = e.mem.alloc(8);
        for (i, &file) in files.iter().enumerate() {
            assert_eq!(e.get(file, NiFile::m_bGood), 1, "file {i}");
            assert_eq!(e.call(NI_FILE_READ, &args![file, buffer, 8u32]).u32(), 8);
            let i = i as u32;
            assert_eq!(
                e.mem.bytes(buffer, 8),
                [i.to_le_bytes(), (!i).to_le_bytes()].concat()
            );
        }
        let handles: Vec<u32> = files
            .iter()
            .map(|&f| e.get(f, NiFile::m_pFile) & !3)
            .collect();
        let distinct: std::collections::BTreeSet<u32> = handles.iter().copied().collect();
        assert_eq!(distinct.len(), 600);
        // No C runtime stream was taken: the table is as full as before, and still refuses.
        let refused = ni_open(&mut e, &path(1), 0, 0x100, false);
        assert_eq!(e.get(refused, NiFile::m_bGood), 0);
        for &file in &files {
            e.call(NI_FILE_DELETE, &args![file, 0u32]);
        }
        assert!(handles.iter().all(|&h| e.kernel.object(h).is_none()));
    }

    /// Removes a scratch file or folder when the test ends, passed or not.
    struct Remove(std::path::PathBuf);

    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// Marks `file` sparse, so a 3 GB scratch file takes only the clusters written.
    #[cfg(windows)]
    fn sparse(file: &std::fs::File) {
        use std::os::windows::io::AsRawHandle;
        #[link(name = "kernel32")]
        extern "system" {
            fn DeviceIoControl(
                device: *mut std::ffi::c_void,
                code: u32,
                input: *const std::ffi::c_void,
                input_size: u32,
                output: *mut std::ffi::c_void,
                output_size: u32,
                returned: *mut u32,
                overlapped: *mut std::ffi::c_void,
            ) -> i32;
        }
        const FSCTL_SET_SPARSE: u32 = 0x0009_00c4;
        let mut returned = 0u32;
        // SAFETY: the handle is this open file's; the call takes no buffers and writes the count.
        let done = unsafe {
            DeviceIoControl(
                file.as_raw_handle(),
                FSCTL_SET_SPARSE,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(
            done,
            0,
            "FSCTL_SET_SPARSE: {}",
            std::io::Error::last_os_error()
        );
    }

    /// A file grown by `set_len` is sparse there already.
    #[cfg(not(windows))]
    fn sparse(_: &std::fs::File) {}

    /// A 3 GB sparse scratch file: the original's seek past 2 GB is refused and its size reads 0;
    /// the row seeks there from the start, from the position and from the end, reads what is there
    /// and keeps the positions (their 32 bits) and the size. A relative seek of 2 GB or more lands
    /// at the game's 32-bit sum, on or back; one before the start is still refused.
    #[test]
    fn seeks_and_sizes_past_2_gb() {
        use std::io::{Seek, SeekFrom, Write};
        const SIZE: u64 = 0xc000_0000;
        let path =
            std::env::temp_dir().join(format!("nv-direct-files-{}-big.bin", std::process::id()));
        let _gone = Remove(path.clone());
        {
            let mut file = std::fs::File::create(&path).unwrap();
            sparse(&file);
            file.set_len(SIZE).unwrap();
            file.seek(SeekFrom::Start(0x1000)).unwrap();
            file.write_all(b"NEAR ONE").unwrap();
            file.seek(SeekFrom::Start(0x9000_0000)).unwrap();
            file.write_all(b"FAR AWAY").unwrap();
            file.seek(SeekFrom::Start(SIZE - 8)).unwrap();
            file.write_all(b"THE END!").unwrap();
        }
        let name = path.to_string_lossy().into_owned();
        let mut e = engine(20, 3);
        install(&mut e);
        let original = ni_open(&mut e, &name, 0, 0x100, false);
        assert_eq!(e.call(NI_FILE_SIZE, &args![original]).u32(), 0);
        e.call(NI_FILE_SEEK, &args![original, 0x9000_0000u32, 0u32]);
        assert_eq!(e.get(original, NiFile::m_bGood), 0);
        let row = ni_open(&mut e, &name, 0, 0x100, true);
        assert_eq!(e.call(NI_FILE_SIZE, &args![row]).u32(), SIZE as u32);
        let buffer = e.mem.alloc(0x10);
        let read = |e: &mut Engine| {
            let got = e.call(NI_FILE_READ, &args![row, buffer, 8u32]).u32();
            e.mem.bytes(buffer, got)
        };
        e.call(NI_FILE_SEEK, &args![row, 0x9000_0000u32, 0u32]);
        assert_eq!(e.get(row, NiFile::m_bGood), 1);
        assert_eq!(e.get(row, NiFile::m_uiCurrentFilePos), 0x9000_0000);
        assert_eq!(read(&mut e), b"FAR AWAY");
        // From the position: the buffer holds 0x100 bytes from 0x9000_0000.
        e.call(NI_FILE_SEEK, &args![row, 0x2fff_fff0u32, 1u32]);
        assert_eq!(e.get(row, NiFile::m_uiCurrentFilePos), 0xbfff_fff8);
        assert_eq!(read(&mut e), b"THE END!");
        assert_eq!(read(&mut e), b"");
        e.call(NI_FILE_SEEK, &args![row, -8i32, 2u32]);
        assert_eq!(e.get(row, NiFile::m_uiAbsoluteCurrentPos), 0xbfff_fff8);
        assert_eq!(read(&mut e), b"THE END!");
        // Relative seeks of 2 GB or more, as an archive's read makes them (the difference of two
        // 32-bit positions): on by a word that, read as signed, is a move before the start; back
        // by one that, read as signed, is a move past 4 GB.
        let (near, far): (&[u8], &[u8]) = (b"NEAR ONE", b"FAR AWAY");
        for (from, word, to, marker) in [
            (0x0010_0000u32, 0x8ff0_0000u32, 0x9000_0000u32, far),
            (0x80ff_0000, 0x7f01_1000, 0x1000, near),
            (0x1000, 0x8fff_f000, 0x9000_0000, far),
            (0x9000_0000, 0x7000_1000, 0x1000, near),
        ] {
            e.call(NI_FILE_SEEK, &args![row, from, 0u32]);
            e.call(NI_FILE_SEEK, &args![row, word, 1u32]);
            assert_eq!(e.get(row, NiFile::m_bGood), 1, "{from:#x} by {word:#x}");
            assert_eq!(e.get(row, NiFile::m_uiCurrentFilePos), to);
            assert_eq!(e.get(row, NiFile::m_uiAbsoluteCurrentPos), to);
            assert_eq!(read(&mut e), marker, "{from:#x} by {word:#x}");
        }
        // Before the start where the sum is past the end: refused, as the original refuses it.
        e.call(NI_FILE_SEEK, &args![row, 0x10u32, 0u32]);
        e.call(NI_FILE_SEEK, &args![row, -0x8000i32, 1u32]);
        assert_eq!(e.get(row, NiFile::m_bGood), 0);
        e.call(NI_FILE_DELETE, &args![row, 0u32]);
        e.call(NI_FILE_DELETE, &args![original, 0u32]);
        // 5 GB: more than the game's 32-bit sizes hold, so 0, as the original answers past 2 GB.
        let huge =
            std::env::temp_dir().join(format!("nv-direct-files-{}-huge.bin", std::process::id()));
        let _gone_too = Remove(huge.clone());
        {
            let file = std::fs::File::create(&huge).unwrap();
            sparse(&file);
            file.set_len(0x1_4000_0000).unwrap();
        }
        let name = huge.to_string_lossy().into_owned();
        let row = ni_open(&mut e, &name, 0, 0x100, true);
        assert_eq!(e.get(row, NiFile::m_bGood), 1);
        assert_eq!(e.call(NI_FILE_SIZE, &args![row]).u32(), 0);
        e.call(NI_FILE_DELETE, &args![row, 0u32]);
    }

    /// A 2.5 GB sparse archive (as TTW's and VNV's generated LOD archives grow) with a file near
    /// its start, one across the 2 GB mark and one past it, each made by the game's lookup by
    /// indices (`00afa550`) and read by the archive's read (`00afb7a0`). The original makes no
    /// file object for the one past 2 GB (bit 31 of its offset word). The row makes all three and
    /// reads each in an order that moves the archive's file back and on by more than 2 GB (the
    /// relative seeks of `00afb7a0`, the difference of two 32-bit positions).
    #[test]
    fn archive_files_past_2_gb_are_read() {
        use std::io::{Seek, SeekFrom, Write};
        const SIZE: u64 = 0xa000_0000;
        let near: &[u8] = b"near the start!!";
        let across: &[u8] = b"across the 2 GB mark: 32 bytes!!";
        let far: &[u8] = b"far past 2 GB...";
        let files = [(0x1000u32, 16u32), (0x7fff_fff0, 32), (0x9000_0000, 16)];
        let path =
            std::env::temp_dir().join(format!("nv-direct-files-{}-big.bsa", std::process::id()));
        let _gone = Remove(path.clone());
        {
            let mut file = std::fs::File::create(&path).unwrap();
            sparse(&file);
            file.set_len(SIZE).unwrap();
            for (&(at, _), bytes) in files.iter().zip([near, across, far]) {
                file.seek(SeekFrom::Start(u64::from(at))).unwrap();
                file.write_all(bytes).unwrap();
            }
        }
        let name = path.to_string_lossy().into_owned();
        let file_name = |e: &mut Engine| c_string(e, "textures\\lod\\x.dds");
        // The original: the files below 2 GB have a file object, the one past it none.
        let mut original = engine(20, 3);
        let archive = archive_on(&mut original, &name, &files);
        let path_text = file_name(&mut original);
        for (index, made) in [(0u32, true), (1, true), (2, false)] {
            let file = original
                .call(
                    ARCHIVE_FILE_BY_INDICES,
                    &args![archive, 0u32, index, 0u32, path_text],
                )
                .u32();
            assert_eq!(file != 0, made, "file {index}");
        }
        drop(original);
        // The row.
        let mut e = engine(20, 3);
        install(&mut e);
        let archive = archive_on(&mut e, &name, &files);
        let own: Ptr<NiFile> = archive.cast();
        assert!(tagged(e.get(own, NiFile::m_pFile)).is_some());
        assert_eq!(e.vcall(archive.addr(), 0x28, &args![]).u32(), SIZE as u32);
        let path_text = file_name(&mut e);
        let made: Vec<u32> = (0..3u32)
            .map(|index| {
                e.call(
                    ARCHIVE_FILE_BY_INDICES,
                    &args![archive, 0u32, index, 0u32, path_text],
                )
                .u32()
            })
            .collect();
        assert!(made.iter().all(|&file| file != 0));
        assert_eq!(e.mem.u32(made[2] + 0x15c), 0x9000_0000);
        let entries = e.mem.u32(e.get(archive, Archive::pDirectories) + 0xc);
        let far_entry = entries + 0x20;
        let by_entry = e
            .call(
                ARCHIVE_FILE_BY_ENTRY,
                &args![archive, far_entry, 0u32, path_text],
            )
            .u32();
        assert_eq!(e.mem.u32(by_entry + 0x15c), 0x9000_0000);
        assert_eq!(
            e.call(FILE_ENTRY_OFFSET, &args![far_entry]).u32(),
            0x9000_0000
        );
        let buffer = e.mem.alloc(0x40);
        let read = |e: &mut Engine, index: usize| {
            let count = files[index].1;
            let got = e
                .call(ARCHIVE_READ, &args![made[index], buffer, count, 0u32])
                .u32();
            e.mem.bytes(buffer, got)
        };
        // Across the mark to its end (the archive's file then past 2 GB), back to the start (a
        // word that read as signed moves past 4 GB), on past 2 GB (one that read as signed moves
        // before the start), and round again.
        for (index, bytes, at) in [
            (1usize, across, 0x8000_0010u32),
            (0, near, 0x1010),
            (2, far, 0x9000_0010),
            (0, near, 0x1010),
            (1, across, 0x8000_0010),
            (2, far, 0x9000_0010),
        ] {
            assert_eq!(read(&mut e, index), bytes, "file {index}");
            assert_eq!(e.get(own, NiFile::m_uiCurrentFilePos), at);
            assert_eq!(e.get(own, NiFile::m_bGood), 1);
        }
    }

    /// The IO thread is not started and its stop does nothing; the wrappers answer at once, a C
    /// runtime `FILE` through the C runtime as the IO thread answered it.
    #[test]
    fn no_io_thread() {
        let mut e = engine(20, 3);
        install(&mut e);
        let threads = format!("{:?}", e.kernel.threads);
        e.call(IO_START, &args![]);
        for global in 0..7 {
            assert_eq!(e.mem.u32(IO_THREAD + 4 * global), 0);
        }
        assert_eq!(format!("{:?}", e.kernel.threads), threads);
        e.call(IO_STOP, &args![]);
        assert_eq!(format!("{:?}", e.kernel.threads), threads);
        put(&mut e, r"C:\Test\plain.bin", &pattern(0x40));
        let original = ni_open(&mut e, r"C:\Test\plain.bin", 0, 0, false);
        let file = e.get(original, NiFile::m_pFile);
        let buffer = e.mem.alloc(0x40);
        assert_eq!(e.call(IO_SEEK, &args![file, 0x10u32, 0u32]).u32(), 0);
        assert_eq!(
            e.call(IO_READ, &args![buffer, 1u32, 0x40u32, file]).u32(),
            0x30
        );
        assert_eq!(e.mem.bytes(buffer, 0x30), pattern(0x40)[0x10..]);
        assert_eq!(e.call(IO_SEEK, &args![file, 0u32, 7u32]).u32(), u32::MAX);
        e.call(NI_FILE_DELETE, &args![original, 0u32]);
        let written = ni_open(&mut e, r"C:\Test\written.bin", 1, 0, false);
        let file = e.get(written, NiFile::m_pFile);
        assert_eq!(e.call(IO_WRITE, &args![buffer, 1u32, 4u32, file]).u32(), 4);
        e.call(NI_FILE_DELETE, &args![written, 0u32]);
        assert_eq!(
            disk_file(&e, r"C:\Test\written.bin").unwrap(),
            pattern(0x40)[0x10..0x14]
        );
    }
}
