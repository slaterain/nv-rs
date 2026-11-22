//! The kept lookup of the `Mass` commands and the sets that would change nothing.

use super::*;

/// The `Tile::SetFloat` and `Tile::SetString` calls made while `f` runs: (tile, id, number or
/// text).
fn sets(e: &mut Engine, f: impl FnOnce(&mut Engine)) -> Vec<(u32, u32, Vec<u8>)> {
    let before = e.mem.u32(SET_CALLS);
    e.call_log = Some(vec![]);
    f(e);
    let log = e.call_log.take().unwrap();
    let out: Vec<_> = log
        .iter()
        .filter(|(at, _)| *at == SET_FLOAT || *at == SET_STRING)
        .map(|(at, w)| {
            let what = if *at == SET_FLOAT {
                w[2].to_le_bytes().to_vec()
            } else {
                b"text".to_vec()
            };
            (w[0], w[1], what)
        })
        .collect();
    assert_eq!(e.mem.u32(SET_CALLS) - before, out.len() as u32);
    out
}

/// A `Mass` command's kept lookup finds what the walk of each path finds, for every kind of part
/// (plain names in any case, `name:n`, `*`, missing parts, a trait under a child, every part a
/// child, other menus), and walks a tile's children again once its list changed.
#[test]
fn the_kept_lookup_finds_what_each_walk_finds() {
    let mut e = engine();
    let t = mcm_tree(&mut e);
    let paths: Vec<String> = [
        "StartMenu/MCM/_MCM",
        "startmenu/mcm/_mcm",
        "StartMenu/MCM/MCM_Options/Option3/_value",
        "StartMenu/MCM/mcm_options/OPTION3/_value",
        "StartMenu/MCM/MCM_Options/O*/_value",
        "StartMenu/MCM/MCM_Options/*/_value",
        "StartMenu/MCM/MCM_Options/Option3:0/_value",
        "StartMenu/MCM/MCM_Options/Option3:1/_value",
        "StartMenu/MCM/:1/_MCM",
        "StartMenu/MCM/nope/_MCM",
        "StartMenu/MCM/_MCM/x",
        "StartMenu/MCM/MCM_Options/Option3/value/text/string",
        "StartMenu/MCM/MCM_Options/Option3/value/text",
        "StartMenu//MCM\\MCM_ModList/Mod5/text/width",
        "StartMenu/MCM/MCM_ModList/SubMenu2/text/width",
        "HUDMainMenu/MCM/_MCM",
        "LastMenu/x",
        "NoMenu/x",
        "StartMenu",
    ]
    .iter()
    .map(|p| p.to_string())
    .chain((0..=6).map(|i| format!("StartMenu/MCM/MCM_Options/Option{i}/_enable")))
    .collect();
    let mut lookup = menu::Lookup::default();
    let mut found = 0;
    for _ in 0..2 {
        for p in &paths {
            let walked = menu::value_at(&mut e, p.as_bytes());
            found += usize::from(walked != 0);
            assert_eq!(lookup.value_at(&mut e, p.as_bytes()), walked, "{p}");
        }
    }
    assert!(found >= 2 * 12, "the paths find traits: {found}");
    // A child added behind the others: found once the list's last node changed.
    let options = menu::component(&mut e, t.mcm, b"MCM_Options/x").unwrap().0;
    let o9 = child(&mut e, options, "Option9");
    value(&mut e, o9, "_enable", 4.0);
    let p = b"StartMenu/MCM/MCM_Options/Option9/_enable";
    assert_ne!(menu::value_at(&mut e, p), 0);
    assert_eq!(lookup.value_at(&mut e, p), menu::value_at(&mut e, p));
}

/// A float set that would change nothing (the same bits, no string, no actions) is not made; one
/// that would (another number, -0.0 for 0.0, a NaN, a string to drop, actions to clear) is.
#[test]
fn an_idle_float_set_is_not_made() {
    let mut e = engine();
    mcm_tree(&mut e);
    let p = b"StartMenu/MCM/MCM_Options/Option3/_value";
    let v = menu::value_at(&mut e, p);
    assert_eq!(e.mem.f32(v + 8), 3.0);
    assert!(sets(&mut e, |e| menu::set_float(e, p, 3.0)).is_empty());
    assert_eq!(sets(&mut e, |e| menu::set_float(e, p, 4.0)).len(), 1);
    assert!(sets(&mut e, |e| menu::set_float(e, p, 4.0)).is_empty());
    let zero = b"StartMenu/MCM/MCM_Options/Option3/_enable";
    assert!(sets(&mut e, |e| menu::set_float(e, zero, 0.0)).is_empty());
    assert_eq!(sets(&mut e, |e| menu::set_float(e, zero, -0.0)).len(), 1);
    e.mem.set_f32(v + 8, f32::NAN);
    assert_eq!(sets(&mut e, |e| menu::set_float(e, p, f32::NAN)).len(), 1);
    e.mem.set_f32(v + 8, 4.0);
    // A string to drop, then an action to clear: the set goes through.
    let text = take(&mut e, 4);
    e.mem.set_u32(v + 0xc, text);
    assert_eq!(sets(&mut e, |e| menu::set_float(e, p, 4.0)).len(), 1);
    e.mem.set_u32(v + 0xc, 0);
    e.mem.set_u32(v + 0x10, 0x0400_8000);
    assert_eq!(sets(&mut e, |e| menu::set_float(e, p, 4.0)).len(), 1);
    e.mem.set_u32(v + 0x10, 0);
    // The kept lookup skips the same sets.
    let mut lookup = menu::Lookup::default();
    assert!(sets(&mut e, |e| lookup.set_float(e, p, 4.0)).is_empty());
    assert_eq!(sets(&mut e, |e| lookup.set_float(e, p, 5.0)).len(), 1);
}

/// A string set that would change nothing but the string's block (the same text, the number 0.0,
/// no actions) is not made; another text, a number to clear, or actions go through.
#[test]
fn an_idle_string_set_is_not_made() {
    let mut e = engine();
    mcm_tree(&mut e);
    let p = b"StartMenu/MCM/MCM_ModList/Mod5/text/string";
    let v = menu::value_at(&mut e, p);
    // No string yet: the set is made.
    assert_eq!(sets(&mut e, |e| menu::set_string(e, p, b"hi")).len(), 1);
    let text = take(&mut e, 8);
    e.mem.set_cstr(text, b"hi");
    e.mem.set_u32(v + 0xc, text);
    assert!(sets(&mut e, |e| menu::set_string(e, p, b"hi")).is_empty());
    for other in [&b"Hi"[..], b"h", b"hi!", b""] {
        assert_eq!(sets(&mut e, |e| menu::set_string(e, p, other)).len(), 1);
    }
    e.mem.set_f32(v + 8, 1.0);
    assert_eq!(sets(&mut e, |e| menu::set_string(e, p, b"hi")).len(), 1);
    e.mem.set_f32(v + 8, -0.0);
    assert_eq!(sets(&mut e, |e| menu::set_string(e, p, b"hi")).len(), 1);
    e.mem.set_f32(v + 8, 0.0);
    e.mem.set_u32(v + 0x10, 0x0400_8000);
    assert_eq!(sets(&mut e, |e| menu::set_string(e, p, b"hi")).len(), 1);
    let mut lookup = menu::Lookup::default();
    e.mem.set_u32(v + 0x10, 0);
    assert!(sets(&mut e, |e| lookup.set_string(e, p, b"hi")).is_empty());
}

/// The `Mass` commands, with their kept lookup, make the same sets in the same order as a walk of
/// each index's path and a set ([`menu::set_float`], [`menu::set_string`]), over two refreshes
/// (the second with every float already set, so only the strings' sets are made again: the test's
/// stand-in keeps no string).
#[test]
fn mass_commands_set_what_one_walk_per_index_sets() {
    struct Run {
        kind: i32,
        min: i32,
        max: i32,
        setting: &'static [u8],
        num: Option<f32>,
    }
    let runs = [
        Run {
            kind: 1,
            min: 0,
            max: 6,
            setting: b"_enable",
            num: Some(2.0),
        },
        Run {
            kind: 1,
            min: 1,
            max: 4,
            setting: b"_value",
            num: Some(3.0),
        },
        Run {
            kind: 5,
            min: 0,
            max: 30,
            setting: b"_enable",
            num: Some(1.0),
        },
        Run {
            kind: 5,
            min: 0,
            max: 30,
            setting: b"text/width",
            num: None,
        },
        Run {
            kind: 1,
            min: 1,
            max: 100_001,
            setting: b"_value",
            num: Some(6.0),
        },
    ];
    for r in runs {
        let mut kept = engine();
        mcm_tree(&mut kept);
        let mut walked = engine();
        mcm_tree(&mut walked);
        for refresh in 0..2 {
            let got = sets(&mut kept, |e| {
                let args = match r.num {
                    Some(num) => vec![
                        int(r.kind),
                        int(r.min),
                        int(r.max),
                        s(r.setting),
                        float(f64::from(num)),
                    ],
                    None => vec![int(r.kind), int(r.min), int(r.max), s(r.setting), s(b"w")],
                };
                let opcode = if r.num.is_some() { 0x21c5 } else { 0x21c6 };
                assert_eq!(call(e, opcode, &args).0, 1);
            });
            let want = sets(&mut walked, |e| {
                for index in mass_indices(e, r.kind, r.min, r.max) {
                    let p = path(r.kind, index, r.setting);
                    match r.num {
                        Some(num) => menu::set_float(e, &p, num),
                        None => menu::set_string(e, &p, b"w"),
                    }
                }
            });
            if refresh == 0 {
                assert!(!want.is_empty(), "{:?}: nothing set", r.setting);
            } else if r.num.is_some() {
                assert!(want.is_empty(), "the second refresh sets nothing new");
            }
            assert_eq!(got, want, "{:?} refresh {refresh}", r.setting);
        }
    }
}
