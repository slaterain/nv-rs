use super::*;
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x011d8000, 0x30000);
    e.map(0x02000000, 0x10000);
    e.mem.set_u32(GLOBALS[0], 0x02000000);
    e.mem.set_u32(GLOBALS[1], 0x02001000);
    e
}
#[test]
fn stock_exclusions_precede_search_and_actor_values_have_unknown_fallback() {
    let mut e = engine();
    e.register(0x0079b390, |_, _| ret(1));
    e.register(0x0079b720, |_, _| ret(1));
    e.register(0x0079be30, |_, _| ret(1));
    assert_eq!(note_filter(&mut e, &[0]).eax, 1);
    assert_eq!(challenge_filter(&mut e, &[0]).eax, 1);
    assert_eq!(radio_filter(&mut e, &[0]).eax, 1);
    state(&mut e, |s| s.menus[1].editor.text = b"known".to_vec());
    assert_eq!(av_filter(&mut e, &[77]).eax, 0);
    state(&mut e, |s| s.menus[1].editor.text = b"Agility".to_vec());
    assert_eq!(av_filter(&mut e, &[u32::MAX]).eax, 1);
}
#[test]
fn marker_visibility_and_name_are_both_required_only_while_hooked() {
    let mut e = engine();
    let data = 0x02003000;
    assert_eq!(marker(&mut e, &[data]), None);
    state(&mut e, |s| {
        s.menus[0].hooked = true;
        s.menus[0].editor.text = b"spring".to_vec();
    });
    let text = nvse::table::guest_str(&mut e, b"Goodsprings");
    e.mem.set_u32(data + 4, text);
    e.mem.set_u16(data + 8, 11);
    assert_eq!(marker(&mut e, &[data]), Some(0));
    e.mem.set_u8(data + 0xc, 1);
    assert_eq!(marker(&mut e, &[data]), Some(1));
    state(&mut e, |s| s.menus[0].editor.text = b"Novac".to_vec());
    assert_eq!(marker(&mut e, &[data]), Some(0));
}
#[test]
fn trait_lookup_reads_only_the_sorted_array() {
    let e = engine();
    let tile = 0x02003000;
    let array = 0x02003100;
    let value = 0x02003200;
    e.mem.set_u32(tile + 0x14, array);
    e.mem.set_u32(tile + 0x18, 1);
    e.mem.set_u32(array, value);
    e.mem.set_u32(value, 0xfc4);
    assert_eq!(trait_value(&e, tile, 0xfc4), value);
    assert_eq!(trait_value(&e, tile, 0xfa3), 0);
}
#[test]
fn scoped_map_refresh_restores_centering_after_native_failure() {
    let mut e = engine();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scope(&mut e, false, |e| {
            assert_eq!(skip_center(e, &[]), Some(1));
            panic!("stop")
        })))
        .is_err()
    );
    assert_eq!(skip_center(&mut e, &[]), None);
}

#[test]
fn quest_objective_visibility_and_completion_settings_do_not_bypass_name_search() {
    let mut e = engine();
    let (quest, objective, vtable, component) = (0x02004000, 0x02005000, 0x02006000, 0x02007000);
    e.mem.set_u32(quest, vtable);
    e.mem.set_u32(vtable + 0xf0, 0x03000000);
    e.register(0x03000000, |_, _| ret(0));
    e.register(0x00ec43fb, |_, _| ret(0x02007000));
    let title = nvse::table::guest_str(&mut e, b"A different quest");
    e.mem.set_u32(component + 4, title);
    e.mem.set_u16(component + 8, 17);
    let text = nvse::table::guest_str(&mut e, b"Find the hidden compound");
    e.mem.set_u32(quest + 0x4c, objective);
    e.mem.set_u32(objective, 0x01047088);
    e.mem.set_u32(objective + 8, text);
    e.mem.set_u16(objective + 0xc, 24);
    e.mem.set_u32(objective + 0x10, quest);
    e.mem.set_u32(objective + 0x20, 1);
    state(&mut e, |s| {
        s.objectives = true;
        s.menus[0].editor.text = b"hidden".to_vec();
    });
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 0);
    e.mem.set_u32(objective + 0x20, 3);
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 1);
    state(&mut e, |s| s.completed = true);
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 0);
    e.mem.set_u32(objective + 0x20, 0);
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 1);
    e.mem.set_u32(objective + 0x20, 1);
    e.mem.set_u32(objective, 0x0104708c);
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 1);
    state(&mut e, |s| {
        s.objectives = false;
        s.menus[0].editor.text = b"different".to_vec();
    });
    assert_eq!(quest_filter(&mut e, &[quest]).eax, 0);
}
#[test]
fn stats_misc_filters_tile_strings_not_row_data_and_restores_highlight() {
    let mut e = engine();
    let (list, first, second, tail, a, b) = (
        0x02002000, 0x02002100, 0x02002200, 0x02002300, 0x02002400, 0x02002500,
    );
    e.mem.set_u32(list + 4, first);
    e.mem.set_u32(list + 8, tail);
    e.mem.set_u32(tail, second);
    e.mem.set_u32(list + 0xc, 0x02002600);
    e.mem.set_u32(list + 0x10, a);
    for (row, tile, array, value, title) in [
        (first, a, 0x02002700, 0x02002800, b"Missing".as_slice()),
        (second, b, 0x02002900, 0x02002a00, b"Matching".as_slice()),
    ] {
        let text = nvse::table::guest_str(&mut e, title);
        e.mem.set_u32(row, tile);
        e.mem.set_u32(row + 4, 0xdeadbeef);
        e.mem.set_u32(tile + 0x14, array);
        e.mem.set_u32(tile + 0x18, 1);
        e.mem.set_u32(array, value);
        e.mem.set_u32(value, 0xfc4);
        e.mem.set_u32(value + 0xc, text);
    }
    e.register(0x00a00940, |_, _| ret(0x1234));
    e.register(0x00a011b0, |_, _| Ret::default());
    for address in [0x00a012d0, 0x007269d0, 0x0071ad30, 0x0071a740] {
        e.register(address, |_, _| Ret::default());
    }
    state(&mut e, |s| s.menus[1].editor.text = b"match".to_vec());
    filter_tiles(&mut e, list);
    assert_eq!(e.mem.u8(first + 8), 1);
    assert_eq!(e.mem.u8(second + 8), 0);
    assert_eq!(e.mem.u16(list + 0x1c), 1);
    assert_eq!(e.mem.u32(list + 0x10), b);
}

#[test]
fn pipboy_close_cleans_map_search_when_inventory_search_is_disabled() {
    let mut e = engine();
    let settings = Settings::new(
        &[(
            "test",
            b"[Tweaks]\nbMenuSearch=1\n[Menu Search]\nbInventoryMenu=0\n",
        )],
        true,
    );
    super::super::install(&mut e, &settings);
    for address in [0x00a01350, 0x00a012d0, 0x00a01860] {
        e.register(address, |_, _| Ret::default());
    }
    state(&mut e, |s| {
        s.menus[0].tile = 0x02003000;
        s.menus[0].hooked = true;
        s.menus[0].editor.active = true;
        s.menus[0].editor.text = b"Goodsprings".to_vec();
    });
    e.site(0x0070f6d3, &[]);
    assert!(!get(&e, 0).hooked);
    assert!(!get(&e, 0).editor.active);
    assert!(get(&e, 0).editor.text.is_empty());
}
