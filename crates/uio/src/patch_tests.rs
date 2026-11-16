use super::*;

/// A tile list: tile -> paths, inserted in the order given.
fn tiles(entries: &[(&str, &[&str])]) -> Map<Set> {
    let mut m: Map<Set> = Map::default();
    for (tile, paths) in entries {
        let set = m.entry(tile.as_bytes());
        for p in *paths {
            set.insert(p.as_bytes());
        }
    }
    m
}

fn patched(text: &str, t: Map<Set>) -> String {
    String::from_utf8(patch(text.as_bytes(), t)).unwrap()
}

#[test]
fn includes_go_before_the_closing_tag_of_their_tile() {
    let text =
        r#"<menu name="HUDMainMenu"><rect name="a"><x>1</x></rect><image name="b"></image></menu>"#;
    let got = patched(
        text,
        tiles(&[("HUDMainMenu", &["mod\\hud.xml"]), ("b", &["b.xml"])]),
    );
    assert_eq!(
        got,
        r#"<menu name="HUDMainMenu"><rect name="a"><x>1</x></rect><image name="b"><include src="b.xml"/></image><include src="mod\hud.xml"/></menu>"#
    );
}

#[test]
fn text_between_the_last_tag_and_the_closing_tag_comes_after_the_includes() {
    let text = r#"<rect name="r"><x>1</x>tail</rect>more"#;
    let got = patched(text, tiles(&[("r", &["i.xml"])]));
    // The includes are written when the closing tag is reached, before the text not yet copied
    // ("tail"); once no tile is left the rest is copied whole.
    assert_eq!(
        got,
        r#"<rect name="r"><x>1</x><include src="i.xml"/>tail</rect>more"#
    );
}

#[test]
fn tile_names_match_without_case_and_the_first_spelling_of_a_path_is_written() {
    let mut t = tiles(&[("glow_branch", &["WMM\\wmm.xml"])]);
    t.entry(b"GLOW_BRANCH").insert(b"wmm\\WMM.XML");
    let text = r#"<menu name="InventoryMenu"><rect name="Glow_Branch"></rect></menu>"#;
    assert_eq!(
        patched(text, t),
        r#"<menu name="InventoryMenu"><rect name="Glow_Branch"><include src="WMM\wmm.xml"/></rect></menu>"#
    );
}

#[test]
fn includes_of_a_tile_follow_uios_set_order() {
    // Two paths in different buckets and two in one: bucket order, newest first in a bucket.
    let paths: Vec<String> = (0..40).map(|i| format!("p{i}.xml")).collect();
    let in_bucket = |b: u32| {
        paths
            .iter()
            .filter(move |p| str_hash(p.as_bytes()) & 7 == b)
    };
    let mut b3 = in_bucket(3);
    let (x, y) = (b3.next().unwrap(), b3.next().unwrap());
    let z = in_bucket(1).next().unwrap();
    let t = tiles(&[("r", &[x.as_str(), y.as_str(), z.as_str()])]);
    let got = patched(r#"<rect name="r"></rect>"#, t);
    assert_eq!(
        got,
        format!(
            r#"<rect name="r"><include src="{z}"/><include src="{y}"/><include src="{x}"/></rect>"#
        )
    );
}

#[test]
fn end_of_file_includes_come_last_and_the_text_after_the_last_tag_is_dropped() {
    let text = r#"<template name="t"><rect></rect></template>trailing"#;
    let got = patched(text, tiles(&[("", &["e1.xml", "e2.xml"])]));
    let set = tiles(&[("", &["e1.xml", "e2.xml"])]);
    let order: Vec<String> = set
        .get(b"")
        .unwrap()
        .iter()
        .map(|p| format!(r#"<include src="{}"/>"#, String::from_utf8_lossy(p)))
        .collect();
    assert_eq!(
        got,
        format!(
            r#"<template name="t"><rect></rect></template>{}"#,
            order.concat()
        )
    );
}

#[test]
fn a_tile_not_found_adds_nothing() {
    let text = r#"<rect name="a"></rect>"#;
    assert_eq!(patched(text, tiles(&[("missing", &["m.xml"])])), text);
}

#[test]
fn duplicate_and_includes_references_inside_the_tile_are_dropped() {
    // No `name=` after the include tags, so they are read as includes (UIO's search for `name=`
    // runs on to near the end of the file).
    let text = concat!(
        r#"<menu name="M"><include src="a.xml"/><include src="includes_hud.xml"/>"#,
        r#"<include src="other.xml"/></menu>"#
    );
    let got = patched(text, tiles(&[("M", &["a.xml"])]));
    assert_eq!(
        got,
        r#"<menu name="M"><include src="other.xml"/><include src="a.xml"/></menu>"#
    );
}

#[test]
fn a_name_later_in_the_file_hides_an_include_from_the_check() {
    // `name=` found in the later tag: the include tag is taken for a named tag whose quote lies
    // past its end, so it is neither a tile nor checked: kept.
    let text = r#"<menu name="M"><include src="a.xml"/><rect name="later"></rect></menu>"#;
    let got = patched(text, tiles(&[("M", &["a.xml"])]));
    assert_eq!(
        got,
        concat!(
            r#"<menu name="M"><include src="a.xml"/><rect name="later"></rect>"#,
            r#"<include src="a.xml"/></menu>"#
        )
    );
}

#[test]
fn a_self_closing_named_tag_opens_a_tile_that_the_next_closing_tag_ends() {
    // `<image name="s"/>` is pushed and never popped by its own tag: `</rect>` then ends "s",
    // and the menu's closing tag ends "r" (UIO's walk).
    let text = r#"<menu name="m"><rect name="r"><image name="s"/></rect></menu>"#;
    let got = patched(text, tiles(&[("r", &["r.xml"]), ("s", &["s.xml"])]));
    assert_eq!(
        got,
        r#"<menu name="m"><rect name="r"><image name="s"/><include src="s.xml"/></rect><include src="r.xml"/></menu>"#
    );
}

#[test]
fn short_tags_and_other_closing_tags_are_copied() {
    let text = r#"<menu name="m"><a>x</a><string>s</string></menu>"#;
    let got = patched(text, tiles(&[("m", &["i.xml"])]));
    assert_eq!(
        got,
        r#"<menu name="m"><a>x</a><string>s</string><include src="i.xml"/></menu>"#
    );
}

#[test]
fn odd_text_never_stops_the_patch() {
    for text in [
        "",
        "<",
        ">",
        "<<<<>>>>",
        "</rect>",
        r#"<rect name="unterminated>"#,
        r#"<rect name=""></rect>"#,
        "\u{e9}\u{e8}<rect name=\"\u{e9}\"></rect>",
    ] {
        let _ = patch(
            text.as_bytes(),
            tiles(&[("x", &["a.xml"]), ("", &["e.xml"])]),
        );
    }
    // A text with bytes that are not UTF-8.
    let _ = patch(
        b"<rect name=\"\xff\"></rect>\xfe",
        tiles(&[("\u{ff}", &["a.xml"])]),
    );
    // `name=""` (an empty value) opens no tile.
    assert_eq!(
        patched(r#"<rect name=""></rect>"#, tiles(&[("", &["e.xml"])])),
        r#"<rect name=""></rect><include src="e.xml"/>"#
    );
}

#[test]
fn a_supported_txt_entry_end_to_end() {
    // The JIP CC&C HUD entry of UIO's supported.txt, into a HUD file of the game's form (one line,
    // compressed by the game).
    let hud = concat!(
        r#"<menu name="HUDMainMenu"><class>&HUDMainMenu;</class>"#,
        r#"<rect name="JHUD"><visible>0</visible></rect></menu>"#
    );
    let got = patched(hud, tiles(&[("hudmainmenu", &["jazzisparis\\ccchud.xml"])]));
    assert_eq!(
        got,
        concat!(
            r#"<menu name="HUDMainMenu"><class>&HUDMainMenu;</class>"#,
            r#"<rect name="JHUD"><visible>0</visible></rect>"#,
            r#"<include src="jazzisparis\ccchud.xml"/></menu>"#
        )
    );
}

#[test]
fn a_tile_the_text_never_closes_is_named_with_its_count_and_the_text_is_unchanged() {
    let text = r#"<menu name="M"><rect name="a"></rect></menu>"#;
    let t = || {
        tiles(&[
            ("a", &["x.xml"]),
            ("gone", &["y.xml", "z.xml"]),
            ("", &["e.xml"]),
        ])
    };
    let (out, missing) = patch_noted(text.as_bytes(), t());
    assert_eq!(out, patch(text.as_bytes(), t()));
    assert_eq!(missing, vec![(b"gone".to_vec(), 2)]);
    // Every tile found: nothing named.
    let (_, missing) = patch_noted(text.as_bytes(), tiles(&[("a", &["x.xml"])]));
    assert!(missing.is_empty());
}
