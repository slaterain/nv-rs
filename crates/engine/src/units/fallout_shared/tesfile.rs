//! `fallout shared/tesfile.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TESFile` is one plugin or master file (`.esm`/`.esp`): its names, the
//! open `BSFile`, a cursor over the record stream (the current `FORM`
//! header, the offsets of the form and of the chunk inside it), the file
//! header information and the lists of masters.
//!
//! Notes for the next session (session 1 translated the first 40 functions
//! of the queue, `00470650` to `00472660`; the queue continues at
//! `004726b0`):
//! - Layouts: [`TESFile`] (0x42C bytes, Xbox PDB names, PC offsets checked
//!   against this unit's code), [`Form`] (the 0x18-byte record header,
//!   `FORM`), [`FileHeader`], [`PathBuffer`] and [`Win32FindData`]. Fields
//!   nobody reads yet are declared anyway so the next session finds them.
//! - The private helpers below the constants (`list_item`, `list_next`,
//!   `seek`, `log`, `report_set_file_pointer_failure`, ...) wrap calls this
//!   file makes over and over. Every callee outside the 40 functions of
//!   this session is called by its address, including the later functions
//!   of this unit (`GetTESChunk`, `ReadFormHeader`, ...); switch them to
//!   direct calls when they are translated.
//! - A `BSSimpleList<T>` is its head node (item at +0, next at +4):
//!   `006815c0(node)` answers the node's own address (the item slot),
//!   `00726070(node)` the next node, `00464df0(this)` the address of
//!   `listMasters` (`+0x3EC`) and `00470b90(this)` that of
//!   `listMastersData` (`+0x3F4`).
//! - The `BSFile` is used through its vtable: `+0x14` seeks to
//!   `(offset, whence)` with `whence` read from the global at
//!   [`SEEK_MODE`] (0 in the exe), `+0x20` is called with `(1, 0)` after the
//!   open, `+0x28` answers the file size and `+0` is the deleting
//!   destructor. The PC table is not the Xbox PDB's, so the slots are
//!   described, not named. `BSFile +0x2C` (byte) is the "file is ready"
//!   flag; `+0x38` and `+0x150` are two copies of the current position
//!   (`004720a0`, `00472380`).
//! - The record tags compared by the stream code (`01187014`, `01187020`,
//!   `011873c8`) are filled in at run time (zero in the exe on disk), so
//!   tests set them.
//! - The decompiler names `Concurrency::details::QuickBitSet`,
//!   `std::_Fiopen` and `CArray<...>::RemoveAll` in this unit are wrong:
//!   they are the inlined `BSSimpleList`/`BSStringT` constructors and
//!   destructors and the compiler's `memset`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, BSStringT};

/// `BSSimpleList` node accessor (`006815c0`, returns its `ECX`): the node's
/// own address, which is the address of its item slot.
pub(crate) const LIST_ITEM_SLOT: u32 = 0x0068_15c0;
/// `BSSimpleList` next node (`00726070`): the word at `node + 4`.
pub(crate) const LIST_NEXT: u32 = 0x0072_6070;
/// `BSSimpleList` removal of the head node (`0063f7b0`).
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
/// `BSSimpleList` clear (`00470470`): frees every node after the head and
/// zeroes the head's item.
const LIST_CLEAR: u32 = 0x0047_0470;
/// `BSSimpleList` destructor (`0046ffb0`), which calls [`LIST_CLEAR`].
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// `BSSimpleList` constructor (`0096a2d0`).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// `BSStringT<char>` constructor and destructor.
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
/// `operator new` and `operator delete`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The logging call (`005b5e40`): a format string address, then arguments.
const LOG: u32 = 0x005b_5e40;
/// CRT wrappers: `memset(dst, value, size)`, `memcpy(dst, src, size)` and
/// `memcmp(a, b, size)`.
const MEMSET: u32 = 0x0040_3d30;
const MEMCPY: u32 = 0x0040_1460;
const MEMCMP: u32 = 0x00ec_4835;
/// `strcpy_s(dst, size, src)` and `strcat_s(dst, size, src)` wrappers; the
/// `snprintf`-style `(dst, size, format, argument)`; `strchr(text, char)`;
/// the case-insensitive compare `(a, b)` (0 when equal).
const STRCPY_S: u32 = 0x0040_6d30;
const STRCAT_S: u32 = 0x0040_6d50;
const SNPRINTF: u32 = 0x0040_6d00;
const STRCHR: u32 = 0x0043_b9d0;
const STRICMP: u32 = 0x0040_4dc0;
/// In-place byte swap of a 32-bit and of a 16-bit value, `(pointer, 0)`.
const SWAP_U32: u32 = 0x0040_1080;
const SWAP_U16: u32 = 0x0040_7a90;
/// `BSFile::BSFile(this; path, mode, buffer size, 0)`.
const BSFILE_CONSTRUCT: u32 = 0x00b0_0260;
/// `BSFile::ReadF(this; buffer, size)`: bytes read.
const BSFILE_READ: u32 = 0x00b0_0650;
/// `BSSystemFile::DeleteFileA(path)` (cdecl): 0 on success.
const DELETE_SYSTEM_FILE: u32 = 0x00af_f0b0;
/// CRT `rename(old, new)` and `_errno()`.
const RENAME: u32 = 0x00ec_862c;
const ERRNO: u32 = 0x00ec_85e3;
/// `BSFile +0x2C`: the "file is ready" byte.
const FILE_IS_READY: u32 = 0x0089_05f0;
/// `this + 0x20` of a `TESFile`: its file name.
const FILE_NAME: u32 = 0x0089_1170;
/// `bhkLiquidAction::QCinfoSize` (Xbox PDB name of folded code): answers
/// 0x18, the size of a record header.
const FORM_HEADER_SIZE: u32 = 0x0047_1840;
/// The byte at `+0x61D` of the data handler, which `CloseTES` reads.
const HANDLER_FLAG: u32 = 0x0042_26e0;
/// `TESForm::GetFormTypeFromFormString(tag)` (Xbox PDB).
const TYPE_FROM_FORM_TAG: u32 = 0x0048_6890;
/// Callees later in this unit or without a unit: `CloseAllOpenGroups`, the
/// release of the decompressed form buffer (`+0x420`), two clean-ups, a
/// stub returning 0, the header status check, `GetTESChunk`,
/// `ReadFormHeader` and `ReadChunkHeader`.
const CLOSE_ALL_GROUPS: u32 = 0x0047_3830;
const FREE_DECOMPRESSED_FORM: u32 = 0x0047_3960;
const DESTRUCT_CLEANUP_A: u32 = 0x0047_3d90;
const DESTRUCT_CLEANUP_B: u32 = 0x0047_3a10;
const STUB_RETURNS_ZERO: u32 = 0x0047_3880;
const READ_HEADER_STATUS: u32 = 0x0047_1400;
const GET_TES_CHUNK: u32 = 0x0047_26b0;
const READ_FORM_HEADER: u32 = 0x0047_2bc0;
const READ_CHUNK_HEADER: u32 = 0x0047_2d30;
/// The master list accessor of `tesdatahandler.cpp` (`00464df0`).
const MASTERS_LIST: u32 = 0x0046_4df0;
/// Header version getter (`004694c0`, `float` in `ST0`).
const HEADER_VERSION: u32 = 0x0046_94c0;
/// Returns 1 whatever its `this` (`008d0360`).
const ALWAYS_TRUE: u32 = 0x008d_0360;
/// Sets the byte global at `01202d63` (`00461300`), done before an error
/// is reported.
const NOTE_FAILURE: u32 = 0x0046_1300;

/// The word at this address is the `whence` the code passes to `BSFile`'s
/// seek (0 in the exe).
pub(crate) const SEEK_MODE: u32 = 0x010a_2480;
/// The `TESDataHandler` singleton pointer.
pub(crate) const DATA_HANDLER: u32 = 0x011c_3f2c;
/// A global the stream code passes as `this` to [`ALWAYS_TRUE`].
const STREAM_OBJECT: u32 = 0x011f_6078;
/// `TESFile +0x22C`'s initial value.
const DEFAULT_BUFFER_SIZE: u32 = 0x0118_6740;
/// The `float` the constructor stores as the header version.
const DEFAULT_HEADER_VERSION: u32 = 0x0101_9dd4;
/// The `float` stored as the header version when a file reports another one
/// than [`EXPECTED_HEADER_VERSION`] (a `double`, 0.85).
const FALLBACK_HEADER_VERSION: u32 = 0x0101_9de0;
const EXPECTED_HEADER_VERSION: u32 = 0x0101_9de8;
/// `".tes"`: the extension appended when a file is not found.
const TES_EXTENSION: u32 = 0x0101_9dd8;
/// Record tags compared against the first word of `m_currentform`; they are
/// set at run time. `01187014` is compared when the file's first record is
/// read (the endian check); for the other two only the header is skipped,
/// not `length` bytes more.
pub(crate) const RECORD_TAG_01187014: u32 = 0x0118_7014;
pub(crate) const RECORD_TAG_01187020: u32 = 0x0118_7020;
pub(crate) const RECORD_TAG_011873C8: u32 = 0x0118_73c8;
/// `"HEDR"` as a little-endian word: the id of a file's first chunk.
const CHUNK_ID_HEDR: u32 = 0x5244_4548;

// Messages (format strings in the exe).
const MESSAGE_NOT_VALID_TES_FILE: u32 = 0x0101_9da4;
const MESSAGE_CLOSE_FAILED: u32 = 0x0101_9d7c;
const MESSAGE_RENAME_FAILED: u32 = 0x0101_9df0;
const MESSAGE_DELETE_FAILED: u32 = 0x0101_9e68;
const MESSAGE_MISSING_MASTER: u32 = 0x0101_9f0c;
const MESSAGE_NEXT_FORM_BAD_FORM: u32 = 0x0101_9f30;
const MESSAGE_NEXT_FORM_SEEK_FAILED: u32 = 0x0101_9f84;
const MESSAGE_SET_OFFSET_SEEK_FAILED: u32 = 0x0101_9fc0;
const MESSAGE_SET_OFFSET_CHUNK_SEEK_FAILED: u32 = 0x0101_a000;
const MESSAGE_SET_OFFSET_CHUNK_COMPRESSED: u32 = 0x0101_a040;
/// The format `00471af0` builds a master's path with.
const MASTER_PATH_FORMAT: u32 = 0x0101_9f08;

// Windows imports, by import slot.
const FIND_CLOSE: u32 = 0x00fd_f064;
const FIND_FIRST_FILE: u32 = 0x00fd_f070;
const LSTRCAT: u32 = 0x00fd_f074;
const LSTRCPY: u32 = 0x00fd_f078;
const LOCAL_FREE: u32 = 0x00fd_f08c;
const FORMAT_MESSAGE: u32 = 0x00fd_f090;
const GET_LAST_ERROR: u32 = 0x00fd_f094;
/// `FindFirstFileA`'s failure value.
const INVALID_HANDLE: u32 = 0xffff_ffff;
/// Size of a path buffer.
const PATH_BUFFER_SIZE: u32 = 0x104;
/// Size of a `WIN32_FIND_DATAA`.
const FIND_DATA_SIZE: u32 = 0x140;
/// Size of the `BSFile` the open allocates.
const BSFILE_SIZE: u32 = 0x158;

/// `TES_RETURN_CODE` values this unit stores in `m_lastError`.
const RETURN_CODE_NOT_FOUND: u32 = 2;
const RETURN_CODE_FAILED_TO_WRITE: u32 = 9;
const RETURN_CODE_FOUND_AS_TES: u32 = 0xc;

/// `m_Flags` bits (`TESFile +0x3E8`). `GetMaster`, `GetOptimizedFile` and
/// `GetActive` name three of them; the others are described by the code
/// that sets them.
const FLAG_MASTER: u32 = 0x01;
/// Set when the file's find data changed since the last open.
const FLAG_FIND_DATA_CHANGED: u32 = 0x02;
const FLAG_BIT_4: u32 = 0x04;
const FLAG_ACTIVE: u32 = 0x08;
const FLAG_OPTIMIZED: u32 = 0x10;
const FLAG_BIT_40: u32 = 0x40;
/// Bit of the record flags (`FORM +8`) of a compressed record.
const RECORD_FLAG_COMPRESSED: u32 = 0x0004_0000;
/// Bit of the record flags that `NextForm` skips records with, when asked.
const RECORD_FLAG_SKIPPED_BY_NEXT_FORM: u32 = 0x1000;

layout! {
    /// `FORM` (Xbox PDB): the 0x18-byte header of a record in the stream.
    #[allow(clippy::upper_case_acronyms)]
    pub struct Form: 0x18 {
        /// `form` (Xbox PDB): the record's four-character tag as a word.
        0x00 form: u32,
        /// `length` (Xbox PDB).
        0x04 length: u32,
        /// `flags` (Xbox PDB).
        0x08 flags: u32,
        /// `iFormID` (Xbox PDB).
        0x0C iFormID: u32,
        /// `iVersionControl` (Xbox PDB).
        0x10 iVersionControl: u32,
        /// `sFormVersion` (Xbox PDB).
        0x14 sFormVersion: u16,
        /// `sVCVersion` (Xbox PDB).
        0x16 sVCVersion: u16,
    }

    /// `FILE_HEADER` (Xbox PDB), 0xC bytes.
    pub struct FileHeader: 0x0C {
        /// `fVersion` (Xbox PDB).
        0x00 fVersion: f32,
        /// `iFormCount` (Xbox PDB).
        0x04 iFormCount: u32,
        /// `iNextFormID` (Xbox PDB).
        0x08 iNextFormID: u32,
    }

    /// A `char[260]` path or name buffer embedded in an object.
    pub struct PathBuffer: 0x104 {}

    /// `_WIN32_FIND_DATAA` (0x140 bytes): only the sizes are read here.
    pub struct Win32FindData: 0x140 {
        /// `nFileSizeHigh`.
        0x1C nFileSizeHigh: u32,
        /// `nFileSizeLow`.
        0x20 nFileSizeLow: u32,
    }

    /// `TESFile` (Xbox PDB), 0x42C bytes on both builds; names are the Xbox
    /// PDB's, offsets were checked against this unit's code.
    pub struct TESFile: 0x42C {
        /// `m_lastError` (Xbox PDB): a `TES_RETURN_CODE`.
        0x000 m_lastError: u32,
        /// `pThreadSafeParent` (Xbox PDB).
        0x004 pThreadSafeParent: Ptr,
        /// `pThreadSafeFileMap` (Xbox PDB).
        0x008 pThreadSafeFileMap: Ptr,
        /// `m_pLockedFile` (Xbox PDB): the temporary `BSFile` being saved.
        0x00C m_pLockedFile: Ptr,
        /// `m_pFile` (Xbox PDB): the open `BSFile`.
        0x010 m_pFile: Ptr,
        /// `m_Filename` (Xbox PDB): `char[260]`.
        0x020 m_Filename: Inline<PathBuffer>,
        /// `m_Path` (Xbox PDB): `char[260]`.
        0x124 m_Path: Inline<PathBuffer>,
        /// `m_pBuffer` (Xbox PDB).
        0x228 m_pBuffer: Ptr,
        /// `m_uiBufferAllocSize` (Xbox PDB).
        0x22C m_uiBufferAllocSize: u32,
        /// `m_firstCellOffset` (Xbox PDB).
        0x230 m_firstCellOffset: u32,
        /// `m_currCellOffset` (Xbox PDB).
        0x234 m_currCellOffset: u32,
        /// `m_pCurrCell` (Xbox PDB).
        0x238 m_pCurrCell: Ptr,
        /// `m_currRefOffset` (Xbox PDB).
        0x23C m_currRefOffset: u32,
        /// `m_currentform` (Xbox PDB): the current record's header.
        0x240 m_currentform: Inline<Form>,
        /// `m_currentchunkID` (Xbox PDB).
        0x258 m_currentchunkID: u32,
        /// `m_actualChunkSize` (Xbox PDB).
        0x25C m_actualChunkSize: u32,
        /// `m_filesize` (Xbox PDB).
        0x260 m_filesize: u32,
        /// `m_fileoffset` (Xbox PDB): the offset of the current record.
        0x264 m_fileoffset: u32,
        /// `m_formoffset` (Xbox PDB): the cursor inside the record.
        0x268 m_formoffset: u32,
        /// `m_chunkoffset` (Xbox PDB).
        0x26C m_chunkoffset: u32,
        /// `m_saveform` (Xbox PDB).
        0x270 m_saveform: Inline<Form>,
        /// `m_saveformoffset` (Xbox PDB).
        0x288 m_saveformoffset: u32,
        /// `m_savechunkoffset` (Xbox PDB).
        0x28C m_savechunkoffset: u32,
        /// `m_grouplist` (Xbox PDB): a `BSSimpleList<FORM_GROUP *>`.
        0x290 m_grouplist: Inline<BSSimpleList>,
        /// `bHasGroups` (Xbox PDB).
        0x298 bHasGroups: bool,
        /// `bMustEndianConvert` (Xbox PDB).
        0x299 bMustEndianConvert: bool,
        /// `bCloseFileOverride` (Xbox PDB).
        0x29A bCloseFileOverride: bool,
        /// `m_FileInfo` (Xbox PDB): the last `WIN32_FIND_DATA` of the file.
        0x29C m_FileInfo: Inline<Win32FindData>,
        /// `fileHeaderInfo` (Xbox PDB).
        0x3DC fileHeaderInfo: Inline<FileHeader>,
        /// `m_Flags` (Xbox PDB).
        0x3E8 m_Flags: u32,
        /// `listMasters` (Xbox PDB): a `BSSimpleList<char *>`.
        0x3EC listMasters: Inline<BSSimpleList>,
        /// `listMastersData` (Xbox PDB): a `BSSimpleList<_ULARGE_INTEGER *>`.
        0x3F4 listMastersData: Inline<BSSimpleList>,
        /// `iMasterCount` (Xbox PDB).
        0x3FC iMasterCount: u32,
        /// `m_pMasterPtrs` (Xbox PDB): a `TESFile *` per master, built by
        /// `GenIndexTable`.
        0x400 m_pMasterPtrs: Ptr,
        /// `DeletedFormTime` (Xbox PDB): a `_FILETIME`.
        0x404 DeletedFormTime: u64,
        /// `cCompileIndex` (Xbox PDB).
        0x40C cCompileIndex: u8,
        /// `cCreatedBy` (Xbox PDB): a `BSStringT<char>`.
        0x410 cCreatedBy: Inline<BSStringT>,
        /// `cSummary` (Xbox PDB): a `BSStringT<char>`.
        0x418 cSummary: Inline<BSStringT>,
        /// `pDecompressedFormBuffer` (Xbox PDB).
        0x420 pDecompressedFormBuffer: Ptr,
        /// `iDecompressedFormBufferSize` (Xbox PDB).
        0x424 iDecompressedFormBufferSize: u32,
        /// `bCached` (Xbox PDB).
        0x428 bCached: bool,
        /// `bCaching` (Xbox PDB).
        0x429 bCaching: bool,
        /// `bDLC` (Xbox PDB).
        0x42A bDLC: bool,
    }
}

/// The address of the item slot of a list node (`006815c0`).
fn list_item_slot(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_ITEM_SLOT, &args![node]).u32()
}

/// The item of a list node.
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = list_item_slot(e, node);
    e.mem.u32(slot)
}

/// The node after `node` (`00726070`).
fn list_next(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_NEXT, &args![node]).u32()
}

/// `listMasters` of `this` (`00464df0`).
fn masters_list(e: &mut Engine, this: Ptr<TESFile>) -> u32 {
    e.call(MASTERS_LIST, &args![this]).u32()
}

fn delete(e: &mut Engine, block: u32) {
    e.call(OPERATOR_DELETE, &args![block]);
}

/// Deletes every non-null item of the list starting at `node`.
fn delete_list_items(e: &mut Engine, mut node: u32) {
    while node != 0 {
        let item = list_item(e, node);
        if item != 0 {
            delete(e, item);
        }
        node = list_next(e, node);
    }
}

/// The logging call with a format address and its arguments.
fn log(e: &mut Engine, words: &[u32]) {
    e.call(LOG, words);
}

/// Seeks the `BSFile` to `offset` (vtable slot `+0x14`, `whence` from the
/// global at [`SEEK_MODE`]).
fn seek(e: &mut Engine, file: Ptr, offset: u32) {
    let whence: u32 = e.global(SEEK_MODE);
    e.vcall(file.addr(), 0x14, &args![offset, whence]);
}

/// `GetLastError` + `FormatMessageA(0x1300, 0, error, 0x400, &text, 0, 0)`,
/// then the log call with `format` and the text, then `LocalFree(text)`:
/// the report of a failed `SetFilePointer`.
fn report_set_file_pointer_failure(e: &mut Engine, format: u32) {
    e.with_stack(4, |e, text| {
        let error = e.call(GET_LAST_ERROR, &args![]).u32();
        e.call(
            FORMAT_MESSAGE,
            &args![0x1300u32, 0u32, error, 0x400u32, text, 0u32, 0u32],
        );
        let message = e.mem.u32(text.addr());
        log(e, &args![format, message]);
        e.call(LOCAL_FREE, &args![message]);
    });
}

/// Zeroes the current record header (`m_currentform`).
fn clear_current_form(e: &mut Engine, this: Ptr<TESFile>) {
    e.call(
        MEMSET,
        &args![this.at(TESFile::m_currentform), 0i32, 0x18u32],
    );
}

fn modify_flags(e: &mut Engine, this: Ptr<TESFile>, set: u32, clear: u32) {
    let flags = e.get(this, TESFile::m_Flags);
    e.set(this, TESFile::m_Flags, (flags | set) & !clear);
}

fn flag_is_set(e: &Engine, this: Ptr<TESFile>, mask: u32) -> bool {
    e.get(this, TESFile::m_Flags) & mask != 0
}

// Translated from 00470650 (decompiled, FalloutNV.exe 1.4.0.525)
/// `FORM::Endian` (Xbox PDB): swaps the byte order of the five words and the
/// two halfwords of a record header in place.
pub fn form_endian(e: &mut Engine, this: Ptr<Form>) {
    for offset in [0x00u32, 0x04, 0x08, 0x0c, 0x10] {
        let word = this.byte_add(offset);
        e.call(SWAP_U32, &args![word, 0u32]);
    }
    for offset in [0x14u32, 0x16] {
        let half = this.byte_add(offset);
        e.call(SWAP_U16, &args![half, 0u32]);
    }
}

// Translated from 00470710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::TESFile` (Xbox PDB): constructs the file object, opens the
/// named file to read its header information, and closes it again. Returns
/// `this`. The compiler's exception-unwinding frame is not translated.
pub fn tes_file_tes_file(
    e: &mut Engine,
    this: Ptr<TESFile>,
    directory: Ptr,
    file_name: Ptr,
    open_mode: u32,
) -> Ptr<TESFile> {
    e.call(LIST_CONSTRUCT, &args![this.at(TESFile::m_grouplist)]);
    e.call(LIST_CONSTRUCT, &args![this.at(TESFile::listMasters)]);
    e.call(LIST_CONSTRUCT, &args![this.at(TESFile::listMastersData)]);
    e.call(STRING_CONSTRUCT, &args![this.at(TESFile::cCreatedBy)]);
    e.call(STRING_CONSTRUCT, &args![this.at(TESFile::cSummary)]);
    e.set(this, TESFile::bCached, false);
    e.set(this, TESFile::bCloseFileOverride, false);
    e.set(this, TESFile::pDecompressedFormBuffer, Ptr::NULL);
    e.set(this, TESFile::iDecompressedFormBufferSize, 0);
    e.set(this, TESFile::m_firstCellOffset, 0);
    e.set(this, TESFile::m_currCellOffset, 0);
    e.set(this, TESFile::m_pCurrCell, Ptr::NULL);
    e.set(this, TESFile::pThreadSafeParent, Ptr::NULL);
    e.set(this, TESFile::pThreadSafeFileMap, Ptr::NULL);
    e.set(this, TESFile::bMustEndianConvert, false);
    e.set(this, TESFile::m_lastError, 0);
    e.set(this, TESFile::m_filesize, 0);
    e.set(this, TESFile::m_fileoffset, 0);
    e.set(this, TESFile::m_formoffset, 0);
    e.set(this, TESFile::m_chunkoffset, 0);
    e.set(this, TESFile::m_Flags, 0);
    e.set(this, TESFile::m_pMasterPtrs, Ptr::NULL);
    e.set(this, TESFile::m_pLockedFile, Ptr::NULL);
    e.set(this, TESFile::m_pFile, Ptr::NULL);
    e.set(this, TESFile::iMasterCount, 0);
    e.set(this, TESFile::cCompileIndex, 0xff);
    e.set(this, TESFile::m_currRefOffset, 0);
    e.set(this, TESFile::m_saveformoffset, 0);
    e.set(this, TESFile::m_savechunkoffset, 0);
    e.set(this, TESFile::m_pBuffer, Ptr::NULL);
    let buffer_size: u32 = e.global(DEFAULT_BUFFER_SIZE);
    e.set(this, TESFile::m_uiBufferAllocSize, buffer_size);
    clear_current_form(e, this);
    e.call(
        MEMSET,
        &args![this.at(TESFile::m_FileInfo), 0i32, FIND_DATA_SIZE],
    );
    e.call(MEMSET, &args![this.byte_add(0x404), 0i32, 8u32]);
    e.set(this, TESFile::m_actualChunkSize, 0);
    e.set(this, TESFile::m_currentchunkID, 0);
    let header = this.at(TESFile::fileHeaderInfo);
    e.call(MEMSET, &args![header, 0i32, 0xcu32]);
    e.set(header, FileHeader::iNextFormID, 0x800);
    let version: f32 = e.global(DEFAULT_HEADER_VERSION);
    e.set(header, FileHeader::fVersion, version);

    if tes_file_open_tes_ov2(e, this, directory, file_name, open_mode, 0) {
        let info = this.at(TESFile::m_FileInfo);
        let size_high = e.get(info, Win32FindData::nFileSizeHigh);
        let size_low = e.get(info, Win32FindData::nFileSizeLow);
        if (size_high != 0 || size_low != 0) && e.call(READ_HEADER_STATUS, &args![this]).i32() != 0
        {
            log(e, &args![MESSAGE_NOT_VALID_TES_FILE, file_name]);
        }
        if !tes_file_close_tes(e, this) {
            log(e, &args![MESSAGE_CLOSE_FAILED, file_name]);
        }
    }
    this
}

// Translated from 004709f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TESFile` destructor body: closes the file (with the close override
/// set), deletes the strings of both master lists and empties them, frees
/// the buffer and the master pointer table, and destroys the members. The
/// compiler's exception-unwinding frame is not translated.
pub fn fn_004709f0(e: &mut Engine, this: Ptr<TESFile>) {
    e.set(this, TESFile::bCloseFileOverride, true);
    tes_file_close_tes(e, this);
    let first = masters_list(e, this);
    delete_list_items(e, first);
    let list = masters_list(e, this);
    e.call(LIST_CLEAR, &args![list]);
    let first = fn_00470b90(e, this).addr();
    delete_list_items(e, first);
    let list = fn_00470b90(e, this);
    e.call(LIST_CLEAR, &args![list]);
    let buffer = e.get(this, TESFile::m_pBuffer);
    delete(e, buffer.addr());
    let master_pointers = e.get(this, TESFile::m_pMasterPtrs);
    delete(e, master_pointers.addr());
    e.set(this, TESFile::m_pMasterPtrs, Ptr::NULL);
    e.call(DESTRUCT_CLEANUP_A, &args![this]);
    e.call(DESTRUCT_CLEANUP_B, &args![this]);
    e.call(STRING_DESTRUCT, &args![this.at(TESFile::cSummary)]);
    e.call(STRING_DESTRUCT, &args![this.at(TESFile::cCreatedBy)]);
    e.call(LIST_DESTRUCT, &args![this.at(TESFile::listMastersData)]);
    e.call(LIST_DESTRUCT, &args![this.at(TESFile::listMasters)]);
    e.call(LIST_DESTRUCT, &args![this.at(TESFile::m_grouplist)]);
}

// Translated from 00470b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMastersData` (`this + 0x3F4`).
pub fn fn_00470b90(_e: &mut Engine, this: Ptr<TESFile>) -> Ptr<BSSimpleList> {
    this.at(TESFile::listMastersData)
}

// Translated from 00470bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Answers 0xF (cdecl, no arguments).
pub fn fn_00470bb0(_e: &mut Engine) -> u32 {
    0xf
}

// Translated from 00470bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the first string of `listMasters`, drops its node, and repeats
/// until the list is empty; the same for `listMastersData`; then zeroes
/// `iMasterCount`.
pub fn fn_00470bc0(e: &mut Engine, this: Ptr<TESFile>) {
    loop {
        let list = masters_list(e, this);
        if list_item(e, list) == 0 {
            break;
        }
        let list = masters_list(e, this);
        let item = list_item(e, list);
        delete(e, item);
        let list = masters_list(e, this);
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
    loop {
        let list = fn_00470b90(e, this).addr();
        if list_item(e, list) == 0 {
            break;
        }
        let list = fn_00470b90(e, this).addr();
        let item = list_item(e, list);
        delete(e, item);
        let list = fn_00470b90(e, this).addr();
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
    e.set(this, TESFile::iMasterCount, 0);
}

// Translated from 00470c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::OpenTES` (Xbox PDB): opens the file named by `m_Path` and
/// `m_Filename` (see [`tes_file_open_tes_ov2`]).
pub fn tes_file_open_tes(
    e: &mut Engine,
    this: Ptr<TESFile>,
    open_mode: u32,
    write_flag: u8,
) -> bool {
    let directory = this.at(TESFile::m_Path).cast();
    let file_name = this.at(TESFile::m_Filename).cast();
    tes_file_open_tes_ov2(e, this, directory, file_name, open_mode, write_flag)
}

// Translated from 00470ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::OpenTES` (Xbox PDB, second overload): remembers `directory`
/// and `file_name`, finds the file (with `.tes` appended when it is not
/// found as named, which only sets `m_lastError`), opens a `BSFile` on it
/// unless one is open, records the file size, reads the first record header
/// when `open_mode` is 0 and rewinds. If the first chunk is not `HEDR` and
/// the header version is not 0.85 it stores the fallback version and opens
/// again. `write_flag` forces `open_mode` 1 and calls the stub at
/// `00473880`. Answers false when the file could not be opened.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn tes_file_open_tes_ov2(
    e: &mut Engine,
    this: Ptr<TESFile>,
    directory: Ptr,
    file_name: Ptr,
    open_mode: u32,
    write_flag: u8,
) -> bool {
    if directory.is_null() || file_name.is_null() {
        return false;
    }
    // The path buffer and two `WIN32_FIND_DATAA` of the game's stack frame.
    let scratch = e.mem.alloc(PATH_BUFFER_SIZE + 2 * FIND_DATA_SIZE);
    let result = open_with_scratch(
        e, this, directory, file_name, open_mode, write_flag, scratch,
    );
    e.mem.free(scratch);
    result
}

fn open_with_scratch(
    e: &mut Engine,
    this: Ptr<TESFile>,
    directory: Ptr,
    file_name: Ptr,
    open_mode: u32,
    write_flag: u8,
    scratch: u32,
) -> bool {
    let path = scratch;
    let found = scratch + PATH_BUFFER_SIZE;
    let found_with_extension = found + FIND_DATA_SIZE;
    let mut open_mode = open_mode;
    if write_flag != 0 {
        open_mode = 1;
    }
    let remembered_path: Ptr = this.at(TESFile::m_Path).cast();
    let remembered_name: Ptr = this.at(TESFile::m_Filename).cast();
    e.call(STRCPY_S, &args![path, PATH_BUFFER_SIZE, directory]);
    e.call(
        STRCPY_S,
        &args![remembered_path, PATH_BUFFER_SIZE, directory],
    );
    e.call(STRCAT_S, &args![path, PATH_BUFFER_SIZE, file_name]);
    e.call(
        STRCPY_S,
        &args![remembered_name, PATH_BUFFER_SIZE, file_name],
    );

    if e.get(this, TESFile::m_pFile).is_null() {
        let handle = e.call(FIND_FIRST_FILE, &args![path, found]).u32();
        if handle == INVALID_HANDLE {
            // Not found as named: look for the name with ".tes" instead.
            e.call(LSTRCPY, &args![path, remembered_path]);
            let dot = e.call(STRCHR, &args![remembered_name, 0x2eu32]).u32();
            if dot != 0 {
                e.mem.set_u8(dot, 0);
                e.call(LSTRCAT, &args![path, remembered_name]);
                e.call(LSTRCAT, &args![path, TES_EXTENSION]);
                e.mem.set_u8(dot, b'.');
            } else {
                e.call(LSTRCAT, &args![path, remembered_name]);
                e.call(LSTRCAT, &args![path, TES_EXTENSION]);
            }
            let handle = e
                .call(FIND_FIRST_FILE, &args![path, found_with_extension])
                .u32();
            if handle == INVALID_HANDLE {
                e.set(this, TESFile::m_lastError, RETURN_CODE_NOT_FOUND);
            } else {
                e.set(this, TESFile::m_lastError, RETURN_CODE_FOUND_AS_TES);
                e.call(FIND_CLOSE, &args![handle]);
            }
            return false;
        }
        e.call(FIND_CLOSE, &args![handle]);
        let info = this.at(TESFile::m_FileInfo);
        if e.call(MEMCMP, &args![info, found, FIND_DATA_SIZE]).i32() != 0 {
            modify_flags(e, this, FLAG_FIND_DATA_CHANGED, 0);
            e.call(MEMCPY, &args![info, found, FIND_DATA_SIZE]);
        }
        let block = e.call(OPERATOR_NEW, &args![BSFILE_SIZE]).u32();
        let file = if block != 0 {
            let buffer_size = e.get(this, TESFile::m_uiBufferAllocSize);
            e.call(
                BSFILE_CONSTRUCT,
                &args![block, path, open_mode, buffer_size, 0u32],
            )
            .u32()
        } else {
            0
        };
        e.set(this, TESFile::m_pFile, Ptr::new(file));
        if file == 0 {
            return false;
        }
        e.vcall(file, 0x20, &args![1u32, 0u32]);
        if !e.call(FILE_IS_READY, &args![file]).bool() {
            let error_slot = e.call(ERRNO, &args![]).u32();
            let error = e.mem.u32(error_slot);
            if error == 2 {
                e.set(this, TESFile::m_lastError, RETURN_CODE_NOT_FOUND);
            } else if error == 0xd {
                e.set(this, TESFile::m_lastError, RETURN_CODE_FAILED_TO_WRITE);
            }
            return false;
        }
    }

    let file = e.get(this, TESFile::m_pFile);
    if e.call(FILE_IS_READY, &args![file]).bool() && write_flag != 0 {
        e.call(STUB_RETURNS_ZERO, &args![this]);
    }
    let file = e.get(this, TESFile::m_pFile);
    let size = e.vcall(file.addr(), 0x28, &args![]).u32();
    e.set(this, TESFile::m_filesize, size);
    if open_mode == 0 {
        fn_00471e60(e, this);
    }
    tes_file_tes_rewind(e, this, (open_mode == 0) as u8);
    let mut opened = true;
    if e.get(this, TESFile::m_currentchunkID) != CHUNK_ID_HEDR {
        let version = e.call(HEADER_VERSION, &args![this]).f32();
        let expected: f64 = e.global(EXPECTED_HEADER_VERSION);
        if version as f64 != expected {
            let fallback: f32 = e.global(FALLBACK_HEADER_VERSION);
            fn_00471110(e, this, fallback);
            opened = tes_file_open_tes_ov2(e, this, directory, file_name, open_mode, 0);
        }
    }
    opened
}

// Translated from 00471110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the header version (`fileHeaderInfo.fVersion`).
pub fn fn_00471110(e: &mut Engine, this: Ptr<TESFile>, version: f32) {
    e.set(
        this.at(TESFile::fileHeaderInfo),
        FileHeader::fVersion,
        version,
    );
}

// Translated from 00471130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::CloseTES` (Xbox PDB): closes the groups and the file, and, when
/// a temporary save file is open, closes it, deletes the original and
/// renames the temporary file over it. Answers false when the delete or the
/// rename failed (and sets `m_lastError` to 9). When the close override is
/// off and the data handler's flag byte is 0 nothing is done and the answer
/// is true.
pub fn tes_file_close_tes(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    let handler: u32 = e.global(DATA_HANDLER);
    if !e.get(this, TESFile::bCloseFileOverride)
        && handler != 0
        && e.call(HANDLER_FLAG, &args![handler]).u8() == 0
    {
        return true;
    }
    e.call(CLOSE_ALL_GROUPS, &args![this]);
    let file = e.get(this, TESFile::m_pFile);
    if !file.is_null() {
        e.vcall(file.addr(), 0, &args![1u32]);
        e.set(this, TESFile::m_pFile, Ptr::NULL);
    }
    e.call(FREE_DECOMPRESSED_FORM, &args![this]);
    let locked = e.get(this, TESFile::m_pLockedFile);
    if locked.is_null() {
        return true;
    }
    let scratch = e.mem.alloc(2 * PATH_BUFFER_SIZE);
    let result = replace_with_saved_file(e, this, scratch);
    e.mem.free(scratch);
    result
}

/// The second half of `CloseTES`: `scratch` holds the original file's path
/// and the temporary file's path.
fn replace_with_saved_file(e: &mut Engine, this: Ptr<TESFile>, scratch: u32) -> bool {
    let original = scratch;
    let temporary = scratch + PATH_BUFFER_SIZE;
    let path: Ptr = this.at(TESFile::m_Path).cast();
    let name: Ptr = this.at(TESFile::m_Filename).cast();
    e.call(STRCPY_S, &args![original, PATH_BUFFER_SIZE, path]);
    e.call(STRCAT_S, &args![original, PATH_BUFFER_SIZE, name]);
    e.call(STRCPY_S, &args![temporary, PATH_BUFFER_SIZE, path]);
    let dot = e.call(STRCHR, &args![name, 0x2eu32]).u32();
    if dot != 0 {
        e.mem.set_u8(dot, 0);
        e.call(STRCAT_S, &args![temporary, PATH_BUFFER_SIZE, name]);
        e.call(STRCAT_S, &args![temporary, PATH_BUFFER_SIZE, TES_EXTENSION]);
        e.mem.set_u8(dot, b'.');
    } else {
        e.call(STRCAT_S, &args![temporary, PATH_BUFFER_SIZE, name]);
        e.call(STRCAT_S, &args![temporary, PATH_BUFFER_SIZE, TES_EXTENSION]);
    }
    let locked = e.get(this, TESFile::m_pLockedFile);
    e.vcall(locked.addr(), 0, &args![1u32]);
    e.set(this, TESFile::m_pLockedFile, Ptr::NULL);
    if e.call(DELETE_SYSTEM_FILE, &args![original]).u32() != 0 {
        e.call(NOTE_FAILURE, &args![]);
        e.set(this, TESFile::m_lastError, RETURN_CODE_FAILED_TO_WRITE);
        log(e, &args![MESSAGE_DELETE_FAILED, original, temporary]);
        return false;
    }
    if e.call(RENAME, &args![temporary, original]).i32() != 0 {
        e.call(NOTE_FAILURE, &args![]);
        e.set(this, TESFile::m_lastError, RETURN_CODE_FAILED_TO_WRITE);
        log(e, &args![MESSAGE_RENAME_FAILED, temporary]);
        return false;
    }
    true
}

// Translated from 00471850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `iMasterCount`.
pub fn fn_00471850(e: &mut Engine, this: Ptr<TESFile>) -> u32 {
    e.get(this, TESFile::iMasterCount)
}

// Translated from 00471870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GenIndexTable` (Xbox PDB): rebuilds `m_pMasterPtrs`, one
/// `TESFile *` per master name, by looking each name up (case-insensitively)
/// in the `files` list (a `BSSimpleList<TESFile *>` head). A master that is
/// not in the list gets a null entry, is reported when `report_missing` is
/// set, and makes the answer false.
pub fn tes_file_gen_index_table(
    e: &mut Engine,
    this: Ptr<TESFile>,
    files: Ptr,
    report_missing: u8,
) -> bool {
    let mut master_node = masters_list(e, this);
    let count = fn_00471850(e, this);
    let old_table = e.get(this, TESFile::m_pMasterPtrs);
    if !old_table.is_null() {
        delete(e, old_table.addr());
    }
    e.set(this, TESFile::m_pMasterPtrs, Ptr::NULL);
    if count == 0 {
        return true;
    }
    let bytes = if (count as u64 * 4) >> 32 != 0 {
        u32::MAX
    } else {
        count * 4
    };
    let table = e.call(OPERATOR_NEW, &args![bytes]).u32();
    e.set(this, TESFile::m_pMasterPtrs, Ptr::new(table));
    let mut index = 0u32;
    let mut all_found = true;
    while master_node != 0 && list_item(e, master_node) != 0 {
        let mut file_node = files.addr();
        loop {
            if file_node == 0 || list_item(e, file_node) == 0 {
                break;
            }
            let file = list_item(e, file_node);
            let name = e.call(FILE_NAME, &args![file]).u32();
            let master = list_item(e, master_node);
            if e.call(STRICMP, &args![master, name]).i32() == 0 {
                e.mem
                    .set_u32(table.wrapping_add(index.wrapping_mul(4)), file);
                break;
            }
            file_node = list_next(e, file_node);
            if file_node == 0 || list_item(e, file_node) == 0 {
                e.mem.set_u32(table.wrapping_add(index.wrapping_mul(4)), 0);
                if report_missing != 0 {
                    let missing = list_item(e, master_node);
                    log(e, &args![MESSAGE_MISSING_MASTER, missing]);
                }
                all_found = false;
            }
        }
        index += 1;
        master_node = list_next(e, master_node);
    }
    all_found
}

// Translated from 00471a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetIndexFile` (Xbox PDB): file number `index` of the load
/// order seen from this file: 0 is the file itself, 1 and up are
/// `m_pMasterPtrs[index - 1]`; null past the master count or without a
/// table.
pub fn tes_file_get_index_file(e: &mut Engine, this: Ptr<TESFile>, index: u32) -> Ptr {
    if index > fn_00471850(e, this) {
        return Ptr::NULL;
    }
    if index == 0 {
        return this.cast();
    }
    let table = e.get(this, TESFile::m_pMasterPtrs);
    if table.is_null() {
        return Ptr::NULL;
    }
    Ptr::new(
        e.mem.u32(
            table
                .addr()
                .wrapping_add(index.wrapping_mul(4))
                .wrapping_sub(4),
        ),
    )
}

// Translated from 00471a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `position`th (1-based) master name of `listMasters`; 0 when the list
/// is empty or shorter.
pub fn fn_00471a60(e: &mut Engine, this: Ptr<TESFile>, position: u32) -> u32 {
    let mut node = masters_list(e, this);
    if list_item(e, node) == 0 {
        return 0;
    }
    let mut step = 1;
    while step < position {
        node = list_next(e, node);
        if node == 0 {
            return 0;
        }
        step += 1;
    }
    list_item(e, node)
}

// Translated from 00471ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The master count (`iMasterCount`), through [`fn_00471850`].
pub fn fn_00471ad0(e: &mut Engine, this: Ptr<TESFile>) -> u32 {
    fn_00471850(e, this)
}

// Translated from 00471af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when a master's file on disk no longer has the size recorded in
/// `listMastersData`: walks the two lists together, formats each master's
/// path, and compares the size found (or, for a missing file, the recorded
/// size against zero).
pub fn fn_00471af0(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    let mut name_node = masters_list(e, this);
    let mut data_node = fn_00470b90(e, this).addr();
    let mut changed = false;
    // The path buffer and the `WIN32_FIND_DATAA` of the game's stack frame.
    let path = e.mem.alloc(PATH_BUFFER_SIZE + FIND_DATA_SIZE);
    let found = Ptr::<Win32FindData>::new(path + PATH_BUFFER_SIZE);
    while name_node != 0
        && list_item(e, name_node) != 0
        && data_node != 0
        && list_item(e, data_node) != 0
    {
        let name = list_item(e, name_node);
        e.call(
            SNPRINTF,
            &args![path, PATH_BUFFER_SIZE, MASTER_PATH_FORMAT, name],
        );
        let recorded = list_item(e, data_node);
        let handle = e.call(FIND_FIRST_FILE, &args![path, found]).u32();
        let recorded_low = e.mem.u32(recorded);
        let recorded_high = e.mem.u32(recorded + 4);
        if handle == INVALID_HANDLE {
            if recorded_low != 0 || recorded_high != 0 {
                changed = true;
            }
        } else {
            e.call(FIND_CLOSE, &args![handle]);
            let low = e.get(found, Win32FindData::nFileSizeLow);
            let high = e.get(found, Win32FindData::nFileSizeHigh);
            if low != recorded_low || high != recorded_high {
                changed = true;
            }
        }
        name_node = list_next(e, name_node);
        data_node = list_next(e, data_node);
    }
    e.mem.free(path);
    changed
}

// Translated from 00471c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetMaster` (Xbox PDB): bit 0 of `m_Flags`.
pub fn tes_file_get_master(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    flag_is_set(e, this, FLAG_MASTER)
}

// Translated from 00471c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 0 of `m_Flags` (the master flag).
pub fn fn_00471c50(e: &mut Engine, this: Ptr<TESFile>, on: bool) {
    if on {
        modify_flags(e, this, FLAG_MASTER, 0);
    } else {
        modify_flags(e, this, 0, FLAG_MASTER);
    }
}

// Translated from 00471ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetOptimizedFile` (Xbox PDB): bit 4 (0x10) of `m_Flags`.
pub fn tes_file_get_optimized_file(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    flag_is_set(e, this, FLAG_OPTIMIZED)
}

// Translated from 00471cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 2 (0x04) of `m_Flags`.
pub fn fn_00471cd0(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    flag_is_set(e, this, FLAG_BIT_4)
}

// Translated from 00471d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets bit 2 (0x04) of `m_Flags`; clearing it also clears bit 3 (0x08, the
/// active flag).
pub fn fn_00471d00(e: &mut Engine, this: Ptr<TESFile>, on: bool) {
    if on {
        modify_flags(e, this, FLAG_BIT_4, 0);
    } else {
        modify_flags(e, this, 0, FLAG_BIT_4);
        modify_flags(e, this, 0, FLAG_ACTIVE);
    }
}

// Translated from 00471d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetActive` (Xbox PDB): bit 3 (0x08) of `m_Flags`.
pub fn tes_file_get_active(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    flag_is_set(e, this, FLAG_ACTIVE)
}

// Translated from 00471d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 3 (0x08) of `m_Flags`, the active flag.
pub fn fn_00471d90(e: &mut Engine, this: Ptr<TESFile>, on: bool) {
    if on {
        modify_flags(e, this, FLAG_ACTIVE, 0);
    } else {
        modify_flags(e, this, 0, FLAG_ACTIVE);
    }
}

// Translated from 00471de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 6 (0x40) of `m_Flags`.
pub fn fn_00471de0(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    flag_is_set(e, this, FLAG_BIT_40)
}

// Translated from 00471e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 6 (0x40) of `m_Flags`.
pub fn fn_00471e10(e: &mut Engine, this: Ptr<TESFile>, on: bool) {
    if on {
        modify_flags(e, this, FLAG_BIT_40, 0);
    } else {
        modify_flags(e, this, 0, FLAG_BIT_40);
    }
}

// Translated from 00471e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the first record header of the open file: seeks to the start,
/// reads 0x18 bytes into `m_currentform`, and, when its tag is not the one
/// at [`RECORD_TAG_01187014`], swaps the header's byte order and checks
/// again. Sets bit 6 of `m_Flags` from the always-true answer of
/// `008d0360` (inverted after the swap), and `bMustEndianConvert` when that
/// bit differs from the answer.
pub fn fn_00471e60(e: &mut Engine, this: Ptr<TESFile>) {
    let file = e.get(this, TESFile::m_pFile);
    if file.is_null() {
        return;
    }
    seek(e, file, 0);
    clear_current_form(e, this);
    let form = this.at(TESFile::m_currentform);
    if e.call(BSFILE_READ, &args![file, form, 0x18u32]).u32() == 0 {
        return;
    }
    let tag: u32 = e.global(RECORD_TAG_01187014);
    if e.get(form, Form::form) == tag {
        let stream = fn_00471f70(e);
        let answer = e.call(ALWAYS_TRUE, &args![stream]).u8();
        fn_00471e10(e, this, answer != 0);
    } else {
        form_endian(e, form);
        if e.get(form, Form::form) == tag {
            let stream = fn_00471f70(e);
            let answer = e.call(ALWAYS_TRUE, &args![stream]).u8();
            fn_00471e10(e, this, answer == 0);
        }
    }
    e.set(this, TESFile::bMustEndianConvert, false);
    let bit = fn_00471de0(e, this) as u8;
    let stream = fn_00471f70(e);
    let answer = e.call(ALWAYS_TRUE, &args![stream]).u8();
    if bit != answer {
        e.set(this, TESFile::bMustEndianConvert, true);
    }
}

// Translated from 00471f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The global at `011f6078` (cdecl, no arguments), the object the stream
/// code asks [`ALWAYS_TRUE`].
pub fn fn_00471f70(e: &mut Engine) -> u32 {
    e.global(STREAM_OBJECT)
}

// Translated from 00471f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::TESRewind` (Xbox PDB): seeks the open file to its start, zeroes
/// the three offsets and the current record header, and, when `load_first`
/// is set, reads the first record ([`tes_file_get_tes_form`]).
pub fn tes_file_tes_rewind(e: &mut Engine, this: Ptr<TESFile>, load_first: u8) {
    let file = e.get(this, TESFile::m_pFile);
    if !file.is_null() {
        seek(e, file, 0);
    }
    e.set(this, TESFile::m_fileoffset, 0);
    e.set(this, TESFile::m_formoffset, 0);
    e.set(this, TESFile::m_chunkoffset, 0);
    clear_current_form(e, this);
    if load_first != 0 {
        tes_file_get_tes_form(e, this);
    }
}

// Translated from 00472000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves the cursor to the first chunk of the current record: zeroes the
/// offset inside the record, seeks past the record header unless the record
/// is compressed, clears the current chunk id and size, and reads the chunk
/// header ([`GET_TES_CHUNK`]).
pub fn fn_00472000(e: &mut Engine, this: Ptr<TESFile>) {
    e.set(this, TESFile::m_formoffset, 0);
    e.set(this, TESFile::m_chunkoffset, 0);
    let header_size = e.call(FORM_HEADER_SIZE, &args![this]).u32();
    let position = e
        .get(this, TESFile::m_fileoffset)
        .wrapping_add(header_size)
        .wrapping_add(e.get(this, TESFile::m_formoffset));
    if !fn_00472100(e, this) {
        let file = e.get(this, TESFile::m_pFile);
        seek(e, file, position);
        // The game reads the position back here and does not use it.
        fn_004720a0(e, file);
    }
    fn_004720d0(e, this);
    e.call(GET_TES_CHUNK, &args![this]);
}

// Translated from 004720a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The current position of a `BSFile`: `+0x38` unless it is -1, else
/// `+0x150`.
pub fn fn_004720a0(e: &mut Engine, file: Ptr) -> u32 {
    let first = e.mem.u32(file.addr() + 0x38);
    if first != u32::MAX {
        first
    } else {
        e.mem.u32(file.addr() + 0x150)
    }
}

// Translated from 004720d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the current chunk id and chunk size.
pub fn fn_004720d0(e: &mut Engine, this: Ptr<TESFile>) {
    e.set(this, TESFile::m_currentchunkID, 0);
    e.set(this, TESFile::m_actualChunkSize, 0);
}

// Translated from 00472100 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the current record is compressed: its tag is not the one at
/// [`RECORD_TAG_01187020`] and bit 18 (0x40000) of its flags is set.
pub fn fn_00472100(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    let form = this.at(TESFile::m_currentform);
    let tag: u32 = e.global(RECORD_TAG_01187020);
    e.get(form, Form::form) != tag && e.get(form, Form::flags) & RECORD_FLAG_COMPRESSED != 0
}

// Translated from 00472150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::NextForm` (Xbox PDB): frees the decompressed form buffer and
/// moves to the next record ([`fn_004721d0`]); with `skip_flagged` it keeps
/// moving while the record (not one of the two header-only tags) has bit 12
/// (0x1000) of its flags set. Answers whether a record was read.
pub fn tes_file_next_form(e: &mut Engine, this: Ptr<TESFile>, skip_flagged: u8) -> bool {
    e.call(FREE_DECOMPRESSED_FORM, &args![this]);
    let mut read = fn_004721d0(e, this);
    let form = this.at(TESFile::m_currentform);
    while read
        && skip_flagged != 0
        && e.get(form, Form::form) != e.global::<u32>(RECORD_TAG_01187020)
        && e.get(form, Form::form) != e.global::<u32>(RECORD_TAG_011873C8)
        && e.get(form, Form::flags) & RECORD_FLAG_SKIPPED_BY_NEXT_FORM != 0
    {
        read = fn_004721d0(e, this);
    }
    read
}

// Translated from 004721d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Advances `m_fileoffset` past the current record (its header only for the
/// two header-only tags, else the header and `length`), seeks the file
/// there and reads the next record header. Answers false at the end of the
/// file (the header is zeroed), when the seek fails (reported with the
/// system's message) or when no record header could be read (reported).
pub fn fn_004721d0(e: &mut Engine, this: Ptr<TESFile>) -> bool {
    let form = this.at(TESFile::m_currentform);
    let tag = e.get(form, Form::form);
    let header_size = e.call(FORM_HEADER_SIZE, &args![this]).u32();
    let fileoffset = e.get(this, TESFile::m_fileoffset);
    if tag == e.global::<u32>(RECORD_TAG_01187020) || tag == e.global::<u32>(RECORD_TAG_011873C8) {
        e.set(
            this,
            TESFile::m_fileoffset,
            header_size.wrapping_add(fileoffset),
        );
    } else {
        let length = e.get(form, Form::length);
        e.set(
            this,
            TESFile::m_fileoffset,
            header_size.wrapping_add(length).wrapping_add(fileoffset),
        );
    }
    if e.get(this, TESFile::m_fileoffset) >= e.get(this, TESFile::m_filesize) {
        clear_current_form(e, this);
        return false;
    }
    let target = e.get(this, TESFile::m_fileoffset);
    let file = e.get(this, TESFile::m_pFile);
    if fn_00472380(e, file) != target {
        seek(e, file, target);
        let position = fn_00472380(e, file);
        e.set(this, TESFile::m_fileoffset, position);
    }
    if e.get(this, TESFile::m_fileoffset) == u32::MAX {
        report_set_file_pointer_failure(e, MESSAGE_NEXT_FORM_SEEK_FAILED);
        return false;
    }
    e.set(this, TESFile::m_formoffset, 0);
    e.set(this, TESFile::m_chunkoffset, 0);
    clear_current_form(e, this);
    tes_file_get_tes_form_no_ret(e, this);
    if e.get(form, Form::form) == 0 {
        let name: Ptr = this.at(TESFile::m_Filename).cast();
        let offset = e.get(this, TESFile::m_fileoffset);
        log(e, &args![MESSAGE_NEXT_FORM_BAD_FORM, name, offset]);
        return false;
    }
    true
}

// Translated from 00472380 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSFile +0x150`: the file's current position.
pub fn fn_00472380(e: &mut Engine, file: Ptr) -> u32 {
    e.mem.u32(file.addr() + 0x150)
}

// Translated from 004723a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::SetOffset` (Xbox PDB): frees the decompressed form buffer,
/// seeks to `offset` and reads the record header there. Answers false when
/// the offset is at or past the end of the file (the header is zeroed) or
/// the seek fails (reported with the system's message).
pub fn tes_file_set_offset(e: &mut Engine, this: Ptr<TESFile>, offset: u32) -> bool {
    e.call(FREE_DECOMPRESSED_FORM, &args![this]);
    e.set(this, TESFile::m_fileoffset, offset);
    if e.get(this, TESFile::m_fileoffset) >= e.get(this, TESFile::m_filesize) {
        clear_current_form(e, this);
        return false;
    }
    let target = e.get(this, TESFile::m_fileoffset);
    let file = e.get(this, TESFile::m_pFile);
    seek(e, file, target);
    let position = fn_004720a0(e, file);
    e.set(this, TESFile::m_fileoffset, position);
    if position == u32::MAX {
        report_set_file_pointer_failure(e, MESSAGE_SET_OFFSET_SEEK_FAILED);
        return false;
    }
    e.set(this, TESFile::m_formoffset, 0);
    e.set(this, TESFile::m_chunkoffset, 0);
    clear_current_form(e, this);
    tes_file_get_tes_form(e, this);
    true
}

// Translated from 004724c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The file offset of the cursor inside the current record:
/// `m_fileoffset + 0x18 + m_formoffset`.
pub fn fn_004724c0(e: &mut Engine, this: Ptr<TESFile>) -> u32 {
    let header_size = e.call(FORM_HEADER_SIZE, &args![this]).u32();
    e.get(this, TESFile::m_fileoffset)
        .wrapping_add(header_size)
        .wrapping_add(e.get(this, TESFile::m_formoffset))
}

// Translated from 004724f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::SetOffsetChunk` (Xbox PDB): moves the cursor to the absolute
/// file offset `offset`, which must lie inside the current record, and
/// reads the chunk header there. Refused (and reported) for a compressed
/// record; answers false for an offset outside the record or when the seek
/// fails.
pub fn tes_file_set_offset_chunk(e: &mut Engine, this: Ptr<TESFile>, offset: u32) -> bool {
    if fn_00472100(e, this) {
        log(e, &args![MESSAGE_SET_OFFSET_CHUNK_COMPRESSED]);
        return false;
    }
    let header_size = e.call(FORM_HEADER_SIZE, &args![this]).u32();
    let inside = offset
        .wrapping_sub(e.get(this, TESFile::m_fileoffset))
        .wrapping_sub(header_size);
    e.set(this, TESFile::m_formoffset, inside);
    let form = this.at(TESFile::m_currentform);
    if inside >= e.get(form, Form::length) {
        fn_004720d0(e, this);
        clear_current_form(e, this);
        return false;
    }
    let file = e.get(this, TESFile::m_pFile);
    seek(e, file, offset);
    if fn_004720a0(e, file) == u32::MAX {
        report_set_file_pointer_failure(e, MESSAGE_SET_OFFSET_CHUNK_SEEK_FAILED);
        return false;
    }
    e.set(this, TESFile::m_chunkoffset, 0);
    fn_004720d0(e, this);
    e.call(GET_TES_CHUNK, &args![this]);
    true
}

// Translated from 00472620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetTESFormNoRet` (Xbox PDB): when no record header is loaded,
/// reads one ([`READ_FORM_HEADER`]) and, if that worked, clears the chunk
/// and reads its header ([`READ_CHUNK_HEADER`]).
pub fn tes_file_get_tes_form_no_ret(e: &mut Engine, this: Ptr<TESFile>) {
    let form = this.at(TESFile::m_currentform);
    if e.get(form, Form::form) == 0 && e.call(READ_FORM_HEADER, &args![this]).bool() {
        fn_004720d0(e, this);
        e.call(READ_CHUNK_HEADER, &args![this]);
    }
}

// Translated from 00472660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESFile::GetTESForm` (Xbox PDB): loads the current record header if
/// none is loaded (answering 0 when none can be read, else reading the
/// first chunk header too), then answers the form type of its tag
/// (`TESForm::GetFormTypeFromFormString`).
pub fn tes_file_get_tes_form(e: &mut Engine, this: Ptr<TESFile>) -> u32 {
    let form = this.at(TESFile::m_currentform);
    if e.get(form, Form::form) == 0 {
        if !e.call(READ_FORM_HEADER, &args![this]).bool() {
            return 0;
        }
        fn_004720d0(e, this);
        e.call(GET_TES_CHUNK, &args![this]);
    }
    let tag = e.get(form, Form::form);
    e.call(TYPE_FROM_FORM_TAG, &args![tag]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00470650, form_endian(Ptr<Form>)),
        entry!(
            0x00470710,
            tes_file_tes_file(Ptr<TESFile>, Ptr, Ptr, u32) -> Ptr<TESFile>
        ),
        entry!(0x004709f0, fn_004709f0(Ptr<TESFile>)),
        entry!(0x00470b90, fn_00470b90(Ptr<TESFile>) -> Ptr<BSSimpleList>),
        entry!(0x00470bb0, fn_00470bb0() -> u32),
        entry!(0x00470bc0, fn_00470bc0(Ptr<TESFile>)),
        entry!(0x00470c70, tes_file_open_tes(Ptr<TESFile>, u32, u8) -> bool),
        entry!(
            0x00470ca0,
            tes_file_open_tes_ov2(Ptr<TESFile>, Ptr, Ptr, u32, u8) -> bool
        ),
        entry!(0x00471110, fn_00471110(Ptr<TESFile>, f32)),
        entry!(0x00471130, tes_file_close_tes(Ptr<TESFile>) -> bool),
        entry!(0x00471850, fn_00471850(Ptr<TESFile>) -> u32),
        entry!(
            0x00471870,
            tes_file_gen_index_table(Ptr<TESFile>, Ptr, u8) -> bool
        ),
        entry!(
            0x00471a10,
            tes_file_get_index_file(Ptr<TESFile>, u32) -> Ptr
        ),
        entry!(0x00471a60, fn_00471a60(Ptr<TESFile>, u32) -> u32),
        entry!(0x00471ad0, fn_00471ad0(Ptr<TESFile>) -> u32),
        entry!(0x00471af0, fn_00471af0(Ptr<TESFile>) -> bool),
        entry!(0x00471c20, tes_file_get_master(Ptr<TESFile>) -> bool),
        entry!(0x00471c50, fn_00471c50(Ptr<TESFile>, bool)),
        entry!(
            0x00471ca0,
            tes_file_get_optimized_file(Ptr<TESFile>) -> bool
        ),
        entry!(0x00471cd0, fn_00471cd0(Ptr<TESFile>) -> bool),
        entry!(0x00471d00, fn_00471d00(Ptr<TESFile>, bool)),
        entry!(0x00471d60, tes_file_get_active(Ptr<TESFile>) -> bool),
        entry!(0x00471d90, fn_00471d90(Ptr<TESFile>, bool)),
        entry!(0x00471de0, fn_00471de0(Ptr<TESFile>) -> bool),
        entry!(0x00471e10, fn_00471e10(Ptr<TESFile>, bool)),
        entry!(0x00471e60, fn_00471e60(Ptr<TESFile>)),
        entry!(0x00471f70, fn_00471f70() -> u32),
        entry!(0x00471f80, tes_file_tes_rewind(Ptr<TESFile>, u8)),
        entry!(0x00472000, fn_00472000(Ptr<TESFile>)),
        entry!(0x004720a0, fn_004720a0(Ptr) -> u32),
        entry!(0x004720d0, fn_004720d0(Ptr<TESFile>)),
        entry!(0x00472100, fn_00472100(Ptr<TESFile>) -> bool),
        entry!(0x00472150, tes_file_next_form(Ptr<TESFile>, u8) -> bool),
        entry!(0x004721d0, fn_004721d0(Ptr<TESFile>) -> bool),
        entry!(0x00472380, fn_00472380(Ptr) -> u32),
        entry!(0x004723a0, tes_file_set_offset(Ptr<TESFile>, u32) -> bool),
        entry!(0x004724c0, fn_004724c0(Ptr<TESFile>) -> u32),
        entry!(
            0x004724f0,
            tes_file_set_offset_chunk(Ptr<TESFile>, u32) -> bool
        ),
        entry!(0x00472620, tes_file_get_tes_form_no_ret(Ptr<TESFile>)),
        entry!(0x00472660, tes_file_get_tes_form(Ptr<TESFile>) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mapped page of test configuration: word 0 is whether the fake
    /// `BSFile` reports ready, word 1 the size its size method answers,
    /// word 2 the position its seek forces when nonzero, word 3 the `errno`.
    const CONFIG: u32 = 0x0200_0000;
    const CONFIG_READY: u32 = CONFIG;
    const CONFIG_SIZE: u32 = CONFIG + 4;
    const CONFIG_SEEK_RESULT: u32 = CONFIG + 8;
    const CONFIG_ERRNO: u32 = CONFIG + 12;
    /// The fake `BSFile` vtable and the addresses of its methods.
    const FAKE_VTABLE: u32 = 0x0200_1000;
    const FAKE_DESTRUCTOR: u32 = 0x7100_0000;
    const FAKE_SEEK: u32 = 0x7100_0010;
    const FAKE_OPEN: u32 = 0x7100_0020;
    const FAKE_SIZE: u32 = 0x7100_0030;
    /// Tags the tests give the record tag globals.
    const TAG_FILE_HEADER: u32 = 0x3453_4554;
    const TAG_HEADER_ONLY: u32 = 0x5055_5247;
    const TAG_OTHER_HEADER_ONLY: u32 = 0x5055_5248;
    const TAG_RECORD: u32 = 0x5252_4552;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// An engine with the exe's data pages this unit reads, the list and
    /// string helpers the game has elsewhere, and a fake file system and
    /// `BSFile`. Everything else a test needs it registers itself.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_9000,
            0x0101_a000,
            0x010a_2000,
            0x0118_6000,
            0x0118_7000,
            0x011c_3000,
            0x011f_6000,
            CONFIG,
            FAKE_VTABLE,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(RECORD_TAG_01187014, TAG_FILE_HEADER);
        e.set_global(RECORD_TAG_01187020, TAG_HEADER_ONLY);
        e.set_global(RECORD_TAG_011873C8, TAG_OTHER_HEADER_ONLY);
        e.set_global(DEFAULT_BUFFER_SIZE, 0x4000u32);
        e.set_global(DEFAULT_HEADER_VERSION, 1.34f32);
        e.set_global(FALLBACK_HEADER_VERSION, 0.85f32);
        e.set_global(EXPECTED_HEADER_VERSION, 0.85f64);
        e.mem.set_cstr(TES_EXTENSION, b".tes");
        e.mem.set_u32(CONFIG_READY, 1);
        e.mem.set_u32(CONFIG_SIZE, 0x1234);

        e.register(LIST_ITEM_SLOT, |_, a| ret(a[0]));
        e.register(LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(LIST_REMOVE_HEAD, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next != 0 {
                let item = e.mem.u32(next);
                let after = e.mem.u32(next + 4);
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            } else {
                e.mem.set_u32(a[0], 0);
            }
            Ret::default()
        });
        e.register(LIST_CLEAR, |_, _| Ret::default());
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(LIST_CONSTRUCT, |_, _| Ret::default());
        e.register(STRING_CONSTRUCT, |_, _| Ret::default());
        e.register(STRING_DESTRUCT, |_, _| Ret::default());
        e.register(STRCPY_S, |e, a| e.call(0x00ec_65a6, &a[..3]));
        e.register(STRCAT_S, |e, a| e.call(0x00ec_6bd2, &a[..3]));
        e.register(SNPRINTF, |e, a| {
            // The format is `"%s"`: the master's name.
            let name = e.mem.cstr(a[3]);
            e.mem.set_cstr(a[0], &name);
            Ret::default()
        });
        e.register(STRICMP, |e, a| e.call(0x00ec_68e4, &a[..2]));
        e.register(FILE_NAME, |_, a| ret(a[0] + 0x20));
        e.register(SWAP_U32, |e, a| {
            let value = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], value.swap_bytes());
            ret(value.swap_bytes())
        });
        e.register(SWAP_U16, |e, a| {
            let value = e.mem.u16(a[0]);
            e.mem.set_u16(a[0], value.swap_bytes());
            ret(value.swap_bytes() as u32)
        });
        e.register(FORM_HEADER_SIZE, |_, _| ret(0x18));
        e.register(FREE_DECOMPRESSED_FORM, |_, _| Ret::default());
        e.register(ALWAYS_TRUE, |_, _| ret(1));

        // Windows: a directory with `Data\Fallout.esm` (4096 bytes) and
        // `Data\Mod.tes`.
        e.register(FIND_FIRST_FILE, |e, a| {
            let path = e.mem.cstr(a[0]);
            if path.ends_with(b"Fallout.esm") {
                e.mem.set_u32(a[1] + 0x1c, 0);
                e.mem.set_u32(a[1] + 0x20, 0x1000);
                ret(5)
            } else if path.ends_with(b"Mod.tes") {
                ret(7)
            } else {
                ret(INVALID_HANDLE)
            }
        });
        e.register(FIND_CLOSE, |_, _| ret(1));
        e.register(GET_LAST_ERROR, |_, _| ret(5));
        e.register(FORMAT_MESSAGE, |e, a| {
            let text = e.mem.alloc(8);
            e.mem.set_cstr(text, b"boom");
            e.mem.set_u32(a[4], text);
            ret(4)
        });
        e.register(LOCAL_FREE, |_, _| Ret::default());
        e.register(LSTRCPY, |e, a| e.call(0x00ec_6370, &a[..2]));
        e.register(LSTRCAT, |e, a| e.call(0x00ec_6bd2, &[a[0], 0x104, a[1]]));
        e.register(ERRNO, |_, _| ret(CONFIG_ERRNO));

        // The fake `BSFile`.
        e.put_vtable(
            FAKE_VTABLE,
            &[
                FAKE_DESTRUCTOR,
                0,
                0,
                0,
                0,
                FAKE_SEEK,
                0,
                0,
                FAKE_OPEN,
                0,
                FAKE_SIZE,
            ],
        );
        e.register(FAKE_DESTRUCTOR, |_, _| Ret::default());
        e.register(FAKE_SEEK, |e, a| {
            let forced = e.mem.u32(CONFIG_SEEK_RESULT);
            let position = if forced != 0 { forced } else { a[1] };
            e.mem.set_u32(a[0] + 0x38, position);
            e.mem.set_u32(a[0] + 0x150, position);
            Ret::default()
        });
        e.register(FAKE_OPEN, |_, _| Ret::default());
        e.register(FAKE_SIZE, |e, _| ret(e.mem.u32(CONFIG_SIZE)));
        e.register(BSFILE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], FAKE_VTABLE);
            let ready = e.mem.u32(CONFIG_READY);
            e.mem.set_u8(a[0] + 0x2c, ready as u8);
            ret(a[0])
        });
        e.register(FILE_IS_READY, |e, a| ret(e.mem.u8(a[0] + 0x2c) as u32));
        e
    }

    fn new_file(e: &mut Engine) -> Ptr<TESFile> {
        e.new_object()
    }

    /// A `BSFile` stand-in with the fake vtable.
    fn fake_bsfile(e: &mut Engine) -> Ptr {
        let file = Ptr::new(e.mem.alloc(0x160));
        e.mem.set_u32(file.addr(), FAKE_VTABLE);
        e.mem.set_u8(file.addr() + 0x2c, 1);
        file
    }

    /// A `BSSimpleList` of the given items; returns the head node.
    fn build_list(e: &mut Engine, items: &[u32]) -> u32 {
        let head = e.mem.alloc(8);
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
        head
    }

    /// A C string on the heap.
    fn text(e: &mut Engine, s: &[u8]) -> u32 {
        let block = e.mem.alloc(s.len() as u32 + 1);
        e.mem.set_cstr(block, s);
        block
    }

    fn set_name(e: &mut Engine, file: Ptr<TESFile>, path: &[u8], name: &[u8]) {
        e.mem.set_cstr(file.addr() + 0x124, path);
        e.mem.set_cstr(file.addr() + 0x20, name);
    }

    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn current_form(file: Ptr<TESFile>) -> Ptr<Form> {
        file.at(TESFile::m_currentform)
    }

    fn set_tag(e: &mut Engine, file: Ptr<TESFile>, tag: u32, length: u32, flags: u32) {
        let form = current_form(file);
        e.set(form, Form::form, tag);
        e.set(form, Form::length, length);
        e.set(form, Form::flags, flags);
    }

    #[test]
    fn layout_offsets_are_the_pc_ones() {
        assert_eq!(TESFile::SIZE, 0x42c);
        assert_eq!(TESFile::m_currentform.off, 0x240);
        assert_eq!(
            TESFile::m_FileInfo.off + Win32FindData::nFileSizeHigh.off,
            0x2b8
        );
        assert_eq!(
            TESFile::fileHeaderInfo.off + FileHeader::iNextFormID.off,
            0x3e4
        );
    }

    #[test]
    fn form_endian_swaps_five_words_and_two_halfwords() {
        let mut e = engine();
        let form: Ptr<Form> = e.new_object();
        e.set(form, Form::form, 0x1122_3344);
        e.set(form, Form::length, 0x0000_0010);
        e.set(form, Form::flags, 0x0100_0000);
        e.set(form, Form::iFormID, 0xaabb_ccdd);
        e.set(form, Form::iVersionControl, 1);
        e.set(form, Form::sFormVersion, 0x1234);
        e.set(form, Form::sVCVersion, 0x00ff);
        e.call_log = Some(vec![]);
        e.call(0x0047_0650, &args![form]);
        assert_eq!(e.get(form, Form::form), 0x4433_2211);
        assert_eq!(e.get(form, Form::length), 0x1000_0000);
        assert_eq!(e.get(form, Form::flags), 1);
        assert_eq!(e.get(form, Form::iFormID), 0xddcc_bbaa);
        assert_eq!(e.get(form, Form::iVersionControl), 0x0100_0000);
        assert_eq!(e.get(form, Form::sFormVersion), 0x3412);
        assert_eq!(e.get(form, Form::sVCVersion), 0xff00);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, SWAP_U32).len(), 5);
        assert_eq!(calls_to(&log, SWAP_U16).len(), 2);
    }

    #[test]
    fn constructor_initialises_and_gives_up_when_the_file_is_missing() {
        let mut e = engine();
        let file = new_file(&mut e);
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Gone.esm");
        // Scribble over what the constructor must reset.
        e.set(file, TESFile::m_filesize, 77);
        e.set(file, TESFile::m_Flags, 0xff);
        e.call_log = Some(vec![]);
        let result = e.call(0x0047_0710, &args![file, directory, name, 0u32]);
        assert_eq!(result.ptr::<TESFile>(), file);
        assert_eq!(e.get(file, TESFile::m_filesize), 0);
        assert_eq!(e.get(file, TESFile::m_Flags), 0);
        assert_eq!(e.get(file, TESFile::cCompileIndex), 0xff);
        assert_eq!(e.get(file, TESFile::m_uiBufferAllocSize), 0x4000);
        let header = file.at(TESFile::fileHeaderInfo);
        assert_eq!(e.get(header, FileHeader::fVersion), 1.34);
        assert_eq!(e.get(header, FileHeader::iNextFormID), 0x800);
        // `Gone.esm` and `Gone.tes` do not exist.
        assert_eq!(e.get(file, TESFile::m_lastError), 2);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LIST_CONSTRUCT).len(), 3);
        assert_eq!(calls_to(&log, STRING_CONSTRUCT).len(), 2);
        assert!(calls_to(&log, CLOSE_ALL_GROUPS).is_empty());
    }

    #[test]
    fn constructor_checks_the_header_and_closes_the_opened_file() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(READ_HEADER_STATUS, |_, _| ret(1));
        let file = new_file(&mut e);
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Fallout.esm");
        e.register(BSFILE_READ, |e, a| {
            e.mem.set_u32(a[1], TAG_FILE_HEADER);
            ret(0x18)
        });
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(TYPE_FROM_FORM_TAG, |_, _| ret(7));
        e.register(GET_TES_CHUNK, |e, a| {
            e.mem.set_u32(a[0] + 0x258, CHUNK_ID_HEDR);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0047_0710, &args![file, directory, name, 0u32]);
        let log = e.call_log.take().unwrap();
        // The file was found and had a size, the header check failed, the
        // complaint names the file, and the file was closed.
        assert_eq!(
            calls_to(&log, LOG),
            vec![vec![MESSAGE_NOT_VALID_TES_FILE, name]]
        );
        assert_eq!(e.get(file, TESFile::m_pFile), Ptr::NULL);
        assert_eq!(calls_to(&log, CLOSE_ALL_GROUPS).len(), 1);
    }

    #[test]
    fn destructor_frees_both_lists_and_the_tables() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(DESTRUCT_CLEANUP_A, |_, _| Ret::default());
        e.register(DESTRUCT_CLEANUP_B, |_, _| Ret::default());
        let file = new_file(&mut e);
        let master_a = e.mem.alloc(8);
        let master_b = e.mem.alloc(8);
        let size = e.mem.alloc(8);
        let buffer = e.mem.alloc(8);
        let table = e.mem.alloc(8);
        let masters = build_list(&mut e, &[master_a, master_b]);
        let sizes = build_list(&mut e, &[size]);
        e.mem.write(file.addr() + 0x3ec, &e.mem.bytes(masters, 8));
        e.mem.write(file.addr() + 0x3f4, &e.mem.bytes(sizes, 8));
        e.set(file, TESFile::m_pBuffer, Ptr::new(buffer));
        e.set(file, TESFile::m_pMasterPtrs, Ptr::new(table));
        e.call_log = Some(vec![]);
        e.call(0x0047_09f0, &args![file]);
        let log = e.call_log.take().unwrap();
        assert!(e.get(file, TESFile::bCloseFileOverride));
        assert_eq!(e.get(file, TESFile::m_pMasterPtrs), Ptr::NULL);
        for freed in [master_a, master_b, size, buffer, table] {
            assert_eq!(e.mem.block_size(freed), None, "{freed:08x} still allocated");
        }
        assert_eq!(calls_to(&log, LIST_CLEAR).len(), 2);
        assert_eq!(calls_to(&log, LIST_DESTRUCT).len(), 3);
        assert_eq!(
            calls_to(&log, STRING_DESTRUCT),
            vec![vec![file.addr() + 0x418], vec![file.addr() + 0x410]]
        );
    }

    #[test]
    fn list_of_master_sizes_is_at_3f4() {
        let mut e = engine();
        let file = new_file(&mut e);
        let list = e.call(0x0047_0b90, &args![file]).u32();
        assert_eq!(list, file.addr() + 0x3f4);
    }

    #[test]
    fn answers_fifteen() {
        let mut e = engine();
        assert_eq!(e.call(0x0047_0bb0, &args![]).u32(), 0xf);
    }

    #[test]
    fn master_lists_are_emptied_and_the_count_reset() {
        let mut e = engine();
        let file = new_file(&mut e);
        let master_a = e.mem.alloc(8);
        let master_b = e.mem.alloc(8);
        let size = e.mem.alloc(8);
        let masters = build_list(&mut e, &[master_a, master_b]);
        let sizes = build_list(&mut e, &[size]);
        e.mem.write(file.addr() + 0x3ec, &e.mem.bytes(masters, 8));
        e.mem.write(file.addr() + 0x3f4, &e.mem.bytes(sizes, 8));
        e.set(file, TESFile::iMasterCount, 2);
        e.call_log = Some(vec![]);
        e.call(0x0047_0bc0, &args![file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::iMasterCount), 0);
        assert_eq!(e.mem.u32(file.addr() + 0x3ec), 0);
        assert_eq!(e.mem.u32(file.addr() + 0x3f4), 0);
        let deleted: Vec<u32> = calls_to(&log, OPERATOR_DELETE)
            .iter()
            .map(|args| args[0])
            .collect();
        assert_eq!(deleted, vec![master_a, master_b, size]);
        assert_eq!(calls_to(&log, LIST_REMOVE_HEAD).len(), 3);
    }

    #[test]
    fn open_tes_opens_the_remembered_path_and_name() {
        let mut e = engine();
        let file = new_file(&mut e);
        set_name(&mut e, file, b"Data\\", b"Gone.esm");
        assert!(!e.call(0x0047_0c70, &args![file, 1u32, 0u8]).bool());
        assert_eq!(e.get(file, TESFile::m_lastError), 2);
        assert_eq!(e.mem.cstr(file.addr() + 0x124), b"Data\\");
        assert_eq!(e.mem.cstr(file.addr() + 0x20), b"Gone.esm");
    }

    #[test]
    fn open_needs_a_directory_and_a_name() {
        let mut e = engine();
        let file = new_file(&mut e);
        let name = text(&mut e, b"Fallout.esm");
        assert!(!e
            .call(0x0047_0ca0, &args![file, 0u32, name, 1u32, 0u8])
            .bool());
        assert!(!e
            .call(0x0047_0ca0, &args![file, name, 0u32, 1u32, 0u8])
            .bool());
        assert_eq!(e.get(file, TESFile::m_pFile), Ptr::NULL);
    }

    #[test]
    fn open_finds_the_file_and_creates_the_bsfile() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_uiBufferAllocSize, 0x4000);
        e.set(file, TESFile::m_currentchunkID, CHUNK_ID_HEDR);
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Fallout.esm");
        e.call_log = Some(vec![]);
        let opened = e.call(0x0047_0ca0, &args![file, directory, name, 1u32, 0u8]);
        assert!(opened.bool());
        let log = e.call_log.take().unwrap();
        let created = calls_to(&log, BSFILE_CONSTRUCT);
        assert_eq!(created.len(), 1);
        let args = &created[0];
        assert_eq!(e.mem.cstr(args[1]), b"Data\\Fallout.esm");
        assert_eq!(&args[2..], &[1, 0x4000, 0]);
        assert_eq!(e.mem.cstr(file.addr() + 0x124), b"Data\\");
        assert_eq!(e.mem.cstr(file.addr() + 0x20), b"Fallout.esm");
        assert_eq!(e.get(file, TESFile::m_pFile).addr(), args[0]);
        assert_eq!(e.get(file, TESFile::m_filesize), 0x1234);
        // The find data is new: remembered, and flagged.
        assert_eq!(
            e.get(file.at(TESFile::m_FileInfo), Win32FindData::nFileSizeLow),
            0x1000
        );
        assert_eq!(e.get(file, TESFile::m_Flags) & FLAG_FIND_DATA_CHANGED, 2);
        assert_eq!(calls_to(&log, FIND_CLOSE), vec![vec![5]]);
        // Mode 1 does not read the header.
        assert!(calls_to(&log, BSFILE_READ).is_empty());
    }

    #[test]
    fn open_reports_missing_files_in_the_error_code() {
        let mut e = engine();
        let file = new_file(&mut e);
        let directory = text(&mut e, b"Data\\");
        let with_extension = text(&mut e, b"Mod.esm");
        let missing = text(&mut e, b"Nothing.esm");
        // `Mod.tes` exists, `Mod.esm` does not.
        assert!(!e
            .call(
                0x0047_0ca0,
                &args![file, directory, with_extension, 1u32, 0u8]
            )
            .bool());
        assert_eq!(e.get(file, TESFile::m_lastError), 0xc);
        // The name keeps its extension.
        assert_eq!(e.mem.cstr(file.addr() + 0x20), b"Mod.esm");
        assert!(!e
            .call(0x0047_0ca0, &args![file, directory, missing, 1u32, 0u8])
            .bool());
        assert_eq!(e.get(file, TESFile::m_lastError), 2);
        assert_eq!(e.get(file, TESFile::m_pFile), Ptr::NULL);
    }

    #[test]
    fn open_maps_a_failed_bsfile_to_an_error_code() {
        let mut e = engine();
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Fallout.esm");
        e.mem.set_u32(CONFIG_READY, 0);
        for (errno, code) in [(2u32, 2u32), (0xd, 9), (5, 0)] {
            let file = new_file(&mut e);
            e.mem.set_u32(CONFIG_ERRNO, errno);
            assert!(!e
                .call(0x0047_0ca0, &args![file, directory, name, 1u32, 0u8])
                .bool());
            assert_eq!(e.get(file, TESFile::m_lastError), code);
            // The `BSFile` object stays set.
            assert!(!e.get(file, TESFile::m_pFile).is_null());
        }
    }

    #[test]
    fn open_in_mode_zero_reads_the_header_and_retries_with_the_fallback_version() {
        let mut e = engine();
        e.register(BSFILE_READ, |e, a| {
            e.mem.set_u32(a[1], TAG_FILE_HEADER);
            ret(0x18)
        });
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(TYPE_FROM_FORM_TAG, |_, _| ret(7));
        // The first chunk is not `HEDR` the first time, then it is.
        let mut reads = 0;
        e.register_double(GET_TES_CHUNK, move |e, a| {
            reads += 1;
            let id = if reads == 1 { 0x1234 } else { CHUNK_ID_HEDR };
            e.mem.set_u32(a[0] + 0x258, id);
            Ret::default()
        });
        let file = new_file(&mut e);
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Fallout.esm");
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x0047_0ca0, &args![file, directory, name, 0u32, 0u8])
            .bool());
        let log = e.call_log.take().unwrap();
        // 1.34 is not 0.85: the fallback version is stored and the open
        // runs again on the already open file.
        let header = file.at(TESFile::fileHeaderInfo);
        assert_eq!(e.get(header, FileHeader::fVersion), 0.85f32);
        assert_eq!(calls_to(&log, BSFILE_CONSTRUCT).len(), 1);
        assert_eq!(calls_to(&log, GET_TES_CHUNK).len(), 2);
        assert!(!e.get(file, TESFile::bMustEndianConvert));
        assert_eq!(e.get(file, TESFile::m_currentchunkID), CHUNK_ID_HEDR);
    }

    #[test]
    fn write_flag_forces_mode_one_and_calls_the_stub() {
        let mut e = engine();
        e.register(STUB_RETURNS_ZERO, |_, _| ret(0));
        let file = new_file(&mut e);
        e.set(file, TESFile::m_currentchunkID, CHUNK_ID_HEDR);
        let directory = text(&mut e, b"Data\\");
        let name = text(&mut e, b"Fallout.esm");
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x0047_0ca0, &args![file, directory, name, 0u32, 1u8])
            .bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, BSFILE_CONSTRUCT)[0][2], 1);
        assert_eq!(calls_to(&log, STUB_RETURNS_ZERO).len(), 1);
    }

    #[test]
    fn header_version_setter_stores_the_float() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.call(0x0047_1110, &args![file, 0.94f32]);
        assert_eq!(e.mem.f32(file.addr() + 0x3dc), 0.94);
    }

    #[test]
    fn close_does_nothing_when_the_handler_says_not_to() {
        let mut e = engine();
        e.register(HANDLER_FLAG, |_, _| ret(0));
        e.set_global(DATA_HANDLER, 0x1234u32);
        let file = new_file(&mut e);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(log.len(), 2, "{log:?}");
        assert_eq!(log[1].0, HANDLER_FLAG);
    }

    #[test]
    fn close_destroys_the_bsfile_and_closes_the_groups() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(HANDLER_FLAG, |_, _| ret(1));
        e.set_global(DATA_HANDLER, 0x1234u32);
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::m_pFile), Ptr::NULL);
        assert_eq!(calls_to(&log, CLOSE_ALL_GROUPS).len(), 1);
        assert_eq!(
            calls_to(&log, FAKE_DESTRUCTOR),
            vec![vec![bsfile.addr(), 1]]
        );
        assert_eq!(calls_to(&log, FREE_DECOMPRESSED_FORM).len(), 1);
    }

    #[test]
    fn close_replaces_the_original_with_the_saved_temporary_file() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(DELETE_SYSTEM_FILE, |_, _| ret(0));
        e.register(RENAME, |_, _| ret(0));
        let file = new_file(&mut e);
        set_name(&mut e, file, b"Data\\", b"Mod.esp");
        let locked = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pLockedFile, locked);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::m_pLockedFile), Ptr::NULL);
        assert_eq!(
            calls_to(&log, FAKE_DESTRUCTOR),
            vec![vec![locked.addr(), 1]]
        );
        let deleted = calls_to(&log, DELETE_SYSTEM_FILE);
        assert_eq!(e.mem.cstr(deleted[0][0]), b"Data\\Mod.esp");
        // `rename(temporary, original)`.
        let renamed = calls_to(&log, RENAME);
        assert_eq!(e.mem.cstr(renamed[0][0]), b"Data\\Mod.tes");
        assert_eq!(e.mem.cstr(renamed[0][1]), b"Data\\Mod.esp");
        assert!(calls_to(&log, LOG).is_empty());
        // The name's dot was put back.
        assert_eq!(e.mem.cstr(file.addr() + 0x20), b"Mod.esp");
    }

    #[test]
    fn close_without_an_extension_appends_it_to_the_temporary_name() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(DELETE_SYSTEM_FILE, |_, _| ret(0));
        e.register(RENAME, |_, _| ret(0));
        let file = new_file(&mut e);
        set_name(&mut e, file, b"Data\\", b"Mod");
        let locked = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pLockedFile, locked);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        let renamed = calls_to(&log, RENAME);
        assert_eq!(e.mem.cstr(renamed[0][0]), b"Data\\Mod.tes");
    }

    #[test]
    fn close_reports_a_failed_delete() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(DELETE_SYSTEM_FILE, |_, _| ret(1));
        e.register(RENAME, |_, _| ret(0));
        e.register(NOTE_FAILURE, |_, _| Ret::default());
        let file = new_file(&mut e);
        set_name(&mut e, file, b"Data\\", b"Mod.esp");
        let locked = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pLockedFile, locked);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::m_lastError), 9);
        assert!(calls_to(&log, RENAME).is_empty());
        assert_eq!(calls_to(&log, NOTE_FAILURE).len(), 1);
        let complaint = calls_to(&log, LOG);
        assert_eq!(complaint[0][0], MESSAGE_DELETE_FAILED);
        assert_eq!(e.mem.cstr(complaint[0][1]), b"Data\\Mod.esp");
        assert_eq!(e.mem.cstr(complaint[0][2]), b"Data\\Mod.tes");
    }

    #[test]
    fn close_reports_a_failed_rename() {
        let mut e = engine();
        e.register(CLOSE_ALL_GROUPS, |_, _| Ret::default());
        e.register(DELETE_SYSTEM_FILE, |_, _| ret(0));
        e.register(RENAME, |_, _| ret(1));
        e.register(NOTE_FAILURE, |_, _| Ret::default());
        let file = new_file(&mut e);
        set_name(&mut e, file, b"Data\\", b"Mod.esp");
        let locked = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pLockedFile, locked);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_1130, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::m_lastError), 9);
        let complaint = calls_to(&log, LOG);
        assert_eq!(complaint[0][0], MESSAGE_RENAME_FAILED);
        assert_eq!(e.mem.cstr(complaint[0][1]), b"Data\\Mod.tes");
    }

    #[test]
    fn master_count_is_read() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::iMasterCount, 3);
        assert_eq!(e.call(0x0047_1850, &args![file]).u32(), 3);
        assert_eq!(e.call(0x0047_1ad0, &args![file]).u32(), 3);
    }

    /// A file with `names` as masters and a list of loaded files named
    /// `loaded`; returns (file, list head, loaded files).
    fn index_setup(
        e: &mut Engine,
        names: &[&[u8]],
        loaded: &[&[u8]],
    ) -> (Ptr<TESFile>, u32, Vec<u32>) {
        let file = new_file(e);
        let name_blocks: Vec<u32> = names.iter().map(|n| text(e, n)).collect();
        let masters = build_list(e, &name_blocks);
        e.mem.write(file.addr() + 0x3ec, &e.mem.bytes(masters, 8));
        e.set(file, TESFile::iMasterCount, names.len() as u32);
        let mut files = Vec::new();
        for name in loaded {
            let other = new_file(e);
            e.mem.set_cstr(other.addr() + 0x20, name);
            files.push(other.addr());
        }
        let list = build_list(e, &files);
        (file, list, files)
    }

    #[test]
    fn index_table_maps_each_master_to_its_loaded_file() {
        let mut e = engine();
        let (file, list, files) = index_setup(&mut e, &[b"B.esm", b"a.esm"], &[b"A.esm", b"B.esm"]);
        let stale = e.mem.alloc(0x40);
        e.set(file, TESFile::m_pMasterPtrs, Ptr::new(stale));
        assert!(e.call(0x0047_1870, &args![file, list, 1u8]).bool());
        assert_eq!(e.mem.block_size(stale), None);
        let table = e.get(file, TESFile::m_pMasterPtrs).addr();
        // Names compare without case.
        assert_eq!(e.mem.u32(table), files[1]);
        assert_eq!(e.mem.u32(table + 4), files[0]);
    }

    #[test]
    fn index_table_reports_missing_masters() {
        let mut e = engine();
        let (file, list, files) =
            index_setup(&mut e, &[b"A.esm", b"Gone.esm"], &[b"A.esm", b"B.esm"]);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_1870, &args![file, list, 1u8]).bool());
        let log = e.call_log.take().unwrap();
        let table = e.get(file, TESFile::m_pMasterPtrs).addr();
        assert_eq!(e.mem.u32(table), files[0]);
        assert_eq!(e.mem.u32(table + 4), 0);
        let complaint = calls_to(&log, LOG);
        assert_eq!(complaint.len(), 1);
        assert_eq!(e.mem.cstr(complaint[0][1]), b"Gone.esm");
        // Silent when not asked.
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_1870, &args![file, list, 0u8]).bool());
        assert!(calls_to(&e.call_log.take().unwrap(), LOG).is_empty());
    }

    #[test]
    fn index_table_without_masters_frees_the_old_table() {
        let mut e = engine();
        let file = new_file(&mut e);
        let stale = e.mem.alloc(8);
        e.set(file, TESFile::m_pMasterPtrs, Ptr::new(stale));
        let list = build_list(&mut e, &[]);
        assert!(e.call(0x0047_1870, &args![file, list, 1u8]).bool());
        assert_eq!(e.get(file, TESFile::m_pMasterPtrs), Ptr::NULL);
        assert_eq!(e.mem.block_size(stale), None);
    }

    #[test]
    fn index_file_zero_is_the_file_itself() {
        let mut e = engine();
        let (file, list, files) = index_setup(&mut e, &[b"A.esm", b"B.esm"], &[b"A.esm", b"B.esm"]);
        // Without a table everything but zero is null.
        assert_eq!(
            e.call(0x0047_1a10, &args![file, 0u32]).ptr::<()>(),
            file.cast()
        );
        assert_eq!(e.call(0x0047_1a10, &args![file, 1u32]).u32(), 0);
        e.call(0x0047_1870, &args![file, list, 0u8]);
        assert_eq!(e.call(0x0047_1a10, &args![file, 1u32]).u32(), files[0]);
        assert_eq!(e.call(0x0047_1a10, &args![file, 2u32]).u32(), files[1]);
        // Past the master count.
        assert_eq!(e.call(0x0047_1a10, &args![file, 3u32]).u32(), 0);
    }

    #[test]
    fn nth_master_name_counts_from_one() {
        let mut e = engine();
        let (file, _, _) = index_setup(&mut e, &[b"A.esm", b"B.esm"], &[]);
        let first = e.call(0x0047_1a60, &args![file, 1u32]).u32();
        let second = e.call(0x0047_1a60, &args![file, 2u32]).u32();
        assert_eq!(e.mem.cstr(first), b"A.esm");
        assert_eq!(e.mem.cstr(second), b"B.esm");
        assert_eq!(e.call(0x0047_1a60, &args![file, 3u32]).u32(), 0);
        // Position 0 behaves as 1.
        assert_eq!(e.call(0x0047_1a60, &args![file, 0u32]).u32(), first);
        let (empty, _, _) = index_setup(&mut e, &[], &[]);
        assert_eq!(e.call(0x0047_1a60, &args![empty, 1u32]).u32(), 0);
    }

    /// A file whose master "Fallout.esm" was recorded with the given size.
    fn size_check_setup(e: &mut Engine, name: &[u8], low: u32, high: u32) -> Ptr<TESFile> {
        let file = new_file(e);
        let master = text(e, name);
        let masters = build_list(e, &[master]);
        let recorded = e.mem.alloc(8);
        e.mem.set_u32(recorded, low);
        e.mem.set_u32(recorded + 4, high);
        let sizes = build_list(e, &[recorded]);
        e.mem.write(file.addr() + 0x3ec, &e.mem.bytes(masters, 8));
        e.mem.write(file.addr() + 0x3f4, &e.mem.bytes(sizes, 8));
        file
    }

    #[test]
    fn size_check_compares_the_recorded_size_with_the_file_on_disk() {
        let mut e = engine();
        let same = size_check_setup(&mut e, b"Fallout.esm", 0x1000, 0);
        assert!(!e.call(0x0047_1af0, &args![same]).bool());
        let grown = size_check_setup(&mut e, b"Fallout.esm", 0x0f00, 0);
        assert!(e.call(0x0047_1af0, &args![grown]).bool());
        let high_differs = size_check_setup(&mut e, b"Fallout.esm", 0x1000, 1);
        assert!(e.call(0x0047_1af0, &args![high_differs]).bool());
    }

    #[test]
    fn size_check_treats_a_missing_file_as_size_zero() {
        let mut e = engine();
        let never_had_one = size_check_setup(&mut e, b"Gone.esm", 0, 0);
        assert!(!e.call(0x0047_1af0, &args![never_had_one]).bool());
        let lost = size_check_setup(&mut e, b"Gone.esm", 0x10, 0);
        assert!(e.call(0x0047_1af0, &args![lost]).bool());
        let empty = new_file(&mut e);
        let sizes = build_list(&mut e, &[]);
        let names = build_list(&mut e, &[]);
        e.mem.write(empty.addr() + 0x3ec, &e.mem.bytes(names, 8));
        e.mem.write(empty.addr() + 0x3f4, &e.mem.bytes(sizes, 8));
        assert!(!e.call(0x0047_1af0, &args![empty]).bool());
    }

    #[test]
    fn flag_getters_and_setters_use_their_bits() {
        let mut e = engine();
        let file = new_file(&mut e);
        // (getter, setter, bit)
        let pairs: [(u32, Option<u32>, u32); 5] = [
            (0x0047_1c20, Some(0x0047_1c50), 0x01),
            (0x0047_1cd0, Some(0x0047_1d00), 0x04),
            (0x0047_1d60, Some(0x0047_1d90), 0x08),
            (0x0047_1de0, Some(0x0047_1e10), 0x40),
            (0x0047_1ca0, None, 0x10),
        ];
        for (getter, setter, bit) in pairs {
            e.set(file, TESFile::m_Flags, 0);
            assert!(!e.call(getter, &args![file]).bool());
            e.set(file, TESFile::m_Flags, bit);
            assert!(e.call(getter, &args![file]).bool());
            e.set(file, TESFile::m_Flags, !bit);
            assert!(!e.call(getter, &args![file]).bool());
            if let Some(setter) = setter {
                e.set(file, TESFile::m_Flags, 0);
                e.call(setter, &args![file, true]);
                assert_eq!(e.get(file, TESFile::m_Flags), bit);
                e.call(setter, &args![file, false]);
                assert_eq!(e.get(file, TESFile::m_Flags), 0);
            }
        }
    }

    #[test]
    fn clearing_bit_four_also_clears_the_active_bit() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_Flags, 0x4 | 0x8 | 0x1);
        e.call(0x0047_1d00, &args![file, false]);
        assert_eq!(e.get(file, TESFile::m_Flags), 0x1);
        // Setting it leaves the others alone.
        e.set(file, TESFile::m_Flags, 0x8);
        e.call(0x0047_1d00, &args![file, true]);
        assert_eq!(e.get(file, TESFile::m_Flags), 0xc);
        // Clearing the active bit alone leaves bit four.
        e.call(0x0047_1d90, &args![file, false]);
        assert_eq!(e.get(file, TESFile::m_Flags), 0x4);
    }

    #[test]
    fn stream_object_is_the_global() {
        let mut e = engine();
        e.set_global(STREAM_OBJECT, 0x4242u32);
        assert_eq!(e.call(0x0047_1f70, &args![]).u32(), 0x4242);
    }

    #[test]
    fn first_header_with_the_expected_tag_needs_no_conversion() {
        let mut e = engine();
        e.register(BSFILE_READ, |e, a| {
            e.mem.set_u32(a[1], TAG_FILE_HEADER);
            ret(0x18)
        });
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::bMustEndianConvert, true);
        e.set(file, TESFile::m_Flags, 0);
        e.call_log = Some(vec![]);
        e.call(0x0047_1e60, &args![file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, FAKE_SEEK), vec![vec![bsfile.addr(), 0, 0]]);
        assert_eq!(e.get(file, TESFile::m_Flags), FLAG_BIT_40);
        assert!(!e.get(file, TESFile::bMustEndianConvert));
        assert!(calls_to(&log, SWAP_U32).is_empty());
    }

    #[test]
    fn first_header_in_the_other_byte_order_is_swapped_and_flagged() {
        let mut e = engine();
        e.register(BSFILE_READ, |e, a| {
            e.mem.set_u32(a[1], TAG_FILE_HEADER.swap_bytes());
            ret(0x18)
        });
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_Flags, FLAG_BIT_40);
        e.call(0x0047_1e60, &args![file]);
        assert_eq!(e.get(current_form(file), Form::form), TAG_FILE_HEADER);
        // Bit 6 became the inverse of the answer, which then differs.
        assert_eq!(e.get(file, TESFile::m_Flags), 0);
        assert!(e.get(file, TESFile::bMustEndianConvert));
    }

    #[test]
    fn first_header_with_an_unknown_tag_leaves_the_bit_alone() {
        let mut e = engine();
        e.register(BSFILE_READ, |e, a| {
            e.mem.set_u32(a[1], 0x0bad_f00d);
            ret(0x18)
        });
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.call(0x0047_1e60, &args![file]);
        // Bit 6 is 0, the answer is 1: they differ.
        assert!(e.get(file, TESFile::bMustEndianConvert));
        assert_eq!(e.get(file, TESFile::m_Flags), 0);
    }

    #[test]
    fn first_header_does_nothing_without_a_file_or_a_read() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0047_1e60, &args![file]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        e.register(BSFILE_READ, |_, _| ret(0));
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::bMustEndianConvert, true);
        e.call(0x0047_1e60, &args![file]);
        assert!(e.get(file, TESFile::bMustEndianConvert));
    }

    #[test]
    fn rewind_goes_back_to_the_start() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        e.register(TYPE_FROM_FORM_TAG, |_, _| ret(7));
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_fileoffset, 100);
        e.set(file, TESFile::m_formoffset, 10);
        e.set(file, TESFile::m_chunkoffset, 4);
        set_tag(&mut e, file, 0x1111, 5, 6);
        e.call_log = Some(vec![]);
        e.call(0x0047_1f80, &args![file, 0u8]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, FAKE_SEEK), vec![vec![bsfile.addr(), 0, 0]]);
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0);
        assert_eq!(e.get(file, TESFile::m_formoffset), 0);
        assert_eq!(e.get(file, TESFile::m_chunkoffset), 0);
        assert_eq!(e.get(current_form(file), Form::form), 0);
        assert!(calls_to(&log, READ_FORM_HEADER).is_empty());
        // Asked to load the first record, it does.
        e.call(0x0047_1f80, &args![file, 1u8]);
        assert_eq!(e.get(current_form(file), Form::form), TAG_RECORD);
    }

    #[test]
    fn rewind_works_without_an_open_file() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_fileoffset, 100);
        e.call_log = Some(vec![]);
        e.call(0x0047_1f80, &args![file, 0u8]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FAKE_SEEK).is_empty());
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0);
    }

    #[test]
    fn first_chunk_of_a_plain_record_is_found_after_its_header() {
        let mut e = engine();
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_fileoffset, 0x100);
        e.set(file, TESFile::m_formoffset, 9);
        e.set(file, TESFile::m_chunkoffset, 9);
        e.set(file, TESFile::m_currentchunkID, 5);
        e.set(file, TESFile::m_actualChunkSize, 5);
        set_tag(&mut e, file, TAG_RECORD, 0x40, 0);
        e.call_log = Some(vec![]);
        e.call(0x0047_2000, &args![file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, FAKE_SEEK),
            vec![vec![bsfile.addr(), 0x118, 0]]
        );
        assert_eq!(e.get(file, TESFile::m_formoffset), 0);
        assert_eq!(e.get(file, TESFile::m_chunkoffset), 0);
        assert_eq!(e.get(file, TESFile::m_currentchunkID), 0);
        assert_eq!(e.get(file, TESFile::m_actualChunkSize), 0);
        assert_eq!(calls_to(&log, GET_TES_CHUNK), vec![vec![file.addr()]]);
    }

    #[test]
    fn first_chunk_of_a_compressed_record_does_not_seek() {
        let mut e = engine();
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        set_tag(&mut e, file, TAG_RECORD, 0x40, RECORD_FLAG_COMPRESSED);
        e.call_log = Some(vec![]);
        e.call(0x0047_2000, &args![file]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FAKE_SEEK).is_empty());
        assert_eq!(calls_to(&log, GET_TES_CHUNK).len(), 1);
    }

    #[test]
    fn bsfile_position_prefers_the_first_copy_unless_it_is_minus_one() {
        let mut e = engine();
        let bsfile = fake_bsfile(&mut e);
        e.mem.set_u32(bsfile.addr() + 0x38, 0x77);
        e.mem.set_u32(bsfile.addr() + 0x150, 0x99);
        assert_eq!(e.call(0x0047_20a0, &args![bsfile]).u32(), 0x77);
        e.mem.set_u32(bsfile.addr() + 0x38, u32::MAX);
        assert_eq!(e.call(0x0047_20a0, &args![bsfile]).u32(), 0x99);
        assert_eq!(e.call(0x0047_2380, &args![bsfile]).u32(), 0x99);
    }

    #[test]
    fn clearing_the_chunk_zeroes_id_and_size() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_currentchunkID, 0x1234);
        e.set(file, TESFile::m_actualChunkSize, 0x55);
        e.call(0x0047_20d0, &args![file]);
        assert_eq!(e.get(file, TESFile::m_currentchunkID), 0);
        assert_eq!(e.get(file, TESFile::m_actualChunkSize), 0);
    }

    #[test]
    fn compressed_means_the_flag_on_a_tag_that_is_not_header_only() {
        let mut e = engine();
        let file = new_file(&mut e);
        set_tag(&mut e, file, TAG_RECORD, 0, 0);
        assert!(!e.call(0x0047_2100, &args![file]).bool());
        set_tag(&mut e, file, TAG_RECORD, 0, RECORD_FLAG_COMPRESSED);
        assert!(e.call(0x0047_2100, &args![file]).bool());
        set_tag(&mut e, file, TAG_HEADER_ONLY, 0, RECORD_FLAG_COMPRESSED);
        assert!(!e.call(0x0047_2100, &args![file]).bool());
    }

    /// A file at the start of a 0x1000-byte stream whose reads produce
    /// records with the given flags in turn.
    fn stream_file(e: &mut Engine, record_flags: &'static [u32]) -> Ptr<TESFile> {
        let mut next = 0;
        e.register_double(READ_FORM_HEADER, move |e, a| {
            let flags = record_flags[next.min(record_flags.len() - 1)];
            next += 1;
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            e.mem.set_u32(a[0] + 0x244, 0x20);
            e.mem.set_u32(a[0] + 0x248, flags);
            ret(1)
        });
        e.register(READ_CHUNK_HEADER, |_, _| Ret::default());
        let file = new_file(e);
        let bsfile = fake_bsfile(e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_filesize, 0x1000);
        set_tag(e, file, TAG_RECORD, 0x20, 0);
        file
    }

    #[test]
    fn next_form_reads_one_record() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0x1000, 0]);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_2150, &args![file, 0u8]).bool());
        let log = e.call_log.take().unwrap();
        // 0x18 + 0x20 bytes on.
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x38);
        assert_eq!(calls_to(&log, READ_FORM_HEADER).len(), 1);
        assert_eq!(calls_to(&log, FREE_DECOMPRESSED_FORM).len(), 1);
        // The record has the skip bit but nobody asked.
        assert_eq!(e.get(current_form(file), Form::flags), 0x1000);
    }

    #[test]
    fn next_form_can_skip_records_with_the_skip_bit() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0x1000, 0x1000, 0]);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_2150, &args![file, 1u8]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, READ_FORM_HEADER).len(), 3);
        assert_eq!(e.get(current_form(file), Form::flags), 0);
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x38 * 3);
    }

    #[test]
    fn next_form_stops_at_the_end_of_the_file() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0]);
        e.set(file, TESFile::m_fileoffset, 0xfd0);
        assert!(!e.call(0x0047_2150, &args![file, 1u8]).bool());
        assert_eq!(e.get(current_form(file), Form::form), 0);
    }

    #[test]
    fn advancing_over_a_header_only_tag_skips_just_the_header() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0]);
        e.set(file, TESFile::m_fileoffset, 0x100);
        set_tag(&mut e, file, TAG_HEADER_ONLY, 0x500, 0);
        assert!(e.call(0x0047_21d0, &args![file]).bool());
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x118);
        set_tag(&mut e, file, TAG_OTHER_HEADER_ONLY, 0x500, 0);
        assert!(e.call(0x0047_21d0, &args![file]).bool());
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x130);
    }

    #[test]
    fn advancing_seeks_and_reads_the_next_header() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0]);
        e.set(file, TESFile::m_fileoffset, 0x100);
        e.set(file, TESFile::m_formoffset, 5);
        e.set(file, TESFile::m_chunkoffset, 5);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_21d0, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        let bsfile = e.get(file, TESFile::m_pFile);
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x138);
        assert_eq!(
            calls_to(&log, FAKE_SEEK),
            vec![vec![bsfile.addr(), 0x138, 0]]
        );
        assert_eq!(e.get(file, TESFile::m_formoffset), 0);
        assert_eq!(e.get(file, TESFile::m_chunkoffset), 0);
    }

    #[test]
    fn advancing_does_not_seek_when_the_file_is_already_there() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0]);
        let bsfile = e.get(file, TESFile::m_pFile);
        e.mem.set_u32(bsfile.addr() + 0x150, 0x38);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_21d0, &args![file]).bool());
        assert!(calls_to(&e.call_log.take().unwrap(), FAKE_SEEK).is_empty());
    }

    #[test]
    fn advancing_reports_a_failed_seek() {
        let mut e = engine();
        let file = stream_file(&mut e, &[0]);
        e.mem.set_u32(CONFIG_SEEK_RESULT, u32::MAX);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_21d0, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        let complaint = calls_to(&log, LOG);
        assert_eq!(complaint[0][0], MESSAGE_NEXT_FORM_SEEK_FAILED);
        assert_eq!(e.mem.cstr(complaint[0][1]), b"boom");
        assert_eq!(calls_to(&log, LOCAL_FREE), vec![vec![complaint[0][1]]]);
        let format = calls_to(&log, FORMAT_MESSAGE);
        assert_eq!(&format[0][..4], &[0x1300, 0, 5, 0x400]);
    }

    #[test]
    fn advancing_reports_a_record_that_cannot_be_read() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |_, _| ret(0));
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_filesize, 0x1000);
        set_name(&mut e, file, b"Data\\", b"Mod.esp");
        set_tag(&mut e, file, TAG_RECORD, 0x20, 0);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_21d0, &args![file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, LOG),
            vec![vec![MESSAGE_NEXT_FORM_BAD_FORM, file.addr() + 0x20, 0x38]]
        );
    }

    #[test]
    fn set_offset_seeks_and_loads_the_record_there() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        e.register(TYPE_FROM_FORM_TAG, |_, _| ret(7));
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_filesize, 0x1000);
        e.set(file, TESFile::m_formoffset, 8);
        e.set(file, TESFile::m_chunkoffset, 8);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_23a0, &args![file, 0x200u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x200);
        assert_eq!(
            calls_to(&log, FAKE_SEEK),
            vec![vec![bsfile.addr(), 0x200, 0]]
        );
        assert_eq!(e.get(file, TESFile::m_formoffset), 0);
        assert_eq!(e.get(current_form(file), Form::form), TAG_RECORD);
        assert_eq!(calls_to(&log, FREE_DECOMPRESSED_FORM).len(), 1);
    }

    #[test]
    fn set_offset_refuses_the_end_of_the_file() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_filesize, 0x1000);
        set_tag(&mut e, file, TAG_RECORD, 1, 2);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_23a0, &args![file, 0x1000u32]).bool());
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FAKE_SEEK).is_empty());
        assert_eq!(e.get(current_form(file), Form::form), 0);
        assert_eq!(e.get(file, TESFile::m_fileoffset), 0x1000);
    }

    #[test]
    fn set_offset_reports_a_failed_seek() {
        let mut e = engine();
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_filesize, 0x1000);
        e.mem.set_u32(CONFIG_SEEK_RESULT, u32::MAX);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_23a0, &args![file, 0x10u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LOG)[0][0], MESSAGE_SET_OFFSET_SEEK_FAILED);
        assert_eq!(e.get(file, TESFile::m_fileoffset), u32::MAX);
    }

    #[test]
    fn cursor_offset_is_file_offset_plus_header_plus_form_offset() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_fileoffset, 0x100);
        e.set(file, TESFile::m_formoffset, 0x23);
        assert_eq!(e.call(0x0047_24c0, &args![file]).u32(), 0x13b);
    }

    #[test]
    fn set_offset_chunk_refuses_a_compressed_record() {
        let mut e = engine();
        let file = new_file(&mut e);
        set_tag(&mut e, file, TAG_RECORD, 0x100, RECORD_FLAG_COMPRESSED);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_24f0, &args![file, 0x120u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, LOG),
            vec![vec![MESSAGE_SET_OFFSET_CHUNK_COMPRESSED]]
        );
        assert!(calls_to(&log, FAKE_SEEK).is_empty());
    }

    #[test]
    fn set_offset_chunk_moves_inside_the_record() {
        let mut e = engine();
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        e.set(file, TESFile::m_fileoffset, 0x100);
        e.set(file, TESFile::m_chunkoffset, 3);
        e.set(file, TESFile::m_currentchunkID, 9);
        set_tag(&mut e, file, TAG_RECORD, 0x100, 0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_24f0, &args![file, 0x150u32]).bool());
        let log = e.call_log.take().unwrap();
        // 0x150 - 0x100 - 0x18.
        assert_eq!(e.get(file, TESFile::m_formoffset), 0x38);
        assert_eq!(
            calls_to(&log, FAKE_SEEK),
            vec![vec![bsfile.addr(), 0x150, 0]]
        );
        assert_eq!(e.get(file, TESFile::m_chunkoffset), 0);
        assert_eq!(e.get(file, TESFile::m_currentchunkID), 0);
        assert_eq!(calls_to(&log, GET_TES_CHUNK).len(), 1);
    }

    #[test]
    fn set_offset_chunk_refuses_an_offset_outside_the_record() {
        let mut e = engine();
        let file = new_file(&mut e);
        e.set(file, TESFile::m_fileoffset, 0x100);
        set_tag(&mut e, file, TAG_RECORD, 0x20, 0);
        e.set(file, TESFile::m_currentchunkID, 9);
        e.call_log = Some(vec![]);
        // 0x100 + 0x18 + 0x20 is one past the last byte.
        assert!(!e.call(0x0047_24f0, &args![file, 0x138u32]).bool());
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FAKE_SEEK).is_empty());
        assert_eq!(e.get(current_form(file), Form::form), 0);
        assert_eq!(e.get(file, TESFile::m_currentchunkID), 0);
        // An offset before the record wraps and is refused too.
        set_tag(&mut e, file, TAG_RECORD, 0x20, 0);
        assert!(!e.call(0x0047_24f0, &args![file, 0x10u32]).bool());
    }

    #[test]
    fn set_offset_chunk_reports_a_failed_seek() {
        let mut e = engine();
        let file = new_file(&mut e);
        let bsfile = fake_bsfile(&mut e);
        e.set(file, TESFile::m_pFile, bsfile);
        set_tag(&mut e, file, TAG_RECORD, 0x100, 0);
        e.mem.set_u32(CONFIG_SEEK_RESULT, u32::MAX);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_24f0, &args![file, 0x20u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, LOG)[0][0],
            MESSAGE_SET_OFFSET_CHUNK_SEEK_FAILED
        );
    }

    #[test]
    fn form_without_return_loads_header_and_chunk_only_when_empty() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(READ_CHUNK_HEADER, |_, _| Ret::default());
        let file = new_file(&mut e);
        e.set(file, TESFile::m_currentchunkID, 5);
        e.call_log = Some(vec![]);
        e.call(0x0047_2620, &args![file]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(current_form(file), Form::form), TAG_RECORD);
        assert_eq!(e.get(file, TESFile::m_currentchunkID), 0);
        assert_eq!(calls_to(&log, READ_CHUNK_HEADER).len(), 1);
        // Already loaded: nothing is read.
        e.call_log = Some(vec![]);
        e.call(0x0047_2620, &args![file]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn form_without_return_stops_when_the_header_cannot_be_read() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |_, _| ret(0));
        e.register(READ_CHUNK_HEADER, |_, _| Ret::default());
        let file = new_file(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0047_2620, &args![file]);
        assert!(calls_to(&e.call_log.take().unwrap(), READ_CHUNK_HEADER).is_empty());
    }

    #[test]
    fn form_type_comes_from_the_record_tag() {
        let mut e = engine();
        e.register(TYPE_FROM_FORM_TAG, |_, a| ret(a[0] & 0xff));
        e.register(READ_FORM_HEADER, |e, a| {
            e.mem.set_u32(a[0] + 0x240, TAG_RECORD);
            ret(1)
        });
        e.register(GET_TES_CHUNK, |_, _| Ret::default());
        let file = new_file(&mut e);
        // Already loaded.
        set_tag(&mut e, file, 0x1234_5603, 0, 0);
        assert_eq!(e.call(0x0047_2660, &args![file]).u32(), 3);
        // Not loaded: read first.
        e.set(current_form(file), Form::form, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0047_2660, &args![file]).u32(), TAG_RECORD & 0xff);
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), GET_TES_CHUNK).len(),
            1
        );
    }

    #[test]
    fn form_type_is_zero_when_no_record_can_be_read() {
        let mut e = engine();
        e.register(READ_FORM_HEADER, |_, _| ret(0));
        e.register(TYPE_FROM_FORM_TAG, |_, _| ret(99));
        let file = new_file(&mut e);
        assert_eq!(e.call(0x0047_2660, &args![file]).u32(), 0);
    }
}
