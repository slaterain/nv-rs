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
//! The checks (the first word of each output line):
//!
//! * **own**: a call on `this` in a unit that names vtables (a `const NAME: u32` whose value is
//!   the start of a table): the words must be what one of those tables takes at that slot;
//! * **none**: any other call: reported only when no table in the exe (and no table of methods the
//!   platform crate registers for a system object, such as the Direct3D device) takes that many
//!   words at that slot;
//! * **group**: calls on one object (one binding of its name) in one function that no single
//!   table takes together;
//! * **typed**: a call on an object that a direct call earlier in the function handed, as its
//!   first word, to a function storing a table at the object's start (its constructor): held to
//!   that table;
//! * **table**: a call written in two steps (`let t = e.mem.u32(obj); let f = e.mem.u32(t +
//!   SLOT); e.call(f, ..)`) whose first word is `t`, the table, where the object belongs;
//! * **unread** (with `--unread`): a `.vcall` whose slot is not a number or a constant of the
//!   unit, or whose list is not written in place, with the reason.
//!
//! What it reads of the source: the game code of every `.rs` file. Test code is left out (its
//! stand-ins' made-up addresses would clash with the code's names): test files (under `tests/`,
//! named `tests.rs`, `test.rs`, `*_tests.rs`, `*_test.rs`, or mounted by a `#[cfg(test)] mod
//! NAME;`) and the span of each item marked `#[cfg(test)]` (or `cfg(all(test, ..))`); the code
//! before and after such a module is read. A slot is read from a number, constant arithmetic, a
//! constant (the function's own, the file's, a crate's `pub` one, the crate's, the tree's: where
//! every definition agrees; a name the file defines twice or unreadably is looked up nowhere
//! else), a `let`, a `for` over a list, an `if`/`else` or a `match` of constants. A call inside a
//! wrapper (a function or closure that takes the slot or the whole list as a parameter) is read
//! at each of the wrapper's callers, through callers that hand the parameter on in turn.
//!
//! An item of an `args![..]` list counts as two words when its text says so (an `f64`, `u64` or
//! `i64` literal; a cast to one at the item's end; a read `.f64(..)`, `.u64(..)` or `.i64(..)`
//! ending the item; a path `f64::..`; `.st0`; a float literal without a suffix; a plain name
//! declared with one of those types in the function's text, as a parameter or as a `let` with a
//! type annotation), else as one. A hit is a place to look at in the exe, not a verdict: an `f64`
//! the text does not show, a slot whose function is variadic, or a `this` of a class the unit
//! does not name all read as hits.
//!
//! What a clean run does not prove:
//!
//! * calls whose slot or list the text does not give are not checked (`--unread` lists each,
//!   with a wrapper's callers one line each);
//! * a wrapper is found by its name: a caller that reaches it another way (a function pointer, a
//!   trait object, a macro) is not read, and is not listed;
//! * a name bound in a way not described above (a pattern, a closure handed to another
//!   function) can give a wrong slot; a name bound in a block that closed, or changed after its
//!   binding, is listed unread;
//! * two-step calls in any shape other than the one above are not read at all;
//! * code that is not a `.vcall` (a hand-written call through a vtable in another shape, a COM
//!   call through a raw pointer) is not read at all;
//! * slots that are not a multiple of 4 are skipped;
//! * a value whose words the text does not show (a two-word `f64` in a name of unknown type, a
//!   16-byte value) counts as one word: that can hide a call with too many words as well as make
//!   a false hit;
//! * **none** compares with every table at that slot: it finds only counts that no class takes
//!   there, so most wrong counts on an object of unknown class pass it.

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
            // An empty entry, or one running past the address space, is no function.
            if size > 0 && address.checked_add(size).is_some() {
                map.insert(address, size);
            }
        }
    }
    map
}

/// The stack words a `RET` at the end of `code` (the function at `addr`) pops, the padding after
/// it left out.
fn ret_at_end(code: &[u8], addr: u32, functions: &BTreeMap<u32, u32>) -> Option<u32> {
    let mut end = code.len();
    while end > 0 && matches!(code[end - 1], 0xcc | 0x90) {
        end -= 1;
    }
    let code = &code[..end];
    let words = match *code {
        [.., 0xc2, low, 0] if low % 4 == 0 && low <= 0x60 => u32::from(low) / 4,
        [.., 0xc3] => 0,
        _ => return None,
    };
    // The bytes may be the operand of a jump that ends the function instead: `EB C3` (a short
    // jump, also `7x` and `E3`, whose one-byte operand is the last byte) or `E9 xx C2 08 00` (a
    // near jump, also `0F 8x`, whose four-byte operand ends the code). Such a jump can only go
    // into the function itself or to another function's start: where its target could be
    // neither (a short getter whose last operand byte happens to be `74`), the `RET` is real.
    let possible =
        |target: u32| target.wrapping_sub(addr) < end as u32 || functions.contains_key(&target);
    let end_va = addr.wrapping_add(end as u32);
    let short = |b: u8| b == 0xeb || b == 0xe3 || (0x70..=0x7f).contains(&b);
    // `C3` as a signed byte is -0x3d.
    let last_is_short_operand = code.last() == Some(&0xc3)
        && end >= 2
        && short(code[end - 2])
        && possible(end_va.wrapping_sub(0x3d));
    let near_jump = end >= 5
        && (code[end - 5] == 0xe9
            || (end >= 6 && code[end - 6] == 0x0f && code[end - 5] & 0xf0 == 0x80));
    let near_operand =
        near_jump && le32(code, end - 4).is_some_and(|rel| possible(end_va.wrapping_add(rel)));
    if last_is_short_operand || near_operand {
        return None;
    }
    Some(words)
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
    let next = functions
        .range(addr.saturating_add(1)..)
        .next()
        .map(|(&a, _)| a)?;
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
    if let Some(n) = ret_at_end(code, addr, functions) {
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
    /// The integer constants whose every definition in the file gives one value.
    pub constants: HashMap<String, u32>,
    /// Constants the file defines with two values, or with a value or type not read: a name
    /// here is not looked up anywhere else (another file's value would be the wrong one).
    pub opaque: HashSet<String>,
    /// The constants declared `pub` (another file can import them).
    pub public: HashSet<String>,
    /// `.vcall(`s left out as test code (a test file, or a `#[cfg(test)]` item's span).
    pub left_out: usize,
    pub sites: Vec<Site>,
    /// Calls whose slot or list is not written in place: line, reason and the call on one line.
    pub unread: Vec<(usize, &'static str, String)>,
    /// Calls written in two steps that hand the table they read the target from to the target
    /// as its first word (the object, not its table, is `this`).
    pub passes_table: Vec<Site>,
    /// Direct calls with a constant callee and at least one word.
    pub direct: Vec<Direct>,
    /// `.vcall`s whose slot the unit's constants do not give.
    pub pending: Vec<Pending>,
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

/// Whether a file is test code by its path: under `tests/`, `benches/` or `examples/`, or named
/// `tests.rs`, `test.rs`, `*_tests.rs` or `*_test.rs`.
pub fn test_file(path: &str) -> bool {
    let path = path.replace('\\', "/");
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or("")
        .strip_suffix(".rs")
        .unwrap_or("");
    ["/tests/", "/benches/", "/examples/"]
        .iter()
        .any(|d| format!("/{path}").contains(d))
        || matches!(stem, "tests" | "test")
        || stem.ends_with("_tests")
        || stem.ends_with("_test")
}

/// The index after the literal or comment that starts at `bytes[i]` (a string, raw or byte
/// string, a character, a `//` or `/* */` comment), or `None` when none starts there (a lifetime's
/// `'` is no literal).
fn skip_literal(bytes: &[u8], i: usize) -> Option<usize> {
    let len = bytes.len();
    let ident_before = i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
    let string_from = |mut j: usize| {
        while j < len && bytes[j] != b'"' {
            if bytes[j] == b'\\' {
                j += 1;
            }
            j += 1;
        }
        (j + 1).min(len)
    };
    match bytes[i] {
        b'"' => Some(string_from(i + 1)),
        b'b' if !ident_before && bytes.get(i + 1) == Some(&b'"') => Some(string_from(i + 2)),
        b'b' if !ident_before && bytes.get(i + 1) == Some(&b'\'') => skip_literal(bytes, i + 1),
        b'r' | b'b' if !ident_before => {
            let mut j = i + 1;
            if bytes[i] == b'b' {
                if bytes.get(j) != Some(&b'r') {
                    return None;
                }
                j += 1;
            }
            let hashes = bytes[j..].iter().take_while(|&&b| b == b'#').count();
            j += hashes;
            if bytes.get(j) != Some(&b'"') {
                return None;
            }
            j += 1;
            while j < len {
                if bytes[j] == b'"'
                    && len >= j + 1 + hashes
                    && bytes[j + 1..j + 1 + hashes].iter().all(|&b| b == b'#')
                {
                    return Some(j + 1 + hashes);
                }
                j += 1;
            }
            Some(len)
        }
        b'\'' => {
            if bytes.get(i + 1) == Some(&b'\\') {
                // `'\n'`, `'\''`, `'\u{..}'`: to the next quote after the escaped character.
                let close = bytes[i + 3.min(len - i)..]
                    .iter()
                    .position(|&b| b == b'\'')?;
                return Some(i + 3 + close + 1);
            }
            // One character (up to four bytes) and the closing quote; else a lifetime.
            (2..=5)
                .find(|&k| bytes.get(i + k) == Some(&b'\''))
                .filter(|&k| k == 2 || bytes[i + 1] >= 0x80)
                .map(|k| i + k + 1)
        }
        b'/' if bytes.get(i + 1) == Some(&b'/') => Some(
            bytes[i..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(len, |p| i + p),
        ),
        b'/' if bytes.get(i + 1) == Some(&b'*') => {
            let (mut depth, mut j) = (1, i + 2);
            while j < len && depth > 0 {
                if bytes[j] == b'/' && bytes.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 1;
                } else if bytes[j] == b'*' && bytes.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 1;
                }
                j += 1;
            }
            Some(j)
        }
        _ => None,
    }
}

/// Whether a `cfg(..)` predicate holds only in tests: `test`, or an `all(..)` with one such part.
fn test_only(predicate: &str) -> bool {
    let predicate = predicate.trim();
    if predicate == "test" {
        return true;
    }
    predicate
        .strip_prefix("all(")
        .and_then(|p| p.strip_suffix(')'))
        .is_some_and(|parts| pieces(parts).iter().any(|p| test_only(p)))
}

/// The test code in `text`: the spans of its items (modules, functions, statements ..) marked
/// `#[cfg(test)]` (or `#[cfg(all(test, ..))]`; `#![cfg(test)]` marks the whole file), and the
/// modules among them mounted from other files (`mod NAME;`, with its `#[path = ".."]` if any).
/// Returns the spans (sorted, not overlapping), the mounts and the number of marked items whose
/// end was not found (left in: read as game code).
pub fn test_spans(text: &str) -> (Vec<(usize, usize)>, Vec<Mount>, usize) {
    let bytes = text.as_bytes();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut mounts = Vec::new();
    let mut open = 0;
    let mut i = 0;
    while i < bytes.len() {
        if let Some(next) = skip_literal(bytes, i) {
            i = next.max(i + 1);
            continue;
        }
        if bytes[i] != b'#' {
            i += 1;
            continue;
        }
        let inner_attr = bytes.get(i + 1) == Some(&b'!');
        let bracket = i + 1 + usize::from(inner_attr);
        if bytes.get(bracket) != Some(&b'[') {
            i += 1;
            continue;
        }
        let Some(attr) = inside(text, bracket + 1) else {
            i += 1;
            continue;
        };
        let attr_end = bracket + 1 + attr.len() + 1;
        let is_test = attr
            .trim()
            .strip_prefix("cfg")
            .map(str::trim_start)
            .and_then(|a| a.strip_prefix('('))
            .and_then(|a| a.strip_suffix(')'))
            .is_some_and(test_only);
        if !is_test {
            i = attr_end;
            continue;
        }
        if inner_attr {
            // `#![cfg(test)]`: the whole file, or the inline module it stands in.
            match enclosing_block(text, i) {
                None => return (vec![(0, text.len())], Vec::new(), 0),
                Some((start, end)) => {
                    spans.retain(|&(s, _)| s < start);
                    spans.push((start, end));
                    i = end;
                    continue;
                }
            }
        }
        // The attributes and comments before the item; a `#[path = ".."]` among them (or just
        // before the `cfg`) names a mounted module's file.
        let path_of = |attr: &str| {
            let rest = attr
                .trim()
                .strip_prefix("path")?
                .trim_start()
                .strip_prefix('=')?;
            Some(rest.trim().trim_matches('"').to_string())
        };
        let mut path = text[..i]
            .trim_end()
            .strip_suffix(']')
            .and_then(|before| before.rfind("#[").map(|at| &before[at + 2..]))
            .and_then(path_of);
        let mut item = attr_end;
        loop {
            while item < bytes.len() && bytes[item].is_ascii_whitespace() {
                item += 1;
            }
            if item < bytes.len() && bytes[item] == b'/' {
                if let Some(next) = skip_literal(bytes, item) {
                    item = next;
                    continue;
                }
            }
            if bytes.get(item) == Some(&b'#') && bytes.get(item + 1) == Some(&b'[') {
                if let Some(other) = inside(text, item + 2) {
                    path = path.or_else(|| path_of(other));
                    item += 2 + other.len() + 1;
                    continue;
                }
            }
            break;
        }
        // The item's end: a `;` or `,` at its own level, the block of its first `{`, or the
        // bracket that closes what holds it.
        let mut end = None;
        let mut depth = 0usize;
        let mut j = item;
        while j < bytes.len() {
            if let Some(next) = skip_literal(bytes, j) {
                j = next.max(j + 1);
                continue;
            }
            match bytes[j] {
                b'{' if depth == 0 => {
                    end = inside(text, j + 1).map(|body| j + 1 + body.len() + 1);
                    break;
                }
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' if depth == 0 => {
                    end = Some(j);
                    break;
                }
                b')' | b']' | b'}' => depth -= 1,
                b';' | b',' if depth == 0 => {
                    end = Some(j + 1);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let Some(end) = end else {
            open += 1;
            i = attr_end;
            continue;
        };
        // A mounted module: `[pub[(..)]] mod NAME;`.
        let head = text[item..end].trim();
        let head = match head.strip_prefix("pub") {
            Some(after) if after.starts_with('(') => after
                .find(')')
                .map_or(head, |c| after[c + 1..].trim_start()),
            Some(after) if after.starts_with(' ') => after.trim_start(),
            _ => head,
        };
        if let Some(name) = head
            .strip_prefix("mod ")
            .and_then(|n| n.strip_suffix(';'))
            .map(str::trim)
            .filter(|n| is_name(n))
        {
            mounts.push(Mount {
                name: name.to_string(),
                path,
            });
        }
        match spans.last_mut() {
            Some(last) if i <= last.1 => last.1 = last.1.max(end),
            _ => spans.push((i, end)),
        }
        i = end;
    }
    (spans, mounts, open)
}

/// A module a file mounts from another file for tests only.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    pub name: String,
    /// Its `#[path = ".."]`, relative to the mounting file's folder.
    pub path: Option<String>,
}

/// The files (relative paths, `/`-separated) a mount in the file `rel` names: its `#[path]`, or
/// `NAME.rs` and `NAME/mod.rs` in the folder of the file's modules; and that folder for the
/// mounted module's own modules (`NAME/`).
pub fn mounted_paths(rel: &str, mount: &Mount) -> (Vec<String>, String) {
    let (dir, file) = rel.rsplit_once('/').unwrap_or(("", rel));
    let join = |a: &str, b: &str| {
        if a.is_empty() {
            b.to_string()
        } else {
            format!("{a}/{b}")
        }
    };
    if let Some(path) = &mount.path {
        let file = join(dir, path);
        let sub = file.strip_suffix(".rs").unwrap_or(&file).to_string();
        return (vec![file], sub);
    }
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    let modules = if matches!(stem, "mod" | "lib" | "main") {
        dir.to_string()
    } else {
        join(dir, stem)
    };
    let sub = join(&modules, &mount.name);
    (vec![format!("{sub}.rs"), format!("{sub}/mod.rs")], sub)
}

/// The spans of `text`'s literals and comments, in order: a call found inside one is no call.
fn literal_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match skip_literal(bytes, i) {
            Some(next) => {
                spans.push((i, next));
                i = next.max(i + 1);
            }
            None => i += 1,
        }
    }
    spans
}

/// Whether `at` lies inside one of `spans` (sorted, not overlapping).
fn in_spans(spans: &[(usize, usize)], at: usize) -> bool {
    let k = spans.partition_point(|&(start, _)| start <= at);
    k > 0 && at < spans[k - 1].1
}

/// `text` with every span of `spans` blanked (spaces, its line breaks kept): positions and line
/// numbers stay as they are.
fn blank(text: &str, spans: &[(usize, usize)]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for &(start, end) in spans {
        for b in &mut bytes[start..end.min(text.len())] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
    }
    // The spans start at `#` and end after an ASCII byte or at one, so no character is cut.
    String::from_utf8(bytes).unwrap_or_default()
}

/// The inside of the innermost `{ .. }` around `text[at]` (from after its `{` to its `}`, or to
/// the end when it is not closed), or `None` at the file's own level.
fn enclosing_block(text: &str, at: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < at.min(bytes.len()) {
        if let Some(next) = skip_literal(bytes, i) {
            i = next.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'{' => open.push(i + 1),
            b'}' => {
                open.pop();
            }
            _ => {}
        }
        i += 1;
    }
    let start = *open.last()?;
    let end = inside(text, start).map_or(text.len(), |body| start + body.len());
    Some((start, end))
}

/// The game code of a file: its text with the test code blanked (all of it for a test file),
/// and the number of `.vcall(`s left out.
pub fn game_code(path: &str, text: &str, mounted_for_tests: bool) -> (String, usize) {
    let calls = |t: &str| t.matches(".vcall(").count();
    if mounted_for_tests || test_file(path) {
        return (blank(text, &[(0, text.len())]), calls(text));
    }
    let (spans, _, _) = test_spans(text);
    let left_out = spans.iter().map(|&(s, e)| calls(&text[s..e])).sum();
    (blank(text, &spans), left_out)
}

/// A `u32` literal: hex or decimal, with underscores and an optional `u32` or `usize` suffix.
fn number(text: &str) -> Option<u32> {
    let text = text.trim();
    let text = text.strip_suffix("u32").unwrap_or(text);
    let text = text.strip_suffix("usize").unwrap_or(text).replace('_', "");
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// The value of a constant expression: numbers and names of `consts` joined by `+` and `*`
/// (with brackets and a trailing `as u32` / `as usize`), as a slot is often written
/// (`4 * 12`, `BASE_SLOT + 8`).
fn eval(expr: &str, consts: &dyn Lookup) -> Option<u32> {
    let mut expr = expr.trim();
    for cast in [" as u32", " as usize"] {
        expr = expr.strip_suffix(cast).unwrap_or(expr).trim();
    }
    if let Some(inner) = expr.strip_prefix('(').and_then(|e| e.strip_suffix(')')) {
        if inside(expr, 1).map(str::len) == Some(inner.len()) {
            return eval(inner, consts);
        }
    }
    let split = |text: &str, op: u8| -> Vec<String> {
        let mut parts = Vec::new();
        let (mut depth, mut start) = (0i32, 0);
        for (i, b) in text.bytes().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ if b == op && depth == 0 => {
                    parts.push(text[start..i].to_string());
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(text[start..].to_string());
        parts
    };
    let sum = split(expr, b'+');
    if sum.len() > 1 {
        return sum
            .iter()
            .try_fold(0u32, |a, t| a.checked_add(eval(t, consts)?));
    }
    let product = split(expr, b'*');
    if product.len() > 1 {
        return product
            .iter()
            .try_fold(1u32, |a, t| a.checked_mul(eval(t, consts)?));
    }
    // A path (`slot::DRAW`, `crate::vtable::IS_ACTOR`) names the constant by its last part.
    let name = expr.rsplit("::").next().unwrap_or(expr);
    number(expr).or_else(|| consts.value(name))
}

/// The last `for PATTERN in [..]` of `function` whose pattern binds `name` (alone or in a
/// tuple): where it is, the value of `name` in each item of the list, as written, and whether
/// the loop's body encloses the end of `function` (the call).
fn for_values(function: &str, name: &str) -> Option<(usize, Vec<String>, bool)> {
    let mut from = function.len();
    while let Some(at) = function[..from].rfind("for ") {
        from = at;
        let rest = &function[at + "for ".len()..];
        let Some(in_at) = rest.find(" in ") else {
            continue;
        };
        let pattern = rest[..in_at].trim();
        let list = rest[in_at + " in ".len()..].trim_start();
        let position =
            if let Some(tuple) = pattern.strip_prefix('(').and_then(|p| p.strip_suffix(')')) {
                pieces(tuple).iter().position(|p| p == name)
            } else if pattern == name {
                None
            } else {
                continue;
            };
        if pattern.starts_with('(') && position.is_none() {
            continue;
        }
        let items = list.strip_prefix('[').and_then(|l| inside(l, 0))?;
        let after = function.len() - list.len() + 1 + items.len() + 1;
        let encloses = function[after..]
            .trim_start()
            .strip_prefix('{')
            .is_some_and(|body| inside(body, 0).is_none());
        let values = pieces(items)
            .into_iter()
            .map(|item| match position {
                Some(k) => item
                    .strip_prefix('(')
                    .and_then(|t| t.strip_suffix(')'))
                    .and_then(|t| pieces(t).into_iter().nth(k))
                    .unwrap_or_default(),
                None => item,
            })
            .collect();
        return Some((at, values, encloses));
    }
    None
}

/// The slots an expression can give: its value; each value of a `for`'s list (`[A|B|C]`, from
/// [`for_values`]); both arms' of `if .. { A } else { B }`; every arm's of a `match` whose arms
/// are all constants.
fn slot_values(expr: &str, consts: &dyn Lookup) -> Vec<u32> {
    if let Some(value) = eval(expr, consts) {
        return vec![value];
    }
    if let Some(list) = expr.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
        let values: Option<Vec<u32>> = list.split('|').map(|v| eval(v, consts)).collect();
        return values.unwrap_or_default();
    }
    if let Some(rest) = expr.trim().strip_prefix("match") {
        let Some(open) = rest.find('{') else {
            return Vec::new();
        };
        let Some(arms) = inside(rest, open + 1) else {
            return Vec::new();
        };
        let values: Option<Vec<u32>> = pieces(arms)
            .iter()
            .map(|arm| arm.split_once("=>").and_then(|(_, v)| eval(v, consts)))
            .collect();
        return values.unwrap_or_default();
    }
    // `if` with or without the space (a `let`'s expression is read with its spaces removed).
    let expr = expr.trim();
    let Some(rest) = expr.strip_prefix("if").filter(|_| expr.contains("else")) else {
        return Vec::new();
    };
    let Some(open) = rest.find('{') else {
        return Vec::new();
    };
    let Some(first) = inside(rest, open + 1) else {
        return Vec::new();
    };
    let after = rest[open + 1 + first.len() + 1..].trim_start();
    let Some(second) = after
        .strip_prefix("else")
        .map(str::trim_start)
        .and_then(|a| a.strip_prefix('{'))
        .and_then(|a| inside(a, 0))
    else {
        return Vec::new();
    };
    // A block's value is its last expression (statements before it are left out).
    let tail = |block: &str| {
        let mut rest = block;
        while let Some((_, after)) = top_level_split(rest, b';') {
            rest = after;
        }
        rest.trim().to_string()
    };
    match (eval(&tail(first), consts), eval(&tail(second), consts)) {
        (Some(a), Some(b)) => vec![a, b],
        _ => Vec::new(),
    }
}

/// What a scope says of a name.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Found {
    Value(u32),
    /// Defined here, but with two values or one not read: no other scope is asked.
    Opaque,
    Absent,
}

/// Names to values for [`eval`]: one map, or a function's constants before its file's and the
/// whole tree's.
trait Lookup {
    fn find(&self, name: &str) -> Found;

    fn value(&self, name: &str) -> Option<u32> {
        match self.find(name) {
            Found::Value(v) => Some(v),
            _ => None,
        }
    }
}

impl Lookup for HashMap<String, u32> {
    fn find(&self, name: &str) -> Found {
        self.get(name).map_or(Found::Absent, |&v| Found::Value(v))
    }
}

/// One scope's constants: the names it gives one value and the ones it defines otherwise.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Names {
    pub values: HashMap<String, u32>,
    pub opaque: HashSet<String>,
}

impl Lookup for Names {
    fn find(&self, name: &str) -> Found {
        if self.opaque.contains(name) {
            return Found::Opaque;
        }
        self.values.find(name)
    }
}

/// A unit's own constants as a scope.
struct Own<'a>(&'a Unit);

impl Lookup for Own<'_> {
    fn find(&self, name: &str) -> Found {
        if self.0.opaque.contains(name) {
            return Found::Opaque;
        }
        self.0.constants.find(name)
    }
}

/// Scopes asked in order, the first that knows the name answering (an opaque name stops there).
struct Chain<'a>(Vec<&'a dyn Lookup>);

impl Lookup for Chain<'_> {
    fn find(&self, name: &str) -> Found {
        for scope in &self.0 {
            match scope.find(name) {
                Found::Absent => continue,
                found => return found,
            }
        }
        Found::Absent
    }
}

/// Whether a name `expr` uses is opaque in `consts` (the reason a slot is not read).
fn uses_opaque(expr: &str, consts: &dyn Lookup) -> bool {
    expr.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| is_name(w))
        .any(|w| consts.find(w) == Found::Opaque)
}

/// Every `const NAME: TYPE = expression;` of `text`: the names whose definitions all have an
/// integer type and give one value [`eval`] reads (from literals, the constants before it and
/// `outer`), and the names defined otherwise (two values, a value not read, another type).
fn constants(text: &str, outer: &dyn Lookup) -> Names {
    // Per name: its value so far, `None` once a definition disagrees or is not read.
    let mut seen: HashMap<String, Option<u32>> = HashMap::new();
    struct Seen<'a>(&'a HashMap<String, Option<u32>>, &'a dyn Lookup);
    impl Lookup for Seen<'_> {
        fn find(&self, name: &str) -> Found {
            match self.0.get(name) {
                Some(Some(v)) => Found::Value(*v),
                Some(None) => Found::Opaque,
                None => self.1.find(name),
            }
        }
    }
    let mut from = 0;
    while let Some(found) = text[from..].find("const ") {
        let at = from + found + "const ".len();
        from = at;
        // A word of its own (not `MY_const `).
        let before = text[..at - "const ".len()].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let rest = &text[at..];
        let Some(colon) = rest.find(':') else { break };
        let name = rest[..colon].trim();
        if !is_name(name) || name == "_" {
            continue;
        }
        // The definition: its type up to the `=`, its value up to the `;` (a `const` generic
        // parameter has neither).
        let Some(stop) = rest[colon..]
            .find([';', '=', '>', ')', '{'])
            .map(|s| colon + s)
        else {
            continue;
        };
        if rest.as_bytes()[stop] != b'=' {
            continue;
        }
        let ty = rest[colon + 1..stop].trim();
        let Some(end) = rest[stop..].find(';').map(|e| stop + e) else {
            continue;
        };
        let value = ["u32", "usize", "i32", "u16", "u8"]
            .contains(&ty)
            .then(|| eval(&rest[stop + 1..end], &Seen(&seen, outer)))
            .flatten();
        seen.entry(name.to_string())
            .and_modify(|v| {
                if *v != value {
                    *v = None;
                }
            })
            .or_insert(value);
    }
    let mut names = Names::default();
    for (name, value) in seen {
        match value {
            Some(v) => {
                names.values.insert(name, v);
            }
            None => {
                names.opaque.insert(name);
            }
        }
    }
    names
}

/// The names of `constants` declared `pub` (`pub(crate)` ..) in `text`: the ones other files of
/// the crate can import.
fn public_constants<'a>(text: &str, names: impl Iterator<Item = &'a String>) -> HashSet<String> {
    names
        .filter(|name| {
            let pattern = format!("const {name}:");
            text.match_indices(&pattern).any(|(at, _)| {
                let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
                text[line_start..at].trim_start().starts_with("pub")
            })
        })
        .cloned()
        .collect()
}

/// The text between the bracket before `text[open..]` and its match, literals and comments
/// skipped; `open` is the index after the opening bracket.
fn inside(text: &str, open: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut i = open;
    while i < bytes.len() {
        if let Some(next) = skip_literal(bytes, i) {
            i = next.max(i + 1);
            continue;
        }
        match bytes[i] {
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
        if let Some(next) = skip_literal(bytes, i) {
            let next = next.max(i + 1).min(text.len());
            // Comments are left out, literals kept.
            if bytes[i] != b'/' {
                piece.push_str(&text[i..next]);
            }
            i = next;
            continue;
        }
        match bytes[i] {
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

/// Reads a unit: its constants and every `.vcall(object, slot, list)` of its game code (its test
/// code left out, see [`game_code`]).
pub fn read_unit(path: &str, text: &str) -> Unit {
    let (code, left_out) = game_code(path, text, false);
    let mut unit = read_code(&code);
    unit.left_out = left_out;
    unit
}

/// A call's list as written.
#[derive(Clone, Debug, PartialEq)]
enum List {
    /// `this` and the words of the items written in place.
    Words(u32),
    /// The list is a name (a parameter it is handed on from, perhaps).
    Name(String),
    /// Anything else.
    Other,
}

/// Reads a call's list: `&args![..]` (each item's words as its text gives them, see
/// [`item_words`]), `&[..]` (one word each item: a plain list holds words), `&[x; N]` (N words),
/// or a plain name. `function` is the function's text so far (for the items' types).
fn list_of(arg: &str, function: &str, consts: &dyn Lookup) -> List {
    let list = arg.trim().trim_start_matches('&').trim_start();
    let (list, typed) = match list.strip_prefix("args!") {
        Some(rest) => (rest.trim_start(), true),
        None => (list, false),
    };
    if !typed && is_name(list) {
        return List::Name(list.to_string());
    }
    let Some(items) = list.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
        return List::Other;
    };
    let items = pieces(items);
    if let [only] = items.as_slice() {
        if let Some((_, count)) = top_level_split(only, b';') {
            return match (typed, eval(count, consts)) {
                (false, Some(n)) => List::Words(1 + n),
                _ => List::Other,
            };
        }
    }
    let words: u32 = if typed {
        items.iter().map(|item| item_words(item, function)).sum()
    } else {
        items.len() as u32
    };
    List::Words(1 + words)
}

/// `text` cut at its first `sep` outside brackets and literals.
fn top_level_split(text: &str, sep: u8) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let (mut depth, mut i) = (0i32, 0);
    while i < bytes.len() {
        if let Some(next) = skip_literal(bytes, i) {
            i = next.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b if b == sep && depth == 0 => return Some((&text[..i], &text[i + 1..])),
            _ => {}
        }
        i += 1;
    }
    None
}

/// The constants a function's own text defines (scoped to it: two functions of a file may each
/// define a name with a value of its own).
fn local_constants(function: &str, outer: &dyn Lookup) -> Names {
    if function.contains("const ") {
        constants(function, outer)
    } else {
        Names::default()
    }
}

/// How a name the slot is written as was bound before the call.
#[derive(Clone, Debug, PartialEq)]
enum Binding {
    /// Not by a `let` or a `for` in the function (a parameter, perhaps).
    None,
    /// By a `let` (its expression, whitespace removed) or a `for` over a list written in place
    /// (`[A|B|..]`), the binding enclosing the call and the name not changed after it.
    Value(String),
    /// By one that ends before the call, or the name is changed or bound again after it.
    Doubtful,
}

/// The binding of `name` nearest the call in `function` (its text up to the call).
fn bound_slot(function: &str, name: &str) -> Binding {
    let let_at = last_let(function, name);
    let let_pos = let_at.as_ref().map(|l| l.0);
    let for_at = for_values(function, name);
    match (let_at, for_at) {
        (_, Some((at, values, encloses))) if let_pos.map_or(true, |l| at > l) => {
            let after = &function[at + "for ".len()..];
            if !encloses || assigned(after.split_once(" in ").map_or(after, |a| a.1), name) {
                return Binding::Doubtful;
            }
            Binding::Value(format!("[{}]", values.join("|")))
        }
        (Some((at, value, end)), _) => {
            if !never_closes(&function[at..]) || assigned(&function[end..], name) {
                return Binding::Doubtful;
            }
            match value {
                Some(value) => Binding::Value(compact(&value)),
                None => Binding::Doubtful,
            }
        }
        _ => Binding::None,
    }
}

/// Whether `text` (from a binding to the call) never closes the block the binding is in.
fn never_closes(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (mut depth, mut i) = (0i32, 0);
    while i < bytes.len() {
        if let Some(next) = skip_literal(bytes, i) {
            i = next.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    true
}

/// Whether `text` may give `name` another value or binding: an assignment (`name =`, `name +=`
/// ..), a `let` of it, a `&mut` of it, or a closure parameter of that name (`|name|`).
fn assigned(text: &str, name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(found) = text[from..].find(name) {
        let at = from + found;
        from = at + name.len();
        let before = text[..at].trim_end();
        let prev = text[..at].chars().next_back();
        if prev.is_some_and(|c| ident(c) || c == '.') || text[from..].starts_with(ident) {
            continue;
        }
        let after = text[from..].trim_start();
        let assigns =
            (after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>"))
                || ["+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "<<=", ">>="]
                    .iter()
                    .any(|op| after.starts_with(op));
        let rebinds = before.ends_with("let")
            || before.ends_with("let mut")
            || before.ends_with("&mut")
            || before.ends_with('|')
            || (after.starts_with('|') && !after.starts_with("||"));
        if assigns || rebinds {
            return true;
        }
    }
    false
}

/// A call inside a function or closure that takes its slot or its list from a parameter: its
/// callers give them.
#[derive(Clone, Debug, PartialEq)]
pub struct Wrapper {
    pub name: String,
    /// The parameter that gives the slot (its index counting `e`, `self` left out) and the factor
    /// the slot is that parameter times.
    pub slot: Option<(usize, u32)>,
    /// The slots the wrapper's own text gives, when no parameter does.
    pub fixed: Vec<u32>,
    /// The parameter that gives the whole list.
    pub list: Option<usize>,
    /// The parameter that is the call's object, if it is one.
    pub object: Option<usize>,
    /// Declared `pub` (callers in the whole crate), else only its own file calls it (files often
    /// have private helpers of the same name).
    pub public: bool,
    /// A method (`self` first): called as `x.name(..)` or `Type::name(x, ..)`.
    pub method: bool,
}

/// A `.vcall` whose slot the unit's own constants do not give, or whose list is a parameter.
#[derive(Clone, Debug)]
pub struct Pending {
    pub site: Site,
    /// The slot as written (or the expression its name is bound to).
    pub slot: String,
    /// The function's own constants.
    pub local: Names,
    /// The list is handed on from a parameter (the wrapper's `list`).
    pub forwards_list: bool,
    pub wrapper: Option<Wrapper>,
}

/// The function whose text starts at `text[scope..]`: its name, its parameter names in order
/// (`self` left out), whether it is `pub`, whether it is a method, and where its body starts.
fn signature(text: &str, scope: usize) -> Option<(String, Vec<String>, bool, bool, usize)> {
    let head = &text[scope..];
    let fn_at = head.find("fn ")?;
    let line_start = text[..scope + fn_at].rfind('\n').map_or(0, |i| i + 1);
    let public = text[line_start..scope + fn_at]
        .trim_start()
        .starts_with("pub");
    let at = fn_at + "fn ".len();
    let rest = &head[at..];
    let name_end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    let name = rest[..name_end].to_string();
    let open = rest.find('(')? + 1;
    let inner = inside(rest, open)?;
    // Generic arguments (`Field<T, u32>`) hold commas: left out before the parameters are cut.
    let mut params = String::new();
    let mut depth = 0usize;
    for c in inner.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => params.push(c),
            _ => {}
        }
    }
    let mut names: Vec<String> = pieces(&params)
        .iter()
        .map(|p| {
            let p = p.trim_start_matches("mut ");
            p.split(':').next().unwrap_or("").trim().to_string()
        })
        .collect();
    let method = names
        .first()
        .is_some_and(|p| matches!(p.as_str(), "self" | "&self" | "&mut self" | "mut self"));
    if method {
        names.remove(0);
    }
    let body = scope + at + open + inner.len() + 1;
    Some((name, names, public, method, body))
}

/// The closure a call at `text[open]` is in, when one of `names` is among its parameters: `let
/// CLOSURE = [move] |e: &mut Engine, slot: u32, ..| ..` above the call in the function at
/// `scope`, its body enclosing the call; its name, parameter names and where its body starts.
fn closure_of(
    text: &str,
    scope: usize,
    open: usize,
    names: &[&str],
) -> Option<(String, Vec<String>, usize)> {
    let function = &text[scope..open];
    let mut from = function.len();
    while let Some(at) = function[..from].rfind('|') {
        from = at;
        // The opening bar: after `=` or `= move`.
        let head = function[..at].trim_end();
        let head = head.strip_suffix("move").map_or(head, str::trim_end);
        let Some(head) = head.strip_suffix('=') else {
            continue;
        };
        if head.ends_with(['=', '!', '<', '>', '|', '&', '+', '-', '*', '/', '%', '^']) {
            continue;
        }
        let params_start = at + 1;
        let Some(close) = function[params_start..].find('|') else {
            continue;
        };
        let params: Vec<String> = pieces(&function[params_start..params_start + close])
            .iter()
            .map(|p| p.split(':').next().unwrap_or("").trim().to_string())
            .collect();
        if !params.iter().any(|p| names.contains(&p.as_str())) {
            continue;
        }
        // The body encloses the call: it does not close, and a body without braces does not end
        // at a `;`, before it.
        let body = params_start + close + 1;
        let rest = &function[body..];
        let encloses = match rest.trim_start().strip_prefix('{') {
            Some(braced) => inside(braced, 0).is_none(),
            None => never_closes(rest) && top_level_split(rest, b';').is_none(),
        };
        if !encloses {
            continue;
        }
        let head = head.trim_end();
        let head = head.rsplit("let ").next()?.trim();
        let head = head.strip_prefix("mut ").unwrap_or(head);
        return is_name(head).then(|| (head.to_string(), params, scope + body));
    }
    None
}

/// [`Wrapper`] for a call whose slot is written `slot` (`4 * slot`, `slot * 4`) or whose list is
/// the name `list`, with that name a parameter of the closure the call is in (see
/// [`closure_of`]), or else of the function at `scope`, and not bound again before the call.
fn wrapper_of(
    text: &str,
    scope: usize,
    open: usize,
    slot: &str,
    list: Option<&str>,
    object: &str,
) -> Option<Wrapper> {
    let slot = slot.trim();
    let (name, factor) = if let Some(n) = slot.strip_prefix("4 *") {
        (n.trim(), 4)
    } else if let Some(n) = slot.strip_suffix("* 4") {
        (n.trim(), 4)
    } else {
        (slot, 1)
    };
    let name = name.strip_suffix(" as u32").unwrap_or(name).trim();
    let slot_name = is_name(name).then_some(name);
    let mut wanted: Vec<&str> = slot_name.into_iter().collect();
    wanted.extend(list);
    if wanted.is_empty() {
        return None;
    }
    // A closure is called only in its own function, so only its own file.
    let (function, params, public, method, body) = match closure_of(text, scope, open, &wanted) {
        Some((closure, params, body)) => (closure, params, false, false, body),
        None => signature(text, scope)?,
    };
    let region = &text[body.min(open)..open];
    let param = |name: &str| {
        params
            .iter()
            .position(|p| p == name)
            .filter(|_| !assigned(region, name))
    };
    let slot = slot_name.and_then(param).map(|index| (index, factor));
    let list = list.and_then(param);
    if slot.is_none() && list.is_none() {
        return None;
    }
    let object = object.strip_suffix(".addr()").unwrap_or(object);
    Some(Wrapper {
        name: function,
        slot,
        fixed: Vec::new(),
        list,
        object: param(object),
        public,
        method,
    })
}

/// How a function is called: `name(..)`, `x.name(..)` or `Path::name(..)`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CallKind {
    Plain,
    Method,
    Path,
}

/// The calls in `text` of the functions named in `names` (not their definitions, not in
/// comments): each call's name, position, argument texts and kind.
fn named_calls<'a>(
    text: &str,
    names: &HashSet<&'a str>,
) -> Vec<(&'a str, usize, Vec<String>, CallKind)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut open = 0;
    while open < bytes.len() {
        if let Some(next) = skip_literal(bytes, open) {
            open = next.max(open + 1);
            continue;
        }
        if bytes[open] != b'(' {
            open += 1;
            continue;
        }
        let mut at = open;
        while at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_') {
            at -= 1;
        }
        let Some(&name) = names.get(&text[at..open]) else {
            open += 1;
            continue;
        };
        if text[..at].trim_end().ends_with("fn") {
            open += 1;
            continue;
        }
        let kind = if text[..at].ends_with('.') {
            CallKind::Method
        } else if text[..at].ends_with("::") {
            CallKind::Path
        } else {
            CallKind::Plain
        };
        if let Some(args) = inside(text, open + 1) {
            out.push((name, at, pieces(args), kind));
        }
        open += 1;
    }
    out
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

/// The last `let name = ..;` (or `let name: type = ..;`, `let mut ..`) of `function`: where it
/// is, the expression it gives (whitespace removed; `None` for `let name;`), and where it ends.
fn last_let(function: &str, name: &str) -> Option<(usize, Option<String>, usize)> {
    let mut found = None;
    let mut from = 0;
    while let Some(at) = function[from..].find("let ") {
        let start = from + at;
        let at = start + "let ".len();
        from = at;
        if function[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let rest = function[at..].trim_start();
        let rest = rest.strip_prefix("mut ").unwrap_or(rest);
        let Some(rest) = rest.strip_prefix(name) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest_at = function.len() - rest.len();
        if let Some(after) = rest.strip_prefix(';') {
            found = Some((start, None, function.len() - after.len()));
            continue;
        }
        let rest = match rest.strip_prefix(':') {
            Some(typed) => match typed.find(['=', ';']) {
                Some(eq) if typed.as_bytes()[eq] == b'=' => &typed[eq..],
                Some(_) => {
                    let end = rest_at + rest.find(';').map_or(rest.len(), |e| e + 1);
                    found = Some((start, None, end));
                    continue;
                }
                None => continue,
            },
            None => rest,
        };
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        if let Some((expr, _)) = top_level_split(value, b';') {
            let value_at = function.len() - value.len();
            found = Some((
                start,
                Some(expr.trim().to_string()),
                value_at + expr.len() + 1,
            ));
        }
    }
    found
}

/// `text` with its whitespace removed (a `let`'s expression, read as a slot).
fn compact(text: &str) -> String {
    text.split_whitespace().collect()
}

/// Whether `text` may change the list in `name`: [`assigned`], or a method of it that is not
/// known to only read (`name.extend_from_slice(..)`).
fn list_changed(text: &str, name: &str) -> bool {
    let reads = [
        "len", "iter", "as_slice", "to_vec", "get", "first", "last", "is_empty", "clone",
    ];
    let pattern = format!("{name}.");
    assigned(text, name)
        || text.match_indices(&pattern).any(|(at, _)| {
            let ident_before = text[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
            let method: String = text[at + pattern.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            !ident_before && !method.is_empty() && !reads.contains(&method.as_str())
        })
}

/// The words of a list given by a call of a closure the function binds to a list in place
/// (`let words = |child: u32| args![child, a, b];` .. `&words(left)`), the closure not bound again.
fn closure_list(function: &str, list: &str, consts: &dyn Lookup) -> Option<u32> {
    let list = list.trim().trim_start_matches('&').trim_start();
    let open = list.find('(')?;
    let name = &list[..open];
    if !is_name(name) || inside(list, open + 1).map(str::len) != Some(list.len() - open - 2) {
        return None;
    }
    let value = plain_let(function, name)?;
    let rest = value
        .trim()
        .strip_prefix("move")
        .unwrap_or(&value)
        .trim_start();
    let params = rest.strip_prefix('|')?;
    let close = params.find('|')?;
    let body = params[close + 1..].trim();
    let body = match body.strip_prefix('{') {
        Some(braced) => braced.strip_suffix('}')?.trim(),
        None => body,
    };
    match list_of(body, function, consts) {
        List::Words(n) => Some(n),
        _ => None,
    }
}

/// The length of the array `name` is declared as in `function` (a parameter or a `let` typed
/// `[u32; N]`, `&[u32; N]` or `&mut [u32; N]`), when no later `let` binds the name again: its
/// type fixes the list's words, whatever value it holds.
fn declared_array(function: &str, name: &str, consts: &dyn Lookup) -> Option<u32> {
    let pattern = format!("{name}:");
    let (at, len) = function
        .match_indices(&pattern)
        .filter(|(at, _)| {
            !function[..*at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
        })
        .filter_map(|(at, _)| {
            let ty = function[at + pattern.len()..].trim_start();
            let ty = ty.strip_prefix('&').map_or(ty, str::trim_start);
            let ty = ty.strip_prefix("mut ").unwrap_or(ty).trim_start();
            let inner = ty.strip_prefix('[').and_then(|t| inside(t, 0))?;
            let (element, count) = top_level_split(inner, b';')?;
            if element.trim() != "u32" {
                return None;
            }
            Some((at, eval(count, consts)?))
        })
        .last()?;
    let rebound = last_let(function, name).is_some_and(|(l, _, _)| l > at);
    (!rebound).then_some(len)
}

/// The word counts of a list held in `name`: a `let` of a list in place, or a `for` over a list
/// of lists in place (each item's count), the binding enclosing the call and the list not
/// changed after it.
fn bound_list(function: &str, name: &str, consts: &dyn Lookup) -> Option<Vec<u32>> {
    let let_at = last_let(function, name);
    let let_pos = let_at.as_ref().map(|l| l.0);
    let mut counts: Vec<u32> = match (let_at, for_values(function, name)) {
        (_, Some((at, items, encloses))) if let_pos.map_or(true, |l| at > l) => {
            let after = &function[at + "for ".len()..];
            let body = after.split_once(" in ").map_or(after, |a| a.1);
            if !encloses || list_changed(body, name) {
                return None;
            }
            items
                .iter()
                .map(|item| match list_of(item, function, consts) {
                    List::Words(n) => Some(n),
                    _ => None,
                })
                .collect::<Option<Vec<u32>>>()?
        }
        (Some((at, Some(value), end)), _) => {
            if !never_closes(&function[at..]) || list_changed(&function[end..], name) {
                return None;
            }
            match list_of(&value, function, consts) {
                List::Words(n) => vec![n],
                _ => vec![closure_list(&function[..at], &value, consts)?],
            }
        }
        _ => return None,
    };
    counts.sort_unstable();
    counts.dedup();
    (!counts.is_empty()).then_some(counts)
}

/// The expression of the last `let name` of `function` when it encloses the end of the text and
/// `name` is not changed after it.
fn plain_let(function: &str, name: &str) -> Option<String> {
    let (at, value, end) = last_let(function, name)?;
    (never_closes(&function[at..]) && !assigned(&function[end..], name))
        .then_some(value)
        .flatten()
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

/// Reads the game code of a unit (its test code already blanked).
fn read_code(code: &str) -> Unit {
    let own = constants(code, &HashMap::new());
    let public = public_constants(code, own.values.keys().chain(&own.opaque));
    let mut unit = Unit {
        public,
        ..Unit::default()
    };
    let literals = literal_spans(code);
    let mut from = 0;
    while let Some(found) = code[from..].find(".vcall(") {
        let open = from + found + ".vcall(".len();
        from = open;
        // A call in a comment (a doc's example) or a string is no call.
        if in_spans(&literals, from - ".vcall(".len()) {
            continue;
        }
        let line = 1 + code[..open].matches('\n').count();
        let Some(call) = inside(code, open) else {
            continue;
        };
        let parts = pieces(call);
        let shown = format!(".vcall({})", parts.join(", "));
        if parts.len() != 3 {
            unit.unread.push((line, "not (object, slot, list)", shown));
            continue;
        }
        let (caller, scope, function) = function_so_far(code, open);
        let local = local_constants(function, &own);
        let consts = Chain(vec![&local, &own]);
        let (mut counts, list_name) = match list_of(&parts[2], function, &consts) {
            List::Words(n) => (vec![n], None),
            List::Name(name) => (Vec::new(), Some(name)),
            List::Other => match closure_list(function, &parts[2], &consts) {
                Some(n) => (vec![n], None),
                None => {
                    unit.unread
                        .push((line, "the list is not written in place", shown));
                    continue;
                }
            },
        };
        let site = Site {
            line,
            caller,
            scope,
            bound: binding(code, scope, open, &parts[0]),
            object: parts[0].clone(),
            slot: 0,
            words: 0,
            text: shown,
        };
        // A slot in a name bound in the function: by a `let` (that `let`'s expression), or by a
        // `for` over a list written in place (each of its values), whichever is nearer.
        let mut slot = parts[1].clone();
        let name = slot.trim_start_matches('*').to_string();
        if is_name(&name) {
            match bound_slot(function, &name) {
                Binding::None => {}
                Binding::Value(value) => slot = value,
                Binding::Doubtful => {
                    unit.unread.push((
                        line,
                        "the slot's name is bound in a block that ends before the call, or changed",
                        site.text,
                    ));
                    continue;
                }
            }
        }
        let mut wrapper = wrapper_of(
            code,
            scope,
            open,
            &parts[1],
            list_name.as_deref(),
            &parts[0],
        );
        // A list in a name: a parameter (the callers give it), or bound to lists in place.
        let mut forwards = false;
        if let Some(name) = &list_name {
            if let Some(len) = declared_array(function, name, &consts) {
                counts = vec![1 + len];
            } else if wrapper.as_ref().is_some_and(|w| w.list.is_some()) {
                forwards = true;
                counts = vec![0];
            } else if let Some(bound) = bound_list(function, name, &consts) {
                counts = bound;
            } else {
                unit.unread
                    .push((line, "the list is not written in place", site.text));
                continue;
            }
        }
        if let Some(w) = wrapper.as_mut().filter(|_| !forwards) {
            w.list = None;
        }
        let value = eval(&slot, &consts);
        for words in counts {
            let mut site = site.clone();
            site.words = words;
            match value {
                Some(value) if !forwards => {
                    site.slot = value;
                    unit.sites.push(site);
                }
                // Resolved later: by the whole tree's constants, or at a wrapper's callers.
                _ => unit.pending.push(Pending {
                    site,
                    slot: slot.clone(),
                    local: local.clone(),
                    forwards_list: forwards,
                    wrapper: wrapper.clone(),
                }),
            }
        }
    }
    unit.constants = own.values;
    unit.opaque = own.opaque;
    read_two_step_calls(code, &mut unit);
    unit
}

/// A call written in two steps, `let table = e.mem.u32(object); let target =
/// e.mem.u32(table + SLOT); e.call(target, &args![..])`: a virtual call whose words are the
/// whole list (`this` included). Read as a site on `object`; one that hands `table` itself over
/// as its first word is recorded in [`Unit::passes_table`].
fn read_two_step_calls(text: &str, unit: &mut Unit) {
    let literals = literal_spans(text);
    let mut from = 0;
    while let Some(found) = text[from..].find(".call(") {
        let open = from + found + ".call(".len();
        from = open;
        if in_spans(&literals, open - ".call(".len()) {
            continue;
        }
        let Some(call) = inside(text, open) else {
            continue;
        };
        let parts = pieces(call);
        if parts.len() != 2 {
            continue;
        }
        let line = 1 + text[..open].matches('\n').count();
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
                    line,
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
        let Some(target) = plain_let(function, &parts[0]).map(|t| compact(&t)) else {
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
        if !is_name(table) {
            continue;
        }
        let Some(object) = plain_let(function, table).and_then(|t| {
            let t = compact(&t);
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
        let shown = format!(".call({})", parts.join(", "));
        let local = local_constants(function, &Own(unit));
        let slot = eval(slot, &Chain(vec![&local, &Own(unit)]));
        let Some(slot) = slot else {
            unit.unread
                .push((line, "a two-step call's slot is not a number", shown));
            continue;
        };
        let list = parts[1].trim_start_matches('&').trim_start();
        let list = list.strip_prefix("args!").unwrap_or(list).trim_start();
        let Some(items) = list.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
            unit.unread.push((
                line,
                "a two-step call's list is not written in place",
                shown,
            ));
            continue;
        };
        let items = pieces(items);
        let words = items.iter().map(|item| item_words(item, function)).sum();
        let site = Site {
            line,
            caller,
            scope,
            bound: binding(text, scope, open, &object),
            object,
            slot,
            words,
            text: shown,
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
        .take_while(|&&at| at < addr.saturating_add(size))
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

/// What [`resolve_pending`] did with the wrappers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WrapperCount {
    /// Calls in wrappers (a caller that hands its own parameter on is one more).
    pub wrappers: usize,
    /// Of them, the ones whose list a parameter gives.
    pub lists: usize,
    /// The wrappers' callers read (each a site per slot), and not read.
    pub read: usize,
    pub not: usize,
}

/// Wrappers handed on from caller to caller at most this deep.
const WRAPPER_DEPTH: usize = 4;

/// Gives the pending calls their slots: by the constants of the whole tree (a name every unit
/// that defines it gives the same value: among the crate's `pub` constants first, then all of
/// the crate's, then the whole tree's; a unit's own name with two values stops there), else, for
/// a call in a wrapper that takes the slot or the list as a parameter, at each of the wrapper's
/// callers in the same crate (a site there, with the caller's slot and list and the caller's
/// object when the wrapper's object is a parameter too; a caller that hands its own parameter on
/// is a wrapper in turn). What stays unread goes to [`Unit::unread`]. `codes` are the units'
/// game code ([`game_code`]).
fn resolve_pending(units: &mut [(String, Unit)], codes: &[String]) -> WrapperCount {
    // A name's value where every definition agrees.
    fn unique<'a>(units: impl Iterator<Item = &'a Unit>, public: bool) -> HashMap<String, u32> {
        let mut all: HashMap<String, Option<u32>> = HashMap::new();
        for unit in units {
            let values = unit.constants.iter().map(|(n, &v)| (n, Some(v)));
            let opaque = unit.opaque.iter().map(|n| (n, None));
            for (name, value) in values.chain(opaque) {
                if public && !unit.public.contains(name) {
                    continue;
                }
                all.entry(name.clone())
                    .and_modify(|v| {
                        if *v != value {
                            *v = None;
                        }
                    })
                    .or_insert(value);
            }
        }
        all.into_iter().filter_map(|(k, v)| Some((k, v?))).collect()
    }
    let crate_of = |rel: &str| rel.split('/').next().unwrap_or("").to_string();
    let global = unique(units.iter().map(|(_, u)| u), false);
    let crate_names: BTreeSet<String> = units.iter().map(|(r, _)| crate_of(r)).collect();
    let by_crate: HashMap<String, [HashMap<String, u32>; 2]> = crate_names
        .into_iter()
        .map(|c| {
            let of_crate = || {
                units
                    .iter()
                    .filter(|(r, _)| crate_of(r) == c)
                    .map(|(_, u)| u)
            };
            let maps = [unique(of_crate(), true), unique(of_crate(), false)];
            (c, maps)
        })
        .collect();
    let empty = [HashMap::new(), HashMap::new()];
    let slot_reason =
        |slot: &str, consts: &dyn Lookup, wrapper: bool| match (uses_opaque(slot, consts), wrapper)
        {
            (true, _) => "the slot's constant has two values in its file, or one not read",
            (false, false) => "the slot is not a number",
            (false, true) => "a wrapper's slot is not a number here",
        };
    let mut added: Vec<(usize, Site)> = Vec::new();
    let mut unread: Vec<(usize, (usize, &'static str, String))> = Vec::new();
    let mut level: Vec<(usize, Site, Wrapper)> = Vec::new();
    let pending: Vec<(usize, Vec<Pending>)> = units
        .iter_mut()
        .enumerate()
        .map(|(i, (_, u))| (i, std::mem::take(&mut u.pending)))
        .collect();
    for (i, calls) in pending {
        let (rel, unit) = &units[i];
        let [public, all] = by_crate.get(&crate_of(rel)).unwrap_or(&empty);
        for pending in calls {
            let own = Own(unit);
            let consts = Chain(vec![&pending.local, &own, public, all, &global]);
            let slots = slot_values(&pending.slot, &consts);
            let reason = slot_reason(&pending.slot, &consts, false);
            let site = pending.site;
            match pending.wrapper {
                Some(mut wrapper) if pending.forwards_list => {
                    if wrapper.slot.is_none() {
                        if slots.is_empty() {
                            unread.push((i, (site.line, reason, site.text)));
                            continue;
                        }
                        wrapper.fixed = slots;
                    }
                    level.push((i, site, wrapper));
                }
                wrapper => {
                    if !slots.is_empty() {
                        for slot in slots {
                            let mut site = site.clone();
                            site.slot = slot;
                            added.push((i, site));
                        }
                    } else if let Some(wrapper) = wrapper.filter(|w| w.slot.is_some()) {
                        level.push((i, site, wrapper));
                    } else {
                        unread.push((i, (site.line, reason, site.text)));
                    }
                }
            }
        }
    }
    let mut count = WrapperCount::default();
    let mut summaries: Vec<(usize, Site, bool, [usize; 3])> = Vec::new();
    // Where each wrapper's name is called: file, position, argument texts and kind.
    type Calls = Vec<(usize, usize, Vec<String>, CallKind)>;
    let mut called: HashMap<String, Calls> = HashMap::new();
    let mut depth = 0;
    while !level.is_empty() {
        let new: BTreeSet<String> = level
            .iter()
            .map(|w| w.2.name.clone())
            .filter(|n| !called.contains_key(n))
            .collect();
        if !new.is_empty() {
            let names: HashSet<&str> = new.iter().map(String::as_str).collect();
            for name in &new {
                called.insert(name.clone(), Vec::new());
            }
            for (i, code) in codes.iter().enumerate() {
                for (name, at, args, kind) in named_calls(code, &names) {
                    if let Some(calls) = called.get_mut(name) {
                        calls.push((i, at, args, kind));
                    }
                }
            }
        }
        let mut next = Vec::new();
        for (wi, site, wrapper) in std::mem::take(&mut level) {
            count.wrappers += 1;
            count.lists += usize::from(wrapper.list.is_some());
            let krate = crate_of(&units[wi].0);
            let [mut read, mut not, mut on] = [0usize; 3];
            for (i, at, args, kind) in &called[&wrapper.name] {
                let (i, at) = (*i, *at);
                let (caller_rel, unit) = &units[i];
                let reaches = if wrapper.public {
                    crate_of(caller_rel) == krate
                } else {
                    i == wi
                };
                // A method's receiver is not among the arguments of `x.name(..)` and is the first
                // of `Type::name(x, ..)`; a free function is not called as `x.name(..)`.
                let shift = match (wrapper.method, kind) {
                    (true, CallKind::Method) | (false, CallKind::Plain | CallKind::Path) => 0,
                    (true, CallKind::Path) => 1,
                    _ => continue,
                };
                if !reaches {
                    continue;
                }
                let arg = |k: usize| args.get(k + shift).map_or("", String::as_str);
                let code = &codes[i];
                let (caller, scope, function) = function_so_far(code, at);
                let own = Own(unit);
                let local = local_constants(function, &own);
                let [public, all] = by_crate.get(&crate_of(caller_rel)).unwrap_or(&empty);
                let consts = Chain(vec![&local, &own, public, all, &global]);
                let line = 1 + code[..at].matches('\n').count();
                let shown = format!("{}({}) -> {}", wrapper.name, args.join(", "), site.text);
                let mut not_read = |reason: &'static str, shown: String| {
                    unread.push((i, (line, reason, shown)));
                };
                // The slot: a value here, or a parameter of the caller handed on.
                let mut slot_on = None;
                let slots: Vec<u32> = match wrapper.slot {
                    None => wrapper.fixed.clone(),
                    Some((k, factor)) => {
                        let given = arg(k);
                        let mut expr = given.to_string();
                        let mut free = false;
                        if is_name(given) {
                            match bound_slot(function, given) {
                                Binding::Value(value) => expr = value,
                                Binding::None => free = true,
                                Binding::Doubtful => {
                                    not += 1;
                                    not_read(
                                        "the slot's name is bound in a block that ends before \
                                         the call, or changed",
                                        shown,
                                    );
                                    continue;
                                }
                            }
                        }
                        let values: Option<Vec<u32>> = slot_values(&expr, &consts)
                            .into_iter()
                            .map(|v| v.checked_mul(factor))
                            .collect();
                        match values {
                            Some(values) if !values.is_empty() => values,
                            _ if free => {
                                slot_on = Some(given);
                                Vec::new()
                            }
                            _ => {
                                not += 1;
                                not_read(slot_reason(&expr, &consts, true), shown);
                                continue;
                            }
                        }
                    }
                };
                // The list: written in place here, or a parameter of the caller handed on.
                let mut list_on = None;
                let in_place = "a wrapper's list is not written in place here";
                let counts = match wrapper.list {
                    None => vec![site.words],
                    Some(k) => match list_of(arg(k), function, &consts) {
                        List::Words(n) => vec![n],
                        List::Name(name) => {
                            let param = wrapper_of(code, scope, at, "", Some(&name), "")
                                .is_some_and(|w| w.list.is_some());
                            if param {
                                list_on = Some(name);
                                vec![0]
                            } else if let Some(counts) = bound_list(function, &name, &consts) {
                                counts
                            } else {
                                not += 1;
                                not_read(in_place, shown);
                                continue;
                            }
                        }
                        List::Other => match closure_list(function, arg(k), &consts) {
                            Some(n) => vec![n],
                            None => {
                                not += 1;
                                not_read(in_place, shown);
                                continue;
                            }
                        },
                    },
                };
                let object = wrapper
                    .object
                    .map(arg)
                    .filter(|o| !o.is_empty())
                    .map_or_else(|| site.object.clone(), str::to_string);
                let caller_site = |slot: u32, words: u32, text: String| Site {
                    line,
                    caller,
                    scope: at,
                    bound: at,
                    object: object.clone(),
                    slot,
                    words,
                    text,
                };
                if slot_on.is_some() || list_on.is_some() {
                    let handed = wrapper_of(
                        code,
                        scope,
                        at,
                        slot_on.unwrap_or(""),
                        list_on.as_deref(),
                        &object,
                    )
                    .filter(|w| {
                        (slot_on.is_none() || w.slot.is_some())
                            && (list_on.is_none() || w.list.is_some())
                    });
                    match handed {
                        Some(mut handed) if depth < WRAPPER_DEPTH => {
                            match (handed.slot, wrapper.slot) {
                                (Some((k, _)), Some((_, factor))) => {
                                    handed.slot = Some((k, factor))
                                }
                                _ => handed.fixed = slots,
                            }
                            on += 1;
                            for words in counts {
                                next.push((
                                    i,
                                    caller_site(0, words, shown.clone()),
                                    handed.clone(),
                                ));
                            }
                        }
                        _ => {
                            not += 1;
                            not_read(
                                "a wrapper's slot or list is a name here that is no parameter \
                                 (or handed on too deep)",
                                shown,
                            );
                        }
                    }
                    continue;
                }
                read += 1;
                for slot in slots {
                    for &words in &counts {
                        added.push((i, caller_site(slot, words, shown.clone())));
                    }
                }
            }
            count.read += read;
            count.not += not;
            summaries.push((wi, site, wrapper.list.is_some(), [read, not, on]));
        }
        level = next;
        depth += 1;
    }
    for (i, site) in added {
        units[i].1.sites.push(site);
    }
    for (i, entry) in unread {
        units[i].1.unread.push(entry);
    }
    for (i, site, list, [read, not, on]) in summaries {
        let reason = match (list, read + not + on == 0) {
            (false, true) => "a wrapper's slot, and no caller found",
            (false, false) => "a wrapper's slot: checked at its callers",
            (true, true) => "a wrapper's list, and no caller found",
            (true, false) => "a wrapper's list: checked at its callers",
        };
        units[i].1.unread.push((
            site.line,
            reason,
            format!(
                "{} ({read} callers read, {not} not, {on} hand it on)",
                site.text
            ),
        ));
    }
    count
}

/// The whole check: reads the exe, the map and the crates under `crates`, and returns one line
/// per hit (tab-separated: kind, file:line, the caller's address, slot, words passed, what the
/// tables take, the call) and a summary. With `list_unread`, also one line per call it could not
/// read (`unread`, file:line, three empty columns, the reason, the call); the summary's hit count
/// leaves them out.
pub fn run(
    exe: &Path,
    map: &Path,
    crates: &Path,
    list_unread: bool,
) -> Result<(Vec<String>, String), String> {
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
    let mut sources = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(crates)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        sources.push((rel, text));
    }
    // Files a `#[cfg(test)] mod NAME;` mounts, and the folders of their own modules, are tests.
    let (mut test_paths, mut test_dirs) = (HashSet::new(), Vec::new());
    for (rel, text) in &sources {
        for mount in test_spans(text).1 {
            let (paths, dir) = mounted_paths(rel, &mount);
            test_paths.extend(paths);
            test_dirs.push(format!("{dir}/"));
        }
    }
    let mut units = Vec::new();
    let mut codes = Vec::new();
    let mut platform = SlotCounts::default();
    let mut test_files = 0usize;
    for (rel, text) in sources {
        let mounted = test_paths.contains(&rel) || test_dirs.iter().any(|d| rel.starts_with(d));
        test_files += usize::from(mounted || test_file(&rel));
        let (code, left_out) = game_code(&rel, &text, mounted);
        drop(text);
        if rel.starts_with("engine-platform/") {
            platform_slots(&code, &mut platform);
        }
        let mut unit = read_code(&code);
        unit.left_out = left_out;
        units.push((rel, unit));
        codes.push(code);
    }
    let wrappers = resolve_pending(&mut units, &codes);
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
    let (mut sites, mut unread, mut own_checked, mut left_out) = (0usize, 0usize, 0usize, 0usize);
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for (rel, unit) in &units {
        sites += unit.sites.len();
        unread += unit.unread.len();
        left_out += unit.left_out;
        if list_unread {
            for (line, reason, call) in &unit.unread {
                lines.push(format!("unread\t{rel}:{line}\t\t\t\t{reason}\t{call}"));
            }
        }
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
            *kinds.entry(hit.kind).or_default() += 1;
            lines.push(format!(
                "{}\t{rel}:{}\t{caller}\t{:#x}\t{}\t{}\t{}",
                hit.kind, hit.site.line, hit.site.slot, hit.site.words, hit.takes, hit.site.text
            ));
        }
    }
    let by_kind: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    let summary = format!(
        "{} tables with {slots} slots ({unread_slots} not read) from {} relocated words; \
         {sites} virtual calls read ({own_checked} against the unit's own tables; {} wrappers, \
         {} of them handed their list, whose callers gave {} calls read and {} not), {unread} \
         not read (`--unread` lists them, one line per wrapper and per caller not read); \
         {left_out} calls left out as test code ({test_files} test files); {} hits ({})",
        tables.len(),
        image.relocated.len(),
        wrappers.wrappers,
        wrappers.lists,
        wrappers.read,
        wrappers.not,
        lines.iter().filter(|l| !l.starts_with("unread\t")).count(),
        by_kind.join(", ")
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
        Image::parse(image_bytes(text, rdata, relocated)).expect("the image parses")
    }

    /// The file [`image`] parses.
    fn image_bytes(text: &[u8], rdata: &[u8], relocated: &[u32]) -> Vec<u8> {
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
        file
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
        // An empty entry and one running past the address space are left out.
        let map = parse_map("00401000\t0\nffffff00\t4096\nfffffff0\t15\n");
        assert_eq!(map.into_iter().collect::<Vec<_>>(), [(0xffff_fff0, 15)]);
    }

    /// A `RET`'s bytes that are really a jump's operand at the end are not read as a `RET`.
    #[test]
    fn a_ret_inside_a_jumps_operand_is_not_a_ret() {
        let none = BTreeMap::new();
        let ret = |code: &[u8]| ret_at_end(code, TEXT, &none);
        assert_eq!(ret(&[0x90, 0xc3]), Some(0));
        assert_eq!(ret(&[0x5d, 0xc2, 0x08, 0x00, 0xcc]), Some(2));
        // A short jump back 0x3d bytes from the end: inside a function that long.
        let mut long = vec![0x90; 0x40];
        long.extend_from_slice(&[0x33, 0xc0, 0xeb, 0xc3]);
        assert_eq!(ret(&long), None, "EB C3");
        long[0x42] = 0x74;
        assert_eq!(ret(&long), None, "JZ short");
        // A near jump whose target is a function's start.
        let near = [0x90, 0xe9, 0x10, 0xc2, 0x08, 0x00];
        let target = TEXT + 6 + 0x0008_c210;
        let at_target = BTreeMap::from([(target, 1)]);
        assert_eq!(ret_at_end(&near, TEXT, &at_target), None, "E9 xx C2 08 00");
        // The same for `0F 8x` (`JZ near`) whose operand ends in `C3`.
        let mut back = vec![0x90; 8];
        back.extend_from_slice(&[0x0f, 0x84]);
        back.extend_from_slice(&(-14i32).to_le_bytes()[..3]);
        back.push(0xc3);
        let rel = le32(&back, back.len() - 4).unwrap();
        let inside_target = TEXT.wrapping_add(back.len() as u32).wrapping_add(rel);
        let maps_it = BTreeMap::from([(inside_target, 1)]);
        assert_eq!(ret_at_end(&back, TEXT, &maps_it), None, "JZ near");
        // Short getters whose last operand byte looks like a jump: their target would lie
        // before the function and is no function, so the RET is real.
        assert_eq!(
            ret(&[0x8b, 0x41, 0x74, 0xc3]),
            Some(0),
            "MOV EAX, [ECX+0x74]"
        );
        assert_eq!(
            ret(&[0x8a, 0x41, 0xeb, 0xc3]),
            Some(0),
            "MOV AL, [ECX-0x15]"
        );
        assert_eq!(
            ret(&[0x8b, 0x81, 0xe9, 0x00, 0x00, 0x00, 0xc3]),
            Some(0),
            "MOV EAX, [ECX+0xe9]"
        );
        assert_eq!(
            ret(&[0x90, 0xe9, 0x10, 0xc2, 0x08, 0x00]),
            Some(2),
            "a near jump to no function"
        );
        // A short jump whose target is a mapped function's start is refused even when short.
        let getter = [0x8b, 0x41, 0x74, 0xc3];
        let before = TEXT + 4 - 0x3d;
        assert_eq!(
            ret_at_end(&getter, TEXT, &BTreeMap::from([(before, 1)])),
            None,
            "a short jump to the function before"
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
                // A plain list of three words.
                (20, "other", 4, 4),
            ]
        );
        assert_eq!(unit.sites[0].caller, Some(0x0040_1000));
        let reasons: Vec<&str> = unit.unread.iter().map(|u| u.1).collect();
        assert_eq!(reasons, ["the list is not written in place"]);
        assert_eq!(unit.left_out, 1, "the test module's call");
        // A slot in a variable waits for the whole tree's constants (or a wrapper's callers).
        let pending: Vec<(usize, &str)> = unit
            .pending
            .iter()
            .map(|p| (p.site.line, p.slot.as_str()))
            .collect();
        assert_eq!(pending, [(18, "slot")]);
        // A `pub(crate) mod tests` is a test module too.
        let crate_tests = "fn f() {}\n#[cfg(test)]\npub(crate) mod tests {\n    fn t(e: &mut Engine) { e.vcall(1, 0, &args![1]); }\n}";
        assert!(read_unit("x.rs", crate_tests).sites.is_empty());
        // A test file is not read at all.
        assert!(read_unit("crate/src/units/x_tests.rs", UNIT)
            .sites
            .is_empty());
    }

    /// A slot written as constant arithmetic is read; a slot taken from the function's parameter
    /// is checked at the wrapper's callers in the same crate, with their objects; a slot that is
    /// neither stays unread and is listed.
    #[test]
    fn slots_from_arithmetic_and_wrappers_are_read() {
        let consts = HashMap::from([("BASE".to_string(), 0x40u32)]);
        assert_eq!(eval("4 * 12", &consts), Some(48));
        assert_eq!(eval("BASE + 4 * 2", &consts), Some(0x48));
        assert_eq!(eval("(BASE + 8) as u32", &consts), Some(0x48));
        assert_eq!(eval("slot", &consts), None);
        assert_eq!(eval("crate::vtable::BASE", &consts), Some(0x40));
        assert_eq!(slot_values("if x { 4 } else { BASE }", &consts), [4, 0x40]);
        assert_eq!(slot_values("ifx==2{8}else{0xc}", &consts), [8, 0xc]);
        assert!(slot_values("if x { 4 } else { y }", &consts).is_empty());
        assert_eq!(
            slot_values("if x { f(e); y = 1; 0x68 } else { 0x64 }", &consts),
            [0x68, 0x64]
        );
        // A slot bound by a `let` in the function is read through it.
        let unit = read_unit(
            "x.rs",
            "fn f(e: &mut Engine, o: u32, up: bool) {
    let slot = if up { 0x10 } else { 0x14 };
    e.vcall(o, slot, &args![]);
}",
        );
        assert_eq!(unit.pending[0].slot, "ifup{0x10}else{0x14}");
        // A `for` over a list written in place, alone or in a tuple; a `match` of constants.
        let unit = read_unit(
            "x.rs",
            "fn f(e: &mut Engine, o: u32, k: u32) {
    for slot in [0x10u32, BASE] {
        e.vcall(o, slot, &args![]);
    }
    for (at, slot) in [(0, 0x20u32), (4, 0x24)] {
        e.vcall(o, slot, &args![at]);
    }
    let slot = match k { 1 => 0x30, _ => 0x34 };
    e.vcall(o, slot, &args![]);
}",
        );
        let got: Vec<Vec<u32>> = unit
            .pending
            .iter()
            .map(|p| slot_values(&p.slot, &consts))
            .collect();
        assert_eq!(got, [vec![0x10, 0x40], vec![0x20, 0x24], vec![0x30, 0x34]]);
        // A closure that takes the slot is a wrapper its own file calls.
        let closure = "fn f(e: &mut Engine, o: u32) {
    let one = |e: &mut Engine, slot: u32| {
        e.vcall(o, slot, &args![1u32]);
    };
    one(e, 0x7d0);
}";
        let unit = read_unit("k/d.rs", closure);
        assert_eq!(
            unit.pending[0].wrapper,
            Some(Wrapper {
                name: "one".to_string(),
                slot: Some((1, 1)),
                fixed: Vec::new(),
                list: None,
                object: None,
                public: false,
                method: false,
            })
        );
        let mut units = vec![("k/d.rs".to_string(), unit)];
        resolve_pending(&mut units, &[closure.to_string()]);
        let sites: Vec<(usize, u32, u32)> = units[0]
            .1
            .sites
            .iter()
            .map(|s| (s.line, s.slot, s.words))
            .collect();
        assert_eq!(sites, [(5, 0x7d0, 2)]);
        let wrapper = "pub fn set(e: &mut Engine, device: u32, slot: u32, value: u32) {
    e.vcall(device, 4 * slot, &args![value]);
}
fn other(e: &mut Engine, x: u32) {
    let s = e.mem.u32(x);
    e.vcall(x, s, &args![]);
}";
        let caller = "const SLOT_SIX: u32 = 6;
fn user(e: &mut Engine, d: u32, n: u32) {
    set(e, d, SLOT_SIX, 1);
    set(e, d, 2 + 1, 1);
    set(e, d, n + 1, 1);
}";
        let mut units = vec![
            ("k/a.rs".to_string(), read_unit("k/a.rs", wrapper)),
            ("k/b.rs".to_string(), read_unit("k/b.rs", caller)),
            ("other/c.rs".to_string(), read_unit("other/c.rs", caller)),
        ];
        let texts = [wrapper.to_string(), caller.to_string(), caller.to_string()];
        assert_eq!(
            units[0].1.pending[0].wrapper,
            Some(Wrapper {
                name: "set".to_string(),
                slot: Some((2, 4)),
                fixed: Vec::new(),
                list: None,
                object: Some(1),
                public: true,
                method: false,
            })
        );
        assert_eq!(resolve_pending(&mut units, &texts).wrappers, 1);
        let got: Vec<(usize, &str, u32, u32)> = units[1]
            .1
            .sites
            .iter()
            .map(|s| (s.line, s.object.as_str(), s.slot, s.words))
            .collect();
        assert_eq!(got, [(3, "d", 24, 2), (4, "d", 12, 2)]);
        let reasons = |u: &Unit| u.unread.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>();
        assert_eq!(
            reasons(&units[1].1),
            [(5, "a wrapper's slot is not a number here")]
        );
        assert_eq!(
            reasons(&units[0].1),
            [
                (6, "the slot is not a number"),
                (2, "a wrapper's slot: checked at its callers")
            ]
        );
        assert!(
            units[2].1.sites.is_empty(),
            "another crate's `set` is not this one"
        );
        // A private wrapper is called only from its own file: a same-named call elsewhere in
        // the crate is another function.
        let private =
            "fn put(e: &mut Engine, o: u32, slot: u32) {\n    e.vcall(o, slot, &args![]);\n}";
        let elsewhere = "fn g(e: &mut Engine, d: u32) {\n    put(e, d, 9);\n}";
        let mut units = vec![
            ("k/p.rs".to_string(), read_unit("k/p.rs", private)),
            ("k/q.rs".to_string(), read_unit("k/q.rs", elsewhere)),
        ];
        resolve_pending(&mut units, &[private.to_string(), elsewhere.to_string()]);
        assert!(units[1].1.sites.is_empty());
        assert_eq!(
            reasons(&units[0].1),
            [(2, "a wrapper's slot, and no caller found")]
        );
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

    /// The whole check on files: an exe without base relocations has no tables to read (an
    /// error, not an empty result); a call it cannot read is listed when asked.
    #[test]
    fn the_whole_check_refuses_an_exe_without_relocations_and_lists_unread_calls() {
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
        let result = run(&exe, &map, &dir, false);
        assert!(result.unwrap_err().contains("no base relocations"));
        // With relocations: a call whose slot is not read is listed with `--unread` (and only
        // then), and is not a hit.
        let (text, functions) = code(&[&[0xc3], &[0xc2, 0x04, 0x00]]);
        let f: Vec<u32> = functions.keys().copied().collect();
        let bytes = image_bytes(&text, &words_of(&[f[0], f[1]]), &[RDATA, RDATA + 4]);
        std::fs::write(&exe, bytes).unwrap();
        std::fs::write(&map, "00401000\t1\n00401001\t3\n").unwrap();
        let crates = dir.join("crates");
        std::fs::create_dir_all(crates.join("k")).unwrap();
        let unit = crates.join("k").join("a.rs");
        std::fs::write(
            &unit,
            "fn f(e: &mut Engine, o: u32, s: u32) {\n    let t = e.mem.u32(s);\n    e.vcall(o, t, &args![]);\n}\n",
        )
        .unwrap();
        // A module the crate mounts for tests only is not read (its call would be listed).
        let lib = crates.join("k").join("lib.rs");
        let mounted = crates.join("k").join("t.rs");
        std::fs::write(&lib, "#[cfg(test)]\nmod t;\n").unwrap();
        std::fs::write(
            &mounted,
            "fn g(e: &mut Engine, o: u32, s: u32) {\n    let t = e.mem.u32(s);\n    e.vcall(o, t, &args![]);\n}\n",
        )
        .unwrap();
        let quiet = run(&exe, &map, &crates, false).unwrap();
        let listed = run(&exe, &map, &crates, true).unwrap();
        std::fs::remove_file(&lib).ok();
        std::fs::remove_file(&mounted).ok();
        std::fs::remove_file(&unit).ok();
        std::fs::remove_dir(crates.join("k")).ok();
        std::fs::remove_dir(&crates).ok();
        std::fs::remove_file(&exe).ok();
        std::fs::remove_file(&map).ok();
        std::fs::remove_dir(&dir).ok();
        assert!(quiet.0.is_empty());
        assert_eq!(
            listed.0,
            ["unread\tk/a.rs:3\t\t\t\tthe slot is not a number\t.vcall(o, t, &args![])"]
        );
        assert!(listed.1.ends_with("; 0 hits ()"), "{}", listed.1);
        assert!(
            listed
                .1
                .contains("1 calls left out as test code (1 test files)"),
            "{}",
            listed.1
        );
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

    /// A table at `RDATA` taking one word at slot 0 and two at slot 4, and every table's counts.
    fn one_table() -> (BTreeMap<u32, Vec<Option<u32>>>, SlotCounts) {
        let (text, map) = code(&[&[0xc3], &[0xc2, 0x04, 0x00]]);
        let f: Vec<u32> = map.keys().copied().collect();
        let image = image(&text, &words_of(&[f[0], f[1]]), &[RDATA, RDATA + 4]);
        let words = table_words(&image, &map, &find_tables(&image, &map));
        let mut anywhere = SlotCounts::default();
        for table in words.values() {
            for (i, n) in table.iter().enumerate() {
                anywhere.add(i, n.unwrap());
            }
        }
        (words, anywhere)
    }

    /// Test code is left out by its own span: the game code after a test module is read (a
    /// wrong count there is found), the test module's constant does not change the file's.
    #[test]
    fn game_code_after_a_test_module_is_read() {
        let text = "const SLOT: u32 = 4; const TABLE: u32 = 0x0040_2000;
// Translated from 00401000 (decompiled)
pub fn before(e: &mut Engine, this: u32) {
    e.vcall(this, SLOT, &args![1u32]);
}
#[cfg(test)]
pub(crate) mod helper {
    const SLOT: u32 = 8;
    fn t(e: &mut Engine) { e.vcall(1, 0, &args![\"}\", '}', 3]); }
}
#[cfg(all(test, windows))]
fn stand_in(e: &mut Engine) { e.vcall(1, 0, &args![]); }
// Translated from 00401010 (decompiled)
pub fn after(e: &mut Engine, this: u32) {
    e.vcall(this, SLOT, &args![1u32, 2u32]);
}
#[cfg(any(test, feature = \"x\"))]
fn kept(e: &mut Engine, o: u32) { e.vcall(o, 0, &args![]); }
";
        let unit = read_unit("k/src/x.rs", text);
        let read: Vec<(usize, u32, u32)> = unit
            .sites
            .iter()
            .map(|s| (s.line, s.slot, s.words))
            .collect();
        assert_eq!(read, [(4, 4, 2), (15, 4, 3), (18, 0, 1)]);
        assert_eq!(unit.left_out, 2);
        assert!(unit.opaque.is_empty(), "{:?}", unit.opaque);
        let (words, anywhere) = one_table();
        let hits = check_unit(
            &unit,
            &words,
            &anywhere,
            &SlotCounts::default(),
            &HashMap::new(),
        );
        let got: Vec<(&str, usize)> = hits.iter().map(|h| (h.kind, h.site.line)).collect();
        assert_eq!(got, [("own", 15)]);
        // `#![cfg(test)]` leaves out the inline module it stands in, or the whole file.
        // A call in a string or a block comment is no call.
        let strings = "fn s(e: &mut Engine, o: u32) {\n    let t = \".vcall(o, 0, &[1])\";\n    /* e.vcall(o, 0, &[1]); */\n    e.vcall(o, 4, &[]);\n}\n";
        let unit = read_unit("k/src/s.rs", strings);
        assert_eq!(
            (unit.sites.len(), unit.unread.len(), unit.sites[0].line),
            (1, 0, 4)
        );
        let inner = "mod t {\n    #![cfg(test)]\n    fn f(e: &mut Engine) { e.vcall(1, 0, &args![]); }\n}\nfn g(e: &mut Engine, o: u32) { e.vcall(o, 0, &args![]); }\n";
        let unit = read_unit("k/src/y.rs", inner);
        assert_eq!((unit.sites.len(), unit.left_out), (1, 1));
        let whole = format!("#![cfg(test)]\n{inner}");
        assert_eq!(read_unit("k/src/y.rs", &whole).left_out, 2);
    }

    /// Test files by name, and the modules a file mounts for tests only.
    #[test]
    fn test_files_are_found_by_name_and_by_their_mount() {
        for path in [
            "k/src/tests.rs",
            "k/src/x/test.rs",
            "k/src/a_test.rs",
            "k/tests/arity.rs",
        ] {
            assert!(test_file(path), "{path}");
        }
        for path in [
            "k/src/testsuite.rs",
            "k/src/latest.rs",
            "k/src/units/contest.rs",
        ] {
            assert!(!test_file(path), "{path}");
        }
        let text = "fn a() {}\n#[cfg(test)]\n#[path = \"a_gc_tests.rs\"]\nmod gc;\n#[cfg(test)]\nmod tests;\nmod real;\nfn b(e: &mut Engine, o: u32) { e.vcall(o, 0, &args![]); }\n";
        let (spans, mounts, open) = test_spans(text);
        assert_eq!(
            mounts,
            [
                Mount {
                    name: "gc".to_string(),
                    path: Some("a_gc_tests.rs".to_string())
                },
                Mount {
                    name: "tests".to_string(),
                    path: None
                }
            ]
        );
        assert_eq!((spans.len(), open), (2, 0));
        assert_eq!(read_unit("k/src/units/a.rs", text).sites.len(), 1);
        assert_eq!(
            mounted_paths("k/src/units/a.rs", &mounts[0]),
            (
                vec!["k/src/units/a_gc_tests.rs".to_string()],
                "k/src/units/a_gc_tests".to_string()
            )
        );
        assert_eq!(
            mounted_paths("k/src/lib.rs", &mounts[1]).0,
            ["k/src/tests.rs", "k/src/tests/mod.rs"]
        );
        assert_eq!(
            mounted_paths("k/src/units/a.rs", &mounts[1]).1,
            "k/src/units/a/tests"
        );
        // A mounted file is all test code.
        let (code, left_out) = game_code("k/src/units/raw.rs", text, true);
        assert_eq!((code.trim(), left_out), ("", 1));
    }

    /// A wrapper handed the whole list (`fn com(.., slot, words: &[u32]) { e.vcall(o, slot,
    /// words) }`): each caller's list is counted, a caller that hands its own parameters on is
    /// read at its callers, a caller whose list is not in place is listed.
    #[test]
    fn calls_through_a_wrapper_handed_the_list_are_read_at_its_callers() {
        let text = "/// A COM call.
fn com(e: &mut Engine, this: Ptr, offset: u32, slot: u32, words: &[u32]) -> u32 {
    let object = field(e, this, offset);
    e.vcall(object, slot, words).u32()
}
fn user(e: &mut Engine, this: Ptr, p: u32, x: f64) {
    com(e, this, 8, 0x4c, &[p, 4, 0, 0]);
    com(e, this, 8, 0x48, &[]);
    com(e, this, 8, 0x10, &args![x, p]);
    com(e, this, 8, 0x14, &list);
    com(e, this, 8, 0x14, &[0; 2]);
}
fn on(e: &mut Engine, this: Ptr, slot: u32, arguments: &[u32]) {
    com(e, this, 4, slot, arguments);
}
fn on_user(e: &mut Engine, this: Ptr) {
    on(e, this, 0x20, &args![1u32]);
}
";
        let mut units = vec![("k/a.rs".to_string(), read_unit("k/a.rs", text))];
        assert_eq!(
            units[0].1.pending[0].wrapper,
            Some(Wrapper {
                name: "com".to_string(),
                slot: Some((3, 1)),
                fixed: Vec::new(),
                list: Some(4),
                object: None,
                public: false,
                method: false,
            })
        );
        let count = resolve_pending(&mut units, &[text.to_string()]);
        assert_eq!(
            count,
            WrapperCount {
                wrappers: 2,
                lists: 2,
                read: 5,
                not: 1
            }
        );
        let sites: Vec<(usize, u32, u32)> = units[0]
            .1
            .sites
            .iter()
            .map(|s| (s.line, s.slot, s.words))
            .collect();
        assert_eq!(
            sites,
            [
                (7, 0x4c, 5),
                (8, 0x48, 1),
                (9, 0x10, 4),
                (11, 0x14, 3),
                (17, 0x20, 2)
            ]
        );
        let reasons: Vec<(usize, &str)> = units[0].1.unread.iter().map(|r| (r.0, r.1)).collect();
        assert_eq!(
            reasons,
            [
                (10, "a wrapper's list is not written in place here"),
                (4, "a wrapper's list: checked at its callers"),
                (14, "a wrapper's list: checked at its callers"),
            ]
        );
        // A method wrapper: `x.name(..)` leaves the receiver out, `Type::name(x, ..)` has it,
        // a plain `name(..)` is another function. A closure wrapper is read at its calls.
        let method = "impl Thing {
    pub fn call(&self, e: &mut Engine, slot: u32, words: &[u32]) {
        e.vcall(self.0, slot, words);
    }
}
fn use_it(e: &mut Engine, t: Thing) {
    t.call(e, 0x8, &[1, 2]);
    Thing::call(&t, e, 0xc, &[]);
    call(e, 0x10, &[]);
}
pub fn draw(e: &mut Engine, set: Ptr) {
    let call_device = |e: &mut Engine, slot: u32, words: &[u32]| {
        let device = e.mem.u32(set);
        e.vcall(device, slot, words);
    };
    call_device(e, SET_RENDER_STATE, &args![7u32, 0u32]);
    call_device(e, 0x104, &args![0u32]);
}
const SET_RENDER_STATE: u32 = 0xe4;
";
        let mut units = vec![("k/m.rs".to_string(), read_unit("k/m.rs", method))];
        resolve_pending(&mut units, &[method.to_string()]);
        let sites: Vec<(usize, u32, u32)> = units[0]
            .1
            .sites
            .iter()
            .map(|s| (s.line, s.slot, s.words))
            .collect();
        assert_eq!(
            sites,
            [(7, 8, 3), (8, 0xc, 1), (16, 0xe4, 3), (17, 0x104, 2)]
        );
    }

    /// A name a file defines with two values, or with one not read, is looked up nowhere else;
    /// a function's own constant is its own.
    #[test]
    fn constants_with_two_values_are_not_taken_from_elsewhere() {
        let text = "const SLOT: u32 = 4;
fn a(e: &mut Engine, o: u32) {
    e.vcall(o, SLOT, &args![]);
}
mod inner {
    const SLOT: u32 = 8;
}
fn b(e: &mut Engine, o: u32) {
    const LOCAL: u32 = 0x10;
    e.vcall(o, LOCAL, &args![]);
}
fn c(e: &mut Engine, o: u32) {
    const LOCAL: u32 = 0x14;
    e.vcall(o, LOCAL, &args![]);
}
const ODD: Slot = Slot(4);
fn d(e: &mut Engine, o: u32) {
    e.vcall(o, ODD, &args![]);
}";
        let other = "pub const SLOT: u32 = 4;\npub const ODD: u32 = 0x30;\n";
        let unit = read_unit("k/x.rs", text);
        let opaque: BTreeSet<&str> = unit.opaque.iter().map(String::as_str).collect();
        assert_eq!(opaque, BTreeSet::from(["LOCAL", "ODD", "SLOT"]));
        let mut units = vec![
            ("k/x.rs".to_string(), unit),
            ("k/y.rs".to_string(), read_unit("k/y.rs", other)),
        ];
        resolve_pending(&mut units, &[text.to_string(), other.to_string()]);
        let sites: Vec<(usize, u32)> = units[0].1.sites.iter().map(|s| (s.line, s.slot)).collect();
        assert_eq!(sites, [(10, 0x10), (14, 0x14)]);
        let reasons: Vec<(usize, &str)> = units[0].1.unread.iter().map(|r| (r.0, r.1)).collect();
        let two = "the slot's constant has two values in its file, or one not read";
        assert_eq!(reasons, [(3, two), (18, two)]);
    }

    /// A slot's name bound in a block that closed before the call, changed after its binding,
    /// or a wrapper's parameter bound again, is not read.
    #[test]
    fn bindings_out_of_scope_or_changed_are_not_read() {
        let text = "fn scoped(e: &mut Engine, o: u32, up: bool, slot: u32) {
    if up {
        let s = 0x10;
        e.vcall(o, s, &args![]);
    }
    let t = 0x20;
    {
        let t = 0x24;
    }
    e.vcall(o, t, &args![]);
    let mut m = 0x30;
    m += 4;
    e.vcall(o, m, &args![]);
    for f in [0x40, 0x44] {
        e.vcall(o, f, &args![]);
    }
    for g in [0x50] {}
    e.vcall(o, g, &args![]);
    let slot = slot + 4;
    e.vcall(o, slot, &args![]);
}";
        let unit = read_unit("k/s.rs", text);
        let lines: Vec<usize> = unit.unread.iter().map(|r| r.0).collect();
        assert_eq!(lines, [10, 13, 18]);
        assert!(unit.pending.iter().all(|p| p.wrapper.is_none()));
        let mut units = vec![("k/s.rs".to_string(), unit)];
        resolve_pending(&mut units, &[text.to_string()]);
        let sites: Vec<(usize, u32)> = units[0].1.sites.iter().map(|s| (s.line, s.slot)).collect();
        assert_eq!(sites, [(4, 0x10), (15, 0x40), (15, 0x44)]);
        assert_eq!(
            units[0].1.unread.last().map(|r| (r.0, r.1)),
            Some((20, "the slot is not a number"))
        );
        // A list held in a name: a `let` of a list in place, each list of a `for`; a list
        // changed after its `let` is not read.
        let lists = "fn f(e: &mut Engine, o: u32, x: u32) {
    let a = args![x, 1u32];
    e.vcall(o, 4, &a);
    for entry in [[0u32, 1], [2, 3, 4]] {
        e.vcall(o, 8, &entry);
    }
    let mut all = [x];
    all.extend_from_slice(&[x]);
    e.vcall(o, 0xc, &all);
    let b = [x, x];
    b.len();
    e.vcall(o, 0x10, &b);
}";
        let unit = read_unit("k/l.rs", lists);
        let sites: Vec<(usize, u32, u32)> = unit
            .sites
            .iter()
            .map(|s| (s.line, s.slot, s.words))
            .collect();
        assert_eq!(sites, [(3, 4, 3), (5, 8, 3), (5, 8, 4), (12, 0x10, 3)]);
        let lines: Vec<usize> = unit.unread.iter().map(|r| r.0).collect();
        assert_eq!(lines, [9]);
        // A list whose type fixes its length: its words, whatever the callers hand over.
        let typed = "fn add(e: &mut Engine, map: u32, words: &[u32; 10]) {\n    e.vcall(map, 8, words);\n}\nfn put(e: &mut Engine, map: u32, words: [u32; N]) {\n    let words = vec![1];\n    e.vcall(map, 8, &words);\n}\nconst N: u32 = 3;\n";
        let unit = read_unit("k/t.rs", typed);
        let sites: Vec<(usize, u32)> = unit.sites.iter().map(|s| (s.line, s.words)).collect();
        assert_eq!(sites, [(2, 11)]);
        assert_eq!(unit.unread.len(), 1, "a `let` binds the name again");
        // A list a closure gives.
        let closure = "fn c(e: &mut Engine, o: u32, a: f64) {\n    let words = |child: u32| {\n        args![child, a, 1u32]\n    };\n    e.vcall(o, 8, &words(o));\n    let held = words(a as u32);\n    e.vcall(o, 0xc, &held);\n}\n";
        let unit = read_unit("k/c.rs", closure);
        let sites: Vec<(usize, u32)> = unit.sites.iter().map(|s| (s.line, s.words)).collect();
        assert_eq!(sites, [(5, 5), (7, 5)]);
    }

    /// The tree's constants: a name every definition agrees on, a crate's `pub` one before its
    /// private ones elsewhere; a `match` with an arm that is no constant gives no slot.
    #[test]
    fn tree_constants_agree_or_are_not_used() {
        let user = "fn f(e: &mut Engine, o: u32) {\n    e.vcall(o, SLOT, &args![]);\n}";
        let resolve = |a: &str, b: &str| {
            let mut units = vec![
                ("k/a.rs".to_string(), read_unit("k/a.rs", a)),
                ("k/b.rs".to_string(), read_unit("k/b.rs", b)),
                ("k/c.rs".to_string(), read_unit("k/c.rs", user)),
            ];
            resolve_pending(
                &mut units,
                &[a.to_string(), b.to_string(), user.to_string()],
            );
            let unit = &units[2].1;
            (
                unit.sites.iter().map(|s| s.slot).collect::<Vec<_>>(),
                unit.unread.len(),
            )
        };
        assert_eq!(
            resolve("const SLOT: u32 = 4;", "const SLOT: u32 = 8;"),
            (vec![], 1)
        );
        assert_eq!(
            resolve("pub const SLOT: u32 = 4;", "const SLOT: u32 = 8;"),
            (vec![4], 0)
        );
        assert_eq!(
            resolve("const SLOT: u32 = 4;", "const OTHER: u32 = 8;"),
            (vec![4], 0)
        );
        let consts = HashMap::from([("A".to_string(), 4u32)]);
        assert!(slot_values("match k { 1 => 4, _ => y }", &consts).is_empty());
        assert_eq!(slot_values("match k { 1 => 4, _ => A }", &consts), [4, 4]);
    }
}
