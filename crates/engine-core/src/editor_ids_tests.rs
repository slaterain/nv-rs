//! The editor id store, from JohnnyGuitar NVSE 5.28's `JohnnyExtraData` and
//! `EditorIDRestoration.cpp`.

use super::*;

/// A form's names: its first, then aliases (case-sensitive, as the game's string pool keeps
/// texts); the newest is its editor id; an empty name is refused.
#[test]
fn a_forms_names_newest_last() {
    let s = EditorIds::default();
    assert_eq!(s.add_name(0x100, b""), SetResult::Failure);
    assert_eq!(s.name(0x100), None);
    assert_eq!(s.add_name(0x100, b"DocMitchell"), SetResult::Success);
    assert_eq!(s.add_name(0x100, b"DocMitchell"), SetResult::AlreadyExists);
    assert_eq!(
        s.add_name(0x100, b"docmitchell"),
        SetResult::Success,
        "another text"
    );
    assert_eq!(s.name(0x100).as_deref(), Some(&b"docmitchell"[..]));
    assert_eq!(s.name_len(0x100), 11);
    s.remove_name(0x100, b"docmitchell");
    assert_eq!(s.name(0x100).as_deref(), Some(&b"DocMitchell"[..]));
    s.remove_name(0x100, b"DocMitchell");
    assert_eq!(s.name(0x100), None);
    assert_eq!(s.name_len(0x100), 0);
}

/// The map is case-blind (the game's `tolower`, ASCII only); a name taken by another form is
/// answered with the old one, then names the new one; the same form again changes nothing.
#[test]
fn the_map_is_case_blind_and_a_name_can_be_taken() {
    let s = EditorIds::default();
    s.add_name(0x14, b"PlayerRef");
    assert_eq!(s.map_insert(0x14, b"PlayerRef"), None);
    assert_eq!(s.lookup(b"playerref"), 0x14);
    assert_eq!(s.lookup(b"PLAYERREF"), 0x14);
    assert_eq!(s.lookup(b"PlayerRe"), 0);
    assert_eq!(s.lookup(b""), 0);
    assert_eq!(s.map_insert(0x14, b"PlayerRef"), Some(0x14));
    s.add_name(0x99, b"PLAYERREF");
    assert_eq!(s.map_insert(0x99, b"PLAYERREF"), Some(0x14));
    assert_eq!(s.lookup(b"PlayerRef"), 0x99);
    // Non-ASCII bytes are not folded.
    s.add_name(0x200, b"Caf\xc9");
    s.map_insert(0x200, b"Caf\xc9");
    assert_eq!(s.lookup(b"caf\xc9"), 0x200);
    assert_eq!(s.lookup(b"caf\xe9"), 0);
}

/// Detaching a form takes its names out of the map and keeps them on the form; forgetting it
/// drops both.
#[test]
fn detach_and_forget() {
    let s = EditorIds::default();
    for name in [&b"A"[..], b"B"] {
        s.add_name(0x10, name);
        s.map_insert(0x10, name);
    }
    s.detach(0x10, false);
    assert_eq!((s.lookup(b"a"), s.lookup(b"b")), (0, 0));
    assert_eq!(s.name(0x10).as_deref(), Some(&b"B"[..]));
    s.map_insert(0x10, b"B");
    s.forget(0x10, false);
    assert_eq!(s.lookup(b"b"), 0);
    assert_eq!(s.name(0x10), None);
}

/// Names whose hashes meet are told apart by their text (a second map holds the later one).
#[test]
fn names_in_one_hash_slot_stay_apart() {
    let s = EditorIds::default();
    // Two names are put in one slot by hand: the second goes to the clash map.
    let mut st = s.write();
    let a = st.push(b"Alpha");
    let b = st.push(b"Beta");
    let h = hash(b"Alpha");
    st.map.insert(h, (a, 1));
    st.clash.insert(folded(b"Beta"), 2);
    assert_eq!(st.map_get(b"ALPHA"), 1);
    assert_eq!(st.map_get(b"beta"), 0, "Beta's own hash slot is empty");
    // Beta in Alpha's slot (as if their hashes met).
    st.map.insert(h, (b, 2));
    st.clash.clear();
    st.clash.insert(folded(b"Alpha"), 1);
    // A lookup of Alpha finds Beta's text in its slot and goes on to the clash map.
    assert_eq!(st.map_get(b"alpha"), 1);
    assert!(st.map_remove(b"Alpha"));
    assert_eq!(st.map_get(b"alpha"), 0);
}

/// The counts for the load's log line: names stored, lookups, forms, entries and the arena.
#[test]
fn the_counts() {
    let s = EditorIds::default();
    s.add_name(1, b"One");
    s.map_insert(1, b"One");
    s.add_name(2, b"Two");
    s.lookup(b"one");
    s.count_conflict();
    let c = s.counts();
    assert_eq!(
        (
            c.named,
            c.lookups,
            c.conflicts,
            c.forms,
            c.entries,
            c.arena_bytes
        ),
        (2, 1, 1, 2, 1, 6)
    );
}

/// Real threads: lookups while another thread names forms, no name lost.
#[test]
fn lookups_while_another_thread_names_forms() {
    let s = std::sync::Arc::new(EditorIds::default());
    let writer = {
        let s = s.clone();
        std::thread::spawn(move || {
            for i in 0..20_000u32 {
                let name = format!("Form{i:05}");
                s.add_name(0x1000 + i, name.as_bytes());
                s.map_insert(0x1000 + i, name.as_bytes());
            }
        })
    };
    let readers: Vec<_> = (0..3)
        .map(|_| {
            let s = s.clone();
            std::thread::spawn(move || {
                let mut found = 0;
                for i in (0..20_000u32).step_by(7) {
                    let f = s.lookup(format!("form{i:05}").as_bytes());
                    assert!(f == 0 || f == 0x1000 + i);
                    found += u32::from(f != 0);
                }
                found
            })
        })
        .collect();
    writer.join().unwrap();
    for r in readers {
        r.join().unwrap();
    }
    for i in 0..20_000u32 {
        assert_eq!(s.lookup(format!("FORM{i:05}").as_bytes()), 0x1000 + i);
    }
}

/// The load's cost: two million forms named and two million lookups (the time printed for the
/// pull request; the counts checked). About a large load order's worth.
#[test]
fn two_million_names() {
    let s = EditorIds::default();
    let t = std::time::Instant::now();
    let mut name = Vec::with_capacity(24);
    for i in 0..2_000_000u32 {
        name.clear();
        name.extend_from_slice(b"SomeModForm");
        name.extend_from_slice(i.to_string().as_bytes());
        s.add_name(0x0100_0000 + i, &name);
        s.map_insert(0x0100_0000 + i, &name);
    }
    let named = t.elapsed();
    let t = std::time::Instant::now();
    let mut hits = 0u32;
    for i in 0..2_000_000u32 {
        name.clear();
        name.extend_from_slice(b"somemodform");
        name.extend_from_slice(i.to_string().as_bytes());
        hits += u32::from(s.lookup(&name) != 0);
    }
    let looked = t.elapsed();
    let c = s.counts();
    eprintln!(
        "editor ids: 2,000,000 named in {named:?}, 2,000,000 looked up in {looked:?}; {} arena bytes, {} entries",
        c.arena_bytes, c.entries
    );
    assert_eq!(hits, 2_000_000);
    assert_eq!((c.forms, c.entries), (2_000_000, 2_000_000));
}

/// Real threads: the loader's threads add names while the main thread looks them up, through
/// the one store all the run's engines share (its reader-writer lock). Every name added is found
/// afterwards, each lookup during the load answered either 0 (not yet there) or its own form.
#[test]
fn two_threads_name_and_look_up_at_once() {
    let s = std::sync::Arc::new(EditorIds::default());
    let writers: Vec<_> = (0..2u32)
        .map(|t| {
            let s = s.clone();
            std::thread::spawn(move || {
                for i in 0..20_000u32 {
                    let form = 0x1000_0000 + t * 0x10_0000 + i;
                    let name = format!("T{t}Form{i}");
                    assert_eq!(s.add_name(form, name.as_bytes()), SetResult::Success);
                    s.map_insert(form, name.as_bytes());
                }
            })
        })
        .collect();
    let reader = {
        let s = s.clone();
        std::thread::spawn(move || {
            for i in 0..20_000u32 {
                let got = s.lookup(format!("t1form{i}").as_bytes());
                assert!(got == 0 || got == 0x1010_0000 + i, "{i}: {got:08x}");
            }
        })
    };
    for w in writers {
        w.join().unwrap();
    }
    reader.join().unwrap();
    for t in 0..2u32 {
        for i in (0..20_000u32).step_by(101) {
            let name = format!("T{t}FORM{i}");
            assert_eq!(s.lookup(name.as_bytes()), 0x1000_0000 + t * 0x10_0000 + i);
        }
    }
    assert_eq!(s.counts().entries, 40_000);
}

/// nv-rs's own: the form table and the map sized once for the load's form total, not again for a
/// smaller or equal one, and still right after.
#[test]
fn the_tables_are_sized_for_the_load() {
    let s = EditorIds::default();
    s.add_name(1, b"Early");
    s.reserve_for_load(10_000);
    let (forms, map) = {
        let st = s.read();
        (st.forms.capacity(), st.map.capacity())
    };
    assert!(forms >= 10_000 && map >= 10_000, "{forms} {map}");
    s.reserve_for_load(10_000);
    s.reserve_for_load(5);
    assert_eq!(s.read().forms.capacity(), forms);
    assert_eq!(s.name(1).as_deref(), Some(&b"Early"[..]));
}

/// The setter sizes the tables from the load's form total (`0x011C3F24`) when it names a form.
#[test]
fn naming_a_form_sizes_the_tables_for_the_load() {
    let mut e = crate::Engine::new();
    if !e.mem.is_mapped(LOAD_FORM_TOTAL) {
        e.map(LOAD_FORM_TOTAL & !0xfff, 0x1000);
    }
    e.mem.set_u32(LOAD_FORM_TOTAL, 50_000);
    let form = e.mem.alloc(0x40);
    e.mem.set_u32(form + FORM_ID, 0x0100_0001);
    let text = e.mem.alloc(16);
    e.mem.write(text, b"DocMitchell\0");
    assert!(set_form_editor_id(&mut e, form, text));
    let st = store(&mut e).read();
    assert!(st.forms.capacity() >= 50_000, "{}", st.forms.capacity());
    assert!(st.map.capacity() >= 50_000, "{}", st.map.capacity());
}
