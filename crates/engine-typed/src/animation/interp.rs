//! The key-based interpolators: `NiTransformInterpolator::Update` (a bone's translation, rotation
//! and scale from its `NiTransformData`'s keys, the per-bone work of every animation every frame)
//! with the data's three key getters, and `NiFloatInterpolator::Update` (a float from
//! `NiFloatData`'s keys: the face morphs, material and alpha controllers). The keys' search and
//! interpolation run in place through the animation keys family's native code ([`anim_keys`]), at
//! the running thread's precision.
//!
//! Fields as the code reads them (Xbox PDB): `NiTransformInterpolator`'s last time (+8), its
//! transform value (`NiQuatTransform`: translation +0xc, rotation +0x18 as w, x, y, z, scale
//! +0x28), its data (+0x2c), the last translation, rotation and scale key indices (+0x30, +0x32,
//! +0x34, `u16`) and the pose flag (+0x44); `NiTransformData`'s rotation keys (count `u16` +8, type
//! +0x10, key size +0x1c, keys +0x20), translation keys (+0xa, +0x14, +0x1d, +0x24) and scale keys
//! (+0xc, +0x18, +0x1e, +0x28). `NiFloatInterpolator`'s last time (+8), value (+0xc), data (+0x10)
//! and last index (+0x14); `NiFloatData`'s key count (+8), keys (+0xc), type (+0x10) and key size
//! (+0x14).

use crate::anim_keys::{float_keys, pos_keys, rot_keys};
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::Engine;

/// `NiTransformInterpolator`'s fields.
const LAST_TIME: u32 = 0x8;
const TRANSFORM: u32 = 0xc;
const TRANSLATE: u32 = 0xc;
const ROTATE: u32 = 0x18;
const ROTATE_X: u32 = 0x1c;
const SCALE: u32 = 0x28;
const DATA: u32 = 0x2c;
const LAST_TRANSLATE_INDEX: u32 = 0x30;
const LAST_ROTATE_INDEX: u32 = 0x32;
const LAST_SCALE_INDEX: u32 = 0x34;
const POSE: u32 = 0x44;
/// The unset part's value (`-FLT_MAX`, the `double` at `010241b0`).
const UNSET: f32 = f32::MIN;
/// `NiInterpolator::INVALID_FLOAT` (`0109a918`, `-FLT_MAX`): a float interpolator's value with
/// no keys.
const INVALID_FLOAT: f32 = f32::MIN;

/// One key list of an `NiTransformData`: where its count (`u16`), type, key size (byte) and keys
/// are.
#[derive(Clone, Copy)]
struct KeyList {
    count: u32,
    key_type: u32,
    size: u32,
    keys: u32,
}

const TRANSLATE_KEYS: KeyList = KeyList {
    count: 0xa,
    key_type: 0x14,
    size: 0x1d,
    keys: 0x24,
};
const ROTATE_KEYS: KeyList = KeyList {
    count: 0x8,
    key_type: 0x10,
    size: 0x1c,
    keys: 0x20,
};
const SCALE_KEYS: KeyList = KeyList {
    count: 0xc,
    key_type: 0x18,
    size: 0x1e,
    keys: 0x28,
};

/// A key list's count, type, size and keys as the getters give them (all zero without data).
fn key_list(e: &Engine, data: u32, list: KeyList) -> (u32, u32, u8, u32) {
    if data == 0 {
        return (0, 0, 0, 0);
    }
    (
        u32::from(e.mem.u16(data.wrapping_add(list.count))),
        e.mem.u32(data.wrapping_add(list.key_type)),
        e.mem.u8(data.wrapping_add(list.size)),
        e.mem.u32(data.wrapping_add(list.keys)),
    )
}

/// The getters' shape (`thiscall(this, &count, &type, &size)`, the keys answered): the
/// interpolator's data (+0x2c) read, its list's count (zero-extended), type and size byte stored
/// (zeros without data).
fn get_keys(e: &mut Engine, this: Ptr, list: KeyList, count: Ptr, kind: Ptr, size: Ptr) -> u32 {
    let data = e.mem.u32(this.addr().wrapping_add(DATA));
    let (n, key_type, key_size, keys) = key_list(e, data, list);
    e.mem.set_u32(count.addr(), n);
    e.mem.set_u32(kind.addr(), key_type);
    e.mem.set_u8(size.addr(), key_size);
    keys
}

// Typed from 00a3f990
/// The translation keys of an interpolator's `NiTransformData` (`thiscall(this, &count, &type,
/// &size)`, the keys answered; the engine map's folded name `NiQuatTransform::IsTransformInvalid`):
/// with data (+0x2c) its `u16` count at +0xa, type at +0x14 and size byte at +0x1d stored and its
/// keys at +0x24 answered; without, zeros.
pub fn translate_keys(e: &mut Engine, this: Ptr, count: Ptr, kind: Ptr, size: Ptr) -> u32 {
    get_keys(e, this, TRANSLATE_KEYS, count, kind, size)
}

// Typed from 0058cb00
/// The rotation keys (`thiscall(this, &count, &type, &size)`; the engine map's folded name
/// `NiQuatTransform::SetTranslate`): with data its `u16` count at +8, type at +0x10 and size byte
/// at +0x1c stored and its keys at +0x20 answered (`0058cb60` on the data); without, zeros.
pub fn rotate_keys(e: &mut Engine, this: Ptr, count: Ptr, kind: Ptr, size: Ptr) -> u32 {
    get_keys(e, this, ROTATE_KEYS, count, kind, size)
}

// Typed from 00a3f9e0
/// The scale keys (`thiscall(this, &count, &type, &size)`; the engine map's folded name
/// `NiQuatTransform::SetRotate`): with data its `u16` count at +0xc, type at +0x18 and size byte at
/// +0x1e stored and its keys at +0x28 answered; without, zeros.
pub fn scale_keys(e: &mut Engine, this: Ptr, count: Ptr, kind: Ptr, size: Ptr) -> u32 {
    get_keys(e, this, SCALE_KEYS, count, kind, size)
}

/// The low 16 bits of an index (the exe keeps the last key indices as `u16`).
fn low16(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// Copies the transform value (eight words) to `to`.
fn copy_transform(e: &mut Engine, this: u32, to: u32) {
    for i in 0..8u32 {
        let word = e.mem.u32(this.wrapping_add(TRANSFORM + 4 * i));
        e.mem.set_u32(to.wrapping_add(4 * i), word);
    }
}

/// Whether the scale, the rotation's `X` and the translation's first float are all unset.
fn transform_unset(e: &Engine, this: u32) -> bool {
    let unset = |offset: u32| f32::from_bits(e.mem.u32(this.wrapping_add(offset))) == UNSET;
    unset(SCALE) && unset(ROTATE_X) && unset(TRANSLATE)
}

/// The three parts from their keys at precision `R`, each stored with its last index as it is done.
fn interpolate_parts<R: Real>(e: &mut Engine, this: u32, time: f32) {
    let data = e.mem.u32(this.wrapping_add(DATA));
    let index_at = |offset: u32| this.wrapping_add(offset);
    let (n, key_type, size, keys) = key_list(e, data, TRANSLATE_KEYS);
    if n != 0 {
        let mut index = u32::from(e.mem.u16(index_at(LAST_TRANSLATE_INDEX)));
        let size = u32::from(size);
        let point = pos_keys::point::<R>(e, time, keys, key_type, n, &mut index, size);
        for (i, word) in (0u32..).zip(point) {
            e.mem.set_u32(this.wrapping_add(TRANSLATE + 4 * i), word);
        }
        e.mem.set_u16(index_at(LAST_TRANSLATE_INDEX), low16(index));
    }
    let (n, key_type, size, keys) = key_list(e, data, ROTATE_KEYS);
    if n != 0 {
        let mut index = u32::from(e.mem.u16(index_at(LAST_ROTATE_INDEX)));
        let q = rot_keys::rotation(e, time, keys, key_type, n, &mut index, size);
        for (i, word) in (0u32..).zip(q) {
            e.mem.set_u32(this.wrapping_add(ROTATE + 4 * i), word);
        }
        e.mem.set_u16(index_at(LAST_ROTATE_INDEX), low16(index));
    }
    let (n, key_type, size, keys) = key_list(e, data, SCALE_KEYS);
    if n != 0 {
        let mut index = u32::from(e.mem.u16(index_at(LAST_SCALE_INDEX)));
        let size = u32::from(size);
        let scale = float_keys::value::<R>(e, time, keys, key_type, n, &mut index, size);
        e.mem.set_u32(this.wrapping_add(SCALE), scale);
        e.mem.set_u16(index_at(LAST_SCALE_INDEX), low16(index));
    }
}

// Typed from 00a3fdb0
/// `NiTransformInterpolator::Update` (Xbox PDB; `thiscall(this, time, target, &transform)`, `RET
/// 0xc`; the target is not read): a pose copies its transform value out after taking `time` as its
/// last time. At the last time the value is copied out unchanged. Otherwise the translation,
/// rotation and scale are interpolated from their keys (`NiPosKey`, `NiRotKey`,
/// `NiFloatKey::GenInterp`, each from its last key index, which it updates; skipped for a list
/// without keys), done in place, and the value copied out. Answers 0 when the scale, the rotation's
/// `X` and the translation's first float are all `-FLT_MAX` (an interpolation then does not take
/// the time), 1 otherwise.
pub fn transform_update(e: &mut Engine, this: Ptr, time: f32, _target: u32, value: Ptr) -> bool {
    let t = this.addr();
    if e.mem.u8(t.wrapping_add(POSE)) != 0 {
        e.mem.set_u32(t.wrapping_add(LAST_TIME), time.to_bits());
        copy_transform(e, t, value.addr());
        return true;
    }
    if time == f32::from_bits(e.mem.u32(t.wrapping_add(LAST_TIME))) {
        copy_transform(e, t, value.addr());
        return !transform_unset(e, t);
    }
    if float_precision() {
        interpolate_parts::<f32>(e, t, time);
    } else {
        interpolate_parts::<f64>(e, t, time);
    }
    copy_transform(e, t, value.addr());
    if transform_unset(e, t) {
        return false;
    }
    e.mem.set_u32(t.wrapping_add(LAST_TIME), time.to_bits());
    true
}

/// `NiFloatInterpolator`'s fields and `NiFloatData`'s.
const FLOAT_LAST_TIME: u32 = 0x8;
const FLOAT_VALUE: u32 = 0xc;
const FLOAT_DATA: u32 = 0x10;
const FLOAT_LAST_INDEX: u32 = 0x14;
const DATA_COUNT: u32 = 0x8;
const DATA_KEYS: u32 = 0xc;
const DATA_TYPE: u32 = 0x10;
const DATA_SIZE: u32 = 0x14;

// Typed from 00a3c6d0
/// `NiFloatInterpolator::Update` (Xbox PDB; `thiscall(this, time, target, &value)`, `RET 0xc`): at
/// the last time the value is copied out, false when it is `INVALID_FLOAT`. Otherwise, with data
/// holding keys, the value from them (`NiFloatKey::GenInterp` from the last index, which it
/// updates), done in place; then `INVALID_FLOAT` answers false, any other value is copied out and
/// `time` becomes the last time.
pub fn float_update(e: &mut Engine, this: Ptr, time: f32, _target: u32, out: Ptr) -> bool {
    let t = this.addr();
    let value_of = |e: &Engine| e.mem.u32(t.wrapping_add(FLOAT_VALUE));
    if time == f32::from_bits(e.mem.u32(t.wrapping_add(FLOAT_LAST_TIME))) {
        let value = value_of(e);
        e.mem.set_u32(out.addr(), value);
        return f32::from_bits(value) != INVALID_FLOAT;
    }
    let data = e.mem.u32(t.wrapping_add(FLOAT_DATA));
    if data != 0 {
        let count = e.mem.u32(data.wrapping_add(DATA_COUNT));
        if count != 0 {
            let size = u32::from(e.mem.u8(data.wrapping_add(DATA_SIZE)));
            let key_type = e.mem.u32(data.wrapping_add(DATA_TYPE));
            let keys = e.mem.u32(data.wrapping_add(DATA_KEYS));
            let searched = count != 1 && time != f32::MIN;
            let index_at = t.wrapping_add(FLOAT_LAST_INDEX);
            let mut index = if searched { e.mem.u32(index_at) } else { 0 };
            let value = if float_precision() {
                float_keys::value::<f32>(e, time, keys, key_type, count, &mut index, size)
            } else {
                float_keys::value::<f64>(e, time, keys, key_type, count, &mut index, size)
            };
            if searched {
                e.mem.set_u32(index_at, index);
            }
            e.mem.set_u32(t.wrapping_add(FLOAT_VALUE), value);
        }
    }
    let value = value_of(e);
    if f32::from_bits(value) == INVALID_FLOAT {
        return false;
    }
    e.mem.set_u32(out.addr(), value);
    e.mem
        .set_u32(t.wrapping_add(FLOAT_LAST_TIME), time.to_bits());
    true
}
