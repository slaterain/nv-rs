//! UI Organizer's own XML reader (`ResolveXMLFileHook`, `ui_organizer.h` (UIO)): the scan of a
//! menu file's text into the game's parsed items, in place of the game's `Tile::ParseFile`. What
//! the scan finds goes to a [`Builder`] (the game's structures: [`crate::build`]):
//!
//! - a tag name `<name ...>` / `</name>`: its trait id (`ResolveTrait(closing, id, 3)`), looked up
//!   in the game's trait names map when the tag has an attribute (the tag's first space), else
//!   through `AddTraitName` (which registers a name starting with `_` as a new user trait);
//! - an attribute `name="x"` (0xBBA), `src="x"` (0xBBB) followed by ` trait="y"` (0xBBC, the trait
//!   name through `AddTraitName`); anything else, or a missing `=`, quote or value, is an error
//!   that skips the rest of the tag;
//! - a value between tags (0xBB9): `&-sName;` a game setting's value (`s`: its text, `f`: its
//!   number, `i`: its number as a float; an unknown or other setting gives nothing), `&name;` a
//!   trait name's id as a number (an unknown name stays text), a run of `-`, `.` and digits a number
//!   (UIO's `StrToDbl`), anything else (from a space up) text.
//!
//! The scan works on a copy of the text, writing zeros into it as UIO writes them into its
//! buffer, and never reads past it (UIO reads up to 7 bytes past the text's end in a cut-off
//! attribute: here those bytes are zeros, which ends the attribute with an error as a zero there
//! would in UIO).

/// A value handed to `ResolveTrait`, by UIO's data type: 1 a number, 2 text, 3 an id.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(f32),
    Str(Vec<u8>),
    /// Text with no text at all (a null pointer: a text game setting with none).
    NoText,
    Id(u32),
}

/// `XMLtoTileData`'s item types (`ParsedXMLTag::tagType`) and the attribute value types.
pub const TAG_VALUE: u32 = 0xbb9;
pub const TAG_NAME: u32 = 0xbba;
pub const TAG_SRC: u32 = 0xbbb;
pub const TAG_TRAIT: u32 = 0xbbc;

/// What the scan builds with: the game's maps and `ResolveTrait` on the storage.
pub trait Builder {
    /// `NiTMap::Lookup` on the game's trait names map (`0x11F32F4`): the name's id.
    fn lookup_trait(&mut self, name: &[u8]) -> Option<u32>;
    /// `AddTraitName` (UIO): a name starting with `_` is a user trait (registered with the next
    /// user id when new); any other name is looked up. The id, or `None` for an unknown name.
    fn add_trait_name(&mut self, name: &[u8]) -> Option<u32>;
    /// `GetGMSTValue` (UIO): the game setting `name`'s value as a number or text.
    fn game_setting(&mut self, name: &[u8]) -> Option<Value>;
    /// `ResolveTrait` (UIO): one item. False when no more can be made (the scan stops).
    fn resolve_trait(&mut self, value_type: u32, value: Value) -> bool;
}

/// `kValidForNumeric` (UIO): `-`, `.` and the digits.
fn numeric(c: u8) -> bool {
    c == b'-' || c == b'.' || c.is_ascii_digit()
}

/// The C string at `at` (to its zero).
fn c_str(d: &[u8], at: usize) -> &[u8] {
    let d = d.get(at..).unwrap_or_default();
    let end = d.iter().position(|&c| c == 0).unwrap_or(d.len());
    &d[..end]
}

/// The four bytes at `at` as UIO reads them with one `mov ecx, [ebx]` (zeros past the end).
fn dword(d: &[u8], at: usize) -> u32 {
    let b = |i: usize| *d.get(at + i).unwrap_or(&0) as u32;
    b(0) | b(1) << 8 | b(2) << 16 | b(3) << 24
}

fn byte(d: &[u8], at: usize) -> u8 {
    *d.get(at).unwrap_or(&0)
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: ResolveXMLFileHook (its scan, from
// `mainIter` to `mainEnd`; the debug log it writes in UIO's developer mode is left out).
/// Scans `text` (a menu file's text as UIO's reader gives it) into `b`.
pub fn scan(text: &[u8], b: &mut dyn Builder) {
    let mut d = text.to_vec();
    // The text ends at its first zero, as UIO's C string does.
    if let Some(end) = d.iter().position(|&c| c == 0) {
        d.truncate(end);
    }
    d.push(0);
    // UIO's locals: in a tag (`[ebp-5]`), a `/` seen (`[ebp-6]`), an attribute read (`[ebp-7]`),
    // the rest of the tag skipped (`[ebp-8]`), the token (`[ebp-0xC]`), a space ended the token
    // (`[ebp-0xD]`), the last attribute's type (`[ebp-0x14]`).
    let mut in_tag = false;
    let mut closing = false;
    let mut has_attr = false;
    let mut skip = false;
    let mut token: Option<usize> = None;
    let mut space = false;
    let mut attr_type: u32 = 0;
    let mut i = 0usize;
    // `ebx` starts one before the text; each turn moves it on first.
    while i < d.len() {
        let c = d[i];
        if c == 0 {
            return;
        }
        if c == b'<' {
            skip = in_tag;
            if !in_tag {
                in_tag = true;
                if let Some(t) = token {
                    d[i] = 0;
                    if !value_between_tags(&mut d, t, b) {
                        return;
                    }
                }
            }
            token = None;
        } else if c == b'>' {
            if in_tag {
                if !has_attr {
                    if let Some(t) = token {
                        d[i] = 0;
                        let name = c_str(&d, t).to_vec();
                        if let Some(id) = b.add_trait_name(&name) {
                            if !b.resolve_trait(closing as u32, Value::Id(id)) {
                                return;
                            }
                        }
                    }
                }
                // `doneClose`: the dword at `[ebp-8]` (skip, attribute, `/`, in a tag) cleared.
                skip = false;
                has_attr = false;
                closing = false;
                in_tag = false;
                space = false;
            }
            token = None;
        } else if !in_tag {
            // `isClosed`: the text after a tag starts here.
            if token.is_none() {
                token = Some(i);
            }
        } else if c == b'/' {
            d[i] = 0;
            closing = true;
            has_attr = false;
            space = false;
        } else if skip {
        } else if c == b' ' {
            space = true;
            d[i] = 0;
        } else if token.is_none() {
            token = Some(i);
            space = false;
        } else if space {
            // `procAction`: the tag's first attribute starts here.
            space = false;
            has_attr = true;
            let name = c_str(&d, token.unwrap_or(i)).to_vec();
            let Some(id) = b.lookup_trait(&name) else {
                token = None;
                skip = true;
                i += 1;
                continue;
            };
            if !b.resolve_trait(0, Value::Id(id)) {
                return;
            }
            match attribute(&mut d, &mut i, &mut attr_type, b) {
                Attr::Done => {}
                Attr::Error => skip = true,
                Attr::End | Attr::Stop => return,
            }
        }
        i += 1;
    }
}

/// How an attribute ended.
enum Attr {
    /// Read; the scan goes on after its closing quote.
    Done,
    /// UIO's `logError`/`logErrorID`: the rest of the tag is skipped.
    Error,
    /// The text ended inside it (`mainEnd`).
    End,
    /// The builder can make no more items.
    Stop,
}

/// The attribute at `*i` (`procAction` after the tag's id, to `mainIter`): `src="x" trait="y"` or
/// `name="x"`; `*i` is left on the last closing quote read.
fn attribute(d: &mut [u8], i: &mut usize, attr_type: &mut u32, b: &mut dyn Builder) -> Attr {
    let word = dword(d, *i) | 0x2020_2020;
    let mut kind = if word == u32::from_le_bytes(*b"src=") {
        *i += 3;
        TAG_SRC
    } else if word == u32::from_le_bytes(*b"name") {
        *i += 4;
        if byte(d, *i) != b'=' {
            return Attr::Error;
        }
        TAG_NAME
    } else {
        return Attr::Error;
    };
    loop {
        // `findDQ0` / `findDQ1`: the value between the next two quotes.
        let Some(open) = next_quote(d, *i) else {
            return Attr::End;
        };
        let start = open + 1;
        let Some(close) = next_quote(d, open) else {
            *i = d.len() - 1;
            return Attr::End;
        };
        *i = close;
        if start == close {
            return Attr::Error;
        }
        d[close] = 0;
        *attr_type = kind;
        let value = c_str(d, start).to_vec();
        let item = if kind == TAG_TRAIT {
            match b.add_trait_name(&value) {
                Some(id) => Value::Id(id),
                None => return Attr::Error,
            }
        } else {
            Value::Str(value)
        };
        if !b.resolve_trait(kind, item) {
            return Attr::Stop;
        }
        if *attr_type != TAG_SRC {
            return Attr::Done;
        }
        // `src="..."` needs ` trait=` after it (`needTrait`: the four bytes after the space
        // masked with `TRAI`).
        if byte(d, close + 1) != b' ' {
            return Attr::Error;
        }
        let mask = u32::from_le_bytes(*b"TRAI");
        if dword(d, close + 2) & mask != mask {
            return Attr::Error;
        }
        *i = close + 7;
        if byte(d, *i) != b'=' {
            return Attr::Error;
        }
        kind = TAG_TRAIT;
    }
}

/// The first `"` after `from` (UIO's loop moves on before it looks); `None` at the text's end.
fn next_quote(d: &[u8], from: usize) -> Option<usize> {
    let mut j = from;
    loop {
        j += 1;
        match d.get(j) {
            None | Some(0) => return None,
            Some(b'"') => return Some(j),
            Some(_) => {}
        }
    }
}

/// The text between two tags at `t` (ended by a zero where the `<` was): a game setting's value,
/// a trait name's id, a number or text, handed to `ResolveTrait` as a value (0xBB9); nothing for
/// an unknown setting or text starting below a space. False when the builder can make no more.
fn value_between_tags(d: &mut [u8], t: usize, b: &mut dyn Builder) -> bool {
    let value = if byte(d, t) == b'&' {
        if byte(d, t + 1) == b'-' {
            // `GetGMSTValue`: the name ends at the first `;` (written over with a zero).
            let name_at = t + 2;
            let name_end = name_at + c_str(d, name_at).len();
            if let Some(semi) = d[name_at..name_end].iter().position(|&c| c == b';') {
                d[name_at + semi] = 0;
            }
            let name = c_str(d, name_at).to_vec();
            match b.game_setting(&name) {
                Some(v) => v,
                None => return true,
            }
        } else {
            let name = c_str(d, t).to_vec();
            match b.lookup_trait(&name) {
                Some(id) => Value::Num(id as i32 as f32),
                None => Value::Str(name),
            }
        }
    } else {
        let s = c_str(d, t);
        if byte(d, t) < b' ' {
            return true;
        }
        if s.iter().all(|&c| numeric(c)) {
            Value::Num(str_to_dbl(s) as f32)
        } else {
            Value::Str(s.to_vec())
        }
    };
    b.resolve_trait(TAG_VALUE, value)
}

/// `kFactor10Div` (UIO's `StrToDbl`).
const FACTOR_10_DIV: [f64; 9] = [
    1.0e-09, 1.0e-08, 1.0e-07, 1.0e-06, 1.0e-05, 0.0001, 0.001, 0.01, 0.1,
];

// Translated from UIO 2.30 (426d963), utility.cpp: StrToDbl.
/// UIO's number reading: an optional `-`, digits (32-bit, unsigned; a wrap that makes the value
/// smaller gives ±4294967296), then `.` and up to nine digits; the sign only when a part is not 0.
pub fn str_to_dbl(s: &[u8]) -> f64 {
    let at = |i: usize| *s.get(i).unwrap_or(&0);
    let mut i = 0usize;
    let mut sign: u8 = 0;
    if at(0) == b'-' {
        sign = 1;
        i = 1;
    }
    let mut v: u32 = 0;
    let mut prev: u32 = 0;
    let mut out = 0.0f64;
    let mut cl;
    loop {
        cl = at(i).wrapping_sub(b'0');
        if cl > 9 {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add(cl as u32);
        if v < prev {
            return if sign != 0 {
                -4294967296.0
            } else {
                4294967296.0
            };
        }
        prev = v;
        i += 1;
    }
    if v != 0 {
        out = v as i32 as f64;
        if (v as i32) < 0 {
            out += 4294967296.0;
        }
        sign <<= 1;
        v = 0;
    }
    if cl == 0xfe {
        let mut left: u8 = 9;
        loop {
            i += 1;
            let c = at(i).wrapping_sub(b'0');
            if c > 9 {
                break;
            }
            v = v.wrapping_mul(10).wrapping_add(c as u32);
            left -= 1;
            if left == 0 {
                break;
            }
        }
        if v != 0 {
            sign <<= 1;
            out += v as i32 as f64 * FACTOR_10_DIV[left as usize];
        }
    }
    if sign & 6 != 0 {
        out = f64::from_bits(out.to_bits() | 1 << 63);
    }
    out
}

// Translated from UIO 2.30 (426d963), utility.cpp: StrToInt.
/// UIO's integer reading: an optional `-`, then digits (a wrap that makes the value smaller gives
/// `i32::MAX`, or `i32::MIN` after a `-`).
pub fn str_to_int(s: &[u8]) -> i32 {
    let at = |i: usize| *s.get(i).unwrap_or(&0);
    let neg = at(0) == b'-';
    let mut i = neg as usize;
    let mut v: i32 = 0;
    let mut prev: i32 = 0;
    loop {
        let c = at(i).wrapping_sub(b'0');
        if c > 9 {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add(c as i32);
        if v < prev {
            return (neg as i32).wrapping_add(0x7fff_ffff);
        }
        prev = v;
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
