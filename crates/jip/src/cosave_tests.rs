//! Tests of JIP's co-save section: each record read into JIP's data and written back in JIP's
//! order, over a stand-in of the game's tables (the save's file table, the form map, the save's
//! changed forms, the player), and the messages that decide what a load reads.

use super::*;
use crate::test_support::{unhex, REAL_COSAVES};
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
/// The script and the event list every `scripted` form shares (the event list's variable list
/// at `+0x200`), and the heap stand-in's next block.
const SCRIPTS: u32 = 0x0700_0000;
const HEAP_TOP: u32 = SCRIPTS + 0xfff0;

struct World {
    e: Engine,
    next_form: u32,
    next_entry: u32,
}

/// The extra list's `GetByType` stub: a stand-in form keeps its extras at +0x80 and +0x84.
fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    let form = a[0] - TESObjectREFR::ExtraData.off;
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
        for page in [0x011d_d000, 0x011c_5000, 0x011d_e000, 0x011c_3000] {
            e.map(page, 0x1000);
        }
        e.map(0x0500_0000, 0x0030_0000);
        e.map(FORM_AT, 0x0010_0000);
        e.mem.set_u32(SAVE_LOAD_GAME, SL);
        e.mem.set_u32(SL, CHANGES);
        for i in 0..0xffu32 {
            e.mem.set_u8(SL + game::save_mod(i as u8), i as u8);
        }
        for (map, buckets) in [(CHANGES, 16u32), (FORMS, 16)] {
            e.mem.set_u32(map + 4, buckets);
            e.mem.set_u32(map + 8, map + 0x100);
        }
        e.mem.set_u32(ALL_FORMS, FORMS);
        e.mem.set_u32(PLAYER, PLAYER_REF);
        e.mem
            .set_u32(PLAYER_REF + TESObjectREFR::pObjectReference.off, PLAYER_NPC);
        e.mem.set_u32(REF_VT + VT_IS_REFERENCE, RETURN_TRUE);
        e.mem.set_u32(REF_VT + VT_IS_ACTOR, RETURN_TRUE);
        e.register(EXTRA_GET_BY_TYPE, get_by_type);
        // The scripts and event lists of `scripted`, and the game's heap (a bump stand-in) for
        // the variables a load makes in them (`Script::AddVariable`).
        e.map(SCRIPTS, 0x0001_0000);
        e.mem.set_u32(HEAP_TOP, SCRIPTS + 0x8000);
        e.register(0x00aa_3e40, |e, w| {
            let at = e.mem.u32(HEAP_TOP);
            e.mem.set_u32(HEAP_TOP, at + w[1].next_multiple_of(8));
            Ret {
                eax: at,
                ..Ret::default()
            }
        });
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
        self.e.mem.set_u8(at + TESForm::cFormType.off, kind);
        self.e.mem.set_u32(at + TESForm::iFormID.off, id);
        self.insert(FORMS, id, at);
        at
    }

    /// A reference with a script and an event list (`ExtraScript`).
    fn scripted(&mut self, id: u32) -> u32 {
        let form = self.form(id, 0x3a, true);
        let extra = form + 0x40;
        self.e.mem.set_u32(extra + 0xc, SCRIPTS);
        self.e.mem.set_u32(extra + 0x10, SCRIPTS + 0x100);
        self.e.mem.set_u32(SCRIPTS + 0x10c, SCRIPTS + 0x200);
        self.e.mem.set_u32(form + XSCRIPT, extra);
        form
    }

    fn changed(&mut self, id: u32) {
        self.insert(CHANGES, id, 1);
    }

    /// The save's file `saved` loaded as `now` today (0xFF: not loaded).
    fn file(&mut self, saved: u8, now: u8) {
        self.e.mem.set_u8(SL + game::save_mod(saved), now);
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
    load_file(
        &mut w.e,
        &cosave(&[chunk(kind::SCRIPT_VARS, 9, data.clone())]),
    );
    // The order worked out by hand from JIP's C++ (`HashKey`: `(k * 13) ^ (k >> 15)`; `StrHashCI`,
    // seed 0xD39CA509): in the owners' map of 8 buckets 0x01000DEF hashes to 0x0D00B723 (bucket
    // 3) and 0x01000ABC to 0x0D00898C (bucket 4); in the names' map of 4, "Foo" (0x8DE884B7) and
    // "Bar" (0x8B7E7C77) share bucket 3, the newer first.
    let expect = Out::default()
        .u16(2)
        .u32(0x0100_0def)
        .u16(1)
        .u8(1)
        .name(b"Cee")
        .u64(7)
        .u32(0x0100_0abc)
        .u16(2)
        .u8(3)
        .name(b"Bar")
        .u64(0x0300_0123)
        .u8(1)
        .name(b"Foo")
        .u64(2.5f64.to_bits());
    assert_eq!(written(&mut w.e), [chunk(kind::SCRIPT_VARS, 9, expect.0)]);
    // JIP answering "not installed": no script was changed (only kept, written back).
    assert_eq!(script_items(&w.e), (vec![], vec![]));
    // JIP's own skip, one byte short: the third owner is read from the wrong place.
    let mut w = World::new();
    let data = record(&mut w);
    load_file(&mut w.e, &cosave(&[chunk(kind::SCRIPT_VARS, 9, data)]));
    assert!(s(&mut w.e).script_vars.get(&0x0100_0def).is_none());
    assert!(s(&mut w.e).script_vars.get(&0x0100_0abc).is_some());
}

/// The script's variable list (`+0x4C`) and the event list's (`+0xC`): each item's id (a
/// `VariableInfo`'s name too), from the first node.
type ScriptItems = (Vec<(u32, Vec<u8>)>, Vec<(u32, u64)>);
fn script_items(e: &Engine) -> ScriptItems {
    let walk = |mut node: u32| {
        let mut items = Vec::new();
        while node != 0 {
            if e.mem.u32(node) != 0 {
                items.push(e.mem.u32(node));
            }
            node = e.mem.u32(node + 4);
        }
        items
    };
    let infos = walk(SCRIPTS + 0x4c)
        .into_iter()
        .map(|i| {
            (
                e.mem.u32(i),
                nvse::table::guest_text(e, e.mem.u32(i + 0x18)),
            )
        })
        .collect();
    let vars = walk(e.mem.u32(SCRIPTS + 0x10c))
        .into_iter()
        .map(|v| (e.mem.u32(v), e.mem.u64(v + 8)))
        .collect();
    (infos, vars)
}

/// `JPSV`'s application (step 3.3): each variable of a loaded owner is made in the script (a new
/// `VariableInfo`, the script's next id, first in its list) and in the event list (a
/// `ScriptVar`) with its value (a form's id resolved); a temporary one too, but not kept; a save
/// then writes the values the variables hold.
#[test]
fn script_variables_are_made_in_the_script_and_saved_live() {
    let mut w = World::new();
    w.e.ext::<Jip>().answers_installed = true;
    w.scripted(0x0100_0abc);
    w.file(2, 3);
    w.e.mem.set_u32(SCRIPTS + 0x24, 6);
    let data = Out::default()
        .u16(1)
        .u32(0x0100_0abc)
        .u16(3)
        .u8(1)
        .name(b"Foo")
        .u64(2.5f64.to_bits())
        .u8(1)
        .name(b"*tmp")
        .u64(1)
        .u8(2)
        .name(b"Bar")
        .u64(0x0200_0123)
        .0;
    load_file(&mut w.e, &cosave(&[chunk(kind::SCRIPT_VARS, 9, data)]));
    let (infos, vars) = script_items(&w.e);
    assert_eq!(
        infos,
        [
            (9, b"Bar".to_vec()),
            (8, b"*tmp".to_vec()),
            (7, b"Foo".to_vec())
        ]
    );
    assert_eq!(w.e.mem.u32(SCRIPTS + 0x24), 9);
    assert_eq!(vars, [(9, 0x0300_0123), (8, 1), (7, 2.5f64.to_bits())]);
    let owner = s(&mut w.e).script_vars.get(&0x0100_0abc).unwrap().clone();
    assert_eq!(owner.len(), 2, "the temporary one not kept");
    // The variables change; the save writes them as they are.
    let first_var = w.e.mem.u32(w.e.mem.u32(SCRIPTS + 0x10c));
    assert_eq!(w.e.mem.u32(first_var), 9);
    w.e.mem.set_f64(first_var + 8, -4.0);
    let expect = Out::default()
        .u16(1)
        .u32(0x0100_0abc)
        .u16(2)
        .u8(3)
        .name(b"Bar")
        .u64((-4.0f64).to_bits())
        .u8(1)
        .name(b"Foo")
        .u64(2.5f64.to_bits());
    assert_eq!(written(&mut w.e), [chunk(kind::SCRIPT_VARS, 9, expect.0)]);
}

/// A name the script already has: made only when JIP added it to the script this session
/// (`GetVariableAdded`), else the variable is neither given its value nor kept.
#[test]
fn script_variables_the_script_has_need_jip_to_have_added_them() {
    let load_foo = |added: bool| {
        let mut w = World::new();
        w.e.ext::<Jip>().answers_installed = true;
        w.scripted(0x0100_0abc);
        // The script's own "foo" (id 3) and its event list's variable.
        let (info, var) = (SCRIPTS + 0x400, SCRIPTS + 0x500);
        w.e.mem.set_u32(info, 3);
        w.e.mem.set_u32(info + 0x18, SCRIPTS + 0x600);
        w.e.mem.write(SCRIPTS + 0x600, b"foo\0");
        w.e.mem.set_u32(SCRIPTS + 0x4c, info);
        w.e.mem.set_u32(var, 3);
        w.e.mem.set_f64(var + 8, 11.0);
        w.e.mem.set_u32(SCRIPTS + 0x200, var);
        if added {
            w.e.ext::<Jip>()
                .script_vars
                .added
                .emplace(0)
                .emplace(Name(b"FOO".to_vec()));
        }
        let data = Out::default()
            .u16(1)
            .u32(0x0100_0abc)
            .u16(1)
            .u8(1)
            .name(b"Foo")
            .u64(2.5f64.to_bits())
            .0;
        load_file(&mut w.e, &cosave(&[chunk(kind::SCRIPT_VARS, 9, data)]));
        let kept = s(&mut w.e).script_vars.get(&0x0100_0abc).is_some();
        (kept, w.e.mem.f64(var + 8), script_items(&w.e).0.len())
    };
    assert_eq!(load_foo(false), (false, 11.0, 1));
    assert_eq!(load_foo(true), (true, 2.5, 1));
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
    crate::cosave::set_changed(&mut w.e, changed::ALL);
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
    // A load that did not change the aux variables does not read them; the data loaded before
    // stays. A record older than version 10 is not read either (the data is cleared) and is
    // carried as it was.
    let before = written(&mut w.e);
    crate::cosave::set_changed(&mut w.e, changed::REF_MAPS);
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, Vec::new())]));
    assert_eq!(written(&mut w.e), before, "not changed: kept, not read");
    crate::cosave::set_changed(&mut w.e, changed::AUX_VARS);
    let old = Out::default().u16(0).u8(0xab).0;
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 9, old.clone())]));
    assert_eq!(
        written(&mut w.e),
        [chunk(kind::AUX_VARS, 9, old)],
        "cleared, the old version carried, not read"
    );
    assert!(crate::commands::jip_fn_aux_variable::aux_maps(&mut w.e, false).is_empty());
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
    crate::cosave::set_changed(&mut w.e, changed::REF_MAPS);
    load_file(&mut w.e, &cosave(&[chunk(kind::REF_MAPS, 10, data)]));
    // Ids in a map of 4 buckets (room for 3 references), worked out by hand from `HashKey`:
    // 0x06000001 hashes to 0x4E000C0D (bucket 1), 0x06000002 to 0x4E000C1A (bucket 2).
    let expect = Out::default()
        .u16(1)
        .u8(6)
        .u16(1)
        .name(b"Map")
        .u16(2)
        .u32(0x0600_0001)
        .u8(1)
        .u64(0.5f64.to_bits())
        .u32(0x0600_0002)
        .u8(4)
        .u16(1)
        .bytes(b"z")
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
        let m1 = extra_mod(
            6,
            0b111,
            0b010,
            &[0x4000_0000, 0x0300_0009, 5],
            [b"ab", b""],
        );
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
        crate::cosave::set_changed(&mut w.e, changed::ALL);
        load_file(
            &mut w.e,
            &cosave(&[chunk(kind::EXTRA_DATA, version, body(per_mod))]),
        );
        assert_eq!(
            written(&mut w.e),
            [chunk(kind::EXTRA_DATA, version, body(per_mod))],
            "jg {jg}"
        );
        // The other format is not read, and is carried as it was.
        crate::cosave::set_changed(&mut w.e, changed::ALL);
        let other = 3 - version;
        load_file(
            &mut w.e,
            &cosave(&[chunk(kind::EXTRA_DATA, other, body(per_mod))]),
        );
        assert!(crate::cosave::extra_data(&mut w.e).is_empty(), "jg {jg}");
        assert_eq!(
            written(&mut w.e),
            [chunk(kind::EXTRA_DATA, other, body(per_mod))],
            "jg {jg} reads no version {other}, carries it"
        );
    }
    // A form id of a file no longer loaded: its index goes unused, its form bit with it.
    let mut w = World::new();
    w.changed(0x0700_0010);
    w.file(3, 0xff);
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &cosave(&[chunk(kind::EXTRA_DATA, 1, body(5))]));
    let entry = crate::cosave::extra_data(&mut w.e)
        .get(&Uint(0x55))
        .cloned()
        .unwrap();
    let data = &entry.data[&6];
    assert_eq!((data.used, data.forms, data.values[1]), (0b101, 0, 0));
    assert_eq!(data.save_size(), 8 + 12);
}

/// A key whose reference has no changes in the save is skipped by its size: JIP's version 1
/// size is 4 bytes short for each mod, so the next key is read from the wrong place.
#[test]
fn a_skipped_extra_data_key() {
    let m = extra_mod(6, 1, 0, &[1], [b"", b""]);
    let key =
        |key: u32, id: u32, size: u32| Out::default().u32(key).u32(id).u8(1).u32(size).bytes(&m).0;
    for (jg, version, per_mod, second_read) in [(false, 1, 5, false), (true, 2, 9, true)] {
        let mut w = World::new();
        w.changed(0x0700_0020);
        set_johnny_guitar_fixes(&mut w.e, jg);
        crate::cosave::set_changed(&mut w.e, changed::ALL);
        let size = per_mod + 12;
        let data = Out::default()
            .u32(2)
            .bytes(&key(1, 0x0700_0010, size))
            .bytes(&key(2, 0x0700_0020, size))
            .0;
        load_file(&mut w.e, &cosave(&[chunk(kind::EXTRA_DATA, version, data)]));
        assert_eq!(
            crate::cosave::extra_data(&mut w.e).get(&Uint(2)).is_some(),
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
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &cosave(&[chunk(kind::LINKED_REFS, 9, data)]));
    let expect = Out::default()
        .u16(1)
        .u32(0x0600_0001)
        .u32(0x0600_0002)
        .u8(6)
        .0;
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
    load_file(
        &mut w.e,
        &cosave(&[chunk(kind::SERIALIZED_VARS, 1, data.clone())]),
    );
    assert_eq!(
        saved_chunks(&mut w.e),
        [chunk(kind::SERIALIZED_VARS, 1, data)]
    );
    listen(
        &mut w.e,
        &Message {
            kind: msg::NEW_GAME,
            data: 0,
            len: 0,
        },
    );
    // A new game holds nothing: while JIP is not installed it writes no section (xNVSE writes
    // none for a plugin that opens no record); installed, `JPGF` as reset, as JIP writes on every
    // save.
    assert!(saved_chunks(&mut w.e).is_empty(), "not installed");
    w.e.ext::<Jip>().answers_installed = true;
    let reset = Out::default().u32(0).u32(1.0f32.to_bits()).0;
    assert_eq!(
        saved_chunks(&mut w.e),
        [chunk(kind::SERIALIZED_VARS, 1, reset)]
    );
    // A record of only the flags reads over the reset values: the cap stays 1.0.
    load_file(
        &mut w.e,
        &cosave(&[chunk(kind::SERIALIZED_VARS, 1, 1u32.to_le_bytes().to_vec())]),
    );
    let short = Out::default().u32(1).u32(1.0f32.to_bits()).0;
    assert_eq!(
        saved_chunks(&mut w.e),
        [chunk(kind::SERIALIZED_VARS, 1, short)]
    );
}

/// The lock-free copy of the flags conditions read (`jip_fn_miscellaneous`) follows each change:
/// a load's record, the reset at a load's start, a new game, the main menu.
#[test]
fn the_conditions_copy_of_the_flags_follows_every_change() {
    use crate::commands::jip_fn_miscellaneous::serialized_flags;
    let mut w = World::new();
    let one = || cosave(&[chunk(kind::SERIALIZED_VARS, 1, 1u32.to_le_bytes().to_vec())]);
    let to = |e: &mut Engine, kind: u32| {
        listen(
            e,
            &Message {
                kind,
                data: 0,
                len: 0,
            },
        )
    };
    load_file(&mut w.e, &one());
    assert_eq!(serialized_flags(&mut w.e), 1, "a load's record");
    // A record of no bytes reads nothing over the load's reset.
    load_file(
        &mut w.e,
        &cosave(&[chunk(kind::SERIALIZED_VARS, 1, Vec::new())]),
    );
    assert_eq!(serialized_flags(&mut w.e), 0, "the load's reset");
    load_file(&mut w.e, &one());
    to(&mut w.e, msg::NEW_GAME);
    assert_eq!(serialized_flags(&mut w.e), 0, "a new game");
    load_file(&mut w.e, &one());
    to(&mut w.e, msg::EXIT_TO_MAIN_MENU);
    assert_eq!(serialized_flags(&mut w.e), 0, "the main menu");
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
    w.e.mem.set_u8(perk + BGSPerk::cNumRanks.off, 2);
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
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(
        &mut w.e,
        &cosave(&[chunk(kind::NPC_PERKS, 10, data.clone())]),
    );
    assert!(s(&mut w.e).npc_perks.is_empty(), "no patch: not read");
    assert_eq!(
        written(&mut w.e),
        [chunk(kind::NPC_PERKS, 10, data.clone())],
        "no patch: carried as it was"
    );
    set_npc_perks_patch(&mut w.e, true);
    crate::cosave::set_changed(&mut w.e, changed::ALL);
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
    assert_eq!(
        (
            crate::cosave::changed_flags(&mut w.e),
            &s(&mut w.e).last_path[..]
        ),
        (changed::ALL, &b"Save 1"[..])
    );
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
    assert_eq!(crate::cosave::changed_flags(&mut w.e), 0);
    assert_eq!(written(&mut w.e).len(), 1);
    // The same save again: nothing changed, the aux variables are not read again (an empty
    // record would clear them if it were).
    tell(&mut w, msg::PRE_LOAD_GAME, b"Save 1");
    load_file(&mut w.e, &cosave(&[chunk(kind::AUX_VARS, 10, Vec::new())]));
    assert_eq!(written(&mut w.e).len(), 1);
    tell(&mut w, msg::SAVE_GAME, b"Save 2");
    assert_eq!(
        (
            crate::cosave::changed_flags(&mut w.e),
            &s(&mut w.e).last_path[..]
        ),
        (0, &b"Save 2"[..])
    );
    tell(&mut w, msg::PRE_LOAD_GAME, b"Save 1");
    assert_eq!(crate::cosave::changed_flags(&mut w.e), changed::ALL);
    tell(&mut w, msg::EXIT_TO_MAIN_MENU, b"");
    assert!(written(&mut w.e).is_empty());
    assert!(s(&mut w.e).last_path.is_empty());
}

/// The real co-saves round trip byte for byte: JIP's section read by JIP's load callback and
/// written again by its save callback (after the `PreLoadGame` and `PostLoadGame` messages, as
/// a load from the menu sends them), xNVSE's own section passed through as it was read, the whole
/// file written again equal to the real one; with JohnnyGuitar's row on and off.
#[test]
fn real_co_saves_round_trip() {
    for (name, lines) in REAL_COSAVES {
        let file = unhex(lines);
        for jg in [false, true] {
            let mut w = World::new();
            set_johnny_guitar_fixes(&mut w.e, jg);
            listen(
                &mut w.e,
                &Message {
                    kind: msg::PRE_LOAD_GAME,
                    data: 0,
                    len: 0,
                },
            );
            let mut r = Reader::open(&file).unwrap();
            let mut sections = Vec::new();
            let mut jip_bytes = Vec::new();
            while let Some(p) = r.next_plugin().unwrap() {
                let start = r.offset();
                let chunks = if p.opcode_base == 0x2200 {
                    jip_bytes = file[start..start + p.length as usize].to_vec();
                    load(&mut w.e, &mut r).unwrap();
                    saved_chunks(&mut w.e)
                } else {
                    let mut chunks = Vec::new();
                    while let Some((kind, version, len)) = r.next_record().unwrap() {
                        let data = r.read_data(len).unwrap();
                        chunks.push(chunk(kind, version, data));
                    }
                    chunks
                };
                r.end_plugin();
                sections.push(nvse::cosave::Plugin {
                    opcode_base: p.opcode_base,
                    chunks,
                });
            }
            listen(
                &mut w.e,
                &Message {
                    kind: msg::POST_LOAD_GAME,
                    data: 1,
                    len: 1,
                },
            );
            let jip = &sections.last().unwrap().chunks;
            assert_eq!(section(jip), jip_bytes, "{name} jg {jg}: JIP's section");
            assert_eq!(
                nvse::cosave::write(&sections),
                file,
                "{name} jg {jg}: the whole co-save"
            );
        }
    }
}

/// What nv-rs does not read is carried byte for byte: an aux variables record of an old version,
/// the saved flags' bytes past their 8, the NPC perks without the patch, a `PJ` type JIP does not
/// know and a type of another kind come back in the same order, the section identical; again
/// after the same save is loaded a second time with nothing changed. A new game drops them.
#[test]
fn records_nv_rs_does_not_read_are_carried() {
    let jpgf = Out::default()
        .u32(1)
        .u32(0.5f32.to_bits())
        .u32(0xdead_beef)
        .0;
    let chunks = [
        chunk(kind::AUX_VARS, 9, Out::default().u16(1).u8(7).u16(0).0),
        chunk(kind::SERIALIZED_VARS, 1, jpgf),
        chunk(
            kind::NPC_PERKS,
            10,
            Out::default().u16(1).u32(0x0600_0001).u8(0).0,
        ),
        chunk(tag(b"ZZPJ"), 3, b"unknown".to_vec()),
        chunk(u32::from_le_bytes(*b"ABCD"), 1, vec![1, 2, 3]),
    ];
    let mut w = World::new();
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &cosave(&chunks));
    assert_eq!(saved_chunks(&mut w.e), chunks);
    assert_eq!(section(&saved_chunks(&mut w.e)), section(&chunks));
    // The same save again, nothing changed.
    crate::cosave::set_changed(&mut w.e, 0);
    load_file(&mut w.e, &cosave(&chunks));
    assert_eq!(saved_chunks(&mut w.e), chunks, "loaded again");
    listen(
        &mut w.e,
        &Message {
            kind: msg::NEW_GAME,
            data: 0,
            len: 0,
        },
    );
    assert!(saved_chunks(&mut w.e).is_empty(), "a new game");
}

/// While JIP is not installed, a co-save without JIP's section (its load callback called with no
/// records, as xNVSE calls it) leaves nothing to save, so the next co-save has no section of
/// JIP's either, as the real game writes without the DLL; one with a section is written back.
#[test]
fn no_section_in_no_section_out() {
    let mut w = World::new();
    assert!(saved_chunks(&mut w.e).is_empty(), "nothing loaded");
    let file = nvse::cosave::write(&[nvse::cosave::Plugin {
        opcode_base: 0x1400,
        chunks: vec![chunk(tag(b"SDOM"), 0, vec![0])],
    }]);
    let mut r = Reader::open(&file).unwrap();
    while let Some(p) = r.next_plugin().unwrap() {
        r.skip(p.length).unwrap();
        r.end_plugin();
    }
    r.no_records();
    load(&mut w.e, &mut r).unwrap();
    assert!(saved_chunks(&mut w.e).is_empty(), "no section loaded");
    let flags = chunk(
        kind::SERIALIZED_VARS,
        1,
        Out::default().u32(0).u32(1.0f32.to_bits()).0,
    );
    load_file(&mut w.e, &cosave(std::slice::from_ref(&flags)));
    assert_eq!(saved_chunks(&mut w.e), [flags], "a section loaded");
}

/// A `JPED` entry whose bit words name indexes past the 18 values (damaged or edited data), with
/// the largest head JIP takes (0x50 bytes): loads without a read past the values, the bits past
/// them cleared, and saves with a head size that matches the bytes written.
#[test]
fn extra_data_bits_past_the_values() {
    let mut head = Out::default()
        .u32((1 << 20) | 1)
        .u32(1 << 20)
        .u32(0x1111_1111);
    for i in 1..18u32 {
        head = head.u32(0x2000_0000 + i);
    }
    let head = head.0;
    assert_eq!(head.len(), 0x50);
    let data = Out::default()
        .u32(1)
        .u32(0x55)
        .u32(0x0700_0010)
        .u8(1)
        .u32(5 + 0x50)
        .u8(6)
        .u32(0x50)
        .u16(0)
        .u16(0)
        .bytes(&head)
        .0;
    let mut w = World::new();
    w.changed(0x0700_0010);
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &cosave(&[chunk(kind::EXTRA_DATA, 1, data)]));
    // Index 0 alone is left: a head of 12 bytes (the two words and one value), the key's size
    // as JIP counts it (5 for the mod's header, which is 9 bytes) 5 + 12.
    let expect = Out::default()
        .u32(1)
        .u32(0x55)
        .u32(0x0700_0010)
        .u8(1)
        .u32(5 + 12)
        .u8(6)
        .u32(12)
        .u16(0)
        .u16(0)
        .u32(1)
        .u32(0)
        .u32(0x1111_1111)
        .0;
    assert_eq!(written(&mut w.e), [chunk(kind::EXTRA_DATA, 1, expect)]);
}

/// Counts and lengths from the file never index or allocate past the bytes there: an absurd key
/// count with three bytes behind it, an absurd owner count, a record whose length runs far past
/// the file and a record cut short all load without a panic, keeping what was read before.
#[test]
fn damaged_records_end_the_read() {
    let mut w = World::new();
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    let absurd = [
        chunk(
            kind::EXTRA_DATA,
            1,
            Out::default().u32(u32::MAX).bytes(&[1, 2, 3]).0,
        ),
        chunk(kind::SCRIPT_VARS, 9, Out::default().u16(u16::MAX).0),
        chunk(kind::NPC_PERKS, 10, Out::default().u16(u16::MAX).u32(5).0),
    ];
    load_file(&mut w.e, &cosave(&absurd));
    assert!(crate::cosave::extra_data(&mut w.e).is_empty());
    assert!(s(&mut w.e).script_vars.is_empty());
    // A record after `JPGF` whose length says 0x7FFFFFF0 bytes.
    let flags = chunk(
        kind::SERIALIZED_VARS,
        1,
        Out::default().u32(1).u32(0.5f32.to_bits()).0,
    );
    let mut file = cosave(&[flags.clone(), chunk(kind::AUX_VARS, 10, vec![0; 4])]);
    let at = file.len() - 4 - 4;
    file[at..at + 4].copy_from_slice(&0x7fff_fff0u32.to_le_bytes());
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &file);
    assert_eq!(
        saved_chunks(&mut w.e),
        std::slice::from_ref(&flags),
        "oversized"
    );
    // The same file cut in the middle of its second record.
    let full = cosave(&[flags.clone(), chunk(kind::AUX_VARS, 10, vec![0; 8])]);
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &full[..full.len() - 3]);
    assert_eq!(saved_chunks(&mut w.e), [flags], "truncated");
}

/// `JPLR` while JIP answers "installed": JIP's `SetLinkedRefID` sets each link (its old link
/// noted as the default) and the temporary map notes it; a reference whose extra holds none is
/// refused and not noted. After the load, with the linked references changed, a default the
/// load did not set again comes back; the main menu gives every default back.
#[test]
fn linked_references_set_when_installed() {
    let mut w = World::new();
    w.e.ext::<Jip>().answers_installed = true;
    let a = w.form(0x0600_0001, 0x3a, true);
    let b = w.form(0x0600_0002, 0x3a, true);
    let c = w.form(0x0600_0003, 0x3a, true);
    let d = w.form(0x0600_0004, 0x3a, true);
    // `a` and `d` link to `c`; `c` has an ExtraLinkedRef holding none.
    for r in [a, c, d] {
        w.e.mem.set_u32(r + XLINKED, r + 0x60);
    }
    w.e.mem.set_u32(a + 0x60 + 0xc, c);
    w.e.mem.set_u32(c + 0x60 + 0xc, 0);
    w.e.mem.set_u32(d + 0x60 + 0xc, c);
    let data = Out::default()
        .u16(2)
        .u32(0x0600_0001)
        .u32(0x0600_0002)
        .u8(6)
        .u32(0x0600_0003)
        .u32(0x0600_0002)
        .u8(6)
        .0;
    crate::cosave::set_changed(&mut w.e, changed::ALL);
    load_file(&mut w.e, &cosave(&[chunk(kind::LINKED_REFS, 9, data)]));
    assert_eq!(w.e.mem.u32(a + 0x60 + 0xc), b);
    assert_eq!(w.e.mem.u32(c + 0x60 + 0xc), 0);
    let st = s(&mut w.e);
    assert_eq!(st.linked_refs_temp.get(&0x0600_0001), Some(&0x0600_0002));
    assert_eq!(st.linked_refs_temp.get(&0x0600_0003), None);
    assert_eq!(st.linked_ref_defaults.get(&0x0600_0001), Some(&0x0600_0003));
    assert_eq!(
        st.linked_refs.get(&0x0600_0001),
        Some(&LinkedRef {
            link_id: 0x0600_0002,
            mod_index: 6
        })
    );
    // A command changed `d` this session (its default `c`); the load did not set it again.
    assert!(crate::commands::jip_fn_linked_ref::set_linked_ref(
        &mut w.e, d, b, 6
    ));
    crate::cosave::set_changed(&mut w.e, changed::LINKED_REFS);
    let to = |e: &mut Engine, kind: u32, data: u32| listen(e, &Message { kind, data, len: 0 });
    to(&mut w.e, msg::POST_LOAD_GAME, 1);
    assert_eq!(w.e.mem.u32(d + 0x60 + 0xc), c, "restored after the load");
    assert_eq!(w.e.mem.u32(a + 0x60 + 0xc), b, "set by the load: kept");
    to(&mut w.e, msg::EXIT_TO_MAIN_MENU, 0);
    assert_eq!(
        w.e.mem.u32(a + 0x60 + 0xc),
        c,
        "the main menu gives it back"
    );
    assert!(s(&mut w.e).linked_ref_defaults.is_empty());
}

/// G33's part of the housekeeping: a load's start and leaving to the main menu clear the player's
/// killer (`+0xC0`) through the messages, as `DoPreLoadGameHousekeeping` does.
#[test]
fn a_load_and_the_main_menu_clear_the_players_killer() {
    let mut w = World::new();
    for kind in [msg::PRE_LOAD_GAME, msg::EXIT_TO_MAIN_MENU] {
        w.e.mem.set_u32(PLAYER_REF + 0xc0, 0x0500_8800);
        listen(
            &mut w.e,
            &Message {
                kind,
                data: 0,
                len: 0,
            },
        );
        assert_eq!(w.e.mem.u32(PLAYER_REF + 0xc0), 0, "{kind}");
    }
}
