//! JohnyGuitar's editor ids end to end, through the game's own run-time compile (translated):
//! `PlayerRef.AddItem Caps001 5` typed in the console compiles its calling reference through
//! `ScriptCompileData::GetReferencedObject` (`005afad0`), and `AddItem`'s object argument through
//! `ScriptCompiler::StandardCompile` (`005b1ba0`), which looks the word up there too. Both ask
//! `TESForm::GetFormByEditorID` (`00483a00`) for the name. The two forms are named as the load
//! names them: through their class's `SetEditorID` (vtable +0x134).
//! - With JohnnyGuitar's row on, the slots are JG's, the names are stored, and both names
//!   resolve: the line compiles, `Caps001` written as the referenced object's index, the calling
//!   reference the player.
//! - With it off, the slots are the exe's stubs, nothing is stored, and the compile reports the
//!   item as not found: the real game without JohnnyGuitar ("Item 'PlayerRef' not found").
//!
//! Here because this crate registers every translation. Not the whole console line: the game's
//! command table, its keywords and the console's own call into the compiler are exe data and code
//! these tests do not have, so the test runs the two functions that read the two names. The
//! stand-ins: the C runtime's locale readers, `strtoul`, the dynamic cast, the allocator, the error
//! report's message (recorded) and printing, and the global lookup (no data handler here).

use engine as _;
use engine_core::prelude::*;
use std::cell::RefCell;

const AREA: u32 = 0x04b0_0000;
/// The compile data, the line, the parameters and their names, a word, the forms, the heap.
const DATA: u32 = AREA;
const LINE: u32 = AREA + 0x200;
const PARAMS: u32 = AREA + 0x800;
const NAMES: u32 = AREA + 0x900;
const WORD: u32 = AREA + 0x1000;
const CAPS: u32 = AREA + 0x2000;
const PLAYER: u32 = AREA + 0x2100;
const TEXTS: u32 = AREA + 0x3000;
/// The exe's data the compile reads: the parameter types (`0118cdd4`, 8 bytes a type), the text
/// "Player" (`01018f54`) and the player (`*011dea3c`); the vtables JG's install reads.
const PARAMETER_TYPES: u32 = 0x0118_cdd4;
const TABLES: u32 = 0x0118_0000;
const GLOBALS: u32 = 0x011c_0000;
const VTABLES: u32 = 0x0101_0000;
const TEXT_PLAYER: u32 = 0x0101_8f54;
const PLAYER_REF: u32 = 0x011d_ea3c;
/// `TESObjectMISC`'s and `PlayerCharacter`'s vtables.
const MISC_VTABLE: u32 = 0x0102_b844;
const PLAYER_VTABLE: u32 = 0x0108_aa3c;
/// `AddItem` (`0x1002`) and its parameters: an inventory object (`0x32`: no variable, an
/// object), a count (an integer), an optional flag.
const ADD_ITEM: u32 = 0x1002;
const INVENTORY_OBJECT: u32 = 0x32;
const INTEGER: u32 = 1;
/// `ScriptError`'s message format (`00407000`, recorded) and its printing.
const VSPRINTF: u32 = 0x0040_7000;
const STRING_PRINTF: u32 = 0x0040_6f60;
const PREPARE_MESSAGE: u32 = 0x0046_4f30;
const CONSOLE_PRINT: u32 = 0x0070_3c00;
const SCRIPT_LOG: u32 = 0x005b_5e40;
/// "Item '%s' not found for parameter %s." (the exe's format at `01039178`, read by the test from
/// the error's format word).
const ISALNUM: u32 = 0x00ec_89e9;
const ISALPHA: u32 = 0x00ec_86e1;
const TOUPPER: u32 = 0x00ec_a7f4;
const TOLOWER: u32 = 0x00ec_67aa;
const STRICMP: u32 = 0x0040_4dc0;
const STRNICMP: u32 = 0x00ec_7ec0;
const IS_A_FLOAT: u32 = 0x004b_12d0;
const ATOF: u32 = 0x00ec_a573;
const STRTOUL: u32 = 0x00ec_8d33;
const TEXT_TO_INTEGER: u32 = 0x00ec_a6bd;
const GET_GLOBAL: u32 = 0x0046_1700;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;

thread_local! {
    static FORMATS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn lower(e: &Engine, at: u32) -> Vec<u8> {
    e.mem.cstr(at).to_ascii_lowercase()
}

/// The engine with the compile's stand-ins, the exe's two parameter types and "Player", the
/// classes' stub slots, the two forms, and JohnnyGuitar installed with its row `on` or off; then
/// each form named through its class's setter slot, as the load names it.
fn engine(on: bool) -> Engine {
    FORMATS.with(|f| f.borrow_mut().clear());
    let mut e = Engine::new_untyped_for_test();
    for (at, size) in [
        (AREA, 0x4_0000),
        (TABLES, 0x2_0000),
        (GLOBALS, 0x2_0000),
        (VTABLES, 0x9_0000),
    ] {
        if !e.mem.is_mapped(at) {
            e.map(at, size);
        }
    }
    nvse::line_reader::install(&mut e);
    nvse::args::install(&mut e);
    // The exe's bytes (Ghidra, `0118cdd4`): an inventory object is no variable and an object; an
    // integer may be a variable.
    e.mem.write(PARAMETER_TYPES + INVENTORY_OBJECT * 8, &[0, 1]);
    e.mem.write(PARAMETER_TYPES + INTEGER * 8, &[1, 0]);
    e.mem.write(TEXT_PLAYER, b"Player\0");
    e.register(VSPRINTF, |e, a| {
        FORMATS.with(|f| f.borrow_mut().push(a[1]));
        e.mem.set_u8(a[0], 0);
        ret(0)
    });
    for quiet in [STRING_PRINTF, CONSOLE_PRINT, SCRIPT_LOG] {
        e.register(quiet, |_, _| ret(0));
    }
    // `atol` (`00eca6bd`) of the count, the C runtime's.
    e.register(TEXT_TO_INTEGER, |e, a| {
        let t = String::from_utf8_lossy(&e.mem.cstr(a[0])).into_owned();
        ret(t.trim().parse::<i32>().unwrap_or(0) as u32)
    });
    e.register(PREPARE_MESSAGE, |_, a| ret(a[0]));
    e.register(ISALNUM, |_, a| {
        ret(u32::from(
            (a[0] as u8).is_ascii_alphanumeric() && a[0] < 0x80,
        ))
    });
    e.register(ISALPHA, |_, a| {
        ret(u32::from((a[0] as u8).is_ascii_alphabetic() && a[0] < 0x80))
    });
    e.register(TOUPPER, |_, a| {
        ret(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(TOLOWER, |_, a| {
        ret(u32::from((a[0] as u8).to_ascii_lowercase()))
    });
    e.register(STRICMP, |e, a| {
        if a[0] == 0 || a[1] == 0 {
            return ret(1);
        }
        ret(u32::from(lower(e, a[0]) != lower(e, a[1])))
    });
    e.register(STRNICMP, |e, a| {
        let n = a[2] as usize;
        let (x, y) = (lower(e, a[0]), lower(e, a[1]));
        let cut = |v: &[u8]| v.iter().take(n).copied().collect::<Vec<u8>>();
        ret(u32::from(cut(&x) != cut(&y)))
    });
    e.register(IS_A_FLOAT, |e, a| {
        let t = String::from_utf8_lossy(&e.mem.cstr(a[0])).into_owned();
        ret(u32::from(t.parse::<f64>().is_ok()))
    });
    e.register(ATOF, |e, a| {
        let t = String::from_utf8_lossy(&e.mem.cstr(a[0])).into_owned();
        Ret {
            st0: t.parse().unwrap_or(0.0),
            ..Ret::default()
        }
    });
    // `strtoul` of a name: no digit read, the end at its start.
    e.register(STRTOUL, |e, a| {
        if a[1] != 0 {
            e.mem.set_u32(a[1], a[0]);
        }
        ret(0)
    });
    e.register(GET_GLOBAL, |_, _| ret(0));
    // The cast to a reference: the player is one, the item is not.
    e.register(DYNAMIC_CAST, |e, a| {
        ret(if a[0] != 0 && e.mem.u32(a[0]) == PLAYER_VTABLE {
            a[0]
        } else {
            0
        })
    });
    // The classes' slots as the exe has them (the stubs), the forms.
    e.mem.set_u32(MISC_VTABLE + 0x130, 0x0040_1280);
    e.mem.set_u32(MISC_VTABLE + 0x134, 0x0040_1290);
    e.mem.set_u32(PLAYER_VTABLE + 0x130, 0x0040_1280);
    e.mem.set_u32(PLAYER_VTABLE + 0x134, 0x0040_1290);
    e.mem.set_u32(CAPS, MISC_VTABLE);
    e.mem.set_u8(CAPS + 4, 0x1f);
    e.mem.set_u32(CAPS + 0xc, 0x0000_000f);
    e.mem.set_u32(PLAYER, PLAYER_VTABLE);
    e.mem.set_u8(PLAYER + 4, 0x3b);
    // Persistent (`kFormFlags_Persistent`, as the player is).
    e.mem.set_u32(PLAYER + 8, 0x400);
    e.mem.set_u32(PLAYER + 0xc, 0x14);
    e.mem.set_u32(PLAYER_REF, PLAYER);
    johnnyguitar::set_on(&mut e, on);
    johnnyguitar::install(&mut e);
    for (form, name) in [(CAPS, &b"Caps001"[..]), (PLAYER, b"PlayerRef")] {
        let at = TEXTS + 0x40 * u32::from(name[0] == b'P');
        e.mem.write(at, name);
        e.mem.set_u8(at + name.len() as u32, 0);
        e.vcall(form, 0x134, &args![at]);
    }
    e
}

/// `GetReferencedObject` of `name` on fresh compile data: whether it answered, the form in the
/// word (+0x210).
fn referenced(e: &mut Engine, name: &[u8]) -> (bool, u32) {
    e.mem.write(DATA, &[0; 0x58]);
    e.mem.write(WORD, &[0; 0x214]);
    e.mem.write(WORD, name);
    let ok = e.call(0x005a_fad0, &[DATA, WORD, 0, 0]).u32() & 0xff != 0;
    (ok, e.mem.u32(WORD + 0x210))
}

/// `AddItem`'s arguments `args` compiled through `StandardCompile` on fresh compile data: the
/// answer, the output, the formats of the errors reported.
fn compile(e: &mut Engine, args: &str) -> (bool, Vec<u8>, Vec<u32>) {
    FORMATS.with(|f| f.borrow_mut().clear());
    e.mem.write(NAMES, b"argument\0");
    for (i, (kind, optional)) in [(INVENTORY_OBJECT, 0), (INTEGER, 0), (INTEGER, 1)]
        .iter()
        .enumerate()
    {
        let p = PARAMS + 12 * i as u32;
        e.mem.set_u32(p, NAMES);
        e.mem.set_u32(p + 4, *kind);
        e.mem.set_u32(p + 8, *optional);
    }
    e.mem.write(DATA, &[0; 0x58]);
    e.mem.write(LINE, &[0; 0x41c]);
    e.mem.set_u32(LINE, 1);
    e.mem.write(LINE + 4, args.as_bytes());
    e.mem.set_u32(LINE + 0x204, args.len() as u32);
    e.mem.set_u32(LINE + 0x410, ADD_ITEM);
    let ok = e.call(0x005b_1ba0, &[3, PARAMS, LINE, DATA]).bool();
    let len = e.mem.u32(LINE + 0x40c).min(0x200);
    let formats = FORMATS.with(|f| f.borrow().clone());
    (ok, e.mem.bytes(LINE + 0x20c, len), formats)
}

/// The lead's end-to-end test: with the row on, `PlayerRef` and `Caps001` are found by the
/// compile (any case: JG's map is case-blind); `AddItem Caps001 5` compiles to its count, the
/// object as `r` and index 1, and the integer.
#[test]
fn with_the_row_on_player_ref_add_item_caps001_compiles() {
    let mut e = engine(true);
    assert_eq!(referenced(&mut e, b"PlayerRef"), (true, PLAYER));
    assert_eq!(referenced(&mut e, b"playerref"), (true, PLAYER));
    let (ok, output, errors) = compile(&mut e, "Caps001 5");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(&output[..2], &[2, 0], "two arguments");
    assert_eq!(
        &output[2..5],
        &[b'r', 1, 0],
        "the item, referenced object 1"
    );
    // The referenced object's form: the first of the compile data's list (+0x44).
    let object = e.mem.u32(DATA + 0x44);
    assert_eq!(e.mem.u32(object + 8), CAPS);
}

/// With the row off (vanilla, or xNVSE and JIP without JohnnyGuitar): neither name is known; the
/// calling reference is refused and `AddItem`'s item is not found, so the line does not compile
/// (as in the real game: "Item 'PlayerRef' not found"). `Player` itself still resolves.
#[test]
fn with_the_row_off_the_names_are_not_found() {
    let mut e = engine(false);
    assert!(!referenced(&mut e, b"PlayerRef").0);
    let (ok, _, errors) = compile(&mut e, "Caps001 5");
    assert!(!ok);
    assert_eq!(errors.len(), 1, "one error reported: the item not found");
    assert_eq!(referenced(&mut e, b"Player"), (true, PLAYER));
}

/// Two of the translations' seams with JG's handlers: `TESObjectCELL::GetCellName` (`00451cb0`,
/// the call `0x451CBA`) answers a cell's stored name, and a reference's +0x130 (`0055d480`: its
/// length through JG's `0x474CB0`, then the call `0x55D498`) its own name. Off: the stub's "".
#[test]
fn the_translations_seams_answer_the_stored_names() {
    for on in [true, false] {
        let mut e = engine(on);
        let cell = AREA + 0x2200;
        e.mem.set_u32(cell, 0x0102_e9b4);
        e.mem.set_u32(cell + 0xc, 0x0001_0000);
        let name = TEXTS + 0x80;
        e.mem.write(name, b"GoodspringsSaloon\0");
        let store = engine_core::editor_ids::store(&mut e);
        if on {
            store.add_name(cell, b"GoodspringsSaloon");
        }
        let got = e.call(0x0045_1cb0, &[cell]).u32();
        let want: &[u8] = if on { b"GoodspringsSaloon" } else { b"" };
        assert_eq!(e.mem.cstr(got), want, "{on}");
        // The player (a reference): its own name through its +0x130.
        let got = e.call(0x0055_d480, &[PLAYER]).u32();
        if on {
            assert_eq!(e.mem.cstr(got), b"PlayerRef");
        } else {
            assert_ne!(e.mem.cstr(got), b"PlayerRef");
        }
    }
}

// ---- The seams through their translated callers (the review of #852): each test fails when its
// caller's `e.site` line is taken out. ----

/// `TESForm::GetFormByEditorID` (`00483a00`) of `name`.
fn by_name(e: &mut Engine, name: &[u8]) -> u32 {
    let at = TEXTS + 0x100;
    e.mem.write(at, name);
    e.mem.set_u8(at + name.len() as u32, 0);
    e.call(0x0048_3a00, &args![at]).u32()
}

/// `TESForm::SetFormID` (`00485c10`, `thiscall`): the id at +0xC (JG names only forms with one).
fn stand_in_form_id(e: &mut Engine) {
    e.register(0x0048_5c10, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        ret(0)
    });
}

/// The callees a test does not cover, stood in (answering 0).
fn quiet(e: &mut Engine, addresses: &[u32]) {
    for &at in addresses {
        e.register(at, |_, _| ret(0));
    }
}

/// `TESForm::SetTemporary` (`00484490`, its `RemoveFromDataStructures` stood in) on a named
/// form: with the row on, the name leaves the map at `0x48449A` (not found any more) while the
/// form still reads it (+0x130); the flag is set either way. Off: never found.
#[test]
fn set_temporary_takes_the_name_out_of_the_map() {
    for on in [true, false] {
        let mut e = engine(on);
        quiet(&mut e, &[0x0048_3c70]);
        assert_eq!(by_name(&mut e, b"Caps001"), if on { CAPS } else { 0 });
        e.call(0x0048_4490, &args![CAPS]);
        assert_eq!(e.mem.u32(CAPS + 8) & 0x4000, 0x4000);
        assert_eq!(by_name(&mut e, b"Caps001"), 0, "{on}");
        let name = e.vcall(CAPS, 0x130, &args![]).u32();
        let want: &[u8] = if on { b"Caps001" } else { b"" };
        assert_eq!(e.mem.cstr(name), want, "{on}");
    }
}

/// `ActorValueInfo`'s constructor (`0066fe40`; its base constructors stood in): the name
/// "AV<name>" it gives its form goes to JG's setter at `0x66FF57` with the row on and is found
/// by name; off, the exe's stub, and not found.
#[test]
fn an_actor_value_info_is_found_by_its_name() {
    for on in [true, false] {
        let mut e = engine(on);
        quiet(
            &mut e,
            &[
                0x0048_3370,
                0x0040_2d00,
                0x0048_2c50,
                0x0040_2c20,
                0x0040_37b0,
                0x004f_15a0,
                0x0043_8390,
            ],
        );
        stand_in_form_id(&mut e);
        // The actor value's form id (`0066fdf0`).
        e.register(0x0066_fdf0, |_, a| ret(0x2bc + a[0]));
        // `_snprintf(buffer, 0x64, "AV%s", name)`.
        e.register(0x0040_6d00, |e, a| {
            let mut text = b"AV".to_vec();
            text.extend(e.mem.cstr(a[3]));
            text.push(0);
            e.mem.write(a[0], &text);
            ret(0)
        });
        let form = AREA + 0x4000;
        let name = TEXTS + 0x200;
        e.mem.write(name, b"Health\0");
        e.call(0x0066_fe40, &args![form, 0x10u32, name]);
        assert_eq!(
            by_name(&mut e, b"avhealth"),
            if on { form } else { 0 },
            "{on}"
        );
    }
}

/// `BGSDefaultObjectManager`'s constructor (`0058d760`; the form constructor, type and id stood
/// in): its name "DefaultObjectManager" (`01032da4`) goes to JG's setter at `0x58D7BC` with the
/// row on and is found; off, not found.
#[test]
fn the_default_object_manager_is_found_by_its_name() {
    for on in [true, false] {
        let mut e = engine(on);
        quiet(&mut e, &[0x0048_3370, 0x004f_15a0]);
        stand_in_form_id(&mut e);
        e.mem.write(0x0103_2da4, b"DefaultObjectManager\0");
        let form = AREA + 0x5000;
        e.call(0x0058_d760, &args![form]);
        assert_eq!(
            by_name(&mut e, b"DefaultObjectManager"),
            if on { form } else { 0 },
            "{on}"
        );
    }
}

/// `TESForm::GetFormDetailedString` (`004868f0`; its id and type readers stood in, the format
/// recorded): the name it prints is JG's (`0x486903`) with the row on, the exe stub's "" off.
#[test]
fn a_forms_detailed_string_shows_its_name() {
    thread_local! {
        static PRINTED: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }
    for on in [true, false] {
        let mut e = engine(on);
        quiet(&mut e, &[0x0084_e3a0, 0x0044_0e30]);
        e.register(STRING_PRINTF, |e, a| {
            let name = e.mem.cstr(a[3]);
            PRINTED.with(|p| *p.borrow_mut() = name);
            ret(0)
        });
        e.call(0x0048_68f0, &args![CAPS, TEXTS + 0x300]);
        let want: &[u8] = if on { b"Caps001" } else { b"" };
        assert_eq!(PRINTED.with(|p| p.borrow().clone()), want, "{on}");
    }
}
