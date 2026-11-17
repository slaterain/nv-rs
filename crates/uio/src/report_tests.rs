use super::*;

fn entry(list: &str, path: &str, file: &str, tile: &str, cond: &str, outcome: Outcome) -> Entry {
    Entry {
        list: list.as_bytes().to_vec(),
        path: path.as_bytes().to_vec(),
        target: Vec::new(),
        file: file.as_bytes().to_vec(),
        tile: tile.as_bytes().to_vec(),
        condition: cond.as_bytes().to_vec(),
        outcome,
    }
}

fn read(name: &str, opened: bool) -> ListRead {
    ListRead {
        name: name.as_bytes().to_vec(),
        opened,
    }
}

/// The scan of the maintainer's oHUD folder (lists_tests' oHUD test gives these entries).
fn ohud_scan() -> Scan {
    Scan {
        lists: vec![read("supported.txt", false), read("oHUDPatches.txt", true)],
        entries: vec![
            entry(
                "oHUDPatches.txt",
                "UIOPatches\\GrenadeKey.xml",
                "Data\\menus\\Prefabs\\GrenadeKeyHUD.xml",
                "GrenadeRECT",
                "#GrenadeKeyHUD.xml",
                Outcome::ConditionFalse,
            ),
            entry(
                "oHUDPatches.txt",
                "UIOPatches\\MineKey.xml",
                "Data\\menus\\Prefabs\\MineKeyHUD.xml",
                "MineRECT",
                "#MineKeyHUD.xml",
                Outcome::ConditionFalse,
            ),
        ],
    }
}

#[test]
fn the_ohud_folder_reports_its_list_no_additions_and_both_reasons() {
    assert_eq!(
        startup_lines(&ohud_scan(), &[]),
        vec![
            "UI Organizer: read 1 list file: oHUDPatches.txt; supported.txt could not be opened \
             (UIO's own list of the UI mods it knows, Data\\uio\\supported.txt in the UIO download)",
            "UI Organizer: no additions",
            "UI Organizer: oHUDPatches.txt: UIOPatches\\GrenadeKey.xml not added: \
             Prefabs\\GrenadeKeyHUD.xml is not installed",
            "UI Organizer: oHUDPatches.txt: UIOPatches\\MineKey.xml not added: \
             Prefabs\\MineKeyHUD.xml is not installed",
        ]
    );
}

#[test]
fn additions_are_grouped_by_file_and_tile_in_the_order_met() {
    let hud = "Data\\Menus\\Main\\hud_main_menu.xml";
    let scan = Scan {
        lists: vec![read("supported.txt", true), read("a.txt", true)],
        entries: vec![
            entry(
                "supported.txt",
                "x.xml",
                hud,
                "HUDMainMenu",
                "true",
                Outcome::Added,
            ),
            entry(
                "a.txt",
                "p.xml",
                "Data\\menus\\prefabs\\k.xml",
                "",
                "true",
                Outcome::Added,
            ),
            // The same file and tile in other case: the same group (UIO's tables ignore case).
            entry(
                "a.txt",
                "y.xml",
                "data\\menus\\main\\HUD_MAIN_MENU.xml",
                "hudmainmenu",
                "true",
                Outcome::Added,
            ),
            entry(
                "a.txt",
                "x.xml",
                hud,
                "HUDMainMenu",
                "true",
                Outcome::AlreadyAdded,
            ),
        ],
    };
    assert_eq!(
        startup_lines(&scan, &[]),
        vec![
            "UI Organizer: read 2 list files: supported.txt, a.txt",
            "UI Organizer: 3 additions: Main\\hud_main_menu.xml at tile HUDMainMenu: x.xml, \
             y.xml; prefabs\\k.xml at the end of the file: p.xml",
            "UI Organizer: a.txt: x.xml not added: it is already added to \
             Main\\hud_main_menu.xml at tile HUDMainMenu",
        ]
    );
    // One addition: the singular.
    let one = Scan {
        lists: vec![read("supported.txt", true)],
        entries: vec![entry(
            "supported.txt",
            "x.xml",
            hud,
            "T",
            "true",
            Outcome::Added,
        )],
    };
    assert_eq!(
        startup_lines(&one, &[])[..2],
        [
            "UI Organizer: read 1 list file: supported.txt",
            "UI Organizer: 1 addition: Main\\hud_main_menu.xml at tile T: x.xml",
        ]
    );
}

#[test]
fn lists_that_would_not_open_and_patching_problems_are_named() {
    let scan = Scan {
        lists: vec![read("supported.txt", true), read("big.txt", false)],
        entries: Vec::new(),
    };
    let problems = [
        Problem::Unreadable {
            file: b"Data\\menus\\prefabs\\gone.xml".to_vec(),
            additions: 1,
        },
        Problem::TileMissing {
            file: b"Data\\Menus\\Main\\hud_main_menu.xml".to_vec(),
            tile: b"JHUD".to_vec(),
            additions: 2,
            opened: false,
        },
        Problem::TileMissing {
            file: b"Data\\Menus\\Main\\hud_main_menu.xml".to_vec(),
            tile: b"Box".to_vec(),
            additions: 1,
            opened: true,
        },
    ];
    assert_eq!(
        startup_lines(&scan, &problems),
        vec![
            "UI Organizer: read 1 list file: supported.txt; big.txt could not be opened",
            "UI Organizer: no additions",
            "UI Organizer: prefabs\\gone.xml could not be read, so its 1 addition was not put in",
            "UI Organizer: Main\\hud_main_menu.xml has no tile JHUD, so 2 additions were not put in",
            "UI Organizer: Main\\hud_main_menu.xml: tile Box is opened but UIO never finds its end \
             (a self-closing named tag inside it counts as one more open tile), so 1 addition was \
             not put in",
        ]
    );
    // Nothing at all.
    let none = Scan {
        lists: vec![read("supported.txt", false)],
        entries: Vec::new(),
    };
    assert_eq!(
        startup_lines(&none, &[])[0].split(';').next(),
        Some("UI Organizer: read no list file")
    );
}

#[test]
fn every_reason_in_plain_words() {
    let mut e = entry("l.txt", "p.xml", "", "", "", Outcome::Added);
    assert_eq!(reason(&e), None);
    e.outcome = Outcome::PrefabMissing;
    assert_eq!(reason(&e).unwrap(), "Prefabs\\p.xml is not installed");
    e.target = b"@main\\hud.xml".to_vec();
    e.outcome = Outcome::EndOfFileNotPrefab;
    assert_eq!(
        reason(&e).unwrap(),
        "an addition at the end of a file must go into a Prefabs file, not @main\\hud.xml"
    );
    e.target = b"NoSuchMenu".to_vec();
    e.outcome = Outcome::UnknownMenu;
    assert_eq!(reason(&e).unwrap(), "NoSuchMenu is not a menu UIO knows");
    e.outcome = Outcome::TargetTooLong;
    assert_eq!(reason(&e).unwrap(), "the target NoSuchMenu is too long");
    e.condition = b"?Radar.esp".to_vec();
    e.outcome = Outcome::ConditionFalse;
    assert_eq!(reason(&e).unwrap(), "Radar.esp is not in plugins.txt");
}

#[test]
fn a_condition_is_named_when_it_is_one_file_or_plugin_else_quoted() {
    assert_eq!(
        condition_reason(b"#a\\b.xml"),
        "Prefabs\\a\\b.xml is not installed"
    );
    assert_eq!(condition_reason(b"!#a.xml"), "Prefabs\\a.xml is installed");
    assert_eq!(
        condition_reason(b"?Radar.esp"),
        "Radar.esp is not in plugins.txt"
    );
    assert_eq!(
        condition_reason(b"!?Radar.esp"),
        "Radar.esp is in plugins.txt"
    );
    // A run of tabs ends one token, so doubled and trailing tabs make no token (as evaluate
    // reads them).
    assert_eq!(
        condition_reason(b"#a.xml\t\t"),
        "Prefabs\\a.xml is not installed"
    );
    // A leading tab (a public list's leading space) is an empty first token, which UIO reads as
    // false whatever is installed: said so, never "not installed".
    assert_eq!(
        condition_reason(b"\t#a.xml\t"),
        "its condition line starts with a blank, which UIO reads as false (#a.xml)"
    );
    // A file name too long for UIO's buffer names no file.
    let long = format!("#{}", "x".repeat(UIO_PATH - PREFABS.len() + 1));
    assert_eq!(
        condition_reason(long.as_bytes()),
        format!(
            "the name Prefabs\\{} is too long for UIO, which reads it as not installed",
            &long[1..]
        )
    );
    let fits = format!("#{}", "x".repeat(UIO_PATH - PREFABS.len()));
    assert_eq!(
        condition_reason(fits.as_bytes()),
        format!("Prefabs\\{} is not installed", &fits[1..])
    );
    assert_eq!(
        condition_reason(b"?Jetpack.esp\t&&\t!darnui"),
        "its condition is false (?Jetpack.esp && !darnui)"
    );
    assert_eq!(condition_reason(b"darn"), "its condition is false (darn)");
    assert_eq!(condition_reason(b"!darn"), "its condition is false (!darn)");
}

#[test]
fn entries_past_the_cap_are_counted_in_one_line() {
    let make = |n: usize| Scan {
        lists: vec![read("supported.txt", true)],
        entries: (0..n)
            .map(|i| {
                entry(
                    "supported.txt",
                    &format!("p{i}.xml"),
                    "",
                    "",
                    "",
                    Outcome::PrefabMissing,
                )
            })
            .collect(),
    };
    let lines = startup_lines(&make(MAX_SKIPPED_LINES), &[]);
    assert_eq!(lines.len(), 2 + MAX_SKIPPED_LINES);
    assert!(lines
        .last()
        .unwrap()
        .contains(&format!("p{}.xml", MAX_SKIPPED_LINES - 1)));
    let lines = startup_lines(&make(MAX_SKIPPED_LINES + 1), &[]);
    assert_eq!(lines.len(), 3 + MAX_SKIPPED_LINES);
    assert_eq!(
        lines.last().unwrap(),
        "UI Organizer: and 1 more entry not added"
    );
    let lines = startup_lines(&make(MAX_SKIPPED_LINES + 5), &[]);
    assert_eq!(
        lines.last().unwrap(),
        "UI Organizer: and 5 more entries not added"
    );
}

#[test]
fn list_files_additions_and_problems_past_the_cap_are_counted() {
    // List files: MAX_NAMED named, the rest counted; the same for lists that would not open.
    let lists = |n: usize, opened: bool| -> Vec<ListRead> {
        (0..n).map(|i| read(&format!("l{i}.txt"), opened)).collect()
    };
    let scan = Scan {
        lists: lists(MAX_NAMED, true),
        entries: Vec::new(),
    };
    let first = &startup_lines(&scan, &[])[0];
    assert!(first.ends_with(&format!("l{}.txt", MAX_NAMED - 1)));
    let scan = Scan {
        lists: [lists(MAX_NAMED + 2, true), lists(MAX_NAMED + 3, false)].concat(),
        entries: Vec::new(),
    };
    let first = &startup_lines(&scan, &[])[0];
    assert!(first.starts_with(&format!(
        "UI Organizer: read {} list files: l0.txt, ",
        MAX_NAMED + 2
    )));
    assert!(first.contains(&format!("l{}.txt and 2 more;", MAX_NAMED - 1)));
    assert!(!first.contains(&format!("l{}.txt,", MAX_NAMED)));
    assert_eq!(first.matches("could not be opened").count(), MAX_NAMED + 1);
    assert!(first.ends_with("; 3 more list files could not be opened"));

    // Additions: MAX_NAMED paths named in all, across the groups in order; the rest counted.
    let added = |n: usize| Scan {
        lists: vec![read("a.txt", true)],
        entries: (0..n)
            .map(|i| {
                let tile = if i < 2 { "A" } else { "B" };
                entry(
                    "a.txt",
                    &format!("p{i}.xml"),
                    "f.xml",
                    tile,
                    "",
                    Outcome::Added,
                )
            })
            .collect(),
    };
    let line = &startup_lines(&added(MAX_NAMED), &[])[1];
    assert!(line.ends_with(&format!("p{}.xml", MAX_NAMED - 1)));
    assert!(!line.contains("more"));
    let line = &startup_lines(&added(MAX_NAMED + 3), &[])[1];
    assert!(line.starts_with(&format!(
        "UI Organizer: {} additions: f.xml at tile A: p0.xml, p1.xml; f.xml at tile B: p2.xml, ",
        MAX_NAMED + 3
    )));
    assert!(line.ends_with(&format!("p{}.xml; and 3 more", MAX_NAMED - 1)));
    assert!(!line.contains(&format!("p{}.xml", MAX_NAMED)));

    // Problems: MAX_NAMED lines, then one count.
    let problems = |n: usize| -> Vec<Problem> {
        (0..n)
            .map(|i| Problem::Unreadable {
                file: format!("f{i}.xml").into_bytes(),
                additions: 1,
            })
            .collect()
    };
    let none = Scan::default();
    let lines = startup_lines(&none, &problems(MAX_NAMED));
    assert_eq!(lines.len(), 2 + MAX_NAMED);
    let lines = startup_lines(&none, &problems(MAX_NAMED + 1));
    assert_eq!(lines.len(), 3 + MAX_NAMED);
    assert!(lines[1 + MAX_NAMED].contains(&format!("f{}.xml", MAX_NAMED - 1)));
    assert_eq!(
        lines.last().unwrap(),
        "UI Organizer: and 1 more problem met while putting additions in"
    );
    let lines = startup_lines(&none, &problems(MAX_NAMED + 4));
    assert_eq!(
        lines.last().unwrap(),
        "UI Organizer: and 4 more problems met while putting additions in"
    );
}

#[test]
fn the_first_read_line() {
    assert_eq!(
        first_read_line(b"Data\\Menus\\Main\\hud_main_menu.xml", 2),
        "UI Organizer: first file with additions read: Main\\hud_main_menu.xml (2 additions)"
    );
    // A path outside Data\Menus\ is shown whole.
    assert_eq!(
        first_read_line(b"x.xml", 1),
        "UI Organizer: first file with additions read: x.xml (1 addition)"
    );
}
