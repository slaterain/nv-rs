//! Every form's editor id at run time, as JohnnyGuitar NVSE's editor-id restoration keeps them
//! (`internal/JG/EditorIDRestoration.cpp`, `JohnnyExtraData`, JohnnyGuitar NVSE 5.28): the store
//! the built-in JohnnyGuitar (`crates/johnnyguitar`) fills while the game loads, and the lookups
//! by name the game's `TESForm::GetFormByEditorID` (`00483A00`) answers from while its row is on.
//!
//! The game itself keeps the names of a few form types only (its editor id map `*011C54C8`, a
//! case-sensitive `NiTMap`); every other form's `SetEditorID` (vtable +0x134) is a stub, so text
//! compiled while the game runs (the console, script runner files, `CompileScript`, lambdas from
//! text) finds a reference such as `PlayerRef` by name only with JohnnyGuitar installed.
//!
//! The store is outside the game's 32-bit memory (one name for every loaded form is many MB on a
//! large load order): each form's names in one growing byte arena, newest last; the map by name
//! keys a 64-bit hash of the name folded to lower case (the game's `tolower` in the "C" locale,
//! which JohnnyGuitar's case-blind compare uses), each hit checked against the stored text, with
//! the rare names whose hash another name holds in a second map. A name reaches the game's memory
//! only when game code asks for it as text ([`EditorIds::guest_text`]): copied once, then kept.
//!
//! One store per run, shared by every real thread's engine; a reader-writer lock as JohnnyGuitar's
//! `SRWLOCK` (lookups read, names written while forms load). Off (the row not on), nothing here
//! is used and the game's own map works as in the exe.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;

/// `JohnnyExtraData::EDIDResult`: a name added, already the form's, or refused (empty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetResult {
    Success,
    AlreadyExists,
    Failure,
}

/// One name: its place in the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    at: u32,
    len: u32,
}

/// A form's names, oldest first (JohnnyGuitar's list adds at its head, so its "editor id" is the
/// newest: here the last), and the guest copy of the newest once asked for (0 before).
#[derive(Default)]
struct Names {
    list: Vec<Span>,
    guest: u32,
}

#[derive(Default)]
struct Store {
    arena: Vec<u8>,
    forms: HashMap<u32, Names>,
    /// The game's map, by the folded name's hash.
    map: HashMap<u64, (Span, u32)>,
    /// Names whose hash slot another name holds (by the folded text).
    clash: HashMap<Box<[u8]>, u32>,
}

/// The game's `tolower` (`00EC67AA`, the "C" locale): ASCII letters only.
fn fold(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

/// FNV-1a over the folded name.
fn hash(name: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in name {
        h ^= u64::from(fold(b));
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn folded(name: &[u8]) -> Box<[u8]> {
    name.iter().map(|&b| fold(b)).collect()
}

impl Store {
    /// The bytes its tables take, by capacity (a hash table's slot is its key and value and one
    /// control byte).
    fn heap_bytes(&self) -> usize {
        use std::mem::size_of;
        let names: usize = self
            .forms
            .values()
            .map(|n| n.list.capacity() * size_of::<Span>())
            .sum();
        let clash: usize = self.clash.keys().map(|k| k.len()).sum();
        self.arena.capacity()
            + self.forms.capacity() * (size_of::<u32>() + size_of::<Names>() + 1)
            + names
            + self.map.capacity() * (size_of::<u64>() + size_of::<(Span, u32)>() + 1)
            + self.clash.capacity() * (size_of::<Box<[u8]>>() + size_of::<u32>() + 1)
            + clash
    }

    fn text(&self, s: Span) -> &[u8] {
        &self.arena[s.at as usize..(s.at + s.len) as usize]
    }

    fn push(&mut self, name: &[u8]) -> Span {
        let at = self.arena.len() as u32;
        self.arena.extend_from_slice(name);
        Span {
            at,
            len: name.len() as u32,
        }
    }

    fn map_get(&self, name: &[u8]) -> u32 {
        match self.map.get(&hash(name)) {
            Some(&(s, form)) if self.text(s).eq_ignore_ascii_case(name) => form,
            Some(_) => self.clash.get(&folded(name)).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// `pAllFormsByEditorID->SetAt(name, form)`: the name's entry now names `form`.
    fn map_set(&mut self, name: Span, form: u32) {
        let h = hash(self.text(name));
        match self.map.get(&h) {
            Some(&(s, _)) if !self.text(s).eq_ignore_ascii_case(self.text(name)) => {
                let key = folded(self.text(name));
                self.clash.insert(key, form);
            }
            _ => {
                self.map.insert(h, (name, form));
            }
        }
    }

    /// `RemoveAt(name)`: whether the name had an entry.
    fn map_remove(&mut self, name: &[u8]) -> bool {
        let h = hash(name);
        match self.map.get(&h) {
            Some(&(s, _)) if self.text(s).eq_ignore_ascii_case(name) => {
                self.map.remove(&h);
                true
            }
            Some(_) => self.clash.remove(&folded(name)).is_some(),
            None => false,
        }
    }
}

/// The store (an engine's lock-free slot, [`crate::Engine::ext_free`]).
#[derive(Default)]
pub struct EditorIds {
    on: AtomicBool,
    store: RwLock<Store>,
    /// Counts for the load's log line: names stored, lookups, conflicts.
    named: AtomicU64,
    lookups: AtomicU64,
    conflicts: AtomicU64,
    /// The load's form total the tables were last sized for ([`EditorIds::reserve_for_load`]).
    reserved: AtomicU32,
}

/// What the store holds (the load's log line).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub named: u64,
    pub lookups: u64,
    pub conflicts: u64,
    /// Forms with a name, entries in the map, and the arena's bytes.
    pub forms: usize,
    pub entries: usize,
    pub arena_bytes: usize,
    /// What the store holds in all, by its tables' capacities (the arena, each form's names, the
    /// map with its control bytes): the load's cost in memory.
    pub heap_bytes: usize,
}

impl EditorIds {
    fn read(&self) -> std::sync::RwLockReadGuard<'_, Store> {
        self.store.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Store> {
        self.store.write().unwrap_or_else(|p| p.into_inner())
    }

    /// Whether the store answers for the game (its row is on).
    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }

    pub fn set_on(&self, on: bool) {
        self.on.store(on, Ordering::Release);
    }

    /// nv-rs's own (no behaviour): the form table and the map sized once for `records` forms, so
    /// they do not grow step by step while the plugins load. Nothing when they were sized for as
    /// many already (one atomic read). The name bytes still grow as they come.
    pub fn reserve_for_load(&self, records: u32) {
        if records <= self.reserved.load(Ordering::Acquire) {
            return;
        }
        let mut s = self.write();
        if records <= self.reserved.load(Ordering::Acquire) {
            return;
        }
        let forms = (records as usize).saturating_sub(s.forms.len());
        s.forms.reserve(forms);
        let entries = (records as usize).saturating_sub(s.map.len());
        s.map.reserve(entries);
        self.reserved.store(records, Ordering::Release);
    }

    /// `JohnnyExtraData::SetEditorID`: an empty name is refused; the form's first name, or a new
    /// alias (case-sensitive, as the game's string pool compares), is added; a name it has
    /// already is not.
    pub fn add_name(&self, form: u32, name: &[u8]) -> SetResult {
        if name.is_empty() {
            return SetResult::Failure;
        }
        let mut s = self.write();
        let has = s
            .forms
            .get(&form)
            .is_some_and(|n| n.list.iter().any(|&sp| s.text(sp) == name));
        if has {
            return SetResult::AlreadyExists;
        }
        let span = s.push(name);
        let names = s.forms.entry(form).or_default();
        names.list.push(span);
        names.guest = 0;
        self.named.fetch_add(1, Ordering::Relaxed);
        SetResult::Success
    }

    /// `JohnnyExtraData::RemoveEditorID`: the name taken off the form's list.
    pub fn remove_name(&self, form: u32, name: &[u8]) {
        let mut s = self.write();
        let Some(list) = s.forms.get(&form).map(|n| n.list.clone()) else {
            return;
        };
        let kept: Vec<Span> = list.into_iter().filter(|&sp| s.text(sp) != name).collect();
        if let Some(n) = s.forms.get_mut(&form) {
            if n.list.len() != kept.len() {
                n.list = kept;
                n.guest = 0;
            }
        }
    }

    /// The form's editor id (its newest name), if it has one.
    pub fn name(&self, form: u32) -> Option<Vec<u8>> {
        let s = self.read();
        let n = s.forms.get(&form)?;
        n.list.last().map(|&sp| s.text(sp).to_vec())
    }

    /// `GetFormEditorIDLength`: the newest name's length, 0 for none.
    pub fn name_len(&self, form: u32) -> u32 {
        let s = self.read();
        s.forms
            .get(&form)
            .and_then(|n| n.list.last())
            .map_or(0, |sp| sp.len)
    }

    /// `GetFormByEditorID` (under the read lock): the form the map names for `name`, 0 for none or
    /// an empty name. Case-blind.
    pub fn lookup(&self, name: &[u8]) -> u32 {
        self.lookups.fetch_add(1, Ordering::Relaxed);
        if name.is_empty() {
            return 0;
        }
        self.read().map_get(name)
    }

    /// `AddToGameMap`'s part under the map's lock: when `name` names another form, `Some` of it
    /// (the caller decides whether that is a conflict); then the name names `form`. `None` when
    /// it was free; `Some(form)` itself when it already named it (nothing changes).
    pub fn map_insert(&self, form: u32, name: &[u8]) -> Option<u32> {
        let mut s = self.write();
        let old = s.map_get(name);
        if old == form && old != 0 {
            return Some(form);
        }
        // The key's text: the form's own copy when it has the name, else a new one.
        let span = s
            .forms
            .get(&form)
            .and_then(|n| n.list.iter().copied().find(|&sp| s.text(sp) == name));
        let span = match span {
            Some(sp) => sp,
            None => s.push(name),
        };
        s.map_set(span, form);
        (old != 0).then_some(old)
    }

    /// One conflict counted (the load's log line).
    pub fn count_conflict(&self) {
        self.conflicts.fetch_add(1, Ordering::Relaxed);
    }

    /// `RemoveAt(name)` on the map alone.
    pub fn map_remove(&self, name: &[u8]) -> bool {
        self.write().map_remove(name)
    }

    /// `JohnnyExtraData::DetachEditorIDs`: each of the form's names taken out of the map (the
    /// names stay the form's); nothing for a form already temporary (`!pOwner->GetTemporary()`),
    /// whose names a permanent form may hold in the map now.
    pub fn detach(&self, form: u32, temporary: bool) {
        if temporary {
            return;
        }
        let mut s = self.write();
        let names: Vec<Vec<u8>> = s
            .forms
            .get(&form)
            .map(|n| n.list.iter().map(|&sp| s.text(sp).to_vec()).collect())
            .unwrap_or_default();
        for name in names {
            s.map_remove(&name);
        }
    }

    /// A form gone (`JohnnyExtraData::OnRemoval`: deleted, or a reference trashed): its names out
    /// of the map (not for a temporary form, as [`EditorIds::detach`]), then its names dropped.
    /// Answers its guest copy (0 for none), for the caller to free.
    pub fn forget(&self, form: u32, temporary: bool) -> u32 {
        self.detach(form, temporary);
        self.write().forms.remove(&form).map_or(0, |n| n.guest)
    }

    /// The guest copy of the form's newest name already made, 0 for none yet.
    pub fn guest_copy(&self, form: u32) -> u32 {
        self.read().forms.get(&form).map_or(0, |n| n.guest)
    }

    /// Keeps `text` as the form's guest copy unless another thread made one first; answers the
    /// one kept.
    pub fn keep_guest_copy(&self, form: u32, text: u32) -> u32 {
        let mut s = self.write();
        match s.forms.get_mut(&form) {
            Some(n) if n.guest == 0 => {
                n.guest = text;
                text
            }
            Some(n) => n.guest,
            None => 0,
        }
    }

    pub fn counts(&self) -> Counts {
        let s = self.read();
        Counts {
            named: self.named.load(Ordering::Relaxed),
            lookups: self.lookups.load(Ordering::Relaxed),
            conflicts: self.conflicts.load(Ordering::Relaxed),
            forms: s.forms.len(),
            entries: s.map.len() + s.clash.len(),
            arena_bytes: s.arena.len(),
            heap_bytes: s.heap_bytes(),
        }
    }
}

/// This run's store.
pub fn store(e: &mut crate::Engine) -> &EditorIds {
    e.ext_free::<EditorIds>()
}

/// Whether the store answers for the game.
pub fn on(e: &mut crate::Engine) -> bool {
    store(e).is_on()
}

/// The form's editor id as guest text for game code (`GetEditorID`): the newest name, copied into
/// the game's memory the first time it is asked for and kept; the empty text (`01011584`) for
/// none.
pub fn guest_text(e: &mut crate::Engine, form: u32) -> u32 {
    const EMPTY_TEXT: u32 = 0x0101_1584;
    let made = store(e).guest_copy(form);
    if made != 0 {
        return made;
    }
    let Some(name) = store(e).name(form) else {
        return EMPTY_TEXT;
    };
    if name.is_empty() {
        return EMPTY_TEXT;
    }
    let text = e.mem.alloc(name.len() as u32 + 1);
    e.mem.write(text, &name);
    e.mem.set_u8(text + name.len() as u32, 0);
    let kept = store(e).keep_guest_copy(form, text);
    if kept != text {
        // Another thread's copy was kept first (or the form went meanwhile): ours goes.
        e.mem.free(text);
    }
    if kept == 0 {
        return EMPTY_TEXT;
    }
    kept
}

/// Whether the form is temporary (its flags +8, 0x4000).
pub fn is_temporary(e: &mut crate::Engine, form: u32) -> bool {
    e.mem.u32(form.wrapping_add(FORM_FLAGS)) & TEMPORARY != 0
}

/// `TESForm::SetTemporary`'s hook (`hk_DetachEditorIDs`, after `RemoveFromDataStructures`): the
/// form's names leave the map unless it is temporary already.
pub fn detach_form(e: &mut crate::Engine, form: u32) {
    let temporary = is_temporary(e, form);
    store(e).detach(form, temporary);
}

/// A form gone (xNVSE's removal, JG's `OnRemoval`): [`EditorIds::forget`], and its guest copy
/// freed back to the game's heap, so streamed named references do not leave copies behind.
pub fn form_gone(e: &mut crate::Engine, form: u32) {
    let temporary = is_temporary(e, form);
    let guest = store(e).forget(form, temporary);
    e.mem.free(guest);
}

// ---- JohnnyGuitar's setter (`TESFormEx::hk_SetFormEditorID`), shared with JIP's light names ----

/// A form's flags (+8: 0x4000 temporary), id (+0xC), files (+0x18, a `tList` of `ModInfo`, its
/// name at +0x20); the game's form type name (`00440e30`, `thiscall`).
const FORM_FLAGS: u32 = 0x08;
const FORM_ID: u32 = 0x0c;
const FORM_FILES: u32 = 0x18;
const FILE_NAME: u32 = 0x20;
const TEMPORARY: u32 = 0x4000;
const FORM_TYPE_NAME: u32 = 0x0044_0e30;
/// `IgnoredConflicts`: the form JIP's note calls "Obsidian's", and the name `Wilderness`.
const IGNORED_FORM_ID: u32 = 0x18e;
const IGNORED_NAME: &[u8] = b"Wilderness";
/// The load's form total (`0x011C3F24`): the data handler's load of the plugin files (`00463070`)
/// sums each file's form count into it before it loads them (the "Loading Files %d%%" count; 0
/// before a load).
const LOAD_FORM_TOTAL: u32 = 0x011c_3f24;

/// The conflict lines this run logged (`LogEDIDConflict`), all of them, for JohnnyGuitar's log.
#[derive(Default)]
pub struct ConflictLog(std::sync::Mutex<Vec<String>>);

/// The lines logged so far.
pub fn conflict_lines(e: &mut crate::Engine) -> Vec<String> {
    e.ext_free::<ConflictLog>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESFormEx::hk_SetFormEditorID.
/// A form's `SetEditorID` with the store: no text or an empty one is refused; the console's
/// `SysWindowCompileAndRun` is named `Console Command`; the name added to the form's names, and
/// when it is new there and the form is neither temporary nor of id 0, put in the map
/// ([`add_to_game_map`]). Whether the name was new.
pub fn set_form_editor_id(e: &mut crate::Engine, form: u32, text: u32) -> bool {
    if text == 0 || e.mem.u8(text) == 0 {
        return false;
    }
    let mut name = e.mem.cstr(text);
    if name == b"SysWindowCompileAndRun" {
        name = b"Console Command".to_vec();
    }
    // nv-rs's own: the tables sized for the load's form total when it is known.
    if e.mem.is_mapped(LOAD_FORM_TOTAL) {
        let records = e.mem.u32(LOAD_FORM_TOTAL);
        store(e).reserve_for_load(records);
    }
    if store(e).add_name(form, &name) != SetResult::Success {
        return false;
    }
    if e.mem.u32(form + FORM_FLAGS) & TEMPORARY == 0 && e.mem.u32(form + FORM_ID) != 0 {
        add_to_game_map(e, form, &name);
    }
    true
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// AddToGameMap, IgnoredConflicts::IsIgnored.
/// The name put in the map for `form`. When another form had it: unless that one is id 0x18E or
/// the name is `Wilderness`, a conflict, logged ([`log_conflict`]), and the name taken off the
/// other form's names; the map then names `form` either way. Nothing when it named `form` already.
pub fn add_to_game_map(e: &mut crate::Engine, form: u32, name: &[u8]) {
    let Some(old) = store(e).map_insert(form, name) else {
        return;
    };
    if old == form {
        return;
    }
    if e.mem.u32(old + FORM_ID) == IGNORED_FORM_ID || name == IGNORED_NAME {
        return;
    }
    log_conflict(e, old, name, form);
    store(e).remove_name(old, name);
}

/// A form's file name: its `n`th file (`-1`: its last), "" for none.
fn file_name(e: &crate::Engine, form: u32, last: bool) -> String {
    let mut node = form + FORM_FILES;
    let mut file = 0;
    let mut steps = 0;
    while node != 0 && steps < 0x1000 {
        let data = e.mem.u32(node);
        if data != 0 {
            file = data;
            if !last {
                break;
            }
        }
        node = e.mem.u32(node + 4);
        steps += 1;
    }
    if file == 0 {
        return String::new();
    }
    String::from_utf8_lossy(&e.mem.cstr(file + FILE_NAME)).into_owned()
}

fn type_name(e: &mut crate::Engine, form: u32) -> String {
    let text = e.call(FORM_TYPE_NAME, &[form]).u32();
    if text == 0 {
        String::new()
    } else {
        String::from_utf8_lossy(&e.mem.cstr(text)).into_owned()
    }
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// LogEDIDConflict.
/// One conflict line (JG's log's text: the new form's id and last file steal the name from the
/// old one's id and first file, with both types when they differ), kept for the log and
/// counted.
fn log_conflict(e: &mut crate::Engine, old: u32, name: &[u8], form: u32) {
    let (file_a, file_b) = (file_name(e, form, true), file_name(e, old, false));
    let (id_a, id_b) = (e.mem.u32(form + FORM_ID), e.mem.u32(old + FORM_ID));
    let name = String::from_utf8_lossy(name);
    let mut line =
        format!("{id_a:08X} (\"{file_a}\") steals EDID \"{name}\" from {id_b:08X} (\"{file_b}\")");
    if e.mem.u8(form + 4) != e.mem.u8(old + 4) {
        let (from, to) = (type_name(e, old), type_name(e, form));
        line.push_str(&format!(" + changes type from {from} to {to}"));
    }
    store(e).count_conflict();
    let mut lines = e
        .ext_free::<ConflictLog>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    lines.push(line);
}

#[cfg(test)]
#[path = "editor_ids_tests.rs"]
mod tests;
