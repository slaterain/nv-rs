use super::*;
fn config() -> Settings {
    Settings::new(&[("test", b"[Tweaks]\nbMenuSearch=1\n")], true)
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x011d_8000, 0x30000);
    e.map(KEY_SLOT & !0xfff, 0x1000);
    e.map(0x0200_0000, 0x5000);
    e.mem.set_u32(MAP, 0x0200_0000);
    e.mem.set_u32(MENU, 0x0200_1000);
    e.mem.set_u32(0x0200_1004, 0x0200_2000);
    e.mem.set_u32(KEY_SLOT, 0x008d_0370);
    e.register(0x00a0_1b00, |e, w| {
        assert_eq!(w[0], 0x0200_2000);
        assert_eq!(e.mem.cstr(w[1]), PATH);
        0x0200_3000u32.into_ret()
    });
    e.register(0x00a0_1350, |_, _| Ret::default());
    e.register(0x00a0_12d0, |_, _| Ret::default());
    e.register(0x00a0_1860, |_, _| 0x1234u32.into_ret());
    e.register(0x008d_0370, |_, w| {
        assert_eq!(w, &[0x0200_1000, 0x8000_0001]);
        55u32.into_ret()
    });
    install(&mut e, &config());
    e
}
#[test]
fn injection_waits_for_map_and_missing_prefab_is_final() {
    let mut e = engine();
    e.mem.set_u32(MAP, 0);
    after_menus(&mut e);
    assert!(!snapshot(&e).attempted);
    assert_eq!(e.mem.u32(KEY_SLOT), 0x008d_0370);
    e.mem.set_u32(MAP, 1);
    e.register(0x00a0_1b00, |_, _| 0u32.into_ret());
    after_menus(&mut e);
    assert!(snapshot(&e).attempted);
    assert_eq!(snapshot(&e).tile, 0);
    assert_eq!(e.mem.u32(KEY_SLOT), 0x008d_0370);
}
#[test]
fn registered_filter_keeps_vanilla_exclusions_and_key_fallback() {
    let mut e = engine();
    after_menus(&mut e);
    assert_eq!(e.mem.u32(KEY_SLOT), KEY);
    e.register(ORIGINAL_FILTER, |_, _| 1u32.into_ret());
    assert_eq!(
        e.call(FILTER, &[0]).eax,
        1,
        "must not dereference excluded item"
    );
    assert_eq!(e.call(KEY, &[0x0200_1000, 0x8000_0001]).eax, 55);
    change(&mut e, |s| {
        s.hooked = true;
        s.editor.active = true;
        s.editor.text = b"retained".to_vec();
    });
    assert_eq!(e.site(FILTERS[0].addr(), &[ORIGINAL_FILTER]), Some(FILTER));
    e.site(CLOSE.addr(), &[]);
    assert!(snapshot(&e).editor.text.is_empty());
    assert!(!snapshot(&e).editor.active);
    assert_eq!(
        e.site(FILTERS[0].addr(), &[ORIGINAL_FILTER]),
        Some(ORIGINAL_FILTER)
    );
}
#[test]
fn editor_inserts_at_cursor_and_preserves_full_editor_rejections() {
    let mut ed = Editor {
        text: b"ab".to_vec(),
        cursor: 1,
        active: true,
        ..Editor::default()
    };
    assert!(edit(&mut ed, b'X' as u32, false, None));
    assert_eq!(ed.text, b"aXb");
    assert_eq!(ed.cursor, 2);
    assert!(edit(&mut ed, 0x8000_0005, false, None));
    assert_eq!(ed.cursor, 0);
    assert!(edit(&mut ed, 0x8000_0007, false, None));
    assert_eq!(ed.text, b"Xb");
    for key in [9, 96, 13, 10, 0x8000_0003] {
        assert!(!edit(&mut ed, key, false, None));
    }
    assert!(edit(&mut ed, 0x8000_0006, false, None));
    assert_eq!(ed.cursor, 2);
    assert!(edit(&mut ed, 0x8000_0008, false, None));
    assert!(!ed.active);
    assert_eq!(ed.text, b"Xb");
}
#[test]
fn paste_is_first_200_bytes_and_word_boundaries_are_alphanumeric() {
    let mut ed = Editor::default();
    let mut bytes = vec![b'x'; 205];
    bytes[0] = b'\t';
    bytes[1] = b'\n';
    assert!(edit(&mut ed, b'V' as u32, true, Some(&bytes)));
    assert_eq!(ed.text.len(), 198);
    assert!(edit(&mut ed, b'A' as u32, true, None));
    assert_eq!(ed.text.len(), 198, "other ctrl letters don't insert");
    assert_eq!(previous_word(b"aa--bb", 6), 2);
    assert_eq!(previous_word(b"aa--", 4), 4);
    assert_eq!(following_word(b"aa--bb", 0), 3);
    assert!(substring(b"Service Rifle", b"RIF"));
    assert!(!substring(b"Rifle", b"rif le"));
    assert!(substring(b"", b""));
    assert!(
        !substring(b"\xc9", b"\xe9"),
        "default private CRT locale only folds ASCII"
    );
}
#[test]
fn refresh_scope_restores_after_unwind_and_caret_threshold_is_strict() {
    let mut e = engine();
    after_menus(&mut e);
    e.register(0x0070_4af0, |e, _| {
        assert!(snapshot(e).refreshing);
        panic!("refresh failure")
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| force_refresh(&mut e))).is_err()
    );
    assert!(!snapshot(&e).refreshing);
    change(&mut e, |s| s.editor.active = true);
    e.register(0x0045_7fe0, |_, _| 530u32.into_ret());
    idle(&mut e, &[]);
    assert!(!snapshot(&e).editor.phase);
    e.register(0x0045_7fe0, |_, _| 531u32.into_ret());
    idle(&mut e, &[]);
    assert!(snapshot(&e).editor.phase);
    assert_eq!(snapshot(&e).editor.last, 531);
}

#[test]
fn name_match_and_keyring_specific_filter_use_real_form_contract() {
    let mut e = engine();
    after_menus(&mut e);
    let item = 0x0200_4000;
    let form = 0x0200_4100;
    let component = 0x0200_4200;
    let vt = 0x0200_4300;
    e.mem.set_u32(item + 8, form);
    e.mem.set_u32(form, vt);
    e.mem.set_u8(form + 4, 0x2e);
    e.mem.set_u32(vt + 0xf0, 0x00fc_0100);
    e.register(0x00fc_0100, |_, _| 0u32.into_ret());
    e.register(0x00ec_43fb, |_, w| {
        assert_eq!(w, &[0x0200_4100, 0, 0x0118_3028, 0x0118_3158, 0]);
        0x0200_4200u32.into_ret()
    });
    let text = nvse::table::guest_str(&mut e, b"Service Rifle Key");
    e.mem.set_u32(component + 4, text);
    e.mem.set_u16(component + 8, 17);
    e.register(ORIGINAL_FILTER, |_, _| 0u32.into_ret());
    change(&mut e, |s| s.editor.text = b"RIFLE".to_vec());
    assert_eq!(e.call(FILTER, &[item]).eax, 0);
    assert_eq!(e.call(KEY_FILTER, &[item]).eax, 0);
    e.mem.set_u8(form + 4, 0x28);
    assert_eq!(e.call(KEY_FILTER, &[item]).eax, 1);
    change(&mut e, |s| s.editor.text = b"missing".to_vec());
    assert_eq!(e.call(FILTER, &[item]).eax, 1);
}

#[test]
fn control_backspace_preserves_original_double_mutating_call() {
    let mut ed = Editor {
        text: b"aa bb".to_vec(),
        cursor: 5,
        active: true,
        ..Editor::default()
    };
    assert!(edit(&mut ed, 0x8000_0000, true, None));
    assert!(ed.text.is_empty());
    assert_eq!(
        ed.cursor, 0,
        "second call sees the original cursor and post-deletion buffer, as native RemoveWord does"
    );
}

#[test]
fn native_startup_only_injects_assets_after_real_threads_and_shared_slots_exist() {
    let mut e = engine();
    // Map every static UI vtable page without replacing Inventory's mapped page.
    e.map(0x01070000, 0x3000);
    e.map(0x01074000, 0x4000);
    for global in [
        0x011da368, 0x011dace0, 0x011d93f8, 0x011d9fdc, 0x011d8e90, 0x011d8fa4, 0x011daac0,
    ] {
        let menu = e.mem.alloc(0x300);
        let root = e.mem.alloc(0x100);
        e.mem.set_u32(menu + 4, root);
        e.mem.set_u32(global, menu);
    }
    e.register(0x00a01b00, |e, w| {
        assert!(e.mem.cstr(w[1]).ends_with(b".xml"));
        e.mem.alloc(0x100).into_ret()
    });
    let sites = [0x007272f2, 0x00726f2a, 0x0079af0d, 0x007dc504, 0x00781bbd];
    for site in sites {
        assert_eq!(e.site_owners(site), vec![Owner::Stewie]);
    }
    let mut child = Engine::new();
    e.share_slots_for_test(&mut child);
    assert_eq!(snapshot(&child).tile, 0);
    // This sets e.real; any late on_site or ext_free slot creation now asserts.
    engine_core::real_threads::start(&mut e).expect("headless real-thread context");
    e.site(0x0086b650, &[]);
    assert_ne!(
        snapshot(&child).tile,
        0,
        "asset state is published through the pre-existing shared slot"
    );
    let original = snapshot(&e).original_key;
    e.site(0x0086b650, &[]);
    assert_eq!(
        snapshot(&e).original_key,
        original,
        "repeat startup does not capture our own alias"
    );
    assert_eq!(child.site_owners(0x00726f2a), vec![Owner::Stewie]);
}
