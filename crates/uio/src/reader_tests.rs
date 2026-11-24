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

    fn read(&self, path: &[u8], room: usize) -> Fetch {
        self.reads.borrow_mut().push(path.to_vec());
        match self.files.get(&str_hash(path)) {
            None => Fetch::Missing,
            Some(t) if t.len() > room => Fetch::TooLarge,
            Some(t) => Fetch::Text(t.clone()),
        }
    }
}

/// The reader's source over the patched files and a disk.
struct Test<'a> {
    patched: &'a mut Patched,
    disk: &'a Disk,
}

impl Source for Test<'_> {
    fn serve(&mut self, hash: u32) -> Option<Served> {
        self.patched.serve(hash)
    }
    fn keep(&mut self, hash: u32, text: Vec<u8>) {
        self.patched.keep(hash, text);
    }
    fn raw(&mut self, path: &[u8], room: usize) -> Fetch {
        self.disk.read(path, room)
    }
}

fn read_bytes(patched: &mut Patched, disk: &Disk, path: &[u8]) -> Option<Vec<u8>> {
    Reader::new(&mut Test { patched, disk }).file(path)
}

fn read(patched: &mut Patched, disk: &Disk, path: &str) -> Option<String> {
    read_bytes(patched, disk, path.as_bytes()).map(|t| String::from_utf8(t).unwrap())
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
    let t = read_bytes(&mut p, &disk, b"data\\menus\\prefabs\\l00.xml").unwrap();
    assert!(t.len() <= MAX_TEXT);
    assert_eq!(t.len(), 64 << 20, "64 copies of 1 MiB fill it");
    // Once full, the read fetches no further include: about 64 leaves and the levels above
    // them, not MAX_READS reads.
    let reads = disk.reads.borrow().len();
    assert!(reads < 300, "{reads} reads");
}

/// Files that fill the text limit exactly: every copy goes in (the limit is "at most").
#[test]
fn a_bomb_landing_exactly_on_the_text_limit_keeps_every_copy() {
    let mut files: Vec<(String, String)> = (0..6)
        .map(|i| {
            (
                format!("data\\menus\\prefabs\\b{i:02}.xml"),
                format!(
                    r#"<include src="b{:02}.xml"/><include src="b{:02}.xml"/>"#,
                    i + 1,
                    i + 1
                ),
            )
        })
        .collect();
    files.push(("data\\menus\\prefabs\\b06.xml".into(), "x".repeat(1 << 20)));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let mut p = Patched::default();
    let t = read_bytes(&mut p, &disk, b"data\\menus\\prefabs\\b00.xml").unwrap();
    assert_eq!(t.len(), MAX_TEXT);
    assert_eq!(disk.reads.borrow().len(), 127, "1 + 2 + 4 + ... + 64 reads");
}

/// Exactly MAX_READS includes are read; the next one gives nothing.
#[test]
fn a_read_reads_exactly_max_reads_files_for_its_includes() {
    let top = r#"<include src="zzzzz.xml"/>"#.repeat(MAX_READS as usize + 1);
    let disk = Disk::new(&[("m.xml", &top), ("data\\menus\\prefabs\\zzzzz.xml", "Z")]);
    let mut p = Patched::default();
    let t = read(&mut p, &disk, "m.xml").unwrap();
    assert_eq!(t, "Z".repeat(MAX_READS as usize));
    assert_eq!(disk.reads.borrow().len(), MAX_READS as usize + 1);
}

/// The search for each `<include` goes forward from the last one and stops at the first match:
/// the work is about the text's length, not the includes times the length.
#[test]
fn includes_are_found_in_one_pass_over_the_text() {
    let top = r#"<x/><include src="aaaaa.xml"/>"#.repeat(2000);
    let disk = Disk::new(&[("m.xml", &top), ("data\\menus\\prefabs\\aaaaa.xml", "A")]);
    let mut p = Patched::default();
    FIND_WORK.with(|w| w.set(0));
    let t = read(&mut p, &disk, "m.xml").unwrap();
    assert_eq!(t, "<x/>A".repeat(2000));
    let work = FIND_WORK.with(std::cell::Cell::get);
    assert!(
        work <= top.len(),
        "{work} bytes looked at for {}",
        top.len()
    );
}

/// UIO copies an include's path into `kPrefabsPath[0x80]`: a path that fits (with its 0) is read;
/// a longer one, up to a 1 MiB one, gives nothing and is never read.
#[test]
fn an_include_path_longer_than_uios_buffer_gives_nothing() {
    let fits = format!("{}.xml", "f".repeat(UIO_PATH - PREFABS.len() - 4));
    let over = format!("{}.xml", "o".repeat(UIO_PATH - PREFABS.len() - 3));
    let huge = format!("{}.xml", "h".repeat(1 << 20));
    let top = format!(r#"<include src="{fits}"/>|<include src="{over}"/>|<include src="{huge}"/>"#);
    let fits_path = format!("data\\menus\\prefabs\\{fits}");
    let over_path = format!("data\\menus\\prefabs\\{over}");
    assert_eq!(fits_path.len(), UIO_PATH);
    let disk = Disk::new(&[("m.xml", &top), (&fits_path, "FITS"), (&over_path, "OVER")]);
    let mut p = Patched::default();
    assert_eq!(read(&mut p, &disk, "m.xml").unwrap(), "FITS||");
    assert_eq!(
        disk.reads.borrow().len(),
        2,
        "the top file and the one that fits"
    );
}

/// A patched file expanded while a limit refused one of its includes (here the depth limit, deep
/// in a chain) is not kept so: the next read expands it whole, and that is kept.
#[test]
fn a_patched_file_expanded_under_a_limit_is_not_kept_cut_short() {
    let mut files: Vec<(String, String)> = (0..MAX_DEPTH)
        .map(|i| {
            let next = if i + 1 == MAX_DEPTH {
                "ppppp.xml".to_string()
            } else {
                format!("c{:03}x.xml", i + 1)
            };
            (
                format!("data\\menus\\prefabs\\c{i:03}x.xml"),
                format!(r#"<include src="{next}"/>"#),
            )
        })
        .collect();
    files.push(("data\\menus\\prefabs\\inner.xml".into(), "IN".into()));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let mut p = Patched::default();
    let pp = b"data\\menus\\prefabs\\ppppp.xml";
    p.insert(pp, br#"<p><include src="inner.xml"/></p>"#.to_vec());
    // At depth 64 the patched file's own include is refused.
    assert_eq!(
        read(&mut p, &disk, "data\\menus\\prefabs\\c000x.xml").unwrap(),
        "<p></p>"
    );
    assert!(!p.expanded(pp), "cut short: not kept");
    assert_eq!(
        read(&mut p, &disk, "data\\menus\\prefabs\\ppppp.xml").unwrap(),
        "<p>IN</p>"
    );
    assert!(p.expanded(pp));
}

/// The expanded texts kept stay under the limit: past it a file keeps its patched text and is
/// expanded again at each read (the same text).
#[test]
fn expanded_texts_past_the_limit_are_expanded_again_at_each_read() {
    let inner = "x".repeat(100);
    let disk = Disk::new(&[("data\\menus\\prefabs\\inner.xml", &inner)]);
    let pp = b"data\\menus\\prefabs\\ppppp.xml";
    let patched = br#"<include src="inner.xml"/>"#.to_vec();
    let mut small = Patched::with_limit(99);
    small.insert(pp, patched.clone());
    for _ in 0..2 {
        assert_eq!(read_bytes(&mut small, &disk, pp).unwrap(), inner.as_bytes());
    }
    assert!(!small.expanded(pp));
    assert_eq!(small.held(), patched.len());
    assert_eq!(disk.reads.borrow().len(), 2, "expanded at each read");
    let mut big = Patched::with_limit(100);
    big.insert(pp, patched);
    assert_eq!(read_bytes(&mut big, &disk, pp).unwrap(), inner.as_bytes());
    assert!(big.expanded(pp));
    assert_eq!(big.held(), 100);
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

/// A patched file already kept expanded is served whole; when it is bigger than the room the read
/// has left it is dropped, and the read fetches no further include.
#[test]
fn an_expanded_text_bigger_than_the_room_left_is_dropped_and_ends_the_includes() {
    let pp = b"data\\menus\\prefabs\\ppppp.xml";
    let big = "x".repeat(MAX_TEXT / 2 + 1);
    let disk = Disk::new(&[
        (
            "m.xml",
            r#"<include src="ppppp.xml"/>|<include src="ppppp.xml"/>|<include src="qqqqq.xml"/>"#,
        ),
        ("data\\menus\\prefabs\\qqqqq.xml", "Q"),
    ]);
    let mut p = Patched::default();
    p.insert(pp, big.clone().into_bytes());
    assert_eq!(read_bytes(&mut p, &disk, pp).unwrap().len(), big.len());
    assert!(p.expanded(pp));
    let t = read_bytes(&mut p, &disk, b"m.xml").unwrap();
    assert_eq!(t.len(), big.len() + 2, "the first copy and the two bars");
    assert!(t.ends_with(b"x||"));
    assert_eq!(
        *disk.reads.borrow(),
        vec![b"m.xml".to_vec()],
        "the include after the dropped one is never read"
    );
}

/// Patched P includes patched Q, whose own include meets the depth limit, then patched R, which
/// meets none: Q and P are not kept expanded (P carries Q's limit past R), R is.
#[test]
fn a_limit_deep_in_a_patched_include_keeps_neither_file_expanded() {
    // c000x .. c062x at depths 0..62, P at 63, Q and R at 64 (where includes are refused).
    let mut files: Vec<(String, String)> = (0..MAX_DEPTH - 1)
        .map(|i| {
            let next = if i + 2 == MAX_DEPTH {
                "ppppp.xml".to_string()
            } else {
                format!("c{:03}x.xml", i + 1)
            };
            (
                format!("data\\menus\\prefabs\\c{i:03}x.xml"),
                format!(r#"<include src="{next}"/>"#),
            )
        })
        .collect();
    files.push(("data\\menus\\prefabs\\inner.xml".into(), "IN".into()));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let (pp, qq, rr) = (
        b"data\\menus\\prefabs\\ppppp.xml",
        b"data\\menus\\prefabs\\qqqqq.xml",
        b"data\\menus\\prefabs\\rrrrr.xml",
    );
    let mut p = Patched::default();
    p.insert(
        pp,
        br#"<p><include src="qqqqq.xml"/><include src="rrrrr.xml"/></p>"#.to_vec(),
    );
    p.insert(qq, br#"<q><include src="inner.xml"/></q>"#.to_vec());
    p.insert(rr, b"R".to_vec());
    assert_eq!(
        read(&mut p, &disk, "data\\menus\\prefabs\\c000x.xml").unwrap(),
        "<p><q></q>R</p>"
    );
    assert!(!p.expanded(qq), "Q met the limit");
    assert!(!p.expanded(pp), "P holds Q cut short");
    assert!(p.expanded(rr));
    // Read from the top, nothing is refused and both are kept.
    assert_eq!(
        read(&mut p, &disk, "data\\menus\\prefabs\\ppppp.xml").unwrap(),
        "<p><q>IN</q>R</p>"
    );
    assert!(p.expanded(pp) && p.expanded(qq));
}

/// A once file's text is handed over at its first read: what is held goes down by its length.
#[test]
fn a_once_file_served_is_no_longer_held() {
    let hud = b"Data\\Menus\\Main\\hud_main_menu.xml";
    let mut p = Patched::default();
    p.insert(hud, b"HUDTEXT".to_vec());
    p.insert(b"Data\\Menus\\dialog_menu.xml", b"DIALOG".to_vec());
    assert_eq!(p.held(), 13);
    assert_eq!(
        p.serve(str_hash(hud)),
        Some(Served::Unexpanded(b"HUDTEXT".to_vec()))
    );
    assert_eq!(p.held(), 6);
    assert_eq!(p.serve(str_hash(hud)), None);
    assert_eq!(p.held(), 6);
}

/// A leaf too large for the room left ends the read: no later include anywhere above it is
/// fetched (else every remaining include would read down to a leaf that no longer fits).
#[test]
fn a_leaf_too_large_for_the_room_left_ends_every_later_include() {
    let leaf = (1 << 20) + 1;
    let mut files: Vec<(String, String)> = (0..40)
        .map(|i| {
            (
                format!("data\\menus\\prefabs\\k{i:02}.xml"),
                format!(
                    r#"<include src="k{:02}.xml"/><include src="k{:02}.xml"/>"#,
                    i + 1,
                    i + 1
                ),
            )
        })
        .collect();
    files.push(("data\\menus\\prefabs\\k40.xml".into(), "x".repeat(leaf)));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let disk = Disk::new(&refs);
    let mut p = Patched::default();
    let t = read_bytes(&mut p, &disk, b"data\\menus\\prefabs\\k00.xml").unwrap();
    // 63 leaves fit; the 64th is refused (the room left is 63 bytes short of it).
    assert_eq!(t.len(), 63 * leaf);
    // k00..k33 down the first includes, then the 127 files of k34's tree, the 64th leaf last.
    assert_eq!(disk.reads.borrow().len(), 34 + 127);
}
