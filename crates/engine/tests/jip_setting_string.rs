//! JIP SetStringSetting through its real command record and the translated
//! native Setting::Set (00C33170), using the actual strict heap. Static values
//! and native key/value joint allocations must never be freed as value blocks.
use engine as _;
use engine_core::prelude::*;

const AREA: u32 = 0x0500_0000;
const STRING_SETTING: u32 = AREA;
const NUMBER_SETTING: u32 = AREA + 0x20;
const STRING_KEY: u32 = AREA + 0x100;
const NUMBER_KEY: u32 = AREA + 0x140;
// The static default implicated by the owned FalloutNV.exe 1.4.0.525 crash.
const EXE_DEFAULT: u32 = 0x0105_ad48;
const SET_STRING: u32 = 0x00c3_3170;
const HEAP_FREE: u32 = 0x00aa_4060;

fn fixture(mut e: Engine, actual_exe: bool) -> Engine {
    jip::test_support::install_into(&mut e);
    e.map(AREA, 0x1000);
    if !actual_exe {
        // A generated image-shaped static page; never a tracked heap block.
        // No bytes from the player's executable are embedded in this test.
        e.map(EXE_DEFAULT & !0xfff, 0x1000);
        e.mem
            .set_cstr(EXE_DEFAULT, b"generated static setting default");
    }
    assert!(!e.mem.cstr(EXE_DEFAULT).is_empty());
    assert_eq!(e.mem.block_size(EXE_DEFAULT), None);
    e.mem.set_cstr(STRING_KEY, b"sPickpocketFail");
    e.mem.set_cstr(NUMBER_KEY, b"fUnchanged");
    e.mem.set_u32(STRING_SETTING + 4, EXE_DEFAULT);
    e.mem.set_u32(STRING_SETTING + 8, STRING_KEY);
    e.mem.set_u32(NUMBER_SETTING + 4, 1.25f32.to_bits());
    e.mem.set_u32(NUMBER_SETTING + 8, NUMBER_KEY);
    for at in [
        0x011c_8048,
        0x011f_96a0,
        0x011f_35a0,
        0x011c_c694,
        0x011f_35a4,
    ] {
        if !e.mem.is_mapped(at) {
            e.map(at & !0xfff, 0x1000);
        }
        let collection = e.mem.alloc(0x200);
        e.mem.write(collection, &[0; 0x200]);
        e.mem.set_u32(at, collection);
        if at == 0x011c_8048 {
            let bucket = e.mem.alloc(4);
            let first = e.mem.alloc(12);
            let second = e.mem.alloc(12);
            e.mem.write(first, &[0; 12]);
            e.mem.write(second, &[0; 12]);
            e.mem.set_u32(collection + 0x110, 1);
            e.mem.set_u32(collection + 0x114, bucket);
            e.mem.set_u32(bucket, first);
            e.mem.set_u32(first, second);
            e.mem.set_u32(first + 8, STRING_SETTING);
            e.mem.set_u32(second + 8, NUMBER_SETTING);
        }
    }
    e
}

fn set(e: &mut Engine, name: &[u8], value: &[u8]) {
    let (handled, result) = jip::test_support::call(
        e,
        0x2284,
        &[jip::test_support::s(name), jip::test_support::s(value)],
    );
    assert_eq!((handled, result), (1, 0.0));
}

fn check_replacements(mut e: Engine) {
    let original = e.mem.cstr(EXE_DEFAULT);
    e.call_log = Some(Vec::new());
    let mut previous = None;
    for value in [b"first replacement".as_slice(), b"second replacement", b""] {
        set(&mut e, b"sPICKPOCKETFAIL", value);
        let key = e.mem.u32(STRING_SETTING + 8);
        let text = e.mem.u32(STRING_SETTING + 4);
        assert_eq!(e.mem.cstr(key), b"SPickpocketFail");
        assert_eq!(text, key + b"sPickpocketFail".len() as u32 + 1);
        assert!(
            e.mem.block_size(key).is_some(),
            "the key owns the joint block"
        );
        assert_eq!(
            e.mem.block_size(text),
            None,
            "the value is an interior pointer"
        );
        assert_eq!(e.mem.cstr(text), value);
        if let Some(old) = previous {
            assert_ne!(
                old, key,
                "replacement allocated before releasing the old key"
            );
            assert_eq!(e.mem.block_size(old), None, "old joint block released");
        }
        previous = Some(key);
    }
    assert_eq!(
        e.mem.cstr(EXE_DEFAULT),
        original,
        "static default untouched"
    );
    assert_eq!(
        e.mem.cstr(STRING_KEY),
        b"sPickpocketFail",
        "static key untouched"
    );
    let log = e.call_log.take().unwrap();
    assert_eq!(log.iter().filter(|(at, _)| *at == SET_STRING).count(), 3);
    let freed: Vec<_> = log
        .iter()
        .filter(|(at, _)| *at == HEAP_FREE)
        .map(|(_, w)| w[1])
        .collect();
    assert_eq!(freed.len(), 2);
    assert!(!freed.contains(&EXE_DEFAULT));
    assert!(!freed.contains(&STRING_KEY));
    // Prefix validation and failed lookup preserve command success and mutate nothing.
    let current = e.mem.u32(STRING_SETTING + 4);
    e.call_log = Some(Vec::new());
    set(&mut e, b"fUnchanged", b"not a string");
    set(&mut e, b"sMissing", b"not found");
    assert_eq!(e.mem.u32(NUMBER_SETTING + 4), 1.25f32.to_bits());
    assert_eq!(e.mem.u32(NUMBER_SETTING + 8), NUMBER_KEY);
    assert_eq!(e.mem.u32(STRING_SETTING + 4), current);
    assert!(!e
        .call_log
        .take()
        .unwrap()
        .iter()
        .any(|(at, _)| *at == SET_STRING));
}

#[test]
fn actual_command_uses_native_static_and_joint_string_ownership() {
    check_replacements(fixture(Engine::new_untyped_for_test(), false));
}

/// Optional proof with the player's real static image, without Main/window/audio.
#[test]
#[ignore = "needs the player's FalloutNV.exe via NV_SETTING_EXE"]
fn actual_executable_static_default_is_replaced_without_freeing_image_bytes() {
    let path = std::env::var_os("NV_SETTING_EXE").expect("NV_SETTING_EXE");
    let engine = Engine::with_exe(std::path::Path::new(&path)).expect("FalloutNV.exe 1.4.0.525");
    check_replacements(fixture(engine, true));
}
