//! JohnnyGuitar's editor ids on stand-in forms and vtables, every expectation from
//! `internal/JG/EditorIDRestoration.cpp` and `JohnnyExtraData` (JohnnyGuitar NVSE 5.28).

use super::*;
use std::cell::RefCell;

const AREA: u32 = 0x0600_0000;
/// The exe's vtables' pages (0x0101_0000 to 0x010A_0000) and the empty text's.
const VTABLES: u32 = 0x0101_0000;
const EMPTY_TEXT: u32 = 0x0101_1584;
/// A class whose getter is a real function (as the reference's, `0055D480`), and one JIP wrote
/// (the light's, `TESObjectLIGH` 0x1028EE4).
const REFR_VTABLE: u32 = 0x0102_f55c;
const LIGH_VTABLE: u32 = 0x0102_8ee4;
const REFR_GETTER: u32 = 0x0055_d480;
const JIP_LIGHT_SETTER: u32 = 0x7eff_5100;
/// The storing classes' own setters (test stand-ins; answer the form's word +0x40).
const OWN_SETTER: u32 = 0x00de_0100;
/// The exe's two functions JG replaces, as stand-ins (never reached once wrapped).
const EXE_LENGTH: u32 = 0x0047_4cb0;
const EXE_LOOKUP: u32 = 0x0048_3a00;
/// The type name function, the console's, the random number, the reference's full name.
const TYPE_NAME: u32 = 0x0044_0e30;
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
const CONSOLE_PRINT: u32 = 0x0071_d0a0;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    static NEXT: std::cell::Cell<u32> = const { std::cell::Cell::new(AREA + 0x1000) };
}

fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn calls_to(f: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        c.borrow()
            .iter()
            .filter(|(a, _)| *a == f)
            .map(|(_, w)| w.clone())
            .collect()
    })
}

fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let at = n.get();
        n.set(at + size.next_multiple_of(0x10));
        at
    })
}

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn text(e: &mut Engine, s: &[u8]) -> u32 {
    let at = alloc(s.len() as u32 + 1);
    e.mem.write(at, s);
    e.mem.set_u8(at + s.len() as u32, 0);
    at
}

/// A form of class `vtable`, type `kind`, id `id` (flags 0).
fn form(e: &mut Engine, vtable: u32, kind: u8, id: u32) -> u32 {
    let f = alloc(0x60);
    e.mem.set_u32(f, vtable);
    e.mem.set_u8(f + 4, kind);
    e.mem.set_u32(f + 0xc, id);
    f
}

/// The engine as the exe leaves the vtables: every listed class's two slots the stubs, but the
/// reference's getter (a real function), JIP's light slots, and the nine storing classes' setters
/// (their own); the two functions JG replaces registered; the game's calls JG makes stood in.
fn engine(on: bool) -> Engine {
    engine_with(on, true)
}

/// [`engine`], with JIP's light setter in the light class's slot or not (JIP leaves its H04 out
/// while JohnnyGuitar's editor ids are in, so the real pair has the stub there).
fn engine_with(on: bool, jip_light_setter: bool) -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    NEXT.with(|n| n.set(AREA + 0x1000));
    let mut e = Engine::new();
    e.map(AREA, 0x10_0000);
    if !e.mem.is_mapped(VTABLES) {
        e.map(VTABLES, 0x9_0000);
    }
    for &(vtable, _) in vtables::FORM_VTABLES.iter() {
        e.mem.set_u32(vtable + GETTER_SLOT, GETTER_STUB);
        e.mem.set_u32(vtable + SETTER_SLOT, SETTER_STUB);
    }
    e.mem.set_u32(REFR_VTABLE + GETTER_SLOT, REFR_GETTER);
    if jip_light_setter {
        e.mem.set_u32(LIGH_VTABLE + SETTER_SLOT, JIP_LIGHT_SETTER);
    }
    for slot in STORING_SETTER_SLOTS {
        e.mem.set_u32(slot, OWN_SETTER);
    }
    e.register(OWN_SETTER, |e, w| {
        record(OWN_SETTER, w);
        ret(e.mem.u32(w[0] + 0x40))
    });
    e.register(EXE_LENGTH, |_, w| {
        record(EXE_LENGTH, w);
        ret(0x5eed)
    });
    e.register(EXE_LOOKUP, |_, w| {
        record(EXE_LOOKUP, w);
        ret(0x5eed)
    });
    e.register(TYPE_NAME, |_, w| {
        record(TYPE_NAME, w);
        ret(0)
    });
    e.register(REFR_FULL_NAME, |e, w| ret(e.mem.u32(w[0] + 0x44)));
    e.register(REFR_GETTER, |e, w| ret(e.mem.u32(w[0] + 0x48)));
    e.register(GETTER_STUB, |_, _| ret(EMPTY_TEXT));
    e.register(SETTER_STUB, |_, _| ret(1));
    e.register(CONSOLE_INSTANCE, |_, _| ret(0x0dea_c000));
    e.register(CONSOLE_PRINT, |e, w| {
        let format = e.mem.cstr(w[1]);
        let word = e.mem.u32(w[2]);
        let arg = e.mem.cstr(word);
        record(CONSOLE_PRINT, &[format.len() as u32]);
        CALLS.with(|c| {
            c.borrow_mut().push((
                CONSOLE_PRINT + 1,
                arg.iter().map(|&b| u32::from(b)).collect(),
            ))
        });
        ret(0)
    });
    e.register(RANDOM_01, |_, _| Ret {
        st0: 0.5,
        ..Ret::default()
    });
    set_on(&mut e, on);
    install(&mut e);
    e
}

fn name_of(e: &Engine, text: u32) -> Vec<u8> {
    e.mem.cstr(text)
}

/// Every site JG uses.
fn every_site() -> Vec<SiteSlot> {
    let mut all: Vec<SiteSlot> = GETTER_SITES.to_vec();
    all.extend(CONSOLE_NAME_SITES);
    all.extend(SETTER_SITES);
    all.extend(SKIPPED_SITES);
    all.push(TEMPORARY_SITE);
    all
}

/// The lead's first addition: with the row off nothing is installed. Every vtable slot is as the
/// exe has it, no seam answers (the translation runs the exe's code), the two functions are not
/// wrapped (the exe's run), the store is off and JG is not in xNVSE's list.
#[test]
fn off_every_site_is_the_exes_path() {
    let mut e = engine(false);
    for &(vtable, name) in vtables::FORM_VTABLES.iter() {
        let getter = if vtable == REFR_VTABLE {
            REFR_GETTER
        } else {
            GETTER_STUB
        };
        let setter = if vtable == LIGH_VTABLE {
            JIP_LIGHT_SETTER
        } else {
            SETTER_STUB
        };
        if STORING_SETTER_SLOTS.contains(&(vtable + SETTER_SLOT)) {
            continue;
        }
        assert_eq!(e.mem.u32(vtable + GETTER_SLOT), getter, "{name}");
        assert_eq!(e.mem.u32(vtable + SETTER_SLOT), setter, "{name}");
    }
    for slot in STORING_SETTER_SLOTS {
        assert_eq!(e.mem.u32(slot), OWN_SETTER);
    }
    for slot in every_site() {
        assert_eq!(
            e.site(slot.addr(), &[0x100, 0x200]),
            None,
            "{:08x}",
            slot.addr()
        );
    }
    assert!(e.wraps_at(EXE_LENGTH).is_empty() && e.wraps_at(EXE_LOOKUP).is_empty());
    let name = text(&mut e, b"PlayerRef");
    assert_eq!(
        e.call(EXE_LOOKUP, &args![name]).u32(),
        0x5eed,
        "the exe's lookup"
    );
    assert!(!editor_ids::on(&mut e));
    assert!(!on(&e));
    assert!(nvse::plugins::built_ins(&e).is_empty());
}

/// On: JG's getter and setter in each listed class's slot that is still the stub, and only
/// there (the reference's real getter and JIP's light setter stay); the nine storing classes'
/// setters are JG's wrappers; JG is in xNVSE's list but not listed (absent to mods).
#[test]
fn on_the_stub_slots_are_jgs() {
    let e = engine(true);
    let mut getters = 0;
    let mut setters = 0;
    for &(vtable, name) in vtables::FORM_VTABLES.iter() {
        let g = e.mem.u32(vtable + GETTER_SLOT);
        let s = e.mem.u32(vtable + SETTER_SLOT);
        if vtable == REFR_VTABLE {
            assert_eq!(g, REFR_GETTER, "a real getter stays");
        } else {
            assert_eq!(g, GETTER, "{name}");
            getters += 1;
        }
        if vtable == LIGH_VTABLE {
            assert_eq!(s, JIP_LIGHT_SETTER, "JIP's light slot stays JIP's");
        } else if let Some(i) = STORING_SETTER_SLOTS
            .iter()
            .position(|&slot| slot == vtable + SETTER_SLOT)
        {
            assert_eq!(s, WRAPPED_SETTERS + 4 * i as u32, "{name}");
        } else {
            assert_eq!(s, SETTER, "{name}");
            setters += 1;
        }
    }
    assert_eq!((getters, setters), (141, 141 - 9));
    let list = nvse::plugins::built_ins(&e);
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].name, list[0].listed), (NAME, false));
    assert!(on(&e));
}

/// The setter through the slot (as the load calls it with each record's name), then the lookup
/// by name (case-blind, through `0x483A00`), the getter (the name as guest text, kept), the length
/// (`0x474CB0`); an empty or missing name is refused; none of the exe's two functions is reached.
#[test]
fn a_named_form_is_found_by_name() {
    let mut e = engine(true);
    let caps = form(&mut e, 0x0102_b844, 0x1f, 0x0000_000f);
    let name = text(&mut e, b"Caps001");
    assert_eq!(e.vcall(caps, SETTER_SLOT, &args![name]).u32() & 0xff, 1);
    for asked in [&b"Caps001"[..], b"caps001", b"CAPS001"] {
        let t = text(&mut e, asked);
        assert_eq!(e.call(EXE_LOOKUP, &args![t]).u32(), caps);
    }
    let missing = text(&mut e, b"Caps002");
    assert_eq!(e.call(EXE_LOOKUP, &args![missing]).u32(), 0);
    let empty = text(&mut e, b"");
    assert_eq!(e.call(EXE_LOOKUP, &args![empty]).u32(), 0);
    assert_eq!(e.call(EXE_LOOKUP, &args![0u32]).u32(), 0);
    let got = e.vcall(caps, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, got), b"Caps001");
    assert_ne!(got, name, "a copy of the store's");
    assert_eq!(e.vcall(caps, GETTER_SLOT, &args![]).u32(), got, "kept");
    assert_eq!(e.call(EXE_LENGTH, &args![caps]).u32(), 7);
    let unnamed = form(&mut e, 0x0102_b844, 0x1f, 0x10);
    assert_eq!(e.vcall(unnamed, GETTER_SLOT, &args![]).u32(), EMPTY_TEXT);
    assert_eq!(e.call(EXE_LENGTH, &args![unnamed]).u32(), 0);
    assert_eq!(e.vcall(unnamed, SETTER_SLOT, &args![empty]).u32() & 0xff, 0);
    assert_eq!(e.vcall(unnamed, SETTER_SLOT, &args![0u32]).u32() & 0xff, 0);
    assert!(calls_to(EXE_LENGTH).is_empty() && calls_to(EXE_LOOKUP).is_empty());
    // The same name again: already the form's (false), the map unchanged.
    assert_eq!(e.vcall(caps, SETTER_SLOT, &args![name]).u32() & 0xff, 0);
}

/// A temporary form (flag 0x4000) or one of id 0 keeps its name but is not put in the map; the
/// console's script name `SysWindowCompileAndRun` is stored as `Console Command`.
#[test]
fn temporary_forms_and_id_0_stay_out_of_the_map() {
    let mut e = engine(true);
    let temporary = form(&mut e, 0x0103_7094, 0x11, 0x0100_0800);
    e.mem.set_u32(temporary + 8, 0x4000);
    let zero = form(&mut e, 0x0103_7094, 0x11, 0);
    let (a, b) = (text(&mut e, b"TempScript"), text(&mut e, b"ZeroScript"));
    e.vcall(temporary, SETTER_SLOT, &args![a]);
    e.vcall(zero, SETTER_SLOT, &args![b]);
    assert_eq!(e.call(EXE_LOOKUP, &args![a]).u32(), 0);
    assert_eq!(e.call(EXE_LOOKUP, &args![b]).u32(), 0);
    let got = e.vcall(temporary, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, got), b"TempScript", "the form keeps it");
    let console = form(&mut e, 0x0103_7094, 0x11, 0x0100_0801);
    let sys = text(&mut e, b"SysWindowCompileAndRun");
    e.vcall(console, SETTER_SLOT, &args![sys]);
    let wanted = text(&mut e, b"Console Command");
    assert_eq!(e.call(EXE_LOOKUP, &args![wanted]).u32(), console);
    assert_eq!(e.call(EXE_LOOKUP, &args![sys]).u32(), 0);
}

/// A name another form has: the newest form takes the map's entry, the old form loses the name
/// (its getter then answers its older one or ""), and the conflict is logged and counted; not for
/// an old form of id 0x18E or the name `Wilderness` (the entry still moves, the old form keeps
/// its name). The type names are asked only when the two types differ.
#[test]
fn a_taken_name_is_a_logged_conflict() {
    let mut e = engine(true);
    let (first, second) = (
        form(&mut e, 0x0102_b844, 0x1f, 0x0000_1234),
        form(&mut e, 0x0102_b844, 0x1f, 0x0100_1234),
    );
    let name = text(&mut e, b"DocsBag");
    e.vcall(first, SETTER_SLOT, &args![name]);
    e.vcall(second, SETTER_SLOT, &args![name]);
    assert_eq!(e.call(EXE_LOOKUP, &args![name]).u32(), second);
    assert_eq!(
        e.vcall(first, GETTER_SLOT, &args![]).u32(),
        EMPTY_TEXT,
        "the name taken off the old form"
    );
    assert_eq!(
        editor_ids::conflict_lines(&mut e),
        ["01001234 (\"\") steals EDID \"DocsBag\" from 00001234 (\"\")"]
    );
    assert!(calls_to(TYPE_NAME).is_empty());
    let other_type = form(&mut e, 0x0102_ac0c, 0x1c, 0x0200_1234);
    e.vcall(other_type, SETTER_SLOT, &args![name]);
    assert_eq!(calls_to(TYPE_NAME).len(), 2, "both types named");
    assert_eq!(editor_ids::store(&mut e).counts().conflicts, 2);

    // Not conflicts: the old form 0x18E, the name `Wilderness`.
    let obsidian = form(&mut e, 0x0102_b844, 0x1f, 0x18e);
    let n = text(&mut e, b"Taken");
    e.vcall(obsidian, SETTER_SLOT, &args![n]);
    let taker = form(&mut e, 0x0102_b844, 0x1f, 0x0300_0000);
    e.vcall(taker, SETTER_SLOT, &args![n]);
    let wild = text(&mut e, b"Wilderness");
    let (w1, w2) = (
        form(&mut e, 0x0102_b844, 0x1f, 0x0300_0001),
        form(&mut e, 0x0102_b844, 0x1f, 0x0300_0002),
    );
    e.vcall(w1, SETTER_SLOT, &args![wild]);
    e.vcall(w2, SETTER_SLOT, &args![wild]);
    assert_eq!(editor_ids::store(&mut e).counts().conflicts, 2);
    assert_eq!(e.call(EXE_LOOKUP, &args![n]).u32(), taker);
    assert_eq!(e.call(EXE_LOOKUP, &args![wild]).u32(), w2);
    let kept = e.vcall(obsidian, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, kept), b"Taken", "an ignored one keeps its name");
}

/// The lead's first addition: the nine classes that store their own name. The class's own setter
/// runs first (as its slot held it); when it answers true, JG stores the name too, so the form is
/// found by name with the row on; when it answers false, nothing is stored and false answered.
/// The real pair (JIP's H04 left out): the light class's stub slots are JohnnyGuitar's, so a light
/// is named as any form, and loses its name to a later form that takes it.
#[test]
fn a_light_loses_its_name_to_a_later_form() {
    let mut e = engine_with(true, false);
    assert_eq!(e.mem.u32(LIGH_VTABLE + GETTER_SLOT), GETTER);
    assert_eq!(e.mem.u32(LIGH_VTABLE + SETTER_SLOT), SETTER);
    let light = form(&mut e, LIGH_VTABLE, 0x1e, 0x0000_5678);
    let later = form(&mut e, 0x0102_b844, 0x1f, 0x0100_5678);
    let name = text(&mut e, b"GoodspringsLamp");
    e.vcall(light, SETTER_SLOT, &args![name]);
    assert_eq!(e.call(EXE_LOOKUP, &args![name]).u32(), light);
    let named = e.vcall(light, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, named), b"GoodspringsLamp");
    e.vcall(later, SETTER_SLOT, &args![name]);
    assert_eq!(e.call(EXE_LOOKUP, &args![name]).u32(), later);
    assert_eq!(e.vcall(light, GETTER_SLOT, &args![]).u32(), EMPTY_TEXT);
}

#[test]
fn the_nine_storing_classes_are_still_found() {
    let mut e = engine(true);
    for (i, slot) in STORING_SETTER_SLOTS.iter().enumerate() {
        let vtable = slot - SETTER_SLOT;
        let f = form(&mut e, vtable, 0x39, 0x0000_2000 + i as u32);
        let name = text(&mut e, format!("Stored{i}").as_bytes());
        e.mem.set_u32(f + 0x40, 0);
        assert_eq!(e.vcall(f, SETTER_SLOT, &args![name]).u32() & 0xff, 0);
        assert_eq!(
            e.call(EXE_LOOKUP, &args![name]).u32(),
            0,
            "refused by its own"
        );
        e.mem.set_u32(f + 0x40, 1);
        assert_eq!(e.vcall(f, SETTER_SLOT, &args![name]).u32() & 0xff, 1);
        assert_eq!(e.call(EXE_LOOKUP, &args![name]).u32(), f, "{i}");
    }
    let own: Vec<u32> = calls_to(OWN_SETTER).iter().map(|w| w[0]).collect();
    assert_eq!(own.len(), 18, "its own setter first, each time");
}

/// The seams: a getter call site answers the form's name (or ""); the console's selected
/// reference shows its full name, or its +0x130 (the reference's own getter) when it has none;
/// a constructor's setter call stores the name; the map insertions JG takes out answer (skipped);
/// after `SetTemporary`'s removal the form's names leave the map (the exe goes on).
#[test]
fn the_seams_answer_as_jg() {
    let mut e = engine(true);
    let f = form(&mut e, 0x0102_b844, 0x1f, 0x0000_3000);
    let name = text(&mut e, b"Lucky38Key");
    e.vcall(f, SETTER_SLOT, &args![name]);
    for slot in GETTER_SITES {
        let got = e.site(slot.addr(), &[f]).unwrap();
        assert_eq!(name_of(&e, got), b"Lucky38Key", "{:08x}", slot.addr());
    }
    let unnamed = form(&mut e, 0x0102_b844, 0x1f, 0x0000_3001);
    assert_eq!(e.site(0x0048_6903, &[unnamed]), Some(EMPTY_TEXT));

    let refr = form(&mut e, REFR_VTABLE, 0x3b, 0x0000_4000);
    let full = text(&mut e, b"Doc Mitchell");
    let own = text(&mut e, b"DocMitchellREF");
    e.mem.set_u32(refr + 0x44, full);
    e.mem.set_u32(refr + 0x48, own);
    for slot in CONSOLE_NAME_SITES {
        assert_eq!(e.site(slot.addr(), &[refr]), Some(full));
        e.mem.set_u32(refr + 0x44, EMPTY_TEXT);
        assert_eq!(
            e.site(slot.addr(), &[refr]),
            Some(own),
            "no name: its editor id"
        );
        e.mem.set_u32(refr + 0x44, 0);
        assert_eq!(e.site(slot.addr(), &[refr]), Some(own));
        e.mem.set_u32(refr + 0x44, full);
    }

    let av = form(&mut e, 0x0106_7a2c, 0x59, 0x0000_0500);
    let av_name = text(&mut e, b"AVStrength");
    assert_eq!(e.site(0x0066_ff57, &[av, av_name]), Some(1));
    assert_eq!(e.call(EXE_LOOKUP, &args![av_name]).u32(), av);
    let dobj = form(&mut e, 0x0103_2dc4, 0x64, 0x31);
    let dobj_name = text(&mut e, b"DefaultObjectManager");
    assert_eq!(e.site(0x0058_d7bc, &[dobj, dobj_name]), Some(1));
    assert_eq!(e.call(EXE_LOOKUP, &args![dobj_name]).u32(), dobj);

    for slot in SKIPPED_SITES {
        assert_eq!(e.site(slot.addr(), &[]), Some(0), "{:08x}", slot.addr());
    }

    assert_eq!(e.site(TEMPORARY_SITE.addr(), &[f]), None, "the exe goes on");
    assert_eq!(e.call(EXE_LOOKUP, &args![name]).u32(), 0, "out of the map");
    let still = e.vcall(f, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, still), b"Lucky38Key", "still the form's");
}

/// The deferred start: the load's line counts forms, names, entries and bytes; with conflicts,
/// JG's console line ("mod" at the game's random 0.5); without, none.
#[test]
fn the_deferred_start_reports_conflicts() {
    let mut e = engine(true);
    let message = nvse::plugins::Message {
        kind: nvse::plugins::msg::DEFERRED_INIT,
        data: 0,
        len: 0,
    };
    let a = form(&mut e, 0x0102_b844, 0x1f, 0x0000_5000);
    let n = text(&mut e, b"Bottle");
    e.vcall(a, SETTER_SLOT, &args![n]);
    listener(&mut e, &message);
    assert!(calls_to(CONSOLE_PRINT).is_empty());
    let line = load_line(&mut e);
    assert!(
        line.starts_with(
            "JohnnyGuitar's editor ids: 1 forms named (1 names), 1 in the map, 6 bytes of names, "
        ) && line.ends_with(" bytes in all, 0 conflicts"),
        "{line}"
    );
    let b = form(&mut e, 0x0102_b844, 0x1f, 0x0000_5001);
    e.vcall(b, SETTER_SLOT, &args![n]);
    listener(&mut e, &message);
    assert_eq!(calls_to(CONSOLE_PRINT).len(), 1);
    let arg: Vec<u8> = calls_to(CONSOLE_PRINT + 1)[0]
        .iter()
        .map(|&b| b as u8)
        .collect();
    assert_eq!(arg, b"mod");
}

/// The row turns the store on before any install, so JIP's install (which comes first) sees it
/// and leaves its H04 out, as JIP does with JohnnyGuitar loaded.
#[test]
fn the_row_turns_the_store_on_before_jips_install() {
    for on in [true, false] {
        let mut e = Engine::new();
        set_on(&mut e, on);
        assert_eq!(editor_ids::on(&mut e), on);
    }
}

/// A gone form's guest copy goes back to the game's heap (streamed named references leave none
/// behind): the next block of that size is the same one.
#[test]
fn a_gone_forms_copy_is_freed() {
    let mut e = engine(true);
    let f = form(&mut e, 0x0102_b844, 0x1f, 0x0000_6000);
    let n = text(&mut e, b"StreamedRef");
    e.vcall(f, SETTER_SLOT, &args![n]);
    let copy = e.vcall(f, GETTER_SLOT, &args![]).u32();
    assert_eq!(name_of(&e, copy), b"StreamedRef");
    editor_ids::form_gone(&mut e, f);
    assert_eq!(e.call(EXE_LOOKUP, &args![n]).u32(), 0);
    assert_eq!(e.vcall(f, GETTER_SLOT, &args![]).u32(), EMPTY_TEXT);
    assert_eq!(e.mem.alloc(12), copy, "the copy's block reused");
}

/// The console line's log: the conflict lines go to `JohnnyGuitarNVSE.log` in one write, made
/// anew, through the game's file calls (as `nvse.log`), `\r\n` after each line.
#[test]
fn the_conflicts_go_to_johnnyguitars_log() {
    const CREATE_FILE_A: u32 = 0x00fd_f088;
    const WRITE_FILE: u32 = 0x00fd_f0dc;
    const CLOSE_HANDLE: u32 = 0x00fd_f07c;
    let mut e = engine(true);
    e.register(CREATE_FILE_A, |e, w| {
        let path: Vec<u32> = e.mem.cstr(w[0]).iter().map(|&b| u32::from(b)).collect();
        record(CREATE_FILE_A, &path);
        record(CREATE_FILE_A + 1, &[w[4]]);
        ret(7)
    });
    e.register(WRITE_FILE, |e, w| {
        let bytes: Vec<u32> = (0..w[2]).map(|i| u32::from(e.mem.u8(w[1] + i))).collect();
        record(WRITE_FILE, &bytes);
        ret(1)
    });
    e.register(CLOSE_HANDLE, |_, w| {
        record(CLOSE_HANDLE, w);
        ret(1)
    });
    let message = nvse::plugins::Message {
        kind: nvse::plugins::msg::DEFERRED_INIT,
        data: 0,
        len: 0,
    };
    let n = text(&mut e, b"Bottle");
    for id in [0x0000_5000, 0x0100_5000, 0x0200_5000] {
        let f = form(&mut e, 0x0102_b844, 0x1f, id);
        e.vcall(f, SETTER_SLOT, &args![n]);
    }
    listener(&mut e, &message);
    let as_bytes = |v: &Vec<u32>| v.iter().map(|&b| b as u8).collect::<Vec<u8>>();
    assert_eq!(
        as_bytes(&calls_to(CREATE_FILE_A)[0]),
        b"JohnnyGuitarNVSE.log"
    );
    assert_eq!(calls_to(CREATE_FILE_A + 1), vec![vec![2]], "made anew");
    let writes = calls_to(WRITE_FILE);
    assert_eq!(writes.len(), 1, "one write");
    assert_eq!(
        as_bytes(&writes[0]),
        b"01005000 (\"\") steals EDID \"Bottle\" from 00005000 (\"\")\r\n02005000 (\"\") steals EDID \"Bottle\" from 01005000 (\"\")\r\n"
    );
    assert_eq!(calls_to(CLOSE_HANDLE), vec![vec![7]]);
}

/// Many names: each stored once in the arena, each found; the counts add up (the load's line).
#[test]
fn a_hundred_thousand_names() {
    let mut e = engine(true);
    let s = editor_ids::store(&mut e);
    for i in 0..100_000u32 {
        let name = format!("Form{i:06}");
        assert_eq!(
            s.add_name(0x1000_0000 + i, name.as_bytes()),
            editor_ids::SetResult::Success
        );
        assert_eq!(s.map_insert(0x1000_0000 + i, name.as_bytes()), None);
    }
    for i in (0..100_000u32).step_by(997) {
        let name = format!("form{i:06}");
        assert_eq!(s.lookup(name.as_bytes()), 0x1000_0000 + i);
    }
    let c = s.counts();
    assert_eq!(
        (c.forms, c.entries, c.arena_bytes),
        (100_000, 100_000, 1_000_000)
    );
}

/// The copies in the game's memory: one per form's name, made the first time game code asks and
/// then the same text every time (a reader called every frame takes no more memory); a new copy
/// only once the form's names change (a load-time event), the old one left where it is (game code
/// may still hold it, as JG leaves an old `NiFixedString` to its holders).
#[test]
fn a_name_asked_for_again_is_the_same_copy() {
    let mut e = engine(true);
    let f = form(&mut e, 0x0102_b844, 0x1f, 0x0000_6000);
    let n = text(&mut e, b"Stimpak");
    e.vcall(f, SETTER_SLOT, &args![n]);
    let first = e.vcall(f, GETTER_SLOT, &args![]).u32();
    let heap = e.mem.alloc(4);
    for _ in 0..10_000 {
        assert_eq!(e.vcall(f, GETTER_SLOT, &args![]).u32(), first);
        assert_eq!(e.site(0x0048_6903, &[f]), Some(first));
    }
    // 20,000 asks: no new copy between the two small blocks (a copy each would be 160 KB).
    let after = e.mem.alloc(4);
    assert!(
        after > heap && after - heap < 0x100,
        "{heap:08x} {after:08x}"
    );
}

/// Real threads: the store is made at the install, row on or off, so a thread's engine (made from
/// main's afterwards) shares main's one store instead of making its own after the threads start.
#[test]
fn a_threads_engine_shares_the_one_store() {
    for on in [true, false] {
        let mut e = engine(on);
        let mut thread = Engine::new();
        e.share_slots_for_test(&mut thread);
        assert_eq!(editor_ids::on(&mut thread), on);
        assert!(std::ptr::eq(
            editor_ids::store(&mut e),
            editor_ids::store(&mut thread)
        ));
        if on {
            editor_ids::store(&mut thread).add_name(0x0600_8000, b"ThreadName");
            editor_ids::store(&mut thread).map_insert(0x0600_8000, b"ThreadName");
            assert_eq!(editor_ids::store(&mut e).lookup(b"threadname"), 0x0600_8000);
        }
    }
}

/// The load's cost (run by hand, `--ignored --nocapture`, in release for the numbers the pull
/// request gives): names stored and put in the map as the setter does, then as many lookups; the
/// times and the store's bytes printed. Two sizes, each in a fresh engine: 465,016 (every record of
/// FalloutNV.esm, more than the vanilla game names) and a million (a large mod list).
#[test]
#[ignore]
fn load_cost_of_a_million_names() {
    for count in [465_016u32, 1_000_000] {
        load_cost_of(count);
    }
}

fn load_cost_of(count: u32) {
    let mut e = engine(true);
    let s = editor_ids::store(&mut e);
    let names: Vec<String> = (0..count)
        .map(|i| format!("ModForm{i:07}EditorId"))
        .collect();
    let start = std::time::Instant::now();
    for (i, name) in names.iter().enumerate() {
        s.add_name(0x1000_0000 + i as u32, name.as_bytes());
        s.map_insert(0x1000_0000 + i as u32, name.as_bytes());
    }
    let stored = start.elapsed();
    let start = std::time::Instant::now();
    let mut found = 0u64;
    for name in &names {
        found += u64::from(s.lookup(name.as_bytes()) != 0);
    }
    let looked = start.elapsed();
    let c = s.counts();
    eprintln!(
        "{} names ({} bytes of text): stored in {} ms, looked up in {} ms ({} ns each), {} bytes in all",
        count,
        c.arena_bytes,
        stored.as_millis(),
        looked.as_millis(),
        looked.as_nanos() / u128::from(count),
        c.heap_bytes
    );
    assert_eq!(found, u64::from(count));
}
