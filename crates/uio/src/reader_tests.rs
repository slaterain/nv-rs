use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// The game's files by UIO's hash of the path, and the reads made.
struct Disk {
    files: HashMap<u32, Vec<u8>>,
    reads: RefCell<Vec<Vec<u8>>>,
}

impl Disk {
    fn new(files: &[(&str, &str)]) -> Disk {
        Disk {
            files: files
                .iter()
                .map(|(p, t)| (str_hash(p.as_bytes()), t.as_bytes().to_vec()))
                .collect(),
            reads: RefCell::new(Vec::new()),
        }
    }

    fn read(&self, path: &[u8]) -> Option<Vec<u8>> {
        self.reads.borrow_mut().push(path.to_vec());
        self.files.get(&str_hash(path)).cloned()
    }
}

fn read(patched: &mut Patched, disk: &Disk, path: &str) -> Option<String> {
    let mut raw = |p: &[u8]| disk.read(p);
    Reader::new(patched, &mut raw)
        .file(path.as_bytes())
        .map(|t| String::from_utf8(t).unwrap())
}

#[test]
fn includes_are_replaced_by_the_prefabs_text_in_turn() {
    let disk = Disk::new(&[
        (
            "Data\\Menus\\m.xml",
            r#"<menu><include src="a.xml"/><x/><include src="sub\b.xml"/></menu>"#,
        ),
        (
            "data\\menus\\prefabs\\a.xml",
            r#"<rect><include src="sub\b.xml"/></rect>"#,
        ),
        ("Data\\Menus\\Prefabs\\SUB\\B.XML", "<image/>"),
    ]);
    let mut p = Patched::default();
    assert_eq!(
        read(&mut p, &disk, "Data\\Menus\\m.xml").unwrap(),
        "<menu><rect><image/></rect><x/><image/></menu>"
    );
    // The paths are UIO's: `data\menus\prefabs\` and the src as written.
    assert!(disk
        .reads
        .borrow()
        .contains(&b"data\\menus\\prefabs\\sub\\b.xml".to_vec()));
}

#[test]
fn a_missing_file_answers_nothing_and_a_missing_prefab_inserts_nothing() {
    let disk = Disk::new(&[("m.xml", r#"<a><include src="gone.xml"/></a>"#)]);
    let mut p = Patched::default();
    assert_eq!(read(&mut p, &disk, "missing.xml"), None);
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), "<a></a>");
}

#[test]
fn short_paths_and_tags_without_quotes_are_dropped() {
    let disk = Disk::new(&[
        // A 4-byte src: the tag goes, nothing comes in.
        ("m.xml", r#"<a><include src="abcd"/></a>"#),
        // No quote after byte 13: dropped.
        ("n.xml", r#"<a><include src=abcdefgh   /></a>"#),
        ("data\\menus\\prefabs\\abcd", "NO"),
    ]);
    let mut p = Patched::default();
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), "<a></a>");
    assert_eq!(read(&mut p, &disk, "n.xml").unwrap(), "<a></a>");
}

#[test]
fn a_short_tag_takes_in_the_next_one() {
    // `<include src="abc"/>` is 20 bytes: its `>` is at byte 19, before where UIO starts looking,
    // so the tag runs to the next `>` (the `<b>`), and its 3-byte path is dropped.
    let disk = Disk::new(&[("m.xml", r#"<a><include src="abc"/><b>t</b></a>"#)]);
    let mut p = Patched::default();
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), "<a>t</b></a>");
}

#[test]
fn an_include_with_no_end_keeps_the_rest_as_it_is() {
    let disk = Disk::new(&[
        ("m.xml", r#"<include src="x.xml"/><include src="y"#),
        ("data\\menus\\prefabs\\x.xml", "X"),
    ]);
    let mut p = Patched::default();
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), r#"X<include src="y"#);
    // With none found at all the text is unchanged.
    let disk = Disk::new(&[("m.xml", "<include")]);
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), "<include");
}

#[test]
fn patched_menus_and_prefabs_are_served_as_uio_serves_them() {
    let disk = Disk::new(&[
        ("Data\\Menus\\dialog_menu.xml", "DISK"),
        ("Data\\Menus\\Main\\hud_main_menu.xml", "HUDDISK"),
        ("data\\menus\\prefabs\\p.xml", "PDISK"),
    ]);
    let mut p = Patched::default();
    p.insert(b"Data\\Menus\\dialog_menu.xml", b"PATCHED".to_vec());
    p.insert(b"Data\\Menus\\Main\\hud_main_menu.xml", b"HUD".to_vec());
    p.insert(b"Data\\menus\\prefabs\\p.xml", b"P".to_vec());
    // A second insert of the same file keeps the first (`Emplace`).
    p.insert(b"DATA\\MENUS\\DIALOG_MENU.XML", b"OTHER".to_vec());
    assert_eq!(p.len(), 3);
    // A menu whose parse UIO keeps: its patched text at every read.
    for _ in 0..3 {
        assert_eq!(
            read(&mut p, &disk, "Data\\Menus\\Dialog_Menu.xml").unwrap(),
            "PATCHED"
        );
    }
    // One of the seven it does not keep: patched once, then the file again.
    assert_eq!(
        read(&mut p, &disk, "Data\\Menus\\Main\\hud_main_menu.xml").unwrap(),
        "HUD"
    );
    assert_eq!(
        read(&mut p, &disk, "Data\\Menus\\Main\\hud_main_menu.xml").unwrap(),
        "HUDDISK"
    );
    // A patched prefab, through an include, at every read.
    let disk2 = Disk::new(&[("m.xml", r#"<include src="p.xml"/><include src="p.xml"/>"#)]);
    assert_eq!(read(&mut p, &disk2, "m.xml").unwrap(), "PP");
    // The game's file was never read for a patched one.
    assert!(!disk2.reads.borrow().iter().any(|r| r.ends_with(b"p.xml")));
}

#[test]
fn a_patched_text_has_its_includes_expanded_once() {
    let disk = Disk::new(&[("data\\menus\\prefabs\\in.xml", "IN")]);
    let mut p = Patched::default();
    p.insert(
        b"Data\\Menus\\x.xml",
        br#"<a><include src="in.xml"/></a>"#.to_vec(),
    );
    assert_eq!(
        read(&mut p, &disk, "Data\\Menus\\x.xml").unwrap(),
        "<a>IN</a>"
    );
    assert_eq!(
        read(&mut p, &disk, "Data\\Menus\\x.xml").unwrap(),
        "<a>IN</a>"
    );
    assert_eq!(disk.reads.borrow().len(), 1, "the prefab read once");
}

#[test]
fn include_loops_end_without_a_crash() {
    // A file that includes itself, and two that include each other: UIO runs out of stack;
    // here the depth limit ends it.
    let disk = Disk::new(&[
        (
            "data\\menus\\prefabs\\self.xml",
            r#"S<include src="self.xml"/>"#,
        ),
        (
            "data\\menus\\prefabs\\one.xml",
            r#"1<include src="two.xml"/>"#,
        ),
        (
            "data\\menus\\prefabs\\two.xml",
            r#"2<include src="one.xml"/>"#,
        ),
    ]);
    let mut p = Patched::default();
    let s = read(&mut p, &disk, "data\\menus\\prefabs\\self.xml").unwrap();
    assert_eq!(s, "S".repeat(MAX_DEPTH as usize + 1));
    let s = read(&mut p, &disk, "data\\menus\\prefabs\\one.xml").unwrap();
    assert_eq!(s.len(), MAX_DEPTH as usize + 1);
    assert!(s.starts_with("1212"));
}

#[test]
fn an_include_bomb_stops_at_the_text_limit() {
    // Each level includes the next twice: 2^40 copies in UIO; here the limit ends it.
    let mut files: Vec<(String, String)> = (0..40)
        .map(|i| {
            (
                format!("data\\menus\\prefabs\\l{i:02}.xml"),
                format!(
                    r#"<include src="l{:02}.xml"/><include src="l{:02}.xml"/>"#,
                    i + 1,
                    i + 1
                ),
            )
        })
        .collect();
    files.push(("data\\menus\\prefabs\\l40.xml".into(), "x".repeat(1 << 20)));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let mut p = Patched::default();
    let mut raw = |q: &[u8]| disk.read(q);
    let t = Reader::new(&mut p, &mut raw)
        .file(b"data\\menus\\prefabs\\l00.xml")
        .unwrap();
    assert!(t.len() <= MAX_TEXT);
    assert!(!t.is_empty());
}

#[test]
fn an_include_bomb_without_text_stops_at_the_read_limit() {
    // Forty levels of two includes each and no text: 2^40 reads in UIO; here at most MAX_READS.
    let files: Vec<(String, String)> = (0..40)
        .map(|i| {
            (
                format!(r"data\menus\prefabs\e{i:02}.xml"),
                format!(
                    r#"<include src="e{:02}.xml"/><include src="e{:02}.xml"/>"#,
                    i + 1,
                    i + 1
                ),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let mut p = Patched::default();
    assert_eq!(
        read(&mut p, &disk, r"data\menus\prefabs\e00.xml").unwrap(),
        ""
    );
    assert!(disk.reads.borrow().len() <= MAX_READS as usize + 1);
}
