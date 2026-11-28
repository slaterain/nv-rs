//! `PackageTarget` (Xbox PDB; the unit `fallout shared/packagetarget` and its type getter at
//! `00519b00`): what an AI package acts on. A target (0x10 bytes) has its type as a byte at +0 (0
//! a reference, 1 a base object, 2 an object type), a word at +4 whose meaning follows the type,
//! a value at +8 and the acquire radius (an `f32`) at +0xc. Types past 2 never touch the word at
//! +4.
//!
//! **Native.** The unit's getters and word setters are done in place inside its own functions (a
//! copy takes the type through the type setter, a typed call, then the word at +4 for types 0 to
//! 2, the value and the radius), and so are the one-line bodies the exe calls: the value's
//! getter and setter (`0044ddc0`, `00403550`), the radius's (`0084d030`, `0063f790`), a form's id (`0084e3a0`), the big-endian byte
//! (`00401500`, `011c54ba`), the chunk's zeroing (`00403d30`, `memset` of the 0x10 bytes) and the
//! three-word swap (`0067ff10`, this family's). The form lookup by id, `TESForm::GetFile`,
//! `AddCompileIndex` and the chunk's write are this family's typed calls, the save buffer's form id
//! the save family's. What stays a call by address: the RTTI cast, the owner's name test
//! (`00474cb0`), the loader's warning (which a fix may replace), the old save object's byte and id
//! writers and readers, the save buffer's writer, the load buffer's reader and form id reader;
//! the owner's name is asked through its slot.
//!
//! Floats: the radius is only moved, but every move the exe makes goes through the x87 stack
//! (`FLD`, `FSTP`), which stores a signalling NaN quiet; the compare is the exe's `FUCOMPP`
//! (an unordered pair differs, 0.0 equals -0.0).

use super::{schedule, tesform, FAMILY};
use crate::save;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine, Ptr};

/// A target's fields.
const TYPE: u32 = 0;
const HELD: u32 = 4;
const VALUE: u32 = 8;
const RADIUS: u32 = 0xc;
const SIZE: u32 = 0x10;
/// A form's id.
const FORM_ID: u32 = 0xc;
/// The records' byte order flag (the files are big-endian).
const BIG_ENDIAN: u32 = 0x011c_54ba;

/// `__RTDynamicCast` (cdecl) and the two type descriptors `InitItem` hands it.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// The owner's name test (`this` = owner) and its name slot; the loader's warning (cdecl), which a
/// fix may replace.
const OWNER_HAS_NAME: u32 = 0x0047_4cb0;
const SLOT_NAME: u32 = 0x130;
const WARNING: u32 = 0x005b_5e40;
/// `InitItem`'s messages for a reference and for an object: with the owner's name, with its id,
/// without an owner.
const TEXTS_REFERENCE: [u32; 3] = [0x0106_9198, 0x0106_9140, 0x0106_90f0];
const TEXTS_OBJECT: [u32; 3] = [0x0106_90a0, 0x0106_9050, 0x0106_9000];
/// The old save object (a global pointer) and its byte and numeric id writers and readers
/// (`this`; data, size).
const SAVE_OBJECT: u32 = 0x011d_e45c;
const SAVE_BYTES: u32 = 0x0085_79b0;
const LOAD_BYTES: u32 = 0x0085_79e0;
const SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
/// The save buffer's writer (`this`; data, size, 0), the load buffer's reader (`this`; out, size)
/// and form id reader (`this`; out).
const BUFFER_WRITE: u32 = 0x0086_5e50;
const BUFFER_READ: u32 = 0x0086_4980;
const BUFFER_READ_FORM_ID: u32 = 0x0086_48e0;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

fn kind(e: &Engine, this: u32) -> u8 {
    e.mem.u8(at(this, TYPE))
}

/// The word at +4 when the type is `wanted`, else 0 (the three getters).
fn held_as(e: &Engine, this: u32, wanted: u8) -> u32 {
    if kind(e, this) == wanted {
        e.mem.u32(at(this, HELD))
    } else {
        0
    }
}

/// The word at +4 stored when the type is `wanted` (the three setters).
fn store_as(e: &Engine, this: u32, wanted: u8, word: u32) {
    if kind(e, this) == wanted {
        e.mem.set_u32(at(this, HELD), word);
    }
}

/// The value and the radius taken from `source` (the radius through the x87 stack).
fn copy_value_and_radius(e: &Engine, this: u32, source: u32) {
    let value = e.mem.u32(at(source, VALUE));
    e.mem.set_u32(at(this, VALUE), value);
    let radius = e.mem.u32(at(source, RADIUS));
    e.mem.set_u32(at(this, RADIUS), x87_quiet_bits(radius));
}

// Typed from 00519b00
/// `PackageTarget::GetTargType` (Xbox PDB): the type byte.
pub fn targ_type(e: &mut Engine, this: u32) -> u8 {
    kind(e, this)
}

// Typed from 0067ff70
/// The constructor: type 2 (an object type), the word, the value and the radius 0. Returns
/// `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u8(at(this, TYPE), 2);
    e.mem.set_u32(at(this, HELD), 0);
    e.mem.set_u32(at(this, VALUE), 0);
    e.mem.set_u32(at(this, RADIUS), 0);
    this
}

// Typed from 0067ffa0
/// The assignment: `other`'s type ([`set_targ_type`]), value and radius, then its word at +4 when the
/// type is 0 to 2. Returns `this`.
pub fn assign(e: &mut Engine, this: u32, other: u32) -> u32 {
    let other_kind = kind(e, other);
    typed_call!(
        FAMILY,
        0x0068_00b0,
        set_targ_type(e, this, u32::from(other_kind))
    );
    copy_value_and_radius(e, this, other);
    if kind(e, this) <= 2 {
        let word = e.mem.u32(at(other, HELD));
        e.mem.set_u32(at(this, HELD), word);
    }
    this
}

// Typed from 00680020
/// `PackageTarget::GetTargReference` (Xbox PDB): the word at +4 for type 0, else 0.
pub fn targ_reference(e: &mut Engine, this: u32) -> u32 {
    held_as(e, this, 0)
}

// Typed from 00680050
/// `PackageTarget::GetTargObject` (Xbox PDB): the word at +4 for type 1, else 0.
pub fn targ_object(e: &mut Engine, this: u32) -> u32 {
    held_as(e, this, 1)
}

// Typed from 00680080
/// `PackageTarget::GetTargObjectType` (Xbox PDB): the word at +4 for type 2, else 0.
pub fn targ_object_type(e: &mut Engine, this: u32) -> u32 {
    held_as(e, this, 2)
}

// Typed from 006800b0
/// `PackageTarget::SetTargType` (Xbox PDB): when the type (compared as the whole word) differs,
/// its low byte stored and, for the types 0 to 2, the word at +4 cleared.
pub fn set_targ_type(e: &mut Engine, this: u32, new_kind: u32) {
    if u32::from(kind(e, this)) != new_kind {
        e.mem.set_u8(at(this, TYPE), new_kind.to_le_bytes()[0]);
        if kind(e, this) <= 2 {
            e.mem.set_u32(at(this, HELD), 0);
        }
    }
}

// Typed from 00680110
/// `PackageTarget::SetTargReference` (Xbox PDB): the word at +4 for type 0.
pub fn set_targ_reference(e: &mut Engine, this: u32, word: u32) {
    store_as(e, this, 0, word);
}

// Typed from 00680140
/// The object's setter: the word at +4 for type 1.
pub fn set_targ_object(e: &mut Engine, this: u32, word: u32) {
    store_as(e, this, 1, word);
}

// Typed from 00680170
/// The object type's setter: the word at +4 for type 2.
pub fn set_targ_object_type(e: &mut Engine, this: u32, word: u32) {
    store_as(e, this, 2, word);
}

// Typed from 006801a0
/// `PackageTarget::Copy` (Xbox PDB): nothing from a null source; else its type, then (the types
/// now being equal) its word at +4 for the types 0 to 2, its value and its radius.
pub fn copy(e: &mut Engine, this: u32, source: u32) {
    if source == 0 {
        return;
    }
    let source_kind = kind(e, source);
    typed_call!(
        FAMILY,
        0x0068_00b0,
        set_targ_type(e, this, u32::from(source_kind))
    );
    if kind(e, this) <= 2 {
        let word = e.mem.u32(at(source, HELD));
        e.mem.set_u32(at(this, HELD), word);
    }
    copy_value_and_radius(e, this, source);
}

// Typed from 00680220
/// `PackageTarget::Compare` (Xbox PDB): 0 when `other` is there and has the same type, word at +4
/// (for the types 0 to 2), value and radius (the x87 compare: an unordered pair differs), else 1.
pub fn compare(e: &mut Engine, this: u32, other: u32) -> u8 {
    if other == 0 || kind(e, this) != kind(e, other) {
        return 1;
    }
    if kind(e, this) <= 2 && e.mem.u32(at(this, HELD)) != e.mem.u32(at(other, HELD)) {
        return 1;
    }
    if e.mem.u32(at(this, VALUE)) != e.mem.u32(at(other, VALUE)) {
        return 1;
    }
    let mine = f32::from_bits(e.mem.u32(at(this, RADIUS)));
    let theirs = f32::from_bits(e.mem.u32(at(other, RADIUS)));
    u8::from(mine != theirs)
}

/// The form's id for the save functions: 0 for none.
fn form_id_of(e: &Engine, form: u32) -> u32 {
    if form == 0 {
        0
    } else {
        e.mem.u32(at(form, FORM_ID))
    }
}

// Typed from 006802f0
/// `PackageTarget::Save` (Xbox PDB): the target written as its 0x10-byte chunk of `record`: the
/// type byte, the value, the radius and, at +4, the form's id (types 0 and 1, 0 for none) or the
/// object type (type 2); the words byte-swapped around the write when the records are
/// big-endian.
pub fn save(e: &mut Engine, this: u32, record: u32) {
    e.with_stack(SIZE, |e, chunk| {
        let chunk = chunk.addr();
        for offset in [0, 4, 8, 0xc] {
            e.mem.set_u32(at(chunk, offset), 0);
        }
        let value = e.mem.u32(at(this, VALUE));
        e.mem.set_u32(at(chunk, VALUE), value);
        let radius = e.mem.u32(at(this, RADIUS));
        e.mem.set_u32(at(chunk, RADIUS), x87_quiet_bits(radius));
        let target_kind = kind(e, this);
        e.mem.set_u8(at(chunk, TYPE), target_kind);
        match target_kind {
            0 | 1 => {
                let form = e.mem.u32(at(this, HELD));
                if form != 0 {
                    let id = e.mem.u32(at(form, FORM_ID));
                    e.mem.set_u32(at(chunk, HELD), id);
                }
            }
            2 => {
                let object_type = e.mem.u32(at(this, HELD));
                e.mem.set_u32(at(chunk, HELD), object_type);
            }
            _ => {}
        }
        if e.mem.u8(BIG_ENDIAN) != 0 {
            typed_call!(FAMILY, 0x0067_ff10, schedule::swap_three_words(e, chunk));
        }
        typed_call!(
            FAMILY,
            0x0048_5990,
            tesform::add_chunk_data(e, record, chunk, SIZE)
        );
        if e.mem.u8(BIG_ENDIAN) != 0 {
            typed_call!(FAMILY, 0x0067_ff10, schedule::swap_three_words(e, chunk));
        }
    });
}

/// `InitItem`'s warning for a form not found: the owner's name when it has one, else its id;
/// without an owner the form's id alone.
fn warn_missing(e: &mut Engine, owner: u32, id: u32, texts: [u32; 3]) {
    if owner == 0 {
        e.call(WARNING, &args![texts[2], id]);
    } else if e.call(OWNER_HAS_NAME, &args![owner]).u32() != 0 {
        let name = e.vcall(owner, SLOT_NAME, &args![]).u32();
        e.call(WARNING, &args![texts[0], id, name]);
    } else {
        let owner_id = e.mem.u32(at(owner, FORM_ID));
        e.call(WARNING, &args![texts[1], id, owner_id]);
    }
}

// Typed from 006803b0
/// `PackageTarget::InitItem` (Xbox PDB): the form id the record held, given `owner`'s file's
/// compile index (`TESForm::GetFile`, `AddCompileIndex`), then for type 0 the form it names cast
/// to a reference, for type 1 the form itself; one not found stored as none and warned of.
pub fn init_item(e: &mut Engine, this: u32, owner: u32) {
    let target_kind = kind(e, this);
    let stored = e.mem.u32(at(this, HELD));
    let file = typed_call!(FAMILY, 0x0048_4e60, tesform::get_file(e, owner, -1));
    let id = e.with_stack(4, |e, slot| {
        let slot = slot.addr();
        e.mem.set_u32(slot, stored);
        typed_call!(
            FAMILY,
            0x0048_5d50,
            tesform::add_compile_index(e, slot, file)
        );
        e.mem.u32(slot)
    });
    let (found, texts) = match target_kind {
        0 => {
            let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id));
            let reference = e
                .call(
                    DYNAMIC_CAST,
                    &args![form, 0u32, TYPE_TES_FORM, TYPE_TES_OBJECT_REFR, 0u32],
                )
                .u32();
            (reference, TEXTS_REFERENCE)
        }
        1 => (
            typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id)),
            TEXTS_OBJECT,
        ),
        _ => return,
    };
    e.mem.set_u32(at(this, HELD), found);
    if found == 0 {
        warn_missing(e, owner, id, texts);
    }
}

// Typed from 00680520
/// The old save's size of the target: 13 bytes for the types 0 to 2, else 9.
pub fn save_size(e: &mut Engine, this: u32) -> u16 {
    if kind(e, this) <= 2 {
        13
    } else {
        9
    }
}

/// The old save object (read at each use, as the exe does).
fn save_object(e: &Engine) -> u32 {
    e.mem.u32(SAVE_OBJECT)
}

// Typed from 00680590
/// The old save: the type byte, the value and the radius, then the form's id (types 0 and 1, 0
/// for none) as a numeric id or the object type's word (type 2).
pub fn save_old(e: &mut Engine, this: u32) {
    for (offset, size) in [(TYPE, 1u32), (VALUE, 4), (RADIUS, 4)] {
        let object = save_object(e);
        e.call(SAVE_BYTES, &args![object, at(this, offset), size]);
    }
    match kind(e, this) {
        0 | 1 => {
            let id = form_id_of(e, e.mem.u32(at(this, HELD)));
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), id);
                let object = save_object(e);
                e.call(SAVE_NUMERIC_ID, &args![object, slot, 4u32]);
            });
        }
        2 => {
            let object = save_object(e);
            e.call(SAVE_BYTES, &args![object, at(this, HELD), 4u32]);
        }
        _ => {}
    }
}

// Typed from 00680640
/// The old load: the type byte, the value and the radius, then a numeric id into the word at +4
/// (types 0 and 1) or the object type's word (type 2).
pub fn load_old(e: &mut Engine, this: u32) {
    for (offset, size) in [(TYPE, 1u32), (VALUE, 4), (RADIUS, 4)] {
        let object = save_object(e);
        e.call(LOAD_BYTES, &args![object, at(this, offset), size]);
    }
    match kind(e, this) {
        0 | 1 => {
            let id = e.with_stack(4, |e, slot| {
                let object = save_object(e);
                e.call(LOAD_NUMERIC_ID, &args![object, slot, 4u32]);
                e.mem.u32(slot.addr())
            });
            e.mem.set_u32(at(this, HELD), id);
        }
        2 => {
            let object = save_object(e);
            e.call(LOAD_BYTES, &args![object, at(this, HELD), 4u32]);
        }
        _ => {}
    }
}

// Typed from 006806e0
/// After the old load: for the types 0 and 1 the form id at +4 replaced by the form it names
/// (none when none).
pub fn after_load(e: &mut Engine, this: u32) {
    if kind(e, this) < 2 {
        let id = e.mem.u32(at(this, HELD));
        let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id));
        e.mem.set_u32(at(this, HELD), form);
    }
}

// Typed from 00680720
/// `PackageTarget::SaveGame` (Xbox PDB): the type byte, the value and the radius into `buffer`,
/// then the form (types 0 and 1, the save family's form id) or the object type's word (type 2).
pub fn save_game(e: &mut Engine, this: u32, buffer: u32) {
    for (offset, size) in [(TYPE, 1u32), (VALUE, 4), (RADIUS, 4)] {
        e.call(BUFFER_WRITE, &args![buffer, at(this, offset), size, 0u32]);
    }
    match kind(e, this) {
        0 | 1 => {
            let form = e.mem.u32(at(this, HELD));
            typed_call!(
                save::FAMILY,
                0x0086_5df0,
                save::refs::save_form_id(e, Ptr::new(buffer), Ptr::new(form), 0)
            );
        }
        2 => {
            e.call(BUFFER_WRITE, &args![buffer, at(this, HELD), 4u32, 0u32]);
        }
        _ => {}
    }
}

// Typed from 006807b0
/// `PackageTarget::LoadGame` (Xbox PDB): the type byte, the value and the radius from `buffer`,
/// then the form id into the word at +4 (types 0 and 1) or the object type's word (type 2).
pub fn load_game(e: &mut Engine, this: u32, buffer: u32) {
    for (offset, size) in [(TYPE, 1u32), (VALUE, 4), (RADIUS, 4)] {
        e.call(BUFFER_READ, &args![buffer, at(this, offset), size]);
    }
    match kind(e, this) {
        0 | 1 => {
            e.call(BUFFER_READ_FORM_ID, &args![buffer, at(this, HELD)]);
        }
        2 => {
            e.call(BUFFER_READ, &args![buffer, at(this, HELD), 4u32]);
        }
        _ => {}
    }
}

// Typed from 00680830
/// `PackageTarget::InitLoadGame` (Xbox PDB; the stack word is never read): for the types 0 and 1
/// the loaded form id at +4 replaced by its form (0 stays 0).
pub fn init_load_game(e: &mut Engine, this: u32, _unused: u32) {
    if kind(e, this) < 2 {
        let id = e.mem.u32(at(this, HELD));
        let form = if id != 0 {
            typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id))
        } else {
            0
        };
        e.mem.set_u32(at(this, HELD), form);
    }
}
