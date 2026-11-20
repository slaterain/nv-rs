//! Virtual calls against the exe: does each `e.vcall(object, slot, &args![..])` in the crates pass
//! as many argument words as the function in that vtable slot takes?
//!
//! A translation that hands a virtual call one word too few stops the game ("call has N argument
//! words, needs more"); one word too many is silent, and is the sign of a word given to the wrong
//! call. A unit test does not find either when the callee is a stand-in that accepts any count
//! (two players' games stopped or drew wrong that way). This check needs no stand-in: it reads
//! the words from the exe itself.
//!
//! What it reads (nothing of the game is in the repository; both files are the contributor's own,
//! given by path):
//!
//! * the unpacked `FalloutNV.exe` 1.4.0.525: its sections and base relocations;
//! * a function map, tab-separated, one function per line: its address (8 hex digits) and its size
//!   in bytes (decimal) in the first two columns (a header line and further columns are ignored);
//! * the crates' source (not their tests).
//!
//! How it reads the exe:
//!
//! * a *table* is a run of relocated words in a data section that each hold the start of a
//!   function. A run is cut where any relocated word (in code or data) holds an address inside it
//!   (two vtables without type information lie back to back, and a constructor stores the
//!   second one's address);
//! * a function *takes* `1 + n / 4` words when it returns with `RET n` (`this` in `ECX`, the rest
//!   on the stack, which the function itself pops) and one word with a plain `RET`. A function
//!   that only jumps to another one (a thunk, with or without an adjustment of `ECX`, or a jump at
//!   its end) takes what that one takes. Where the last instruction is neither, the count is not
//!   read and the slot says so.
//!
//! The two checks:
//!
//! * **own**: a call on `this` in a unit that names vtables (a `const NAME: u32` whose value is
//!   the start of a table): the words must be what one of those tables takes at that slot;
//! * **none**: any other call: reported only when no table in the exe (and no table of methods the
//!   platform crate registers for a system object, such as the Direct3D device) takes that many
//!   words at that slot.
//!
//! An item of an `args![..]` list counts as two words when its text says so (an `f64`, `u64` or
//! `i64` literal, cast, conversion, `let` or parameter; a float literal without a suffix), else
//! as one. A hit is a place to look at in the exe, not a verdict: an `f64` the text does not show,
//! a slot whose function is variadic, or a `this` of a class the unit does not name all read as
//! hits.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

/// A section of the image: where it is in memory and in the file.
#[derive(Clone, Debug)]
struct Section {
    start: u32,
    /// Bytes present in the file.
    len: u32,
    file: usize,
    code: bool,
    data: bool,
}

/// The exe: its bytes, its sections and the addresses of its relocated words.
pub struct Image {
    bytes: Vec<u8>,
    sections: Vec<Section>,
    relocated: Vec<u32>,
}

fn le16(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes(
        b.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl Image {
    /// Reads a 32-bit PE file: the section table and the base relocations (the addresses of
    /// every word that holds an absolute address).
    pub fn parse(bytes: Vec<u8>) -> Result<Image, String> {
        let bad = |what: &str| format!("not a 32-bit PE file: {what}");
        let pe = le32(&bytes, 0x3c).ok_or_else(|| bad("too short"))? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(bad("no PE signature"));
        }
        let count = le16(&bytes, pe + 6).ok_or_else(|| bad("header"))? as usize;
        let optional_len = le16(&bytes, pe + 20).ok_or_else(|| bad("header"))? as usize;
        let optional = pe + 24;
        if le16(&bytes, optional) != Some(0x10b) {
            return Err(bad("optional header is not PE32"));
        }
        let base = le32(&bytes, optional + 28).ok_or_else(|| bad("header"))?;
        let mut sections = Vec::new();
        for i in 0..count {
            let at = optional + optional_len + 40 * i;
            let field = |k: usize| le32(&bytes, at + k).ok_or_else(|| bad("section table"));
            let (virtual_len, rva, raw_len, raw) = (field(8)?, field(12)?, field(16)?, field(20)?);
            let flags = field(36)?;
            sections.push(Section {
                start: base.wrapping_add(rva),
                len: virtual_len.min(raw_len),
                file: raw as usize,
                code: flags & 0x20 != 0,
                data: flags & 0x40 != 0,
            });
        }
        let mut image = Image {
            bytes,
            sections,
            relocated: Vec::new(),
        };
        // Data directory 5: the base relocations, blocks of (page, size, entries); an entry of
        // type 3 is a 32-bit address at the page plus its low 12 bits.
        let (rva, len) = (
            le32(&image.bytes, optional + 96 + 5 * 8).unwrap_or(0),
            le32(&image.bytes, optional + 96 + 5 * 8 + 4).unwrap_or(0),
        );
        let mut relocated = Vec::new();
        if let Some(mut at) = image.offset(base.wrapping_add(rva), len) {
            let end = at + len as usize;
            while at + 8 <= end {
                let page = le32(&image.bytes, at).unwrap_or(0);
                let size = le32(&image.bytes, at + 4).unwrap_or(0) as usize;
                if size < 8 || at + size > end {
                    break;
                }
                for entry in (at + 8..at + size).step_by(2) {
                    let word = le16(&image.bytes, entry).unwrap_or(0);
                    if word >> 12 == 3 {
                        relocated.push(base.wrapping_add(page).wrapping_add(word & 0xfff));
                    }
                }
                at += size;
            }
        }
        relocated.sort_unstable();
        image.relocated = relocated;
        Ok(image)
    }

    /// The file offset of `len` bytes at address `va`, when a section holds all of them.
    fn offset(&self, va: u32, len: u32) -> Option<usize> {
        let s = self
            .sections
            .iter()
            .find(|s| va >= s.start && va - s.start < s.len)?;
        let at = va - s.start;
        (len <= s.len - at).then_some(s.file + at as usize)
    }

    /// The word at address `va`.
    pub fn u32(&self, va: u32) -> Option<u32> {
        le32(&self.bytes, self.offset(va, 4)?)
    }

    /// `len` bytes at address `va`.
    pub fn bytes(&self, va: u32, len: u32) -> Option<&[u8]> {
        let at = self.offset(va, len)?;
        self.bytes.get(at..at + len as usize)
    }
}

/// The function map: start address to size in bytes. Lines that do not start with an address and
/// a size (the header) are skipped.
pub fn parse_map(text: &str) -> BTreeMap<u32, u32> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let mut columns = line.split('\t');
        let (Some(address), Some(size)) = (columns.next(), columns.next()) else {
            continue;
        };
        if let (Ok(address), Ok(size)) = (
            u32::from_str_radix(address.trim(), 16),
            size.trim().parse::<u32>(),
        ) {
            map.insert(address, size);
        }
    }
    map
}

/// The stack words a `RET` at the end of `code` pops (the padding after it left out).
fn ret_at_end(code: &[u8]) -> Option<u32> {
    let mut end = code.len();
    while end > 0 && matches!(code[end - 1], 0xcc | 0x90) {
        end -= 1;
    }
    let code = &code[..end];
    match *code {
        [.., 0xc2, low, 0] if low % 4 == 0 && low <= 0x60 => Some(u32::from(low) / 4),
        [.., 0xc3] => Some(0),
        _ => None,
    }
}

/// The stack words every `RET` after an epilogue instruction in `code` pops (`POP` of a saved
/// register, `LEAVE`, `MOV ESP, EBP`, `ADD ESP, n`), when they all agree; an empty set when none
/// is found.
fn rets_after_epilogues(code: &[u8]) -> BTreeSet<u32> {
    let mut found = BTreeSet::new();
    for i in 1..code.len() {
        let after_epilogue = matches!(code[i - 1], 0x5d | 0xc9 | 0x5b | 0x5e | 0x5f)
            || (i >= 3 && code[i - 3] == 0x83 && code[i - 2] == 0xc4)
            || (i >= 2 && code[i - 2] == 0x8b && code[i - 1] == 0xe5);
        if !after_epilogue {
            continue;
        }
        match code[i..] {
            [0xc3, ..] => {
                found.insert(0);
            }
            [0xc2, low, 0, ..] if low % 4 == 0 && low <= 0x60 => {
                found.insert(u32::from(low) / 4);
            }
            _ => {}
        }
    }
    found
}

/// The target of a `JMP rel32` whose five bytes start at `code[at]`, the instruction being at
/// address `va + at`.
fn jump_target(code: &[u8], at: usize, va: u32) -> Option<u32> {
    if code.get(at) != Some(&0xe9) {
        return None;
    }
    let rel = le32(code, at + 1)?;
    Some(va.wrapping_add(at as u32 + 5).wrapping_add(rel))
}

/// The stack words the function at `addr` pops when it returns (`RET n`: `n / 4`; plain `RET`:
/// none), or `None` when its end is not read.
pub fn stack_words(image: &Image, functions: &BTreeMap<u32, u32>, addr: u32) -> Option<u32> {
    stack_words_from(image, functions, addr, 0)
}

fn stack_words_from(
    image: &Image,
    functions: &BTreeMap<u32, u32>,
    addr: u32,
    depth: u32,
) -> Option<u32> {
    if depth > 6 {
        return None;
    }
    let size = function_size(image, functions, addr)?;
    let found = words_in(image, functions, addr, size, depth);
    if found.is_some() {
        return found;
    }
    // A map size can stop short of the function's last instructions: read up to the next
    // function's start instead (the padding between them is skipped).
    let next = functions.range(addr + 1..).next().map(|(&a, _)| a)?;
    let to_next = next - addr;
    (to_next > size && to_next - size <= 64)
        .then(|| words_in(image, functions, addr, to_next, depth))
        .flatten()
}

/// The size of the function at `addr`: the map's, or for an address in code between two of the
/// map's functions (one the map does not list), the bytes up to the next one (at most 8 KiB).
fn function_size(image: &Image, functions: &BTreeMap<u32, u32>, addr: u32) -> Option<u32> {
    if let Some(&size) = functions.get(&addr) {
        return Some(size);
    }
    let in_code = image
        .sections
        .iter()
        .any(|s| s.code && addr >= s.start && addr - s.start < s.len);
    let inside_previous = functions
        .range(..addr)
        .next_back()
        .is_some_and(|(&start, &size)| addr - start < size);
    if !in_code || inside_previous {
        return None;
    }
    let next = functions.range(addr..).next().map(|(&a, _)| a)?;
    if next - addr > 0x2000 {
        return None;
    }
    // The gap may hold several functions: the first ends at the first `RET` the compiler's
    // padding (`INT3`) follows.
    let code = image.bytes(addr, next - addr)?;
    let end = (0..code.len()).find_map(|i| match code[i..] {
        [0xc3, 0xcc, ..] => Some(i + 1),
        [0xc2, _, 0, 0xcc, ..] => Some(i + 3),
        _ => None,
    });
    Some(end.map_or(next - addr, |e| e as u32))
}

/// [`stack_words_from`] for the `size` bytes at `addr`.
fn words_in(
    image: &Image,
    functions: &BTreeMap<u32, u32>,
    addr: u32,
    size: u32,
    depth: u32,
) -> Option<u32> {
    let code = image.bytes(addr, size)?;
    // A thunk: `JMP target`, after `SUB ECX, n` or `ADD ECX, n` (a vtable of a second base) or
    // with nothing before it.
    let jump_at = match *code {
        [0xe9, ..] => Some(0),
        [0x83, 0xe9 | 0xc1, _, 0xe9, ..] => Some(3),
        [0x81, 0xe9 | 0xc1, _, _, _, _, 0xe9, ..] => Some(6),
        _ => None,
    };
    if let Some(at) = jump_at {
        if code.len() <= at + 5 + 3 {
            let target = jump_target(code, at, addr)?;
            return stack_words_from(image, functions, target, depth + 1);
        }
    }
    let after = rets_after_epilogues(code);
    if let Some(n) = ret_at_end(code) {
        // A `RET` at the end and another count after an epilogue inside: not read.
        return after.iter().all(|&a| a == n).then_some(n);
    }
    if after.len() == 1 {
        return after.first().copied();
    }
    // No `RET` at all: a jump to another function at the end takes that function's words.
    let mut end = code.len();
    while end > 0 && matches!(code[end - 1], 0xcc | 0x90) {
        end -= 1;
    }
    if after.is_empty() && end >= 5 {
        let target = jump_target(code, end - 5, addr)?;
        if target != addr && functions.contains_key(&target) {
            return stack_words_from(image, functions, target, depth + 1);
        }
    }
    None
}

/// The tables of the image: start address to the functions in its slots.
///
/// A table is a run of relocated words in a data section, each the start of a function of
/// `functions`. A run is cut at every address inside it that a relocated word (in a code or a
/// data section) holds. The units' constants do not cut: a constant may name one slot of a table
/// (a patch of that slot) and would cut a real table in two.
pub fn find_tables(image: &Image, functions: &BTreeMap<u32, u32>) -> BTreeMap<u32, Vec<u32>> {
    let in_data = |va: u32| {
        image
            .sections
            .iter()
            .any(|s| s.data && !s.code && va >= s.start && va - s.start < s.len)
    };
    // The runs.
    let mut runs: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut last = None;
    for &at in &image.relocated {
        if !in_data(at) {
            continue;
        }
        let Some(word) = image.u32(at) else { continue };
        if function_size(image, functions, word).is_none() {
            continue;
        }
        match runs.last_mut() {
            Some((_, run)) if last == Some(at.wrapping_sub(4)) => run.push(word),
            _ => runs.push((at, vec![word])),
        }
        last = Some(at);
    }
    // The addresses relocated words hold.
    let referred: HashSet<u32> = image
        .relocated
        .iter()
        .filter_map(|&at| image.u32(at))
        .collect();
    let mut tables = BTreeMap::new();
    for (start, run) in runs {
        let mut table_start = start;
        let mut table = Vec::new();
        for (i, &target) in run.iter().enumerate() {
            let at = start + 4 * i as u32;
            if i > 0 && referred.contains(&at) {
                tables.insert(table_start, std::mem::take(&mut table));
                table_start = at;
            }
            table.push(target);
        }
        tables.insert(table_start, table);
    }
    tables
}

/// For each slot (its byte offset divided by four): the word counts the tables take there, and
/// the functions there that were not read.
#[derive(Clone, Default, Debug)]
pub struct SlotCounts {
    pub counts: Vec<BTreeSet<u32>>,
    pub unread: Vec<BTreeSet<u32>>,
}

impl SlotCounts {
    fn slot(&mut self, index: usize) -> (&mut BTreeSet<u32>, &mut BTreeSet<u32>) {
        if self.counts.len() <= index {
            self.counts.resize(index + 1, BTreeSet::new());
            self.unread.resize(index + 1, BTreeSet::new());
        }
        (&mut self.counts[index], &mut self.unread[index])
    }

    /// Adds a count at `index`.
    pub fn add(&mut self, index: usize, words: u32) {
        self.slot(index).0.insert(words);
    }
}

/// The words (`this` and the stack words) each table's slots take; `None` where the function's
/// end was not read.
pub fn table_words(
    image: &Image,
    functions: &BTreeMap<u32, u32>,
    tables: &BTreeMap<u32, Vec<u32>>,
) -> BTreeMap<u32, Vec<Option<u32>>> {
    let mut memo: HashMap<u32, Option<u32>> = HashMap::new();
    tables
        .iter()
        .map(|(&start, slots)| {
            let words = slots
                .iter()
                .map(|&target| {
                    memo.entry(target)
                        .or_insert_with(|| stack_words(image, functions, target))
                        .map(|n| n + 1)
                })
                .collect();
            (start, words)
        })
        .collect()
}

/// A `.vcall(object, slot, list)` in a unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub line: usize,
    /// The address of the function the call is in (the nearest `Translated from` above it).
    pub caller: Option<u32>,
    /// Where the function the call is in starts in the unit's text (calls with the same scope
    /// are in one function).
    pub scope: usize,
    /// Where the object's name was last bound before the call (see [`binding`]).
    pub bound: usize,
    pub object: String,
    pub slot: u32,
    /// `this` and the list's words.
    pub words: u32,
    /// The call as written, on one line.
    pub text: String,
}

/// What a unit's text gives: its constants, its calls and the calls that were not read.
#[derive(Default, Debug)]
pub struct Unit {
    pub constants: HashMap<String, u32>,
    pub sites: Vec<Site>,
    /// Calls whose slot or list is not written in place: line and reason.
    pub unread: Vec<(usize, &'static str)>,
    /// Calls written in two steps that hand the table they read the target from to the target
    /// as its first word (the object, not its table, is `this`).
    pub passes_table: Vec<Site>,
    /// Direct calls with a constant callee and at least one word.
    pub direct: Vec<Direct>,
}

/// A direct call `.call(CALLEE, &args![first, ..])`.
#[derive(Clone, Debug, PartialEq)]
pub struct Direct {
    pub line: usize,
    pub scope: usize,
    pub callee: u32,
    /// The first word as written.
    pub first: String,
}

/// Where the unit's tests begin: the whole file for a test file, else its first
/// `#[cfg(test)] mod name {`.
fn test_start(path: &str, text: &str) -> usize {
    let path = path.replace('\\', "/");
    if path.ends_with("_tests.rs") || path.contains("/tests/") {
        return 0;
    }
    let mut from = 0;
    while let Some(found) = text[from..].find("#[cfg(test)]") {
        let at = from + found;
        let rest = text[at + "#[cfg(test)]".len()..].trim_start();
        let rest = rest.strip_prefix("pub ").unwrap_or(rest).trim_start();
        if let Some(rest) = rest.strip_prefix("mod ") {
            let name_end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if rest[name_end..].trim_start().starts_with('{') {
                return at;
            }
        }
        from = at + 1;
    }
    text.len()
}

/// A `u32` literal: hex or decimal, with underscores and an optional `u32` suffix.
fn number(text: &str) -> Option<u32> {
    let text = text.trim();
    let text = text.strip_suffix("u32").unwrap_or(text).replace('_', "");
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// Every `const NAME: u32 = literal;` of `text`.
fn constants(text: &str) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("const ") {
        let at = from + found + "const ".len();
        from = at;
        let rest = &text[at..];
        let Some(colon) = rest.find(':') else { break };
        let name = rest[..colon].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let Some(after) = rest[colon + 1..].trim_start().strip_prefix("u32") else {
            continue;
        };
        let Some(after) = after.trim_start().strip_prefix('=') else {
            continue;
        };
        let Some(end) = after.find(';') else { continue };
        if let Some(value) = number(&after[..end]) {
            out.insert(name.to_string(), value);
        }
    }
    out
}

/// The text between the bracket before `text[open..]` and its match, strings skipped; `open` is
/// the index after the opening bracket.
fn inside(text: &str, open: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `text` cut at its top-level commas, comments left out, each piece trimmed; an empty last piece
/// (a trailing comma) is dropped.
fn pieces(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut piece = String::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                piece.push_str(&text[start..(i + 1).min(text.len())]);
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b',' if depth == 0 => out.push(std::mem::take(&mut piece)),
            c => {
                match c {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                // Whole characters: the text is UTF-8 and only ASCII bytes are matched above.
                let ch_len = text[i..].chars().next().map_or(1, char::len_utf8);
                piece.push_str(&text[i..i + ch_len]);
                i += ch_len;
                continue;
            }
        }
        i += 1;
    }
    out.push(piece);
    let mut out: Vec<String> = out
        .into_iter()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    if out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

/// Whether `name` is declared with a two-word type (`name: f64`, `u64` or `i64`) in `before`
/// (the text of the function so far).
fn declared_two_words(before: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(found) = before[from..].find(name) {
        let at = from + found;
        from = at + name.len();
        let starts = at == 0 || {
            let c = before.as_bytes()[at - 1];
            !(c.is_ascii_alphanumeric() || c == b'_')
        };
        let rest = before[from..].trim_start();
        if starts {
            if let Some(ty) = rest.strip_prefix(':') {
                let ty = ty.trim_start();
                if ["f64", "u64", "i64"].iter().any(|t| {
                    ty.strip_prefix(t)
                        .is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric()))
                }) {
                    return true;
                }
            }
        }
    }
    false
}

/// The words of one item of an argument list, as its text gives them.
fn item_words(item: &str, before: &str) -> u32 {
    let two = ["f64", "u64", "i64"];
    // A cast at the end decides: `x as u32` is one word whatever `x` is, `x as f64` two.
    if let Some(at) = item.rfind(" as ") {
        let ty = item[at + " as ".len()..].trim();
        if ty.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return if two.contains(&ty) { 2 } else { 1 };
        }
    }
    // A read of two words that ends the item: `e.mem.f64(at)`.
    for t in two {
        let read = format!(".{t}(");
        if let Some(at) = item.rfind(&read) {
            if inside(item, at + read.len()).map(str::len) == Some(item.len() - at - read.len() - 1)
            {
                return 2;
            }
        }
    }
    let is_float_literal = {
        let digits = item.trim_start_matches('-');
        digits.contains('.')
            && digits
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '_')
    };
    let named = item.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !item.starts_with(|c: char| c.is_ascii_digit())
        && declared_two_words(before, item);
    if two.iter().any(|t| {
        item.ends_with(t)
            || item.ends_with(&format!(".{t}()"))
            || item.starts_with(&format!("{t}::"))
    }) || item.ends_with(".st0")
        || is_float_literal
        || named
    {
        2
    } else {
        1
    }
}

/// Reads a unit: its constants and every `.vcall(object, slot, list)` before its tests.
pub fn read_unit(path: &str, text: &str) -> Unit {
    let constants = constants(text);
    let end = test_start(path, text);
    let mut unit = Unit {
        constants,
        ..Unit::default()
    };
    let mut from = 0;
    while from < end {
        let Some(found) = text[from..end].find(".vcall(") else {
            break;
        };
        let open = from + found + ".vcall(".len();
        from = open;
        let line = 1 + text[..open].matches('\n').count();
        let Some(call) = inside(text, open) else {
            continue;
        };
        let parts = pieces(call);
        if parts.len() != 3 {
            unit.unread.push((line, "not (object, slot, list)"));
            continue;
        }
        let slot = number(&parts[1]).or_else(|| unit.constants.get(&parts[1]).copied());
        let Some(slot) = slot else {
            unit.unread.push((line, "the slot is not a number"));
            continue;
        };
        let list = parts[2].trim_start_matches('&').trim_start();
        let list = list.strip_prefix("args!").unwrap_or(list).trim_start();
        let Some(items) = list.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
            unit.unread.push((line, "the list is not written in place"));
            continue;
        };
        if pieces(items).last().is_some_and(|p| p.contains(';')) {
            unit.unread.push((line, "the list is a repeat"));
            continue;
        }
        let (caller, scope, function) = function_so_far(text, open);
        let words = 1 + pieces(items)
            .iter()
            .map(|item| item_words(item, function))
            .sum::<u32>();
        unit.sites.push(Site {
            line,
            caller,
            scope,
            bound: binding(text, scope, open, &parts[0]),
            object: parts[0].clone(),
            slot,
            words,
            text: format!(".vcall({})", parts.join(", ")),
        });
    }
    read_two_step_calls(text, end, &mut unit);
    unit
}

/// The function a call at `text[open]` is in, so far: the address of the nearest `Translated
/// from` marker above it and the text from that marker on.
fn function_so_far(text: &str, open: usize) -> (Option<u32>, usize, &str) {
    let before = &text[..open];
    let marker = before.rfind("Translated from ");
    let caller = marker.and_then(|at| {
        let digits: String = before[at + "Translated from ".len()..]
            .trim_start_matches("0x")
            .chars()
            .take(8)
            .collect();
        (digits.len() == 8)
            .then(|| u32::from_str_radix(&digits, 16).ok())
            .flatten()
    });
    // The text is the call's own function: from the later of the marker and the last `fn `
    // above the call (a helper after a translated function has the marker above it too).
    let fn_start = before
        .rfind("\nfn ")
        .into_iter()
        .chain(before.rfind(" fn "))
        .max();
    let start = marker.unwrap_or(0).max(fn_start.unwrap_or(0));
    // A second `fn` after the marker: the call is in a helper the marker does not name.
    let fns = before[marker.unwrap_or(0)..].matches("fn ").count();
    let caller = if fns > 1 { None } else { caller };
    (caller, start, &before[start..])
}

/// The expression the last `let name = ..;` (or `let name: type = ..;`) of `function` gives, its
/// whitespace removed.
fn last_let(function: &str, name: &str) -> Option<String> {
    let mut found = None;
    let mut from = 0;
    while let Some(at) = function[from..].find("let ") {
        let at = from + at + "let ".len();
        from = at;
        let rest = function[at..].trim_start();
        let rest = rest.strip_prefix("mut ").unwrap_or(rest);
        let Some(rest) = rest.strip_prefix(name) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = match rest.strip_prefix(':') {
            Some(typed) => match typed.find('=') {
                Some(eq) => &typed[eq..],
                None => continue,
            },
            None => rest,
        };
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        if let Some(end) = rest.find(';') {
            found = Some(rest[..end].split_whitespace().collect::<String>());
        }
    }
    found
}

/// Where `object` was last bound (`let object =`, `let mut object:` ..) between `scope` and
/// `open`, or `scope` when it was not: calls with the same binding are on the same object.
fn binding(text: &str, scope: usize, open: usize, object: &str) -> usize {
    let function = &text[scope..open];
    let mut found = scope;
    for prefix in ["let ", "let mut "] {
        let pattern = format!("{prefix}{object}");
        let mut from = 0;
        while let Some(at) = function[from..].find(&pattern) {
            let at = from + at;
            from = at + pattern.len();
            let next = function[from..].chars().next();
            if next.is_some_and(|c| !(c.is_alphanumeric() || c == '_')) {
                found = found.max(scope + at);
            }
        }
    }
    found
}

/// Whether `text` is a plain name.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !text.starts_with(|c: char| c.is_ascii_digit())
}

/// A call written in two steps, `let table = e.mem.u32(object); let target =
/// e.mem.u32(table + SLOT); e.call(target, &args![..])`: a virtual call whose words are the
/// whole list (`this` included). Read as a site on `object`; one that hands `table` itself over
/// as its first word is recorded in [`Unit::passes_table`].
fn read_two_step_calls(text: &str, end: usize, unit: &mut Unit) {
    let mut from = 0;
    while from < end {
        let Some(found) = text[from..end].find(".call(") else {
            break;
        };
        let open = from + found + ".call(".len();
        from = open;
        let Some(call) = inside(text, open) else {
            continue;
        };
        let parts = pieces(call);
        if parts.len() != 2 {
            continue;
        }
        let (caller, scope, function) = function_so_far(text, open);
        // A direct call: its callee and first word, for the tables a callee stores.
        let callee = number(&parts[0]).or_else(|| unit.constants.get(&parts[0]).copied());
        if let Some(callee) = callee {
            let list = parts[1].trim_start_matches('&').trim_start();
            let list = list.strip_prefix("args!").unwrap_or(list).trim_start();
            if let Some(first) = list
                .strip_prefix('[')
                .and_then(|l| l.strip_suffix(']'))
                .and_then(|items| pieces(items).into_iter().next())
            {
                unit.direct.push(Direct {
                    line: 1 + text[..open].matches('\n').count(),
                    scope,
                    callee,
                    first,
                });
            }
            continue;
        }
        if !is_name(&parts[0]) {
            continue;
        }
        let Some(target) = last_let(function, &parts[0]) else {
            continue;
        };
        let Some(read) = target
            .strip_prefix("e.mem.u32(")
            .and_then(|t| t.strip_suffix(')'))
        else {
            continue;
        };
        let (table, slot) = if let Some((table, slot)) = read.split_once(".wrapping_add(") {
            (table, slot.strip_suffix(')').unwrap_or(""))
        } else if let Some((table, slot)) = read.split_once('+') {
            (table, slot)
        } else {
            continue;
        };
        let Some(slot) = number(slot).or_else(|| unit.constants.get(slot).copied()) else {
            continue;
        };
        if !is_name(table) {
            continue;
        }
        let Some(object) = last_let(function, table).and_then(|t| {
            t.strip_prefix("e.mem.u32(")
                .and_then(|t| t.strip_suffix(')'))
                .map(str::to_string)
        }) else {
            continue;
        };
        // A table is read at the object's own address; a word read at an offset in it is a
        // pointer the object keeps (a stream, whose function pointers are called with it).
        if !is_name(object.strip_suffix(".addr()").unwrap_or(&object)) {
            continue;
        }
        let list = parts[1].trim_start_matches('&').trim_start();
        let list = list.strip_prefix("args!").unwrap_or(list).trim_start();
        let Some(items) = list.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
            continue;
        };
        let items = pieces(items);
        let words = items.iter().map(|item| item_words(item, function)).sum();
        let site = Site {
            line: 1 + text[..open].matches('\n').count(),
            caller,
            scope,
            bound: binding(text, scope, open, &object),
            object,
            slot,
            words,
            text: format!(".call({})", parts.join(", ")),
        };
        if items.first().map(String::as_str) == Some(table) {
            unit.passes_table.push(site.clone());
        }
        unit.sites.push(site);
    }
}

/// The tables of methods the platform crate registers for a system object, read from its
/// source: every `(index, crate::entry!(0, name(types) ..` gives the words of slot `index`.
pub fn platform_slots(text: &str, into: &mut SlotCounts) {
    let mut from = 0;
    while let Some(found) = text[from..].find("entry!(0,") {
        let at = from + found;
        from = at + 1;
        // The slot index: the number before the comma before `crate::entry!`.
        let head = text[..at].trim_end().trim_end_matches("crate::").trim_end();
        let Some(head) = head.strip_suffix(',') else {
            continue;
        };
        let digits_from = head
            .rfind(|c: char| !c.is_ascii_digit())
            .map_or(0, |i| i + 1);
        let Ok(index) = head[digits_from..].parse::<usize>() else {
            continue;
        };
        let Some(open) = text[at..].find('(').and_then(|first| {
            text[at + first + 1..]
                .find('(')
                .map(|second| at + first + 1 + second + 1)
        }) else {
            continue;
        };
        let Some(types) = inside(text, open) else {
            continue;
        };
        let words: u32 = pieces(types)
            .iter()
            .map(|ty| match ty.as_str() {
                "f64" | "u64" | "i64" => 2,
                _ => 1,
            })
            .sum();
        into.add(index, words);
    }
}

/// A call whose words no table takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    /// `own`: a call on `this` that none of the unit's own tables takes; `none`: a call no table
    /// anywhere takes; `group`: calls on one object in one function that no single table takes
    /// together (the site is the first of them).
    pub kind: &'static str,
    pub site: Site,
    /// What the tables take at the slot: for `own`, each of the unit's tables (`address=words`);
    /// for `none`, every count any table takes.
    pub takes: String,
}

/// Whether the object of a call is the unit's own `this`.
fn is_this(object: &str) -> bool {
    matches!(object, "this" | "this.addr()" | "this.0")
}

/// Checks a unit's calls. `words` is [`table_words`]' result, `anywhere` the counts of every
/// table (the exe's and the platform's) by slot, `platform` the platform's alone.
pub fn check_unit(
    unit: &Unit,
    words: &BTreeMap<u32, Vec<Option<u32>>>,
    anywhere: &SlotCounts,
    platform: &SlotCounts,
    stored: &HashMap<u32, u32>,
) -> Vec<Hit> {
    // The unit's own tables, by address.
    let own: BTreeMap<u32, &Vec<Option<u32>>> = unit
        .constants
        .values()
        .filter_map(|value| words.get(value).map(|table| (*value, table)))
        .collect();
    let mut hits: Vec<Hit> = unit
        .passes_table
        .iter()
        .map(|site| Hit {
            kind: "table",
            site: site.clone(),
            takes: "its first word is the table the target was read from".to_string(),
        })
        .collect();
    for site in &unit.sites {
        if unit.passes_table.contains(site) {
            continue;
        }
        if site.slot % 4 != 0 {
            continue;
        }
        let index = (site.slot / 4) as usize;
        if is_this(&site.object) {
            let known: Vec<(u32, u32)> = own
                .iter()
                .filter_map(|(&addr, table)| Some((addr, (*table.get(index)?)?)))
                .collect();
            if !known.is_empty() {
                if known.iter().all(|&(_, n)| n != site.words) {
                    let takes: Vec<String> = known
                        .iter()
                        .map(|(addr, n)| format!("{addr:08x}={n}"))
                        .collect();
                    hits.push(Hit {
                        kind: "own",
                        site: site.clone(),
                        takes: takes.join(" "),
                    });
                }
                continue;
            }
        }
        let counts = anywhere.counts.get(index);
        if counts.is_some_and(|c| c.contains(&site.words)) {
            continue;
        }
        let unread = anywhere.unread.get(index);
        let takes: Vec<String> = counts.into_iter().flatten().map(u32::to_string).collect();
        let count = unread.map_or(0, BTreeSet::len);
        // The functions not read, when they are few: one of them is likely the callee.
        let listed = match unread {
            Some(set) if !set.is_empty() && set.len() <= 8 => {
                let a: Vec<String> = set.iter().map(|a| format!("{a:08x}")).collect();
                format!(" ({})", a.join(" "))
            }
            _ => String::new(),
        };
        hits.push(Hit {
            kind: "none",
            site: site.clone(),
            takes: format!("{{{}}} and {count} not read{listed}", takes.join(",")),
        });
    }
    hits.extend(check_groups(unit, &hits, words, platform));
    hits.extend(check_typed(unit, &hits, words, stored));
    hits
}

/// The table a function stores at an object's start (`MOV dword ptr [reg], table`, the way a
/// constructor or destructor sets an object's class): the last such store in its code.
pub fn stored_table(
    image: &Image,
    functions: &BTreeMap<u32, u32>,
    tables: &BTreeMap<u32, Vec<u32>>,
    addr: u32,
) -> Option<u32> {
    let size = *functions.get(&addr)?;
    let from = image.relocated.partition_point(|&at| at < addr);
    image.relocated[from..]
        .iter()
        .take_while(|&&at| at < addr + size)
        .filter(|&&at| at >= addr + 2)
        .filter(|&&at| {
            let op = image.bytes(at - 2, 2);
            // C7 /0 with mod 00 and a plain register (not SIB, not an absolute address).
            op.is_some_and(|b| b[0] == 0xc7 && b[1] & 0xf8 == 0 && !matches!(b[1], 4 | 5))
        })
        .filter_map(|&at| image.u32(at))
        .filter(|value| tables.contains_key(value))
        .last()
}

/// Calls on an object a direct call before them in the same function handed as its first word
/// to a function that stores a table at that object's start (its constructor): the object's
/// class is that table's (or one derived from it, whose functions take the same words).
fn check_typed(
    unit: &Unit,
    hits: &[Hit],
    words: &BTreeMap<u32, Vec<Option<u32>>>,
    stored: &HashMap<u32, u32>,
) -> Vec<Hit> {
    let mut out = Vec::new();
    for site in &unit.sites {
        if site.slot % 4 != 0 || hits.iter().any(|h| h.site == *site) {
            continue;
        }
        let typed = unit.direct.iter().rev().find(|d| {
            d.scope == site.scope
                && d.line <= site.line
                && d.first == site.object
                && stored.contains_key(&d.callee)
        });
        let Some(direct) = typed else { continue };
        let table_at = stored[&direct.callee];
        let Some(table) = words.get(&table_at) else {
            continue;
        };
        let takes = match table.get((site.slot / 4) as usize) {
            Some(Some(n)) if *n == site.words => continue,
            Some(Some(n)) => format!("{table_at:08x}={n} (stored by {:08x})", direct.callee),
            // Not read, or past the table's end: the stored table is a base class's (a derived
            // constructor stores its own after calling the base one) or was cut short.
            Some(None) | None => continue,
        };
        out.push(Hit {
            kind: "typed",
            site: site.clone(),
            takes,
        });
    }
    out
}

/// The calls on one object in one function, taken together: each alone may be what some table
/// takes, but one object has one table, so some table must take all of them. Calls already hit
/// and calls on `this` held to the unit's own tables are left out. A slot a table does not read
/// counts as taking anything there; the platform's registered methods count as one table.
fn check_groups(
    unit: &Unit,
    hits: &[Hit],
    words: &BTreeMap<u32, Vec<Option<u32>>>,
    platform: &SlotCounts,
) -> Vec<Hit> {
    let own_checked = |site: &Site| {
        is_this(&site.object)
            && unit.constants.values().any(|v| {
                words
                    .get(v)
                    .and_then(|t| t.get((site.slot / 4) as usize))
                    .is_some_and(Option::is_some)
            })
    };
    let mut groups: BTreeMap<(usize, usize, &str), Vec<&Site>> = BTreeMap::new();
    for site in &unit.sites {
        if site.slot % 4 != 0 || own_checked(site) || hits.iter().any(|h| h.site == *site) {
            continue;
        }
        groups
            .entry((site.scope, site.bound, site.object.as_str()))
            .or_default()
            .push(site);
    }
    let mut out = Vec::new();
    for sites in groups.values() {
        let slots: BTreeSet<u32> = sites.iter().map(|s| s.slot).collect();
        if slots.len() < 2 {
            continue;
        }
        let table_takes = |table: &Vec<Option<u32>>| {
            sites
                .iter()
                .all(|s| match table.get((s.slot / 4) as usize) {
                    Some(Some(n)) => *n == s.words,
                    Some(None) => true,
                    None => false,
                })
        };
        let platform_takes = sites.iter().all(|s| {
            platform
                .counts
                .get((s.slot / 4) as usize)
                .is_some_and(|c| c.contains(&s.words))
        });
        if platform_takes || words.values().any(table_takes) {
            continue;
        }
        let calls: Vec<String> = sites
            .iter()
            .map(|s| format!("{}:{:#x}={}", s.line, s.slot, s.words))
            .collect();
        out.push(Hit {
            kind: "group",
            site: sites[0].clone(),
            takes: format!("no table takes all of {}", calls.join(" ")),
        });
    }
    out
}

/// Every `.rs` file under `dir`, sorted.
fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The words (`this` and the stack words) each function of `addresses` takes as the check reads
/// them, one line each (`address<TAB>words`, or `not read`): for checking the reading by hand.
pub fn words_of(exe: &Path, map: &Path, addresses: &[u32]) -> Result<Vec<String>, String> {
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let image = Image::parse(read(exe)?)?;
    let functions = parse_map(&String::from_utf8_lossy(&read(map)?));
    let tables = find_tables(&image, &functions);
    Ok(addresses
        .iter()
        .map(|&a| {
            let words = match stack_words(&image, &functions, a) {
                Some(n) => (n + 1).to_string(),
                None => "not read".to_string(),
            };
            // The tables that hold it, and the slot (byte offset) there.
            let held: Vec<String> = tables
                .iter()
                .flat_map(|(start, slots)| {
                    slots
                        .iter()
                        .enumerate()
                        .filter(move |(_, &f)| f == a)
                        .map(move |(i, _)| format!("{start:08x}+{:#x}", 4 * i))
                })
                .collect();
            let mut line = format!("{a:08x}\t{words}\t{}", held.join(" "));
            // A table's start: each slot's function and words.
            if let Some(slots) = tables.get(&a) {
                for (i, &f) in slots.iter().enumerate() {
                    let n = stack_words(&image, &functions, f).map(|n| n + 1);
                    let n = n.map_or_else(|| "?".to_string(), |n| n.to_string());
                    line.push_str(&format!("\n  +{:#x}\t{f:08x}\t{n}", 4 * i));
                }
            }
            line
        })
        .collect())
}

/// The whole check: reads the exe, the map and the crates under `crates`, and returns one line
/// per hit (tab-separated: kind, file:line, the caller's address, slot, words passed, what the
/// tables take, the call) and a summary.
pub fn run(exe: &Path, map: &Path, crates: &Path) -> Result<(Vec<String>, String), String> {
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let image = Image::parse(read(exe)?)?;
    if image.relocated.is_empty() {
        return Err(format!(
            "{}: no base relocations (tables are read from them; the unpacked exe keeps them)",
            exe.display()
        ));
    }
    let functions = parse_map(&String::from_utf8_lossy(&read(map)?));
    if functions.is_empty() {
        return Err(format!("{}: no functions read", map.display()));
    }
    let mut files = Vec::new();
    rust_files(crates, &mut files);
    let mut units = Vec::new();
    let mut platform = SlotCounts::default();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(crates)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("engine-platform/") {
            platform_slots(&text, &mut platform);
        }
        let unit = read_unit(&rel, &text);
        units.push((rel, unit));
    }
    let tables = find_tables(&image, &functions);
    let words = table_words(&image, &functions, &tables);
    let mut anywhere = platform.clone();
    let (mut slots, mut unread_slots) = (0usize, 0usize);
    for (start, table) in &words {
        for (index, taken) in table.iter().enumerate() {
            slots += 1;
            let (counts, unread) = anywhere.slot(index);
            match taken {
                Some(n) => {
                    counts.insert(*n);
                }
                None => {
                    unread.insert(tables[start][index]);
                    unread_slots += 1;
                }
            }
        }
    }
    let mut stored = HashMap::new();
    for (_, unit) in &units {
        for direct in &unit.direct {
            if let std::collections::hash_map::Entry::Vacant(slot) = stored.entry(direct.callee) {
                if let Some(table) = stored_table(&image, &functions, &tables, direct.callee) {
                    slot.insert(table);
                }
            }
        }
    }
    let mut lines = Vec::new();
    let (mut sites, mut unread, mut own_checked) = (0usize, 0usize, 0usize);
    for (rel, unit) in &units {
        sites += unit.sites.len();
        unread += unit.unread.len();
        own_checked += unit
            .sites
            .iter()
            .filter(|s| {
                is_this(&s.object)
                    && unit.constants.values().any(|v| {
                        words
                            .get(v)
                            .and_then(|t| t.get((s.slot / 4) as usize))
                            .is_some_and(Option::is_some)
                    })
            })
            .count();
        for hit in check_unit(unit, &words, &anywhere, &platform, &stored) {
            let caller = hit
                .site
                .caller
                .map_or_else(String::new, |c| format!("{c:08x}"));
            lines.push(format!(
                "{}\t{rel}:{}\t{caller}\t{:#x}\t{}\t{}\t{}",
                hit.kind, hit.site.line, hit.site.slot, hit.site.words, hit.takes, hit.site.text
            ));
        }
    }
    let summary = format!(
        "{} tables with {slots} slots ({unread_slots} not read) from {} relocated words; \
         {sites} virtual calls read ({own_checked} against the unit's own tables), {unread} not \
         read (slot or list not written in place); {} hits",
        tables.len(),
        image.relocated.len(),
        lines.len()
    );
    Ok((lines, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: u32 = 0x0040_0000;
    const TEXT: u32 = BASE + 0x1000;
    const RDATA: u32 = BASE + 0x2000;

    /// A PE image with a code section at `TEXT`, a data section at `RDATA` and base relocations
    /// for the addresses `relocated`.
    fn image(text: &[u8], rdata: &[u8], relocated: &[u32]) -> Image {
        let mut reloc = Vec::new();
        for page in [0x1000u32, 0x2000] {
            let entries: Vec<u16> = relocated
                .iter()
                .map(|va| va - BASE)
                .filter(|rva| rva & !0xfff == page)
                .map(|rva| 0x3000 | (rva & 0xfff) as u16)
                .collect();
            if entries.is_empty() {
                continue;
            }
            reloc.extend_from_slice(&page.to_le_bytes());
            reloc.extend_from_slice(&(8 + 2 * entries.len() as u32).to_le_bytes());
            for e in entries {
                reloc.extend_from_slice(&e.to_le_bytes());
            }
        }
        let mut file = vec![0u8; 0x400];
        file[0x3c] = 0x80;
        file[0x80..0x84].copy_from_slice(b"PE\0\0");
        file[0x86] = 3; // sections
        file[0x94] = 0xe0; // optional header size
        file[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        file[0x98 + 28..0x98 + 32].copy_from_slice(&BASE.to_le_bytes());
        file[0x98 + 96 + 40..0x98 + 96 + 44].copy_from_slice(&0x3000u32.to_le_bytes());
        file[0x98 + 96 + 44..0x98 + 96 + 48].copy_from_slice(&(reloc.len() as u32).to_le_bytes());
        let table = 0x98 + 0xe0;
        let mut at = 0x400u32;
        for (i, (rva, body, flags)) in [
            (0x1000u32, text, 0x6000_0020u32),
            (0x2000, rdata, 0x4000_0040),
            (0x3000, &reloc[..], 0x4200_0040),
        ]
        .into_iter()
        .enumerate()
        {
            let s = table + 40 * i;
            for (k, v) in [
                (8, body.len() as u32),
                (12, rva),
                (16, body.len() as u32),
                (20, at),
            ] {
                file[s + k..s + k + 4].copy_from_slice(&v.to_le_bytes());
            }
            file[s + 36..s + 40].copy_from_slice(&flags.to_le_bytes());
            at += body.len() as u32;
        }
        file.extend_from_slice(text);
        file.extend_from_slice(rdata);
        file.extend_from_slice(&reloc);
        Image::parse(file).expect("the image parses")
    }

    /// Code laid out function by function: returns the bytes and the map (address to size).
    fn code(functions: &[&[u8]]) -> (Vec<u8>, BTreeMap<u32, u32>) {
        let mut bytes = Vec::new();
        let mut map = BTreeMap::new();
        for f in functions {
            map.insert(TEXT + bytes.len() as u32, f.len() as u32);
            bytes.extend_from_slice(f);
        }
        (bytes, map)
    }

    fn words_of(addresses: &[u32]) -> Vec<u8> {
        addresses.iter().flat_map(|a| a.to_le_bytes()).collect()
    }

    #[test]
    fn the_map_reads_addresses_and_sizes_and_skips_the_header() {
        let map = parse_map("address\tsize\tname\n00401000\t21\tx\n00401020\t10\n\nbad\tline\n");
        assert_eq!(
            map.into_iter().collect::<Vec<_>>(),
            [(0x0040_1000, 21), (0x0040_1020, 10)]
        );
    }

    #[test]
    fn a_function_takes_what_its_ret_pops() {
        let (text, map) = code(&[
            &[0x55, 0x8b, 0xec, 0x5d, 0xc2, 0x08, 0x00], // RET 8
            &[0xb8, 0x0d, 0x00, 0x00, 0x00, 0xc3],       // plain RET
            &[0x5d, 0xc2, 0x04, 0x00, 0xcc, 0xcc],       // RET 4, then padding
            &[0xe9, 0xe8, 0xff, 0xff, 0xff],             // JMP to the first
            &[0x83, 0xe9, 0x04, 0xe9, 0xe0, 0xff, 0xff, 0xff], // SUB ECX,4; JMP to the first
            &[0x55, 0x5d, 0xc3, 0x55, 0x5d, 0xc2, 0x04, 0x00], // two different RETs: not read
            &[0x90, 0xe8, 0x00, 0x00, 0x00, 0x00, 0xcc], // ends in a call: not read
            &[0x33, 0xc0, 0xe9, 0xca, 0xff, 0xff, 0xff], // work, then JMP to the first
        ]);
        let image = image(&text, &[], &[]);
        let at: Vec<u32> = map.keys().copied().collect();
        let got: Vec<Option<u32>> = at.iter().map(|&a| stack_words(&image, &map, a)).collect();
        assert_eq!(
            got,
            [
                Some(2),
                Some(0),
                Some(1),
                Some(2),
                Some(2),
                None,
                None,
                Some(2)
            ]
        );
        assert_eq!(
            stack_words(&image, &map, TEXT + 1),
            None,
            "not a function start"
        );
    }

    /// Two tables back to back: the second starts where code refers to it; a word that is not
    /// relocated, or not a function, ends a run.
    #[test]
    fn tables_are_runs_of_function_words_cut_where_code_refers_to_them() {
        let (mut text, map) = code(&[&[0xc3], &[0xc2, 0x04, 0x00], &[0xc2, 0x08, 0x00]]);
        let f: Vec<u32> = map.keys().copied().collect();
        // Code that refers to the second table: MOV dword ptr [ECX], RDATA + 8.
        let refers = TEXT + text.len() as u32 + 2;
        text.extend_from_slice(&[0xc7, 0x01]);
        text.extend_from_slice(&(RDATA + 8).to_le_bytes());
        let rdata = words_of(&[f[0], f[1], f[2], f[1], 0x1234_5678, f[2], f[0]]);
        let relocated = [RDATA, RDATA + 4, RDATA + 8, RDATA + 12, RDATA + 20, refers];
        let image = image(&text, &rdata, &relocated);
        let tables = find_tables(&image, &map);
        assert_eq!(
            tables.clone().into_iter().collect::<Vec<_>>(),
            [
                (RDATA, vec![f[0], f[1]]),
                (RDATA + 8, vec![f[2], f[1]]),
                // The last word holds a function's address but is not relocated: not in the run.
                (RDATA + 20, vec![f[2]]),
            ]
        );
        let words = table_words(&image, &map, &tables);
        assert_eq!(words[&(RDATA + 8)], [Some(3), Some(2)]);
    }

    const UNIT: &str = "
const TABLE: u32 = 0x0040_2000;
const SLOT_B: u32 = 4;
// Translated from 00401000 (decompiled)
pub fn f(e: &mut Engine, this: u32, other: u32, scale: f64) {
    e.vcall(this, 0, &args![]);
    e.vcall(this.addr(), SLOT_B, &args![other, 1u32]);
    e.vcall(
        other,
        0x8,
        &args![
            scale, // two words
            e.mem.f64(other + 8),
            1.5,
            f(a, b),
        ],
    );
    e.vcall(other, slot, &args![]);
    e.vcall(other, 4, &words);
    e.vcall(other, 4, &[0; 3]);
}
#[cfg(test)]
mod tests {
    fn t(e: &mut Engine) { e.vcall(1, 0, &args![1, 2, 3]); }
}
";

    #[test]
    fn a_unit_gives_its_constants_and_its_calls_before_the_tests() {
        let unit = read_unit("crate/src/units/x.rs", UNIT);
        assert_eq!(unit.constants["TABLE"], 0x0040_2000);
        let read: Vec<(usize, &str, u32, u32)> = unit
            .sites
            .iter()
            .map(|s| (s.line, s.object.as_str(), s.slot, s.words))
            .collect();
        assert_eq!(
            read,
            [
                (6, "this", 0, 1),
                (7, "this.addr()", 4, 3),
                // An f64 parameter, an f64 read, a float literal and a call of unknown type.
                (8, "other", 8, 8),
            ]
        );
        assert_eq!(unit.sites[0].caller, Some(0x0040_1000));
        let reasons: Vec<&str> = unit.unread.iter().map(|u| u.1).collect();
        assert_eq!(
            reasons,
            [
                "the slot is not a number",
                "the list is not written in place",
                "the list is a repeat"
            ]
        );
        // A test file is not read at all.
        assert!(read_unit("crate/src/units/x_tests.rs", UNIT)
            .sites
            .is_empty());
    }

    #[test]
    fn the_platform_tables_give_their_slots_words() {
        let mut slots = SlotCounts::default();
        platform_slots(
            "(3, crate::entry!(0, release(Ptr) -> u32).1),\n(\n 5,\n crate::entry!(0, set(Ptr, u32, f64)).1,\n),",
            &mut slots,
        );
        assert_eq!(slots.counts[3], BTreeSet::from([1]));
        assert_eq!(slots.counts[5], BTreeSet::from([4]));
        assert!(slots.counts[4].is_empty());
    }

    /// A call on `this` is held to the unit's own table; any other call only to "some table
    /// takes that many words at that slot".
    #[test]
    fn calls_are_checked_against_the_own_table_or_any_table() {
        let (text, map) = code(&[&[0xc3], &[0xc2, 0x04, 0x00], &[0xc2, 0x08, 0x00]]);
        let f: Vec<u32> = map.keys().copied().collect();
        // The unit's table takes (1, 2) words; another table takes (3, 3).
        let rdata = words_of(&[f[0], f[1], 0, f[2], f[2]]);
        let image = image(&text, &rdata, &[RDATA, RDATA + 4, RDATA + 12, RDATA + 16]);
        let tables = find_tables(&image, &map);
        let words = table_words(&image, &map, &tables);
        let mut anywhere = SlotCounts::default();
        for table in words.values() {
            for (i, n) in table.iter().enumerate() {
                anywhere.add(i, n.unwrap());
            }
        }
        let unit = read_unit(
            "x.rs",
            "const TABLE: u32 = 0x0040_2000;
fn f() {
    e.vcall(this, 0, &args![]);
    e.vcall(this, 4, &args![a]);
    e.vcall(this, 4, &args![a, b]);
    e.vcall(this, 8, &args![a]);
    e.vcall(other, 4, &args![a, b]);
    e.vcall(other, 4, &args![a, b, c]);
    e.vcall(other, 0, &args![a]);
}",
        );
        let hits = check_unit(
            &unit,
            &words,
            &anywhere,
            &SlotCounts::default(),
            &HashMap::new(),
        );
        let got: Vec<(&str, usize, &str)> = hits
            .iter()
            .map(|h| (h.kind, h.site.line, h.takes.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                // Three words on `this` at slot 4: the own table takes two (another table's
                // three does not excuse it).
                ("own", 5, "00402000=2"),
                // Slot 8 is past the own table, and no table has it.
                ("none", 6, "{} and 0 not read"),
                // Four words at slot 4: no table takes four.
                ("none", 8, "{2,3} and 0 not read"),
                // Two words at slot 0: tables take one or three.
                ("none", 9, "{1,3} and 0 not read"),
            ]
        );
    }

    /// A map size that stops short is read to the next function; a function the map does not
    /// list (between two it does) ends at its first `RET` the padding follows.
    #[test]
    fn short_and_missing_map_functions_are_read_from_their_bytes() {
        let (text, mut map) = code(&[
            &[0x55, 0x5d, 0xc2, 0x04, 0x00, 0xcc], // RET 4, map size cut to 2
            &[0xc3],
            &[0xc2, 0x08, 0x00, 0xcc, 0xc3, 0xcc, 0xcc, 0xcc], // RET 8 + a second, unlisted
            &[0xc3],
        ]);
        let f: Vec<u32> = map.keys().copied().collect();
        map.insert(f[0], 2);
        map.remove(&f[2]);
        let image = image(&text, &[], &[]);
        assert_eq!(stack_words(&image, &map, f[0]), Some(1));
        assert_eq!(stack_words(&image, &map, f[2]), Some(2), "the first RET 8");
        assert_eq!(
            stack_words(&image, &map, f[0] + 1),
            None,
            "inside a listed function"
        );
    }

    /// A cast at the end of an item decides its words, whatever the value's own type.
    #[test]
    fn a_cast_decides_an_items_words() {
        let function = "fn f(x: f64, n: u32) {";
        assert_eq!(item_words("x", function), 2);
        assert_eq!(item_words("x as u32", function), 1);
        assert_eq!(item_words("n as f64", function), 2);
        assert_eq!(item_words("n", function), 1);
    }

    /// An exe without base relocations has no tables to read: an error, not an empty result.
    #[test]
    fn an_exe_without_relocations_is_refused() {
        let (text, _) = code(&[&[0xc3]]);
        let dir = std::env::temp_dir().join(format!("vcall-words-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("no-reloc.exe");
        let map = dir.join("map.tsv");
        let mut bytes = vec![0u8; 0x400];
        bytes[0x3c] = 0x80;
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x94] = 0xe0;
        bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes.extend_from_slice(&text);
        std::fs::write(&exe, bytes).unwrap();
        std::fs::write(&map, "00401000\t1\n").unwrap();
        let result = run(&exe, &map, &dir);
        std::fs::remove_file(&exe).ok();
        std::fs::remove_file(&map).ok();
        std::fs::remove_dir(&dir).ok();
        assert!(result.unwrap_err().contains("no base relocations"));
    }

    /// The checks that need no table named by the unit: an object handed to its constructor
    /// (which stores its class's table at the object's start) is held to that table; calls on
    /// one object in one function must all fit one table; a call written in two steps must not
    /// hand over the table it read the target from.
    #[test]
    fn typed_group_and_two_step_calls_are_checked() {
        // f0 takes 1 word, f1 2; constructors A and B store tables A (1, 2) and B (2, 1).
        let mut fns: Vec<Vec<u8>> = vec![vec![0xc3], vec![0xc2, 0x04, 0x00]];
        for table in [RDATA, RDATA + 8] {
            let mut ctor = vec![0xc7, 0x01];
            ctor.extend_from_slice(&table.to_le_bytes());
            ctor.push(0xc3);
            fns.push(ctor);
        }
        let slices: Vec<&[u8]> = fns.iter().map(Vec::as_slice).collect();
        let (text, map) = code(&slices);
        let f: Vec<u32> = map.keys().copied().collect();
        let rdata = words_of(&[f[0], f[1], f[1], f[0]]);
        let relocated = [RDATA, RDATA + 4, RDATA + 8, RDATA + 12, f[2] + 2, f[3] + 2];
        let image = image(&text, &rdata, &relocated);
        let tables = find_tables(&image, &map);
        assert_eq!(
            tables.len(),
            2,
            "cut where constructor B refers to its table"
        );
        let words = table_words(&image, &map, &tables);
        let mut anywhere = SlotCounts::default();
        for table in words.values() {
            for (i, n) in table.iter().enumerate() {
                anywhere.add(i, n.unwrap());
            }
        }
        let unit = read_unit(
            "x.rs",
            &format!(
                "const CTOR: u32 = {:#x};
// Translated from 00401000 (decompiled)
fn g() {{
    e.call(CTOR, &args![obj]);
    e.vcall(obj, 4, &args![]);
    e.vcall(p, 0, &args![a]);
    e.vcall(p, 4, &args![a]);
    let vtable = e.mem.u32(dev);
    let target = e.mem.u32(vtable + 4);
    e.call(target, &args![vtable, dev]);
}}",
                f[2]
            ),
        );
        let stored: HashMap<u32, u32> = unit
            .direct
            .iter()
            .filter_map(|d| Some((d.callee, stored_table(&image, &map, &tables, d.callee)?)))
            .collect();
        assert_eq!(stored, HashMap::from([(f[2], RDATA)]));
        let hits = check_unit(&unit, &words, &anywhere, &SlotCounts::default(), &stored);
        let got: Vec<(&str, usize)> = hits.iter().map(|h| (h.kind, h.site.line)).collect();
        assert_eq!(
            got,
            [
                // The two-step call hands over the table: always reported.
                ("table", 10),
                // p: two words at slot 0 (table B) and two at slot 4 (table A): no one table.
                ("group", 6),
                // obj is of table A, whose slot 4 takes two words, not one.
                ("typed", 5),
            ]
        );
    }
}
