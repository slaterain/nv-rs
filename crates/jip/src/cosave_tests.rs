//! Tests of JIP's co-save section: each record read into JIP's data and written back in JIP's
//! order, over a stand-in of the game's tables (the save's file table, the form map, the save's
//! changed forms, the player), and the messages that decide what a load reads.

use super::*;
use crate::containers::mixed_hash;
use nvse::cosave::{tag, Chunk};

const SL: u32 = 0x0500_0000;
const CHANGES: u32 = 0x0500_1000;
const FORMS: u32 = 0x0500_4000;
const PLAYER_REF: u32 = 0x0500_8000;
const PLAYER_NPC: u32 = 0x0500_9000;
const REF_VT: u32 = 0x0510_0000;
const PLAIN_VT: u32 = 0x0511_0000;
const FORM_AT: u32 = 0x0600_0000;
const ENTRIES: u32 = 0x0520_0000;
/// Where a stand-in form keeps its `ExtraScript` and `ExtraLinkedRef` for the extra list stub.
const XSCRIPT: u32 = 0x80;
const XLINKED: u32 = 0x84;

struct World {
    e: Engine,
    next_form: u32,
    next_entry: u32,
}

/// The extra list's `GetByType` stub: a stand-in form keeps its extras at +0x80 and +0x84.
fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    let form = a[0] - 0x44;
    let at = match a[1] {
        0x0d => e.mem.u32(form + XSCRIPT),
        0x51 => e.mem.u32(form + XLINKED),
        _ => 0,
    };
    Ret {
        eax: at,
        ..Ret::default()
    }
}

impl World {
    fn new() -> World {
        let mut e = Engine::new();
        for page in [0x011d_d000, 0x011c_5000, 0x011d_e000] {
            e.map(page, 0x1000);
        }
        e.map(0x0500_0000, 0x0030_0000);
        e.map(FORM_AT, 0x0010_0000);
        e.mem.set_u32(SAVE_LOAD_GAME, SL);
        e.mem.set_u32(SL, CHANGES);
        for i in 0..0xffu32 {
            e.mem.set_u8(SL + SAVE_MODS + i, i as u8);
        }
        for (map, buckets) in [(CHANGES, 16u32), (FORMS, 16)] {
            e.mem.set_u32(map + 4, buckets);
            e.mem.set_u32(map + 8, map + 0x100);
        }
        e.mem.set_u32(ALL_FORMS, FORMS);
        e.mem.set_u32(PLAYER, PLAYER_REF);
        e.mem.set_u32(PLAYER_REF + BASE_FORM, PLAYER_NPC);
        e.mem.set_u32(REF_VT + VT_IS_REFERENCE, RETURN_TRUE);
        e.mem.set_u32(REF_VT + VT_IS_ACTOR, RETURN_TRUE);
        e.register(EXTRA_GET_BY_TYPE, get_by_type);
        World {
            e,
            next_form: FORM_AT,
            next_entry: ENTRIES,
        }
    }

    fn insert(&mut self, map: u32, id: u32, value: u32) {
        let at = self.next_entry;
        self.next_entry += 0x10;
        let bucket = self.e.mem.u32(map + 8) + 4 * (id % self.e.mem.u32(map + 4));
        self.e.mem.set_u32(at, self.e.mem.u32(bucket));
        self.e.mem.set_u32(at + 4, id);
        self.e.mem.set_u32(at + 8, value);
        self.e.mem.set_u32(bucket, at);
    }

    /// A loaded form: a reference (and actor) or a plain one of `kind`.
    fn form(&mut self, id: u32, kind: u8, reference: bool) -> u32 {
        let at = self.next_form;
        self.next_form += 0x100;
        let vt = if reference { REF_VT } else { PLAIN_VT };
        self.e.mem.set_u32(at, vt);
        self.e.mem.set_u8(at + FORM_TYPE, kind);
        self.e.mem.set_u32(at + FORM_ID, id);
        self.insert(FORMS, id, at);
        at
    }

    /// A reference with a script and an event list (`ExtraScript`).
    fn scripted(&mut self, id: u32) -> u32 {
        let form = self.form(id, 0x3a, true);
        let extra = form + 0x40;
        self.e.mem.set_u32(extra + 0xc, 0x0700_0000);
        self.e.mem.set_u32(extra + 0x10, 0x0700_0100);
        self.e.mem.set_u32(form + XSCRIPT, extra);
        form
    }

    fn changed(&mut self, id: u32) {
        self.insert(CHANGES, id, 1);
    }

    /// The save's file `saved` loaded as `now` today (0xFF: not loaded).
    fn file(&mut self, saved: u8, now: u8) {
        self.e.mem.set_u8(SL + SAVE_MODS + u32::from(saved), now);
    }
}

fn s(e: &mut Engine) -> &mut Saved {
    saved(e)
}

/// The section's records as a co-save holds them (each chunk's header and data).
fn section(chunks: &[Chunk]) -> Vec<u8> {
    let mut out = Vec::new();
    for c in chunks {
        out.extend_from_slice(&c.kind.to_le_bytes());
        out.extend_from_slice(&c.version.to_le_bytes());
        out.extend_from_slice(&(c.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&c.data);
    }
    out
}

/// A co-save holding JIP's section with `chunks`.
fn cosave(chunks: &[Chunk]) -> Vec<u8> {
    nvse::cosave::write(&[nvse::cosave::Plugin {
        opcode_base: 0x2200,
        chunks: chunks.to_vec(),
    }])
}

/// Reads JIP's section of `file` as xNVSE's load pass hands it to JIP's callback.
fn load_file(e: &mut Engine, file: &[u8]) {
    let mut r = Reader::open(file).unwrap();
    let plugin = r.next_plugin().unwrap().unwrap();
    assert_eq!(plugin.opcode_base, 0x2200);
    load(e, &mut r).unwrap();
}

fn saved_chunks(e: &mut Engine) -> Vec<Chunk> {
    let mut r = Records::default();
    save(e, &mut r);
    r.chunks
}

/// The chunks written, less the saved flags' record every save holds.
fn written(e: &mut Engine) -> Vec<Chunk> {
    saved_chunks(e)
        .into_iter()
        .filter(|c| c.kind != kind::SERIALIZED_VARS)
        .collect()
}

fn chunk(kind: u32, version: u32, data: Vec<u8>) -> Chunk {
    Chunk {
        kind,
        version,
        data,
    }
}

#[derive(Default)]
struct Out(Vec<u8>);
impl Out {
    fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }
    fn u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u64(mut self, v: u64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn bytes(mut self, b: &[u8]) -> Self {
        self.0.extend_from_slice(b);
        self
    }
    /// A name: its length byte and bytes.
    fn name(self, n: &[u8]) -> Self {
        self.u8(n.len() as u8).bytes(n)
    }
}

/// The record types' bytes in the file are `JPSV` and so on; the low half reads `PJ`.
#[test]
fn record_types() {
    assert_eq!(kind::SCRIPT_VARS.to_le_bytes(), *b"JPSV");
    assert_eq!(kind::EXTRA_DATA, 0x4445_504a, "the DLL's push");
    for k in [
        kind::AUX_VARS,
        kind::REF_MAPS,
        kind::LINKED_REFS,
        kind::SERIALIZED_VARS,
        kind::APPEARANCE_UNDO,
        kind::NPC_PERKS,
    ] {
        assert_eq!(k & 0xffff, u32::from_le_bytes(*b"JP\0\0"));
    }
    assert_eq!(tag(b"VAPJ").to_le_bytes(), *b"JPAV");
}

/// The save's file table and the game's maps as JIP reads them.
#[test]
fn ids_resolve_through_the_saves_file_table() {
    let mut w = World::new();
    w.file(2, 5);
    w.file(3, 0xff);
    let e = &w.e;
    assert_eq!(resolved_mod_index(e, 2), Some(5));
    assert_eq!(resolved_mod_index(e, 3), None);
    assert_eq!(resolved_mod_index(e, 0xff), Some(0xff));
    assert_eq!(resolved_ref_id(e, 0x0200_1234), 0x0500_1234);
    assert_eq!(resolved_ref_id(e, 0x0300_1234), 0);
    assert_eq!(resolved_ref_id(e, 0xff00_0001), 0xff00_0001);
    assert_eq!(resolved_ref_id(e, 0), 0);
    let form = w.form(0x14, 0x3b, true);
    w.changed(0x0001_0000);
    assert_eq!(lookup_form(&w.e, 0x14), form);
    assert_eq!(lookup_form(&w.e, 0x24), 0);
    assert!(has_change_data(&w.e, 0x0001_0000));
    assert!(!has_change_data(&w.e, 0x14));
}

/// `JPSV`: an owner with a script keeps its variables but a temporary one; a variable of a file
/// gone goes; a value that holds a form's id is resolved; an owner not loaded is skipped by 9
/// bytes past each name (JIP), which reads the next owner wrong, or by 10 (JohnnyGuitar). Saved
/// in the map's order.
#[test]
fn script_variables() {
    let record = |w: &mut World| {
        w.scripted(0x0100_0abc);
        w.scripted(0x0100_0def);
        w.file(2, 3);
        w.file(4, 0xff);
        Out::default()
            .u16(3)
            .u32(0x0100_0abc)
            .u16(4)
            .u8(1)
            .name(b"Foo")
            .u64(2.5f64.to_bits())
            .u8(1)
            .name(b"*tmp")
            .u64(1)
            .u8(2)
            .name(b"Bar")
            .u64(0x0200_0123)
            .u8(4)
            .name(b"Gone")
            .u64(3)
            // Not loaded: its two variables skipped.
            .u32(0x0100_0999)
            .u16(2)
            .u8(1)
            .name(b"A")
            .u64(0)
            .u8(1)
            .name(b"B")
            .u64(0)
            .u32(0x0100_0def)
            .u16(1)
            .u8(1)
            .name(b"Cee")
            .u64(7)
            .0
    };
    let mut w = World::new();
    let data = record(&mut w);
    set_johnny_guitar_fixes(&mut w.e, true);
    load_file(&mut w.e, &cosave(&[chunk(kind::SCRIPT_VARS, 9, data.clone())]));
    // Buckets of 8: 0x01000DEF hashes to 3, 0x01000ABC to 4.
    assert_eq!(mixed_hash(0x0100_0def) & 7, 3);
    assert_eq!(mixed_hash(0x0100_0abc) & 7, 4);
    let expect = Out::default()
        .u16(2)
        .u32(0x0100_0def)
        .u16(1)
        .u8(1)
        .name(b"Cee")
        .u64(7)
        .u32(0x0100_0abc)
        .u16(2);
    // The owner's names in a map of 4 buckets: by bucket, the newer first in one.
    let mut names = [
        (0, b"Foo".to_vec(), 1u8, 2.5f64.to_bits()),
        (1, b"Bar".to_vec(), 3, 0x0300_0123),
    ];
    names.sort_by_key(|(i, n, _, _)| (str_hash_ci_bucket(n), std::cmp::Reverse(*i)));
    let expect = names
        .iter()
        .fold(expect, |o, (_, n, m, v)| o.u8(*m).name(n).u64(*v));
    assert_eq!(written(&mut w.e), [chunk(kind::SCRIPT_VARS, 9, expect.0)]);
    // JIP's own skip, one byte short: the third owner is read from the wrong place.
    let mut w = World::new();
    let data = record(&mut w);
    load_file(&mut w.e, &cosave(&[chunk(kind::SCRIPT_VARS, 9, data)]));
    assert!(s(&mut w.e).script_vars.get(&0x0100_0def).is_none());
    assert!(s(&mut w.e).script_vars.get(&0x0100_0abc).is_some());
}

/// The bucket a name falls in, in a map of 4 buckets, with the order inside it (newest first:
/// the second inserted before the first when they share one).
fn str_hash_ci_bucket(name: &[u8]) -> u32 {
    crate::containers::str_hash_ci(name) & 3
}

/// `JPAV`: mods from 6 up and loaded, owners loaded or changed in the save; values of each type,
/// a form's resolved; an empty name drops its owner's rest. Written in JIP's map order: mod 7
/// (bucket 3 of 8) before mod 6 (bucket 6).
#[test]
fn aux_variables() {
    let mut w = World::new();
    w.form(0x14, 0x3b, true);
    w.changed(0x0700_0001);
    w.file(9, 0xff);
    w.file(8, 7);
    w.file(0x10, 0x11);
    let text = |t: &[u8]| Out::default().u8(4).u16(t.len() as u16).bytes(t).0;
    let data = Out::default()
        .u16(4)
        // Mod 6: the player, one variable of three values.
        .u8(6)
        .u16(1)
        .u32(0x14)
        .u16(1)
        .name(b"Count")
        .u16(3)
        .u8(1)
        .u64(4.0f64.to_bits())
        .u8(2)
        .u32(0x1000_0005)
        .bytes(&text(b"hi"))
        // Mod 5 and below: never read.
        .u8(5)
        .u16(1)
        .u32(0x14)
        .u16(1)
        .name(b"Low")
        .u16(1)
        .u8(1)
        .u64(0)
        // Mod 9: not loaded.
        .u8(9)
        .u16(1)
        .u32(0x14)
        .u16(1)
        .name(b"X")
        .u16(0)
        // Mod 8 (loaded as 7): an owner changed in the save, and one gone; then an empty name
        // ends the first owner's variables after one.
        .u8(8)
        .u16(2)
        .u32(0x0700_0001)
        .u16(3)
        .name(b"Kept")
        .u16(1)
        .u8(1)
        .u64(1.0f64.to_bits())
        .u8(0)
        .u16(1)
        .u8(2)
        .u32(5)
        .name(b"Lost")
        .u16(0)
        .u32(0x0700_0099)
        .u16(1)
        .name(b"Y")
        .u16(1)
        .bytes(&text(b"gone"))
        .0;
    s(&mut w.e).changed = changed::ALL;
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, data)]));
    let expect = Out::default()
        .u16(2)
        .u8(7)
        .u16(1)
        .u32(0x0700_0001)
        .u16(1)
        .name(b"Kept")
        .u16(1)
        .u8(1)
        .u64(1.0f64.to_bits())
        .u8(6)
        .u16(1)
        .u32(0x14)
        .u16(1)
        .name(b"Count")
        .u16(3)
        .u8(1)
        .u64(4.0f64.to_bits())
        .u8(2)
        .u32(0x1100_0005)
        .bytes(&text(b"hi"))
        .0;
    assert_eq!(written(&mut w.e), [chunk(kind::AUX_VARS, 10, expect)]);
    // A record older than version 10, or a load that did not change the aux variables, is
    // not read; the data loaded before stays.
    let before = written(&mut w.e);
    s(&mut w.e).changed = changed::REF_MAPS;
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, Vec::new())]));
    assert_eq!(written(&mut w.e), before, "not changed: kept, not read");
    s(&mut w.e).changed = changed::AUX_VARS;
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 9, Vec::new())]));
    assert!(written(&mut w.e).is_empty(), "cleared, the old version not read");
}

/// `JPRM`: by mod, name and reference; a reference neither loaded nor changed goes; written in
/// the maps' order.
#[test]
fn ref_maps() {
    let mut w = World::new();
    w.form(0x0600_0001, 0x3a, true);
    w.form(0x0600_0002, 0x3a, true);
    let data = Out::default()
        .u16(1)
        .u8(6)
        .u16(1)
        .name(b"Map")
        .u16(3)
        .u32(0x0600_0001)
        .u8(1)
        .u64(0.5f64.to_bits())
        .u32(0x0600_0003)
        .u8(2)
        .u32(9)
        .u32(0x0600_0002)
        .u8(4)
        .u16(1)
        .bytes(b"z")
        .0;
    s(&mut w.e).changed = changed::REF_MAPS;
    load_file(&mut w.e, &cosave(&[chunk(kind::REF_MAPS, 10, data)]));
    // Ids in a map of 4 buckets (room for 3 references): 0x06000001 and 0x06000002 by hash.
    let (a, b) = (mixed_hash(0x0600_0001) & 3, mixed_hash(0x0600_0002) & 3);
    let one = Out::default().u32(0x0600_0001).u8(1).u64(0.5f64.to_bits()).0;
    let two = Out::default().u32(0x0600_0002).u8(4).u16(1).bytes(b"z").0;
    let refs = if a <= b {
        [one, two].concat()
    } else {
        [two, one].concat()
    };
    let expect = Out::default()
        .u16(1)
        .u8(6)
        .u16(1)
        .name(b"Map")
        .u16(2)
        .bytes(&refs)
        .0;
    assert_eq!(written(&mut w.e), [chunk(kind::REF_MAPS, 10, expect)]);
}

/// One mod's extra data as written: index, the head's size, the texts' sizes, the head, texts.
fn extra_mod(index: u8, used: u32, forms: u32, values: &[u32], texts: [&[u8]; 2]) -> Vec<u8> {
    let head = 8 + 4 * values.len() as u32;
    let mut o = Out::default()
        .u8(index)
        .u32(head)
        .u16(texts[0].len() as u16)
        .u16(texts[1].len() as u16)
        .u32(used)
        .u32(forms);
    for v in values {
        o = o.u32(*v);
    }
    o.bytes(texts[0]).bytes(texts[1]).0
}

/// `JPED`: a key's reference must have changes in the save; each loaded mod's data kept, a form
/// id of a file gone unused. JIP writes version 1 with 5 bytes counted for each mod's header,
/// JohnnyGuitar version 2 with 9; each reads only its own (JohnnyGuitar gives up on version 1).
#[test]
fn extra_data_in_both_formats() {
    let body = |per_mod: u32| {
        let m1 = extra_mod(6, 0b111, 0b010, &[0x4000_0000, 0x0300_0009, 5], [b"ab", b""]);
        let m2 = extra_mod(9, 0b1, 0, &[7], [b"", b"xyz"]);
        let size = 2 * per_mod + (8 + 12 + 2) + (8 + 4 + 3);
        Out::default()
            .u32(1)
            .u32(0x55)
            .u32(0x0700_0010)
            .u8(2)
            .u32(size)
            .bytes(&m1)
            .bytes(&m2)
            .0
    };
    for (jg, version, per_mod) in [(false, 1, 5), (true, 2, 9)] {
        let mut w = World::new();
        w.changed(0x0700_0010);
        set_johnny_guitar_fixes(&mut w.e, jg);
        s(&mut w.e).changed = changed::ALL;
        load_file(
            &mut w.e,
            &cosave(&[chunk(kind::EXTRA_DATA, version, body(per_mod))]),
        );
        assert_eq!(
            written(&mut w.e),
            [chunk(kind::EXTRA_DATA, version, body(per_mod))],
            "jg {jg}"
        );
        // The other format is not read.
        s(&mut w.e).changed = changed::ALL;
        let other = 3 - version;
        load_file(
            &mut w.e,
            &cosave(&[chunk(kind::EXTRA_DATA, other, body(per_mod))]),
        );
        assert!(written(&mut w.e).is_empty(), "jg {jg} reads no version {other}");
    }
    // A form id of a file no longer loaded: its index goes unused, its form bit with it.
    let mut w = World::new();
    w.changed(0x0700_0010);
    w.file(3, 0xff);
    s(&mut w.e).changed = changed::ALL;
    load_file(&mut w.e, &cosave(&[chunk(kind::EXTRA_DATA, 1, body(5))]));
    let entry = s(&mut w.e).extra_data.get(&Uint(0x55)).cloned().unwrap();
    let data = &entry.data[&6];
    assert_eq!((data.used, data.forms, data.values[1]), (0b101, 0, 0));
    assert_eq!(data.save_size(), 8 + 12);
}

/// A key whose reference has no changes in the save is skipped by its size: JIP's version 1
/// size is 4 bytes short for each mod, so the next key is read from the wrong place.
#[test]
fn a_skipped_extra_data_key() {
    let m = extra_mod(6, 1, 0, &[1], [b"", b""]);
    let key = |key: u32, id: u32, size: u32| {
        Out::default()
            .u32(key)
            .u32(id)
            .u8(1)
            .u32(size)
            .bytes(&m)
            .0
    };
    for (jg, version, per_mod, second_read) in [(false, 1, 5, false), (true, 2, 9, true)] {
        let mut w = World::new();
        w.changed(0x0700_0020);
        set_johnny_guitar_fixes(&mut w.e, jg);
        s(&mut w.e).changed = changed::ALL;
        let size = per_mod + 12;
        let data = Out::default()
            .u32(2)
            .bytes(&key(1, 0x0700_0010, size))
            .bytes(&key(2, 0x0700_0020, size))
            .0;
        load_file(&mut w.e, &cosave(&[chunk(kind::EXTRA_DATA, version, data)]));
        assert_eq!(
            s(&mut w.e).extra_data.get(&Uint(2)).is_some(),
            second_read,
            "jg {jg}"
        );
    }
}

/// `JPLR`: kept for two loaded references from a loaded file, not for a reference whose linked
/// reference extra data holds none; saved with the link's id.
#[test]
fn linked_references() {
    let mut w = World::new();
    let a = w.form(0x0600_0001, 0x3a, true);
    w.form(0x0600_0002, 0x3a, true);
    let c = w.form(0x0600_0003, 0x3a, true);
    w.form(0x0600_0004, 0x26, false);
    // `c` has an ExtraLinkedRef holding none.
    w.e.mem.set_u32(c + XLINKED, c + 0x60);
    w.e.mem.set_u32(c + 0x60 + 0xc, 0);
    let _ = a;
    let data = Out::default()
        .u16(4)
        .u32(0x0600_0001)
        .u32(0x0600_0002)
        .u8(6)
        .u32(0x0600_0003)
        .u32(0x0600_0002)
        .u8(6)
        .u32(0x0600_0002)
        .u32(0x0600_0004)
        .u8(6)
        .u32(0x0600_0002)
        .u32(0x0600_0001)
        .u8(0xfe)
        .0;
    w.file(0xfe, 0xff);
    s(&mut w.e).changed = changed::ALL;
    load_file(&mut w.e, &cosave(&[chunk(kind::LINKED_REFS, 9, data)]));
    let expect = Out::default().u16(1).u32(0x0600_0001).u32(0x0600_0002).u8(6).0;
    assert_eq!(written(&mut w.e), [chunk(kind::LINKED_REFS, 9, expect)]);
    assert_eq!(
        s(&mut w.e).linked_refs_temp.get(&0x0600_0001),
        Some(&0x0600_0002)
    );
}

/// `JPGF`: read whatever the changes; written on every save; a new game, and every load before
/// its records, set it back to no flags and a cap of 1.0.
#[test]
fn saved_flags() {
    let mut w = World::new();
    let data = Out::default().u32(1).u32(0.5f32.to_bits()).0;
    load_file(&mut w.e, &cosave(&[chunk(kind::SERIALIZED_VARS, 1, data.clone())]));
    assert_eq!(saved_chunks(&mut w.e), [chunk(kind::SERIALIZED_VARS, 1, data)]);
    listen(
        &mut w.e,
        &Message {
            kind: msg::NEW_GAME,
            data: 0,
            len: 0,
        },
    );
    let reset = Out::default().u32(0).u32(1.0f32.to_bits()).0;
    assert_eq!(saved_chunks(&mut w.e), [chunk(kind::SERIALIZED_VARS, 1, reset)]);
}

/// `JPUA`: the player's appearance undo, its forms resolved and a gone race, hair or eyes
/// replaced by the defaults (the female hair for flag 1), a gone head part left out; gone at the
/// next load's start.
#[test]
fn appearance_undo() {
    let mut w = World::new();
    w.form(0x0600_0001, 0x0c, false);
    w.form(DEFAULT_HAIR_FEMALE, 0x0a, false);
    w.form(0x0600_0003, 0x0b, false);
    w.form(0x0600_0004, 0x0d, false);
    let mut head = vec![0u8; 0x214];
    head[0] = 0x42;
    head[0x208] = 1;
    let data = Out::default()
        .bytes(&head)
        .u32(0x0600_0001)
        .u32(0x0600_0099)
        .u32(0x0600_0003)
        .u8(2)
        .u32(0x0600_0098)
        .u32(0x0600_0004)
        .0;
    load_file(&mut w.e, &cosave(&[chunk(kind::APPEARANCE_UNDO, 9, data)]));
    let expect = Out::default()
        .bytes(&head)
        .u32(0x0600_0001)
        .u32(DEFAULT_HAIR_FEMALE)
        .u32(0x0600_0003)
        .u8(1)
        .u32(0x0600_0004)
        .0;
    assert_eq!(written(&mut w.e), [chunk(kind::APPEARANCE_UNDO, 9, expect)]);
    listen(
        &mut w.e,
        &Message {
            kind: msg::PRE_LOAD_GAME,
            data: 0,
            len: 0,
        },
    );
    assert!(written(&mut w.e).is_empty());
}

/// `JPNP`: only with JIP's NPC perks patch: a loaded actor's perks, a rank at most the perk's
/// count, a form that is not a perk left out; by perk address.
#[test]
fn npc_perks() {
    let mut w = World::new();
    w.form(0x0600_0001, 0x2a, true);
    let perk = w.form(0x0600_0010, PERK, false);
    w.e.mem.set_u8(perk + PERK_RANKS, 2);
    w.form(0x0600_0011, 0x0e, false);
    let data = Out::default()
        .u16(1)
        .u32(0x0600_0001)
        .u8(2)
        .u32(0x0600_0010)
        .u8(5)
        .u32(0x0600_0011)
        .u8(1)
        .0;
    s(&mut w.e).changed = changed::ALL;
    load_file(&mut w.e, &cosave(&[chunk(kind::NPC_PERKS, 10, data.clone())]));
    assert!(written(&mut w.e).is_empty(), "no patch: not read");
    s(&mut w.e).npc_perks_patch = true;
    s(&mut w.e).changed = changed::ALL;
    load_file(&mut w.e, &cosave(&[chunk(kind::NPC_PERKS, 10, data)]));
    let expect = Out::default()
        .u16(1)
        .u32(0x0600_0001)
        .u8(1)
        .u32(0x0600_0010)
        .u8(2)
        .0;
    assert_eq!(written(&mut w.e), [chunk(kind::NPC_PERKS, 10, expect)]);
}

/// The messages: a save records its name and clears the changes; a load of another save marks
/// everything changed; a load of the same one keeps what is loaded; a load that succeeded
/// clears the changes; leaving to the main menu clears the data and the name.
#[test]
fn what_a_load_reads_again() {
    let mut w = World::new();
    w.form(0x14, 0x3b, true);
    let text = 0x0500_a000;
    let tell = |w: &mut World, kind: u32, name: &[u8]| {
        for (i, b) in name.iter().enumerate() {
            w.e.mem.set_u8(text + i as u32, *b);
        }
        listen(
            &mut w.e,
            &Message {
                kind,
                data: text,
                len: name.len() as u32,
            },
        );
    };
    let aux = Out::default()
        .u16(1)
        .u8(6)
        .u16(1)
        .u32(0x14)
        .u16(1)
        .name(b"V")
        .u16(1)
        .u8(1)
        .u64(1)
        .0;
    tell(&mut w, msg::PRE_LOAD_GAME, b"Save 1");
    assert_eq!((s(&mut w.e).changed, &s(&mut w.e).last_path[..]), (changed::ALL, &b"Save 1"[..]));
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, aux.clone())]));
    // `PostLoadGame`'s data is the success byte.
    listen(
        &mut w.e,
        &Message {
            kind: msg::POST_LOAD_GAME,
            data: 1,
            len: 1,
        },
    );
    assert_eq!(s(&mut w.e).changed, 0);
    assert_eq!(written(&mut w.e).len(), 1);
    // The same save again: nothing changed, the aux variables are not read again (an empty
    // record would clear them if it were).
    tell(&mut w, msg::PRE_LOAD_GAME, b"Save 1");
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, Vec::new())]));
    assert_eq!(written(&mut w.e).len(), 1);
    tell(&mut w, msg::SAVE_GAME, b"Save 2");
    assert_eq!((s(&mut w.e).changed, &s(&mut w.e).last_path[..]), (0, &b"Save 2"[..]));
    tell(&mut w, msg::PRE_LOAD_GAME, b"Save 1");
    assert_eq!(s(&mut w.e).changed, changed::ALL);
    tell(&mut w, msg::EXIT_TO_MAIN_MENU, b"");
    assert!(written(&mut w.e).is_empty());
    assert!(s(&mut w.e).last_path.is_empty());
}
