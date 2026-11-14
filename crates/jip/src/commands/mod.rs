//! JIP LN NVSE's script commands built so far (the plan's step 3.6), one module per JIP source
//! file (`functions_jip/jip_fn_climate.h` (JIP LN NVSE) is [`jip_fn_climate`], ...), each with
//! its own list of handlers, and what they share: JIP's helpers for forms, strings and the
//! game's heap. [`install`] gives each listed command its handler in JIP's table; a JIP command
//! that is not listed keeps its record with no handler (a call fails, as without the DLL).
//!
//! Handlers and names exist only while JIP answers "installed" ([`crate::ANSWERS_INSTALLED`], off
//! until step 3.10): until then every JIP opcode, built or not, is as without the DLL, and only
//! tests (on an engine installed with the flag on) reach the commands built here.

pub mod arrays;
pub mod extra;
pub mod forms;
pub mod jip_fn_ammo;
pub mod jip_fn_ammo_effect;
pub mod jip_fn_armor;
pub mod jip_fn_aux_variable;
pub mod jip_fn_body_part_data;
pub mod jip_fn_class;
pub mod jip_fn_climate;
pub mod jip_fn_combat_style;
pub mod jip_fn_destruction_data;
pub mod jip_fn_effect;
pub mod jip_fn_explosion;
pub mod jip_fn_faction;
pub mod jip_fn_global_var;
pub mod jip_fn_grass;
pub mod jip_fn_imagespace;
pub mod jip_fn_impact_data;
pub mod jip_fn_land_texture;
pub mod jip_fn_light;
pub mod jip_fn_package;
pub mod jip_fn_perk;
pub mod jip_fn_projectile;
pub mod jip_fn_quest;
pub mod jip_fn_ref_map;
pub mod jip_fn_texture_set;
pub mod jip_fn_water;
pub mod jip_fn_weapon;
pub mod jip_fn_weather;
pub mod jip_fn_worldspace;
pub mod ln_fn_activator;
pub mod ln_fn_casino;
pub mod ln_fn_challenge;
pub mod ln_fn_leveled_list;
pub mod ln_fn_map_marker;
pub mod ln_fn_math;
pub mod ln_fn_perk;
pub mod ln_fn_race;
pub mod ln_fn_recipe;
pub mod ln_fn_weapon;

use engine_core::prelude::*;
use engine_core::ptr::Layout;
use forms::{BSStringT, ListNode, TESForm, BASE_COMPONENT_OFFSETS};
use nvse::args::Arg;
use nvse::commands::array::{fix_on, LIST_GUARD};
use nvse::commands::Read;
use nvse::CommandArgs;

/// A module's plain commands: each opcode with its execute.
pub type Handlers = &'static [(u32, AbiFn)];

/// A module's conditions (`DEFINE_CMD_COND_PLUGIN` (JIP LN NVSE)): each opcode with its execute
/// and its eval (`COMMAND_ARGS_EVAL`: the reference, the condition's two parameters, the result).
pub type Conditions = &'static [(u32, AbiFn, AbiFn)];

/// Every module's conditions, one line per JIP source file that has any.
pub const ALL_CONDITIONS: &[Conditions] = &[jip_fn_effect::CONDITIONS, ln_fn_math::CONDITIONS];

/// A module's condition-only commands (`DEFINE_CMD_COND_ONLY` (JIP LN NVSE)): each opcode with its
/// eval; no execute.
pub type ConditionsOnly = &'static [(u32, AbiFn)];

/// Every module's condition-only commands.
pub const ALL_CONDITIONS_ONLY: &[ConditionsOnly] = &[jip_fn_aux_variable::CONDITIONS_ONLY];

/// Every module's list, one line per JIP source file.
pub const ALL: &[Handlers] = &[
    jip_fn_ammo::COMMANDS,
    jip_fn_aux_variable::COMMANDS,
    jip_fn_ammo_effect::COMMANDS,
    jip_fn_armor::COMMANDS,
    jip_fn_body_part_data::COMMANDS,
    jip_fn_class::COMMANDS,
    jip_fn_climate::COMMANDS,
    jip_fn_combat_style::COMMANDS,
    jip_fn_destruction_data::COMMANDS,
    jip_fn_effect::COMMANDS,
    jip_fn_explosion::COMMANDS,
    jip_fn_faction::COMMANDS,
    jip_fn_global_var::COMMANDS,
    jip_fn_grass::COMMANDS,
    jip_fn_imagespace::COMMANDS,
    jip_fn_impact_data::COMMANDS,
    jip_fn_land_texture::COMMANDS,
    jip_fn_light::COMMANDS,
    jip_fn_package::COMMANDS,
    jip_fn_perk::COMMANDS,
    jip_fn_projectile::COMMANDS,
    jip_fn_quest::COMMANDS,
    jip_fn_ref_map::COMMANDS,
    jip_fn_texture_set::COMMANDS,
    jip_fn_water::COMMANDS,
    jip_fn_weapon::COMMANDS,
    jip_fn_weather::COMMANDS,
    jip_fn_worldspace::COMMANDS,
    ln_fn_activator::COMMANDS,
    ln_fn_casino::COMMANDS,
    ln_fn_challenge::COMMANDS,
    ln_fn_leveled_list::COMMANDS,
    ln_fn_map_marker::COMMANDS,
    ln_fn_math::COMMANDS,
    ln_fn_perk::COMMANDS,
    ln_fn_race::COMMANDS,
    ln_fn_recipe::COMMANDS,
    ln_fn_weapon::COMMANDS,
];

/// How many of JIP's commands have a handler.
pub fn built() -> usize {
    ALL.iter().map(|list| list.len()).sum::<usize>()
        + ALL_CONDITIONS.iter().map(|list| list.len()).sum::<usize>()
        + ALL_CONDITIONS_ONLY
            .iter()
            .map(|list| list.len())
            .sum::<usize>()
}

/// Whether `opcode` is a built command.
pub fn is_built(opcode: u32) -> bool {
    ALL.iter()
        .any(|list| list.iter().any(|&(o, _)| o == opcode))
        || ALL_CONDITIONS
            .iter()
            .any(|list| list.iter().any(|&(o, _, _)| o == opcode))
        || ALL_CONDITIONS_ONLY
            .iter()
            .any(|list| list.iter().any(|&(o, _)| o == opcode))
}

/// Gives every built command its handler (a condition its eval too).
pub fn install(e: &mut Engine) {
    for list in ALL {
        for &(opcode, execute) in list.iter() {
            crate::table::set_handlers(e, opcode, Some(execute), None);
        }
    }
    for list in ALL_CONDITIONS {
        for &(opcode, execute, eval) in list.iter() {
            crate::table::set_handlers(e, opcode, Some(execute), Some(eval));
        }
    }
    for list in ALL_CONDITIONS_ONLY {
        for &(opcode, eval) in list.iter() {
            crate::table::set_handlers(e, opcode, None, Some(eval));
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

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetFullName (asm).
/// The form's `TESFullName`: a reference's (its vtable answers true, `008D0360`, at +0xF0) is
/// its base form's (+0x20); a cell's (type 0x39) is its own at +0x18 unless that is empty, then
/// its worldspace's (+0xC0, at +0x18); else `kBaseComponentOffsets`' byte 0. 0 for none.
pub fn full_name(e: &Engine, form: u32) -> u32 {
    let vtbl = e.get(Ptr::<forms::Object>::new(form), forms::Object::vtbl);
    let form = if e.mem.u32(vtbl + 0xf0) == 0x008d_0360 {
        e.mem.u32(form + 0x20)
    } else {
        form
    };
    if type_of(e, form) == 0x39 {
        let own = form + 0x18;
        if e.mem.u16(own + 8) as i16 > 0 {
            return own;
        }
        let ws = e.mem.u32(form + 0xc0);
        return if ws == 0 { 0 } else { ws + 0x18 };
    }
    component(e, form, 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetTheName (asm).
/// The form's name: its full name's text when that has a length above 0 (signed 16 bits), else
/// `""`.
pub fn the_name(e: &Engine, form: u32) -> Vec<u8> {
    let name = full_name(e, form);
    if name == 0 || e.mem.u16(name + 8) as i16 <= 0 {
        return Vec::new();
    }
    nvse::table::guest_text(e, e.mem.u32(name + 4))
}

/// `IS_ACTOR(form)` (JIP LN NVSE): the form's vtable answers true (`008D0360`) at +0x100.
pub fn is_actor(e: &Engine, form: u32) -> bool {
    form != 0
        && e.mem
            .u32(e.get(Ptr::<forms::Object>::new(form), forms::Object::vtbl) + 0x100)
            == 0x008d_0360
}

/// `IS_TYPE(form, Class)` (JIP LN NVSE): the form's vtable is the class's own.
pub fn is_type(e: &Engine, form: u32, vtbl: u32) -> bool {
    e.get(Ptr::<forms::Object>::new(form), forms::Object::vtbl) == vtbl
}

// ---- the game's lists (`tList`, nvse/GameTypes.h (JIP LN NVSE)), first node inline ----

/// `tList::Count`: 0 for an empty first node, else every node.
pub fn list_count(e: &Engine, head: Ptr<ListNode>) -> u32 {
    if e.get(head, ListNode::data) == 0 {
        return 0;
    }
    let mut count = 1;
    let mut node = e.get(head, ListNode::next);
    while node != 0 {
        count += 1;
        node = e.get(Ptr::<ListNode>::new(node), ListNode::next);
    }
    count
}

/// `tList::GetNthItem`: the item of node `index` (a negative index none), 0 past the end.
pub fn nth_item(e: &Engine, head: Ptr<ListNode>, index: i32) -> u32 {
    if index < 0 {
        return 0;
    }
    let mut node = head;
    for _ in 0..index {
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            return 0;
        }
    }
    e.get(node, ListNode::data)
}

/// `tList::IsInList`: some node holds `item` (an empty first node holds 0).
pub fn list_contains(e: &Engine, head: Ptr<ListNode>, item: u32) -> bool {
    let mut node = head;
    loop {
        if e.get(node, ListNode::data) == item {
            return true;
        }
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            return false;
        }
    }
}

/// `tList::Prepend`: a non-null item into an empty first node, else first (`ListNode::Insert`:
/// the first item moves to a new 8-byte node of the game's heap after the first).
pub fn list_prepend(e: &mut Engine, head: Ptr<ListNode>, item: u32) {
    if item == 0 {
        return;
    }
    let first = e.get(head, ListNode::data);
    if first == 0 {
        e.set(head, ListNode::data, item);
        return;
    }
    let added = Ptr::<ListNode>::new(heap_alloc(e, ListNode::SIZE));
    let next = e.get(head, ListNode::next);
    e.set(added, ListNode::data, first);
    e.set(head, ListNode::data, item);
    e.set(added, ListNode::next, next);
    e.set(head, ListNode::next, added.addr());
}

/// `ListNode::RemoveMe` on the first node or `RemoveNext` on the one before: a later node
/// unlinked and freed; the first node takes the next node's item and link (that node freed), or
/// is only emptied when alone.
pub fn list_unlink(e: &mut Engine, node: Ptr<ListNode>, prev: Ptr<ListNode>) {
    if !prev.is_null() {
        let gone = e.get(prev, ListNode::next);
        let after = e.get(Ptr::<ListNode>::new(gone), ListNode::next);
        e.set(prev, ListNode::next, after);
        heap_free(e, gone);
        return;
    }
    let next = e.get(node, ListNode::next);
    if next == 0 {
        e.set(node, ListNode::data, 0);
        return;
    }
    let next_node = Ptr::<ListNode>::new(next);
    let item = e.get(next_node, ListNode::data);
    let after = e.get(next_node, ListNode::next);
    e.set(node, ListNode::data, item);
    e.set(node, ListNode::next, after);
    heap_free(e, next);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.h: tList::RemoveAll.
/// `tList::RemoveAll`: every node after the first unlinked and freed, the first emptied (the
/// items themselves are left alone).
pub fn list_remove_all(e: &mut Engine, head: Ptr<ListNode>) {
    loop {
        let next = e.get(head, ListNode::next);
        if next == 0 {
            break;
        }
        let after = e.get(Ptr::<ListNode>::new(next), ListNode::next);
        e.set(head, ListNode::next, after);
        heap_free(e, next);
    }
    e.set(head, ListNode::data, 0);
}

/// `tList::Remove`: the first node holding `item` taken out ([`list_unlink`]); whether there was
/// one.
pub fn list_remove(e: &mut Engine, head: Ptr<ListNode>, item: u32) -> bool {
    let (mut node, mut prev) = (head, Ptr::<ListNode>::NULL);
    loop {
        if e.get(node, ListNode::data) == item {
            list_unlink(e, node, prev);
            return true;
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            return false;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.h: tList::Append.
/// `tList::Append`: a non-null item into the last node when that node is empty, else into a new
/// 8-byte node of the game's heap after it (`Node::Append`).
pub fn list_append(e: &mut Engine, head: Ptr<ListNode>, item: u32) {
    if item == 0 {
        return;
    }
    let mut last = head;
    loop {
        let next = e.get(last, ListNode::next);
        if next == 0 {
            break;
        }
        last = Ptr::new(next);
    }
    if e.get(last, ListNode::data) == 0 {
        e.set(last, ListNode::data, item);
        return;
    }
    let added = Ptr::<ListNode>::new(heap_alloc(e, ListNode::SIZE));
    e.set(added, ListNode::data, item);
    e.set(added, ListNode::next, 0);
    e.set(last, ListNode::next, added.addr());
}

/// The items of a list from its first node, in order, empty items included (`tList::Head`
/// walked with `do ... while (iter = iter->next)`).
pub fn list_items(e: &Engine, head: Ptr<ListNode>) -> Vec<u32> {
    let mut items = Vec::new();
    let mut node = head;
    loop {
        items.push(e.get(node, ListNode::data));
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            return items;
        }
    }
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

/// A formatted text (xNVSE's `ExtractFormatStringArgs`) as JIP's `size`-byte buffer holds it:
/// xNVSE copies the whole text and its 0 into the buffer, so a text of `size` bytes or more runs
/// into JIP's frame: [`buffer_arg`]'s rule (whole with the list guard row, else the port stops).
pub fn formatted_in_buffer(e: &Engine, text: &[u8], size: usize, command: &str) -> Vec<u8> {
    if text.len() >= size && !fix_on(e, LIST_GUARD) {
        panic!(
            "{command}: a formatted text of {} bytes runs past JIP's {size}-byte buffer into its \
             frame: the port stops here (NV_NVSE_LIST_GUARD reads it whole)",
            text.len()
        );
    }
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

/// `Dbl180dPI` and `FltPId180` (JIP LN NVSE, `internal/utility.h`), as JIP writes them (the float
/// literal has more digits than an `f32` keeps; it rounds to the float JIP's code holds).
pub const DBL_180_D_PI: f64 = 57.295_779_513_082_32;
#[allow(clippy::excessive_precision)]
pub const FLT_PI_D_180: f32 = 0.017_453_292_38;

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
        assert_eq!(cvttsd2si(-2_147_483_648.5), i32::MIN);
        assert_eq!(cvttss2si(255.9), 255);
        assert_eq!(cvtd2ul(4_294_967_295.9), u32::MAX);
        assert_eq!(cvtd2ul(-1.0), u32::MAX);
        assert_eq!(cvtd2ul(f64::NAN), 0);
    }

    /// Each module lists only commands of its own JIP source file, once each, and the count is
    /// README's ("JIP LN: N of 1,118", read from README.md).
    #[test]
    fn each_module_holds_its_own_file() {
        let files = [
            "jip_fn_ammo",
            "jip_fn_aux_variable",
            "jip_fn_ammo_effect",
            "jip_fn_armor",
            "jip_fn_body_part_data",
            "jip_fn_class",
            "jip_fn_climate",
            "jip_fn_combat_style",
            "jip_fn_destruction_data",
            "jip_fn_effect",
            "jip_fn_explosion",
            "jip_fn_faction",
            "jip_fn_global_var",
            "jip_fn_grass",
            "jip_fn_imagespace",
            "jip_fn_impact_data",
            "jip_fn_land_texture",
            "jip_fn_light",
            "jip_fn_package",
            "jip_fn_perk",
            "jip_fn_projectile",
            "jip_fn_quest",
            "jip_fn_ref_map",
            "jip_fn_texture_set",
            "jip_fn_water",
            "jip_fn_weapon",
            "jip_fn_weather",
            "jip_fn_worldspace",
            "ln_fn_activator",
            "ln_fn_casino",
            "ln_fn_challenge",
            "ln_fn_leveled_list",
            "ln_fn_map_marker",
            "ln_fn_math",
            "ln_fn_perk",
            "ln_fn_race",
            "ln_fn_recipe",
            "ln_fn_weapon",
        ];
        let mut seen = std::collections::HashSet::new();
        for (list, file) in ALL.iter().zip(files) {
            for &(opcode, _) in list.iter() {
                let slot = crate::table::slot(opcode).expect("a JIP slot");
                assert!(slot.file.ends_with(file), "{opcode:04x} {}", slot.name);
                assert!(seen.insert(opcode), "{opcode:04x} listed twice");
            }
        }
        assert_eq!(ALL.len(), files.len());
        for (list, file) in ALL_CONDITIONS.iter().zip(["jip_fn_effect", "ln_fn_math"]) {
            for &(opcode, _, _) in list.iter() {
                let slot = crate::table::slot(opcode).expect("a JIP slot");
                assert!(slot.file.ends_with(file), "{opcode:04x} {}", slot.name);
                assert!(slot.kind == crate::table::Kind::Condition, "{opcode:04x}");
                assert!(seen.insert(opcode), "{opcode:04x} listed twice");
            }
        }
        assert_eq!(ALL_CONDITIONS.len(), 2);
        for (list, file) in ALL_CONDITIONS_ONLY.iter().zip(["jip_fn_aux_variable"]) {
            for &(opcode, _) in list.iter() {
                let slot = crate::table::slot(opcode).expect("a JIP slot");
                assert!(slot.file.ends_with(file), "{opcode:04x} {}", slot.name);
                assert!(
                    slot.kind == crate::table::Kind::ConditionOnly,
                    "{opcode:04x}"
                );
                assert!(seen.insert(opcode), "{opcode:04x} listed twice");
            }
        }
        assert_eq!(ALL_CONDITIONS_ONLY.len(), 1);
        // README's JIP LN count is checked in one place, against the table's every handled JIP
        // opcode: `ui_hooks::tests::the_readme_counts_the_built_commands`.
    }

    /// Answering "installed", a built command has its execute and its name in its record; an
    /// unbuilt one has neither.
    #[test]
    fn built_commands_are_in_the_table() {
        let e = test_support::engine();
        let execute = |e: &Engine, op: u32| crate::test_support::execute_of(e, op);
        assert_eq!(execute(&e, 0x266b), nvse::table::execute_address(0x266b));
        let record = nvse::table::record_of(&e, 0x266b);
        assert_eq!(
            nvse::table::guest_text(&e, e.mem.u32(record)),
            b"GetClimateSunTexture"
        );
        // ForceClimate waits for JIP's hook: no handler.
        assert_eq!(execute(&e, 0x28d8), 0);
    }

    /// Answering "not installed" (players' state until step 3.10), every opcode built here is as
    /// without the DLL: its whole record what the table alone leaves (no name, no execute, no
    /// eval), as an unbuilt one's.
    #[test]
    fn not_installed_the_built_commands_are_as_without_the_dll() {
        let mut e = crate::test_support::engine();
        crate::install_as(&mut e, false);
        let mut plain = crate::test_support::engine();
        crate::table::install(&mut plain, false);
        let words = |e: &Engine, op: u32| -> Vec<u32> {
            let at = nvse::table::record_of(e, op);
            (0..10).map(|i| e.mem.u32(at + 4 * i)).collect()
        };
        let conditions = ALL_CONDITIONS
            .iter()
            .flat_map(|list| list.iter().map(|&(op, _, _)| op));
        let opcodes: Vec<u32> = ALL
            .iter()
            .flat_map(|list| list.iter().map(|&(op, _)| op))
            .chain(conditions)
            .chain(
                ALL_CONDITIONS_ONLY
                    .iter()
                    .flat_map(|list| list.iter().map(|&(op, _)| op)),
            )
            .collect();
        {
            for op in opcodes {
                let w = words(&e, op);
                assert_eq!(w, words(&plain, op), "{op:04x}");
                assert_eq!((w[6], w[8]), (0, 0), "{op:04x}: no execute, no eval");
                assert_eq!(nvse::table::guest_text(&e, w[0]), b"", "{op:04x}: no name");
            }
        }
    }

    /// `kBaseComponentOffsets`: a weapon (type 0x28, `0x0F2D1A0C`) has its destructible
    /// component at 0x2D words (byte 2 of the entry); a climate (0x36) has none.
    #[test]
    fn destructible_components_by_type() {
        let e = test_support::engine();
        let form = 0x0200_0000;
        e.mem.set_u8(form + 4, 0x28);
        assert_eq!(destructible_form(&e, form), form + 0x2d * 4);
        e.mem.set_u8(form + 4, 0x36);
        assert_eq!(destructible_form(&e, form), 0);
        // A negative byte (type 0x22, `0x0CFE00FB`: byte 2 is 0xFE) goes back two words.
        e.mem.set_u8(form + 4, 0x22);
        assert_eq!(destructible_form(&e, form), form - 8);
    }
}
