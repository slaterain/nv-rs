//! The arrays JIP's commands return: its `TempElements` (a `Vector<ArrayElementL>` it fills, then
//! hands to xNVSE) and xNVSE's array interface as JIP calls it (`CreateArray`, `CreateStringMap`,
//! `CreateMap`, `AppendElement`, `NVSEArrayVarInterface` in nvse/PluginAPI.h (JIP LN NVSE)).
//!
//! JIP returns an array by writing its id straight into the result (`*result = (int)CreateArray(
//! ...)`), not through `AssignCommandResult`; a command with nothing to return mostly leaves the
//! result at 0 and makes no array (each command says which). The array itself is xNVSE's own,
//! made by [`nvse::vars::create_array`] exactly as xNVSE's `ArrayAPI` makes it (ArrayVar.cpp
//! (xNVSE)): owned by the calling script's mod, temporary until a script variable refers to it,
//! so the tick's clean-up deletes it like any array an xNVSE command returns.

use super::ref_id;
use engine_core::prelude::*;
use nvse::vars::{self, Key, Value, KEY_NUMBER, KEY_STRING};

/// An element as JIP builds it (`ArrayElementL`: a number, a form pointer, a text, an array id).
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Number(f64),
    /// The form's address (0 for none); the array stores its id.
    Form(u32),
    Str(Vec<u8>),
    Array(u32),
}

impl Element {
    /// `Element::raw`: the low 32 bits of the element's 8-byte value, what `operator==` and
    /// `operator<` compare (unsigned). A number's raw word is the low half of its double's bits,
    /// so whole numbers below 2^21 all read 0 (JIP's own quirk, kept). A text compares by its
    /// pointer in JIP; no command built here inserts texts uniquely.
    fn raw(&self) -> u32 {
        match self {
            Element::Number(v) => v.to_bits() as u32,
            Element::Form(f) => *f,
            Element::Array(id) => *id,
            Element::Str(_) => unreachable!("JIP compares a text element by its pointer"),
        }
    }
}

/// `TempElements` (JIP LN NVSE): the elements in the order they were added, or kept sorted by
/// [`TempElements::insert_unique`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TempElements(pub Vec<Element>);

impl TempElements {
    pub fn new() -> TempElements {
        TempElements(Vec::new())
    }

    /// `Vector::Append`: at the end.
    pub fn append(&mut self, element: Element) {
        self.0.push(element);
    }

    // Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.h: Vector::InsertUnique.
    /// `Vector::InsertUnique`: a binary search by the raw word; an element whose raw word is
    /// there already is not added (false), else it goes in at its sorted place.
    pub fn insert_unique(&mut self, element: Element) -> bool {
        let raw = element.raw();
        let (mut low, mut high) = (0usize, self.0.len());
        while low != high {
            let mid = (low + high) >> 1;
            let at = self.0[mid].raw();
            if raw < at {
                high = mid;
            } else if at < raw {
                low = mid + 1;
            } else {
                return false;
            }
        }
        self.0.insert(low, element);
        true
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp: ArrayVar::SetElementFromAPI.
/// The value xNVSE stores for an element: a form as its id (`form ? form->refID : 0`).
fn value(e: &Engine, element: &Element) -> Value {
    match element {
        Element::Number(v) => Value::Number(*v),
        Element::Form(f) => Value::Form(if *f == 0 { 0 } else { ref_id(e, *f) }),
        Element::Str(t) => Value::Str(t.clone()),
        Element::Array(id) => Value::Array(*id),
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp: ArrayAPI::CreateArray.
/// `CreateArray(data, size, scriptObj)`: a packed array of numbers owned by `script`'s mod,
/// element `i` at index `i`; its id.
pub fn create_array(e: &mut Engine, script: u32, elements: &[Element]) -> u32 {
    let modi = vars::script_mod_index(e, script);
    let id = vars::create_array(e, KEY_NUMBER, true, modi);
    for (i, element) in elements.iter().enumerate() {
        let v = value(e, element);
        vars::set_element(e, id, &Key::Number(i as f64), v);
    }
    id
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp: ArrayAPI::CreateStringMap.
/// `CreateStringMap(keys, values, size, scriptObj)`: a map by text owned by `script`'s mod.
pub fn create_string_map(e: &mut Engine, script: u32, keys: &[Vec<u8>], values: &[Element]) -> u32 {
    let modi = vars::script_mod_index(e, script);
    let id = vars::create_array(e, KEY_STRING, false, modi);
    for (k, element) in keys.iter().zip(values) {
        let v = value(e, element);
        vars::set_element(e, id, &Key::Str(k.clone()), v);
    }
    id
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp: ArrayAPI::CreateMap.
/// `CreateMap(keys, values, size, scriptObj)`: a map by number owned by `script`'s mod.
pub fn create_map(e: &mut Engine, script: u32, keys: &[f64], values: &[Element]) -> u32 {
    let modi = vars::script_mod_index(e, script);
    let id = vars::create_array(e, KEY_NUMBER, false, modi);
    for (k, element) in keys.iter().zip(values) {
        let v = value(e, element);
        vars::set_element(e, id, &Key::Number(*k), v);
    }
    id
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp: ArrayAPI::AppendElement.
/// `AppendElement(arr, value)`: at the end of a packed array of numbers; nothing for a map or a
/// missing array.
pub fn append_element(e: &mut Engine, id: u32, element: &Element) {
    let Some(len) = vars::array(e, id)
        .filter(|a| a.key_type == KEY_NUMBER && a.packed)
        .map(|a| a.len())
    else {
        return;
    };
    let v = value(e, element);
    vars::set_element(e, id, &Key::Number(len as f64), v);
}

#[cfg(test)]
#[path = "arrays_tests.rs"]
mod tests;
