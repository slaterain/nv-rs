//! JIP LN NVSE's script commands built so far (the plan's step 3.6), one module per JIP source
//! file (`functions_jip/jip_fn_climate.h` (JIP LN NVSE) is [`jip_fn_climate`], ...), each with
//! its own list of handlers, and what they share: JIP's helpers for forms, strings and the
//! game's heap. [`install`] gives each listed command its handler in JIP's table; a JIP command
//! that is not listed keeps its record with no handler (a call fails, as without the DLL).
//!
//! Built commands work whatever [`crate::ANSWERS_INSTALLED`] says: a mod checking for JIP is told
//! "not installed" until step 3.10, so only tests (and a script that calls them anyway) reach
//! them today.

pub mod forms;
pub mod jip_fn_body_part_data;
pub mod jip_fn_climate;
pub mod jip_fn_destruction_data;
pub mod jip_fn_weather;

use engine_core::prelude::*;
use forms::{BSStringT, TESForm, BASE_COMPONENT_OFFSETS};
use nvse::args::Arg;
use nvse::commands::array::{fix_on, LIST_GUARD};
use nvse::commands::Read;
use nvse::CommandArgs;

/// A module's commands: each opcode with its execute (all of them plain commands so far).
pub type Handlers = &'static [(u32, AbiFn)];

/// Every module's list, one line per JIP source file.
pub const ALL: &[Handlers] = &[
    jip_fn_body_part_data::COMMANDS,
    jip_fn_climate::COMMANDS,
    jip_fn_destruction_data::COMMANDS,
    jip_fn_weather::COMMANDS,
];

/// How many of JIP's commands have a handler.
pub fn built() -> usize {
    ALL.iter().map(|list| list.len()).sum()
}

/// Gives every built command its handler.
pub fn install(e: &mut Engine) {
    for list in ALL {
        for &(opcode, execute) in list.iter() {
            crate::table::set_handlers(e, opcode, Some(execute), None);
        }
    }
}

// ---- forms ----

/// `TESForm::typeID` (JIP LN NVSE), `+4`.
pub fn type_of(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

/// `TESForm::refID` (JIP LN NVSE), `+0xC`.
pub fn ref_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

/// `REFR_RES = form->refID` (JIP LN NVSE): the form's id in the result's low word.
pub fn set_form_result(e: &mut Engine, a: &CommandArgs, form: u32) {
    let id = ref_id(e, form);
    a.set_form_result(e, id);
}

/// A component of `form` by `kBaseComponentOffsets`' byte `which`: the form plus four times the
/// signed byte, or 0 for none (`GetDestructibleForm` is byte 2).
fn component(e: &Engine, form: u32, which: u32) -> u32 {
    let entry = BASE_COMPONENT_OFFSETS[usize::from(type_of(e, form))];
    let words = (entry >> (8 * which)) as u8 as i8;
    if words == 0 {
        0
    } else {
        form.wrapping_add((i32::from(words) * 4) as u32)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetDestructibleForm.
/// The form's `BGSDestructibleObjectForm`, 0 for a form type without one.
pub fn destructible_form(e: &Engine, form: u32) -> u32 {
    component(e, form, 2)
}

// ---- the game's heap ----

/// `GAME_HEAP` (JIP LN NVSE), the game's `MemoryManager`, and its allocate and free
/// (`00AA3E40`, `00AA4060`, `thiscall`), which JIP's `Game_DoHeapAlloc` and `Game_HeapFree`
/// jump to.
const GAME_HEAP: u32 = 0x011f_6238;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;

/// `Game_HeapAlloc` (JIP LN NVSE): `size` bytes of the game's heap.
pub fn heap_alloc(e: &mut Engine, size: u32) -> u32 {
    e.call(HEAP_ALLOCATE, &args![GAME_HEAP, size]).u32()
}

/// `Game_HeapFree` (JIP LN NVSE).
pub fn heap_free(e: &mut Engine, block: u32) {
    e.call(HEAP_FREE, &args![GAME_HEAP, block]);
}

// ---- strings ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.cpp: String::Set.
/// The game's string at `string` set to `text` (a C string JIP holds in a buffer: never null in
/// the commands built here). Its length is kept in 16 bits, as the string's own field; an empty
/// text empties the buffer it has; a longer one than the buffer holds gets a new buffer of its
/// length plus one, the old one freed.
pub fn string_set(e: &mut Engine, string: u32, text: &[u8]) {
    let s = Ptr::<BSStringT>::new(string);
    let len = text.len() as u16;
    e.set(s, BSStringT::sLen, len);
    if len == 0 {
        let data = e.get(s, BSStringT::pString);
        if data != 0 {
            e.mem.set_u8(data, 0);
        }
        return;
    }
    if e.get(s, BSStringT::sMaxLen) < len {
        e.set(s, BSStringT::sMaxLen, len);
        let old = e.get(s, BSStringT::pString);
        if old != 0 {
            heap_free(e, old);
        }
        let data = heap_alloc(e, u32::from(len) + 1);
        e.set(s, BSStringT::pString, data);
    }
    let data = e.get(s, BSStringT::pString);
    e.mem.write(data, &text[..usize::from(len)]);
    e.mem.set_u8(data + u32::from(len), 0);
}

/// The text of the game's string at `string` (`m_data`), `None` for no buffer.
pub fn string_text(e: &Engine, string: u32) -> Option<Vec<u8>> {
    let data = e.get(Ptr::<BSStringT>::new(string), BSStringT::pString);
    (data != 0).then(|| nvse::table::guest_text(e, data))
}

/// `AssignString` (JIP LN NVSE, xNVSE's `AssignToStringVar`): the command's string result; no
/// text (a null pointer) is assigned as `""`, as xNVSE does.
pub fn assign_string(e: &mut Engine, a: &CommandArgs, text: Option<Vec<u8>>) {
    let text = text.unwrap_or_default();
    nvse::commands::string::assign_to_string_var(e, a, &text);
}

/// String argument `i` as JIP's `size`-byte buffer on its stack holds it: its text up to the
/// first 0. A text of `size` bytes or more runs past the buffer into JIP's frame: the rule
/// xNVSE's buffers have (`nvse::commands::arg_in_buffer`; the identity's name buffer too): with
/// the list guard row it is read whole, without it the port stops there.
pub fn buffer_arg(e: &Engine, read: &Read, i: usize, size: usize, command: &str) -> Vec<u8> {
    let written = match read.args.get(i) {
        Some(Arg::Str { raw, text }) => raw.len().max(text.len()),
        _ => 0,
    };
    if written >= size && !fix_on(e, LIST_GUARD) {
        panic!(
            "{command}: a text of {written} bytes runs past JIP's {size}-byte buffer into its \
             frame: the port stops here (NV_NVSE_LIST_GUARD reads it whole)"
        );
    }
    let text = read.text(i);
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    text[..end].to_vec()
}

/// `TESModel::GetModelPath` (the virtual function, [`forms::GET_MODEL_PATH`]) of the model at
/// `model`: its text, `None` for a null answer.
pub fn model_path(e: &mut Engine, model: u32) -> Option<Vec<u8>> {
    let text = e.vcall(model, forms::GET_MODEL_PATH, &[]).u32();
    (text != 0).then(|| nvse::table::guest_text(e, text))
}

/// `TESModel::SetModelPath` (the virtual function, [`forms::SET_MODEL_PATH`]) with `path`, given
/// as JIP gives it: a C string in a buffer on its stack.
pub fn set_model_path(e: &mut Engine, model: u32, path: &[u8]) {
    let size = (path.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, buffer| {
        e.mem.write(buffer.addr(), path);
        e.mem.set_u8(buffer.addr() + path.len() as u32, 0);
        e.vcall(model, forms::SET_MODEL_PATH, &[buffer.addr()]);
    });
}

// ---- numbers ----

/// `cvttsd2si`, as the compiled C++ converts a double to an integer (and through it to a
/// narrower one): toward zero, the integer indefinite (`i32::MIN`) for NaN and past the range.
pub fn cvttsd2si(value: f64) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 || value <= -2_147_483_649.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// `cvttss2si`, a float to an integer the same way.
pub fn cvttss2si(value: f32) -> i32 {
    cvttsd2si(f64::from(value))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: cvtd2ul.
/// A double to an unsigned integer as JIP's helper does it: the x87's 64-bit truncation
/// (`fisttp qword`, the indefinite `0x8000000000000000` past the range) and its low word.
pub fn cvtd2ul(value: f64) -> u32 {
    engine_core::x87::fistp_chop64(value) as u32
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: RGBHexToDec.
/// A colour word as its decimal `RRRGGGBBB`: the low byte times a million, the next times a
/// thousand, plus the whole top half (the fourth byte included, as JIP's `shr ecx, 0x10`).
pub fn rgb_hex_to_dec(rgb: u32) -> u32 {
    (rgb & 0xff)
        .wrapping_mul(1_000_000)
        .wrapping_add(((rgb >> 8) & 0xff) * 1000)
        .wrapping_add(rgb >> 16)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: RGBDecToHex.
/// The other way: millions in the low byte, thousands shifted by 8, the rest by 16 (added, not
/// masked).
pub fn rgb_dec_to_hex(rgb: u32) -> u32 {
    let red = rgb / 1_000_000;
    let rest = rgb % 1_000_000;
    ((rest / 1000) << 8)
        .wrapping_add(red)
        .wrapping_add((rest % 1000) << 16)
}

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests {
    use super::*;

    /// The conversions JIP's colour commands use, worked out by hand from its asm.
    #[test]
    fn colours_between_hex_and_decimal() {
        assert_eq!(rgb_hex_to_dec(0x0003_0201), 1_002_003);
        assert_eq!(rgb_hex_to_dec(0x00ff_ffff), 255_255_255);
        // The fourth byte is added in at 2^24 / 2^16 = 256 times its value.
        assert_eq!(rgb_hex_to_dec(0x0100_0000), 256);
        assert_eq!(rgb_dec_to_hex(1_002_003), 0x0003_0201);
        assert_eq!(rgb_dec_to_hex(255_255_255), 0x00ff_ffff);
        // 999 thousands: 999 << 8 overlaps the next byte (no mask).
        assert_eq!(rgb_dec_to_hex(999_000), 999 << 8);
    }

    #[test]
    fn conversions_toward_zero() {
        assert_eq!(cvttsd2si(2.9), 2);
        assert_eq!(cvttsd2si(-2.9), -2);
        assert_eq!(cvttsd2si(f64::NAN), i32::MIN);
        assert_eq!(cvttsd2si(3e9), i32::MIN);
        assert_eq!(cvttsd2si(-2_147_483_648.5), i32::MIN + 0);
        assert_eq!(cvttss2si(255.9), 255);
        assert_eq!(cvtd2ul(4_294_967_295.9), u32::MAX);
        assert_eq!(cvtd2ul(-1.0), u32::MAX);
        assert_eq!(cvtd2ul(f64::NAN), 0);
    }

    /// `kBaseComponentOffsets`: a weapon (type 0x28, `0x0F2D1A0C`) has its destructible
    /// component at 0x1A words; a climate (0x36) has none.
    #[test]
    fn destructible_components_by_type() {
        let mut e = test_support::engine();
        let form = 0x0200_0000;
        e.mem.set_u8(form + 4, 0x28);
        assert_eq!(destructible_form(&e, form), form + 0x1a * 4);
        e.mem.set_u8(form + 4, 0x36);
        assert_eq!(destructible_form(&e, form), 0);
        // A negative byte (type 0x22, `0x0CFE00FB`: byte 2 is 0xFE) goes back two words.
        e.mem.set_u8(form + 4, 0x22);
        assert_eq!(destructible_form(&e, form), form - 8);
    }
}
