//! Tests on folders the tests make in a scratch folder, removed afterwards.

use crate::installs::*;
use crate::overlay::*;
use crate::RECORD;
use std::path::{Path, PathBuf};

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("nv-mod-install-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn plugin(names: &[&str]) -> Vec<u8> {
    let mut fields = Vec::new();
    fields.extend_from_slice(b"HEDR\x0c\0");
    fields.extend_from_slice(&[0; 12]);
    for name in names {
        fields.extend_from_slice(b"MAST");
        fields.extend_from_slice(&(name.len() as u16 + 1).to_le_bytes());
        fields.extend_from_slice(name.as_bytes());
        fields.push(0);
        fields.extend_from_slice(b"DATA\x08\0");
        fields.extend_from_slice(&[0; 8]);
    }
    let mut file = b"TES4".to_vec();
    file.extend_from_slice(&(fields.len() as u32).to_le_bytes());
    file.extend_from_slice(&[0; 16]);
    file.extend_from_slice(&fields);
    file
}

/// A made-up game folder: the exe, the master and one texture.
fn game(dir: &Path) -> PathBuf {
    let game = dir.join("game");
    write(&game.join("FalloutNV.exe"), b"exe");
    write(&game.join("Data").join("FalloutNV.esm"), b"esm");
    write(&game.join("Data").join("Textures").join("a.dds"), b"game's");
    game
}

// ------------------------------------------------------------------------------------------------
// Plugins

#[test]
fn a_plugins_masters_are_read() {
    assert_eq!(
        masters(&plugin(&["FalloutNV.esm", "DeadMoney.esm"])),
        ["FalloutNV.esm", "DeadMoney.esm"]
    );
    assert!(masters(b"not a plugin").is_empty());
}

#[test]
fn the_list_has_named_masters_then_esm_then_esp() {
    let list = plugin_list(
        &Default::default(),
        &[
            (
                "b.esp".into(),
                plugin(&["FalloutNV.esm", "lonesomeroad.esm"]),
            ),
            ("A.ESP".into(), plugin(&["FalloutNV.esm"])),
            ("Own.esm".into(), plugin(&["DeadMoney.esm"])),
        ],
    );
    assert_eq!(
        list,
        "DeadMoney.esm\r\nLonesomeRoad.esm\r\nOwn.esm\r\nA.ESP\r\nb.esp\r\n"
    );
}

/// Every official master in the game's Data stays on, named by a mod or not, in the official
/// order; a mod's copy of one is not listed twice.
#[test]
fn official_masters_present_stay_on() {
    let present = ["gunrunnersarsenal.esm", "deadmoney.esm"]
        .map(String::from)
        .into_iter()
        .collect();
    let list = plugin_list(
        &present,
        &[
            ("Gun.esp".into(), plugin(&["FalloutNV.esm"])),
            ("DEADMONEY.ESM".into(), plugin(&["FalloutNV.esm"])),
        ],
    );
    assert_eq!(
        list,
        "DeadMoney.esm\r\nGunRunnersArsenal.esm\r\nGun.esp\r\n"
    );
}

/// The whole way: DeadMoney.esm in the game's Data, a mod that does not name it, no loose
/// plugin list: Dead Money is on.
#[test]
fn play_with_mods_keeps_the_dlc_on() {
    let dir = Scratch::new("dlc-on");
    let game = game(&dir.0);
    write(&game.join("Data").join("DeadMoney.esm"), b"dm");
    let mods = dir.0.join("mods");
    write(&mods.join("Gun.esp"), &plugin(&["FalloutNV.esm"]));
    let plugins = dir.0.join("p").join("plugins.txt");
    build(
        &game,
        std::slice::from_ref(&mods),
        &dir.0.join("modgame"),
        &plugins,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(&plugins).unwrap(),
        "DeadMoney.esm\r\nGun.esp\r\n"
    );
}

// ------------------------------------------------------------------------------------------------
// The game folder for mods

/// The game's own files are links, a mod's file replaces its link, and neither the build nor
/// removing the folder changes the game's files.
#[test]
fn the_game_folder_is_never_written() {
    let dir = Scratch::new("never");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    write(&mods.join("textures").join("a.dds"), b"mod's");
    write(
        &mods.join("Mine.esp"),
        &plugin(&["FalloutNV.esm", "HonestHearts.esm"]),
    );
    write(&mods.join("READ ME.txt"), b"not a mod");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p").join("plugins.txt");
    let built = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(
        (built.linked, built.mod_files, built.extender_plugins),
        (2, 2, false)
    );
    assert_eq!(
        std::fs::read(target.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"mod's"
    );
    assert_eq!(
        std::fs::read(target.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
    assert!(!target.join("Data").join("READ ME.txt").exists());
    assert_eq!(
        std::fs::read_to_string(&plugins).unwrap(),
        "HonestHearts.esm\r\nMine.esp\r\n"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    // Again over the first one, then gone.
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
    assert_eq!(
        std::fs::read(game.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    write(&mods.join("plugins.txt"), b"Own order.esp\r\n");
    write(&mods.join("nvse").join("plugins").join("x.dll"), b"x");
    let built = build(&game, &[mods], &target, &plugins).unwrap();
    assert!(built.extender_plugins);
    assert_eq!(std::fs::read(&plugins).unwrap(), b"Own order.esp\r\n");
}

/// Installed mods in their order, then the loose folder; records, archives and the installed
/// archives folder stay out; a folder of another case is filled, not doubled (on every system).
#[test]
fn layers_go_over_the_game_in_order() {
    let dir = Scratch::new("layers");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    let first = installs.join("01 Base");
    let second = installs.join("02 Patch");
    write(&first.join("Base.esm"), &plugin(&["FalloutNV.esm"]));
    write(&first.join("textures").join("b.dds"), b"base");
    write(&first.join("TEXTURES").join("c.dds"), b"base c");
    write(&first.join(RECORD), b"name: Base\n");
    write(
        &second.join("Patch.esp"),
        &plugin(&["Base.esm", "DeadMoney.esm"]),
    );
    write(&second.join("Textures").join("b.dds"), b"patch");
    write(&second.join(RECORD), b"name: Patch\n");
    write(&loose.join("Textures").join("c.dds"), b"by hand");
    write(&loose.join("Gun Pack.7z"), b"7z...");
    write(&loose.join("other.FOMOD"), b"PK...");
    write(&loose.join("installed archives").join("old.zip"), b"PK...");
    std::fs::create_dir_all(installs.join("not a mod")).unwrap();
    let layers = layers(&installs, &loose).unwrap();
    assert_eq!(layers, [first.clone(), second.clone(), loose.clone()]);
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("plugins.txt");
    let built = build(&game, &layers, &target, &plugins).unwrap();
    // In the folder: Base.esm, Patch.esp, b.dds (the patch's) and c.dds (the loose one).
    assert_eq!(built.mod_files, 4);
    let data = target.join("Data");
    // The game's folder is "Textures": every layer's files went there.
    let tex = data.join("Textures");
    assert_eq!(std::fs::read(tex.join("a.dds")).unwrap(), b"game's");
    assert_eq!(std::fs::read(tex.join("b.dds")).unwrap(), b"patch");
    assert_eq!(std::fs::read(tex.join("c.dds")).unwrap(), b"by hand");
    let folders: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.eq_ignore_ascii_case("textures"))
        .collect();
    assert_eq!(folders, ["Textures"]);
    for left in [RECORD, "Gun Pack.7z", "other.FOMOD", "installed archives"] {
        assert!(!data.join(left).exists(), "{left}");
    }
    assert_eq!(
        std::fs::read_to_string(&plugins).unwrap(),
        "DeadMoney.esm\r\nBase.esm\r\nPatch.esp\r\n"
    );
}

#[test]
fn each_failure_gets_its_own_words() {
    #[cfg(windows)]
    {
        assert_eq!(classify(Some(17), true), Failure::OtherDrive);
        assert_eq!(classify(Some(1), true), Failure::NoLinks);
        assert_eq!(classify(Some(1), false), Failure::Other);
        assert_eq!(classify(Some(32), false), Failure::Locked);
        assert_eq!(classify(Some(33), false), Failure::Locked);
        assert_eq!(classify(Some(5), false), Failure::Denied);
        assert_eq!(classify(Some(112), false), Failure::Full);
    }
    #[cfg(not(windows))]
    {
        assert_eq!(classify(Some(13), false), Failure::Denied);
        // EPERM linking falls back to a symbolic link: failing both is NoLinks.
        assert_eq!(classify(Some(1), true), Failure::NoLinks);
        assert_eq!(classify(Some(1), false), Failure::Denied);
        assert_eq!(classify(Some(16), false), Failure::Locked);
        assert_eq!(classify(Some(28), false), Failure::Full);
        assert_eq!(classify(Some(2), false), Failure::Other);
    }
    let text = |kind| {
        ModError {
            kind,
            step: "linking X: boom".into(),
        }
        .player_text()
    };
    assert!(text(Failure::OtherDrive).contains("another drive"));
    // Windows names the drive kind; Linux and macOS say neither kind of link could be made.
    let drive_words = if cfg!(windows) {
        "NTFS"
    } else {
        "symbolic link"
    };
    assert!(text(Failure::NoLinks).contains(drive_words));
    assert!(text(Failure::Locked).contains("open"));
    assert!(text(Failure::Denied).contains("read-only"));
    assert!(!text(Failure::Locked).contains("same drive"));
    assert!(text(Failure::Locked).contains("linking X: boom"));
}

/// A file of the old folder held open by another program: the error says locked, not drive.
#[cfg(windows)]
#[test]
fn a_locked_file_is_reported_as_locked() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = Scratch::new("lock");
    let game = game(&dir.0);
    let (mods, target) = (dir.0.join("mods"), dir.0.join("modgame"));
    let plugins = dir.0.join("plugins.txt");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(target.join("FalloutNV.exe"))
        .unwrap();
    // Without its record the folder is made again from nothing: removing it meets the held file.
    std::fs::remove_file(state_path(&target)).unwrap();
    let error = build(&game, &[mods], &target, &plugins).unwrap_err();
    drop(held);
    assert_eq!(error.kind, Failure::Locked, "{}", error.step);
    assert!(error.step.starts_with("removing "));
    assert_eq!(std::fs::read(game.join("FalloutNV.exe")).unwrap(), b"exe");
}

// ------------------------------------------------------------------------------------------------
// Installed mods

#[test]
fn a_record_reads_back_as_written() {
    let record = Record {
        name: "Gun Pack".into(),
        version: "1.2".into(),
        author: "Some\none".into(),
        archive: "Gun Pack 1.2.7z".into(),
        archive_size: 1_048_576,
        choices: vec!["Options / DLC patches / Honest Hearts".into()],
        files: vec!["GunPack.esp".into(), "Meshes/Weapons/gun.nif".into()],
        other: vec![],
    };
    let text = record.text();
    assert!(text.contains("author: Some one\n"));
    let mut back = Record::parse(&text);
    assert_eq!(back.author, "Some one");
    back.author = record.author.clone();
    assert_eq!(back, record);
    // Unknown keys and odd lines are kept.
    let later = Record::parse("name: X\r\nchecksum: abc\r\nodd line\r\n\r\nfile: a.esp\r\n");
    assert_eq!(later.name, "X");
    assert_eq!(later.files, ["a.esp"]);
    assert_eq!(later.other, ["checksum: abc", "odd line"]);
    assert!(later.text().ends_with("checksum: abc\nodd line\n"));
}

#[test]
fn installed_mods_come_in_their_order() {
    let dir = Scratch::new("installed");
    let installs = dir.0.join("mod installs");
    assert!(installed(&installs).unwrap().is_empty());
    assert_eq!(
        next_folder(&installs, "Gun Pack").unwrap(),
        installs.join("01 Gun Pack")
    );
    for name in [
        "10 Ten",
        "2 Two",
        "02 Also two",
        "junk",
        "x Not numbered",
        "+3 Plus",
    ] {
        write(&installs.join(name).join(RECORD), b"name: x\n");
    }
    // No record (a half-finished install), or a staging folder: not a mod, but no number of a
    // numbered folder is given again.
    std::fs::create_dir_all(installs.join("12 Unfinished")).unwrap();
    write(
        &installs.join(format!("{STAGING}13 Big")).join("a.dds"),
        b"x",
    );
    write(&installs.join("3 a file"), b"not a folder");
    let names: Vec<(u32, String)> = installed(&installs)
        .unwrap()
        .into_iter()
        .map(|i| (i.number, i.name))
        .collect();
    assert_eq!(
        names,
        [
            (2, "Also two".to_owned()),
            (2, "Two".to_owned()),
            (10, "Ten".to_owned())
        ]
    );
    assert_eq!(
        next_folder(&installs, "Weapon: Mod?").unwrap(),
        installs.join("13 Weapon Mod")
    );
}

#[test]
fn names_every_system_can_hold() {
    assert_eq!(safe_name("Gun Pack"), "Gun Pack");
    assert_eq!(safe_name("a/b\\c:d*e?f\"g<h>i|j"), "a b c d e f g h i j");
    assert_eq!(safe_name("Ends with dots... "), "Ends with dots");
    assert_eq!(safe_name("  \t "), "Mod");
    assert_eq!(safe_name("CON"), "CON mod");
    assert_eq!(safe_name("com1.txt"), "com1.txt mod");
    assert_eq!(safe_name("Console"), "Console");
    assert_eq!(safe_name(&"x".repeat(100)).len(), 60);
}

#[test]
fn removing_takes_only_a_mods_own_folder() {
    let dir = Scratch::new("remove");
    let installs = dir.0.join("mod installs");
    let one = installs.join("01 One");
    write(&one.join("Textures").join("a.dds"), b"x");
    write(&dir.0.join("keep.txt"), b"keep");
    let game = game(&dir.0);
    assert!(remove(&installs, &installs, &game).is_err());
    assert!(remove(&installs, &dir.0, &game).is_err());
    assert!(remove(&installs, &installs.join("01 One").join("Textures"), &game).is_err());
    assert!(remove(&installs, &installs.join(".."), &game).is_err());
    assert!(one.exists());
    remove(&installs, &one, &game).unwrap();
    assert!(!one.exists());
    assert!(dir.0.join("keep.txt").exists());
}

// ------------------------------------------------------------------------------------------------
// Caps and indexes (review of #720, applied here)

/// A folder deeper than the cap in a layer stops the build in words, as does a folder link that
/// loops back (Linux and macOS); nothing runs away.
#[test]
fn deep_folders_stop_the_build_and_layer_links_are_not_followed() {
    let dir = Scratch::new("deep");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    let mut deep = mods.clone();
    for _ in 0..=MAX_DEPTH {
        deep.push("d");
    }
    write(&deep.join("x.dds"), b"x");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("plugins.txt");
    let error = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap_err();
    assert!(error.step.contains("folders deep"), "{}", error.step);
    #[cfg(unix)]
    {
        std::fs::remove_dir_all(&mods).unwrap();
        std::fs::create_dir_all(mods.join("a")).unwrap();
        std::os::unix::fs::symlink(&mods, mods.join("a").join("loop")).unwrap();
        // A folder link in a layer is never followed: the build goes on, the link left out.
        build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
        assert!(!target.join("Data").join("a").join("loop").exists());
        assert_eq!(
            std::fs::read(game.join("Data").join("FalloutNV.esm")).unwrap(),
            b"esm"
        );
    }
}

/// 5,000 files into one folder of the game's: each placed with one look in that folder's names,
/// not a reading of the folder per file.
#[test]
fn many_files_in_one_folder_are_placed_by_index() {
    let dir = Scratch::new("many");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    let tex = mods.join("TEXTURES");
    std::fs::create_dir_all(&tex).unwrap();
    for i in 0..5_000 {
        std::fs::write(tex.join(format!("t{i}.dds")), b"x").unwrap();
    }
    let started = std::time::Instant::now();
    let built = build(
        &game,
        std::slice::from_ref(&mods),
        &dir.0.join("modgame"),
        &dir.0.join("plugins.txt"),
    )
    .unwrap();
    assert_eq!(built.mod_files, 5_000);
    let folders: Vec<String> = std::fs::read_dir(dir.0.join("modgame").join("Data"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.eq_ignore_ascii_case("textures"))
        .collect();
    assert_eq!(folders, ["Textures"]);
    assert!(
        started.elapsed().as_secs() < 60,
        "took {:?}",
        started.elapsed()
    );
}

/// More installed mods than the cap is refused in words.
#[test]
fn installed_mods_are_capped() {
    let dir = Scratch::new("cap");
    let installs = dir.0.join("mod installs");
    for i in 0..=MAX_INSTALLS {
        std::fs::create_dir_all(installs.join(format!("{i} m"))).unwrap();
    }
    let error = installed(&installs).unwrap_err();
    assert!(error.to_string().contains("more than"), "{error}");
}

// ------------------------------------------------------------------------------------------------
// Review of #725

/// A folder link to `to` at `link`: a symbolic link on Linux and macOS, a junction on Windows
/// (made without rights a player lacks).
fn folder_link(to: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(to, link).unwrap();
    #[cfg(windows)]
    {
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(to)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }
}

/// A second build with nothing changed makes and removes nothing; a changed file is redone
/// alone; a mod's file gone brings the game's back. Mods' files are hard links, not copies.
#[test]
fn only_what_changed_is_redone() {
    let dir = Scratch::new("incremental");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    let one = installs.join("01 One");
    write(&one.join("Textures").join("a.dds"), b"one's");
    write(&one.join("Textures").join("b.dds"), b"one's b");
    write(&one.join(RECORD), b"name: One\n");
    write(&loose.join("Meshes").join("m.nif"), b"loose");
    let layers = layers(&installs, &loose).unwrap();
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("plugins.txt");
    let first = build(&game, &layers, &target, &plugins).unwrap();
    assert_eq!(first.made, 5, "{first:?}");
    assert_eq!(first.copied, 0);
    let again = build(&game, &layers, &target, &plugins).unwrap();
    assert_eq!(
        (again.made, again.removed, again.linked),
        (0, 0, 0),
        "{again:?}"
    );
    // A hard link: a change made in place to the installed file is the folder's too.
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(one.join("Textures").join("b.dds"))
            .unwrap();
        f.write_all(b"+").unwrap();
    }
    assert_eq!(
        std::fs::read(target.join("Data").join("Textures").join("b.dds")).unwrap(),
        b"one's b+"
    );
    // b.dds's size changed: only it is redone.
    let changed = build(&game, &layers, &target, &plugins).unwrap();
    assert_eq!((changed.made, changed.removed), (1, 1), "{changed:?}");
    // The installed a.dds gone: the game's a.dds is back, alone.
    std::fs::remove_file(one.join("Textures").join("a.dds")).unwrap();
    let gone = build(&game, &layers, &target, &plugins).unwrap();
    assert_eq!((gone.made, gone.removed), (1, 1), "{gone:?}");
    assert_eq!(
        std::fs::read(target.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
}

/// A folder link in the game's Data: left out on Windows (as before, so a junction to another
/// drive never stops a build); followed once on Linux and macOS.
#[test]
fn a_folder_link_in_the_games_data() {
    let dir = Scratch::new("data-link");
    let game = game(&dir.0);
    let elsewhere = dir.0.join("elsewhere");
    write(&elsewhere.join("far.dds"), b"far");
    folder_link(&elsewhere, &game.join("Data").join("Linked"));
    let target = dir.0.join("modgame");
    let built = build(&game, &[dir.0.join("mods")], &target, &dir.0.join("p.txt")).unwrap();
    let far = target.join("Data").join("Linked").join("far.dds");
    if cfg!(windows) {
        assert!(!far.exists(), "{built:?}");
    } else {
        assert_eq!(std::fs::read(&far).unwrap(), b"far");
    }
    assert_eq!(std::fs::read(elsewhere.join("far.dds")).unwrap(), b"far");
    #[cfg(windows)]
    std::fs::remove_dir(game.join("Data").join("Linked")).unwrap();
}

/// A link inside the folder pointing at the game: made again from nothing it is removed, not
/// followed; with its record, a name is never made through it. Every game file stays as it was.
#[test]
fn a_link_in_the_old_folder_never_reaches_the_game() {
    let dir = Scratch::new("old-link");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    write(&mods.join("Textures").join("a.dds"), b"mod's");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    // The folder's Textures made a link to the game's own Textures; then the mod's file changes.
    let textures = target.join("Data").join("Textures");
    std::fs::remove_dir_all(&textures).unwrap();
    folder_link(&game.join("Data").join("Textures"), &textures);
    write(&mods.join("Textures").join("a.dds"), b"mod's, changed");
    let error = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap_err();
    assert!(error.step.contains("not a plain folder"), "{}", error.step);
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    // The stopped build left no record: made again from nothing, the link removed, not followed.
    assert!(!state_path(&target).exists());
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
    assert_eq!(
        std::fs::read(textures.join("a.dds")).unwrap(),
        b"mod's, changed"
    );
}

/// The loose folder's own plugins.txt: the installed mods' plugins it does not name are added
/// after it, masters first, and said.
#[test]
fn installed_plugins_follow_a_loose_plugin_list() {
    let dir = Scratch::new("plugins-list");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    write(
        &installs.join("01 Mod").join("Mod.esp"),
        &plugin(&["FalloutNV.esm", "DeadMoney.esm"]),
    );
    write(
        &installs.join("01 Mod").join("Base.esm"),
        &plugin(&["FalloutNV.esm"]),
    );
    write(&installs.join("01 Mod").join(RECORD), b"name: Mod\n");
    write(&loose.join("plugins.txt"), b"Mine.esp\r\nBase.esm");
    let layers = layers(&installs, &loose).unwrap();
    let plugins = dir.0.join("plugins.txt");
    let built = build(&game, &layers, &dir.0.join("modgame"), &plugins).unwrap();
    assert_eq!(built.added_plugins, ["DeadMoney.esm", "Mod.esp"]);
    assert_eq!(
        std::fs::read_to_string(&plugins).unwrap(),
        "Mine.esp\r\nBase.esm\r\nDeadMoney.esm\r\nMod.esp\r\n"
    );
}

/// The caps, lowered: too many files and too deep stop the build in words; the plugin header is
/// read only so far (a header cut before its masters names none).
#[test]
fn the_caps_hold() {
    let dir = Scratch::new("caps");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    for i in 0..5 {
        write(&mods.join(format!("f{i}.dds")), b"x");
    }
    write(&mods.join("a").join("b").join("c").join("d.dds"), b"x");
    write(
        &mods.join("Mine.esp"),
        &plugin(&["FalloutNV.esm", "DeadMoney.esm"]),
    );
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    let caps = |depth, files, header_bytes| Caps {
        depth,
        files,
        header_bytes,
    };
    let few = build_with(
        &game,
        std::slice::from_ref(&mods),
        &target,
        &plugins,
        caps(64, 4, 1 << 20),
        &System,
    );
    assert!(few.unwrap_err().step.contains("more than 4 files"));
    let shallow = build_with(
        &game,
        std::slice::from_ref(&mods),
        &target,
        &plugins,
        caps(2, 100, 1 << 20),
        &System,
    );
    assert!(shallow.unwrap_err().step.contains("folders deep"));
    build_with(
        &game,
        std::slice::from_ref(&mods),
        &target,
        &plugins,
        caps(64, 100, 20),
        &System,
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(&plugins).unwrap(), "Mine.esp\r\n");
    build_with(
        &game,
        std::slice::from_ref(&mods),
        &target,
        &plugins,
        caps(64, 100, 1 << 20),
        &System,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(&plugins).unwrap(),
        "DeadMoney.esm\r\nMine.esp\r\n"
    );
}

/// Linux and macOS: hard links refused as a drive without them refuses them: the game's files are
/// symbolic links instead, a mod's file a copy, and the game's file is never written.
#[cfg(unix)]
#[test]
fn without_hard_links_symbolic_links_and_copies() {
    struct NoHardLinks(i32);
    impl Linker for NoHardLinks {
        fn hard_link(&self, _: &crate::fsops::Guard, _: &Path, _: &Path) -> std::io::Result<()> {
            Err(std::io::Error::from_raw_os_error(self.0))
        }
    }
    for code in [18, 1, 95, 31] {
        let dir = Scratch::new(&format!("nolinks-{code}"));
        let game = game(&dir.0);
        let mods = dir.0.join("mods");
        write(&mods.join("Textures").join("a.dds"), b"mod's");
        let target = dir.0.join("modgame");
        let built = build_with(
            &game,
            std::slice::from_ref(&mods),
            &target,
            &dir.0.join("p.txt"),
            Caps::default(),
            &NoHardLinks(code),
        )
        .unwrap();
        assert_eq!(built.copied, 1, "{code}");
        let esm = target.join("Data").join("FalloutNV.esm");
        assert!(std::fs::symlink_metadata(&esm)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
            b"game's"
        );
    }
    assert_eq!(classify(Some(95), true), Failure::NoLinks);
}

/// Linux: of `Data` and `data`, the one holding FalloutNV.esm; `Data` exactly first; a relative
/// file link in the game's Data is linked as the file it leads to; two links back to the same
/// folder are walked once.
#[cfg(unix)]
#[test]
fn the_games_data_folder_and_links_on_unix() {
    let dir = Scratch::new("unix-data");
    let game = dir.0.join("game");
    write(&game.join("FalloutNV.exe"), b"exe");
    write(&game.join("data").join("stray.txt"), b"stray");
    // macOS's usual file system ignores case: one folder under three spellings, so the choice
    // between them is checked only where they can differ.
    let case_sensitive = !game.join("DATA").join("stray.txt").exists();
    if case_sensitive {
        write(&game.join("DATA").join("FalloutNV.esm"), b"esm");
        assert_eq!(game_data(&game).unwrap(), game.join("DATA"));
    }
    write(&game.join("Data").join("FalloutNV.esm"), b"exact");
    if case_sensitive {
        assert_eq!(game_data(&game).unwrap(), game.join("Data"));
    }
    std::os::unix::fs::symlink("FalloutNV.esm", game.join("Data").join("Alias.esm")).unwrap();
    std::fs::create_dir_all(game.join("Data").join("Loop")).unwrap();
    std::os::unix::fs::symlink("..", game.join("Data").join("Loop").join("up1")).unwrap();
    std::os::unix::fs::symlink("..", game.join("Data").join("Loop").join("up2")).unwrap();
    let target = dir.0.join("modgame");
    build(&game, &[dir.0.join("mods")], &target, &dir.0.join("p.txt")).unwrap();
    assert_eq!(
        std::fs::read(target.join("Data").join("Alias.esm")).unwrap(),
        b"exact"
    );
}

// ------------------------------------------------------------------------------------------------
// Second review of #725: nothing outside the folder, by construction

/// A record line inserted before its `end` line, the count raised to match.
fn forge_record(target: &Path, line: &str) {
    let state = state_path(target);
    let text = std::fs::read_to_string(&state).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let end = lines.pop().unwrap();
    let count: usize = end.split('\t').nth(1).unwrap().parse().unwrap();
    lines.push(line.to_owned());
    lines.push(format!("end\t{}", count + 1));
    std::fs::write(&state, lines.join("\n") + "\n").unwrap();
}

/// Records naming something outside the folder (`..`, a full path, a drive) are not trusted:
/// the folder is made again from nothing and the file outside is untouched.
#[test]
fn a_record_naming_outside_is_not_trusted() {
    let dir = Scratch::new("bad-record");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    write(&mods.join("Textures").join("a.dds"), b"mod's");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    let outside = dir.0.join("outside.txt");
    write(&outside, b"keep");
    let full = outside.to_string_lossy().into_owned();
    for name in [
        "../outside.txt".to_owned(),
        "Data/../../outside.txt".to_owned(),
        full.clone(),
        full.replace('\\', "/"),
        "Data/Textures/a.dds/../../../../outside.txt".to_owned(),
    ] {
        build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
        forge_record(&target, &format!("m\t4\t0\t{full}\t{name}"));
        // The mod's file is gone: the record would have the build remove the forged name.
        std::fs::remove_file(mods.join("Textures").join("a.dds")).unwrap();
        let built = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
        assert!(built.made > 0, "{name}: made again from nothing");
        assert_eq!(std::fs::read(&outside).unwrap(), b"keep", "{name}");
        write(&mods.join("Textures").join("a.dds"), b"mod's");
    }
    // A record cut short (its end line and the line for a mod's own file lost) is not trusted:
    // made again from nothing, the mod's file, gone since, does not stay in the folder.
    write(&mods.join("Meshes").join("only.nif"), b"only the mod's");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    let state = state_path(&target);
    let text = std::fs::read_to_string(&state).unwrap();
    let cut: String = text
        .lines()
        .filter(|l| !l.contains("only.nif") && !l.starts_with("end"))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&state, cut).unwrap();
    std::fs::remove_file(mods.join("Meshes").join("only.nif")).unwrap();
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert!(!target.join("Data").join("Meshes").join("only.nif").exists());
}

/// An unchanged start leaves the record as it is (not rewritten).
#[test]
fn an_unchanged_start_leaves_the_record_alone() {
    let dir = Scratch::new("record-kept");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    write(&mods.join("x.dds"), b"x");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    let state = state_path(&target);
    let before = std::fs::metadata(&state).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(
        std::fs::metadata(&state).unwrap().modified().unwrap(),
        before
    );
    assert!(std::fs::read_to_string(&state)
        .unwrap()
        .ends_with("end\t4\n"));
}

/// The folder swapped for a folder link to a copy of a game folder, then a mod changed: the link
/// is removed itself, the copy is not deleted or changed in any way.
#[test]
fn a_link_at_the_folders_place_is_never_followed() {
    let dir = Scratch::new("target-link");
    let game = game(&dir.0);
    let copy = dir.0.join("copy of the game");
    write(&copy.join("FalloutNV.exe"), b"copy exe");
    write(&copy.join("Data").join("Textures").join("a.dds"), b"copy's");
    write(&copy.join("Data").join("FalloutNV.esm"), b"copy esm");
    let mods = dir.0.join("mods");
    write(&mods.join("Textures").join("a.dds"), b"mod's");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
    folder_link(&copy, &target);
    std::fs::remove_file(mods.join("Textures").join("a.dds")).unwrap();
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(
        std::fs::read(copy.join("FalloutNV.exe")).unwrap(),
        b"copy exe"
    );
    assert_eq!(
        std::fs::read(copy.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"copy's"
    );
    assert_eq!(
        std::fs::read(copy.join("Data").join("FalloutNV.esm")).unwrap(),
        b"copy esm"
    );
    let meta = std::fs::symlink_metadata(&target).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert_eq!(
        std::fs::read(target.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
}

/// The guard itself: no change outside its folder, none inside the game's or a layer's folder,
/// no file written whole but the ones it names.
#[test]
fn the_guard_refuses_changes_outside() {
    use crate::fsops::{act, Guard, Op};
    let dir = Scratch::new("guard");
    let root = dir.0.join("root");
    let game = dir.0.join("game");
    write(&game.join("g.txt"), b"game");
    write(&dir.0.join("outside.txt"), b"out");
    write(&root.join("mine.txt"), b"mine");
    // Inside one another, either way round: no guard.
    assert!(Guard::new(&root, &[root.join("game inside")], &[]).is_err());
    assert!(Guard::new(&root, std::slice::from_ref(&dir.0), &[]).is_err());
    // A file named to be written whole, but inside the game's folder: refused.
    let in_game = game.join("named.txt");
    let guard = Guard::new(
        &root,
        std::slice::from_ref(&game),
        std::slice::from_ref(&in_game),
    )
    .unwrap();
    assert!(act(
        &guard,
        Op::WriteWhole {
            to: &in_game,
            temporary: &game.join("named.part"),
            bytes: b"x"
        }
    )
    .is_err());
    assert!(!in_game.exists() && !game.join("named.part").exists());
    // A copy onto an existing name (here a hard link to the game's file) is refused, never
    // written through.
    let source = dir.0.join("source.txt");
    write(&source, b"new");
    std::fs::hard_link(game.join("g.txt"), root.join("linked.txt")).unwrap();
    assert!(act(
        &guard,
        Op::Copy {
            from: &source,
            to: &root.join("linked.txt")
        }
    )
    .is_err());
    assert_eq!(std::fs::read(game.join("g.txt")).unwrap(), b"game");
    let guard = Guard::new(
        &root,
        std::slice::from_ref(&game),
        &[dir.0.join("allowed.txt")],
    )
    .unwrap();
    assert!(act(&guard, Op::RemoveName(&dir.0.join("outside.txt"))).is_err());
    assert!(act(&guard, Op::RemoveName(&game.join("g.txt"))).is_err());
    assert!(act(&guard, Op::RemoveName(&root.join("..").join("outside.txt"))).is_err());
    assert!(act(&guard, Op::RemoveTree(&dir.0)).is_err());
    assert!(act(
        &guard,
        Op::WriteWhole {
            to: &dir.0.join("outside.txt"),
            temporary: &dir.0.join("outside.part"),
            bytes: b"x"
        }
    )
    .is_err());
    assert_eq!(std::fs::read(dir.0.join("outside.txt")).unwrap(), b"out");
    assert_eq!(std::fs::read(game.join("g.txt")).unwrap(), b"game");
    act(&guard, Op::RemoveName(&root.join("mine.txt"))).unwrap();
    act(
        &guard,
        Op::WriteWhole {
            to: &dir.0.join("allowed.txt"),
            temporary: &dir.0.join("allowed.part"),
            bytes: b"ok",
        },
    )
    .unwrap();
    assert_eq!(std::fs::read(dir.0.join("allowed.txt")).unwrap(), b"ok");
    for bad in ["..", ".", "", "a\\b", "c:d", "a\tb", "a\nb"] {
        assert!(!crate::fsops::valid_part(bad), "{bad:?}");
    }
}

/// Nothing in the crate changes the disk except through `fsops`: the source is read and every
/// call that removes, renames, copies or links a file is looked for outside it.
#[test]
fn only_fsops_changes_the_disk() {
    // Anywhere in the code.
    let banned = [
        "remove_file(",
        "remove_dir",
        "fs::copy(",
        "io::copy(",
        "symlink_file(",
        "symlink_dir(",
        "File::create",
        "File::options",
        "OpenOptions",
        "create_dir",
        "set_permissions(",
        "set_len(",
        "set_times(",
        "set_modified(",
        "set_readonly(",
        "process::Command",
        "Command::new",
        "unix::fs",
        "windows::fs",
    ];
    // Called as functions (not a method, not part of a longer name, not a `fn` of its own): with
    // or without a path before them.
    let calls = ["hard_link(", "symlink(", "copy(", "write(", "rename("];
    let called = |code: &str, name: &str| {
        code.match_indices(name).any(|(at, _)| {
            let before = &code[..at];
            let joined = before
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
            !joined && !before.trim_end().ends_with("fn")
        })
    };
    // The code of a line: up to a `//` outside a string literal.
    fn code_of(line: &str) -> &str {
        let bytes = line.as_bytes();
        let mut in_string = false;
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if in_string => i += 1,
                b'\'' if !in_string && bytes.get(i + 2) == Some(&b'\'') => i += 2,
                b'"' => in_string = !in_string,
                b'/' if !in_string && bytes.get(i + 1) == Some(&b'/') => return &line[..i],
                _ => {}
            }
            i += 1;
        }
        line
    }
    fn walk(folder: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    // This crate's code, and the mod chooser's (crates/mod-chooser: every change it makes goes
    // through this crate's guarded functions); its tests make their own scratch folders.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(&crate_dir.join("src"), &mut files);
    assert!(files.len() >= 4, "{files:?}");
    let mut chooser = Vec::new();
    walk(
        &crate_dir.join("..").join("mod-chooser").join("src"),
        &mut chooser,
    );
    assert!(!chooser.is_empty());
    let mut sources: Vec<(String, String, &[&str])> = Vec::new();
    for path in files.into_iter().chain(chooser) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "fsops.rs" || name == "tests.rs" {
            continue;
        }
        sources.push((name, std::fs::read_to_string(&path).unwrap(), &[]));
    }
    // The `--install-mods` code of nv-rs's program and of Play.exe (Windows; the launcher's
    // play/mod.rs): only the function that starts the chooser. Play.exe's starts nv-rs's
    // program, so it may name `Command::new`.
    let root = crate_dir.join("..").join("..");
    for (file, allowed) in [
        (root.join("viewer").join("src").join("main.rs"), &[][..]),
        (
            root.join("crates")
                .join("nv-launcher")
                .join("src")
                .join("play")
                .join("mod.rs"),
            &["Command::new"][..],
        ),
    ] {
        let text = std::fs::read_to_string(&file).unwrap();
        let at = text
            .find("\nfn install_mods(")
            .unwrap_or_else(|| panic!("{}: no fn install_mods", file.display()));
        let end = at + text[at..].find("\n}\n").expect("the function's end");
        sources.push((
            format!("{} install_mods", file.display()),
            text[at..end].to_owned(),
            allowed,
        ));
    }
    for (name, text, allowed) in &sources {
        let code: Vec<&str> = text.lines().map(code_of).collect();
        for (n, line) in code.iter().enumerate() {
            for b in banned.iter().filter(|b| !allowed.contains(b)) {
                assert!(!line.contains(b), "{name}:{}: {b} outside fsops", n + 1);
            }
            for c in calls {
                assert!(!called(line, c), "{name}:{}: {c} outside fsops", n + 1);
            }
        }
        // `use std::...;` statements (over lines too): none names `fs`, so no alias can hide a
        // banned call.
        let all = code.join("\n");
        for (at, _) in all.match_indices("use ") {
            if at > 0 && all[..at].chars().last().is_some_and(char::is_alphanumeric) {
                continue;
            }
            let statement = &all[at..at + all[at..].find(';').unwrap_or(all.len() - at)];
            let words: Vec<&str> = statement
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .filter(|w| !w.is_empty())
                .collect();
            if words.get(1) == Some(&"std") {
                assert!(
                    !words.contains(&"fs"),
                    "{name}: {statement:?} imports from std::fs"
                );
            }
        }
    }
}

/// A leftover temporary name that is a hard link to a game file (on Linux and macOS also a
/// symbolic link): never opened for writing, so the game's file is unchanged; the build removes
/// the name and writes its own.
#[test]
fn a_link_at_a_temporary_name_never_leads_a_write() {
    let dir = Scratch::new("part-link");
    let game = game(&dir.0);
    let esm = game.join("Data").join("FalloutNV.esm");
    let mods = dir.0.join("mods");
    write(&mods.join("Mine.esp"), &plugin(&["FalloutNV.esm"]));
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p").join("plugins.txt");
    std::fs::create_dir_all(plugins.parent().unwrap()).unwrap();
    let mut parts = vec![
        (dir.0.join("modgame.nv-rs-state.part"), false),
        (dir.0.join("p").join("plugins.txt.part"), false),
    ];
    if cfg!(unix) {
        parts.push((dir.0.join("p").join("plugins.txt.part"), true));
        parts.push((dir.0.join("modgame.nv-rs-state.part"), true));
    }
    for (part, symbolic) in parts {
        let _ = std::fs::remove_file(&part);
        if symbolic {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&esm, &part).unwrap();
        } else {
            std::fs::hard_link(&esm, &part).unwrap();
        }
        // From nothing each time, so the record is written too.
        let _ = std::fs::remove_file(state_path(&target));
        build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
        assert_eq!(std::fs::read(&esm).unwrap(), b"esm", "{}", part.display());
        assert!(!part.exists(), "{}", part.display());
        assert_eq!(std::fs::read_to_string(&plugins).unwrap(), "Mine.esp\r\n");
    }
}

/// A folder deleted by hand inside the built folder, then its mod's file removed: nothing there
/// to remove, the build goes on.
#[test]
fn a_folder_deleted_by_hand_is_skipped() {
    let dir = Scratch::new("hand-deleted");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    let mesh = mods.join("meshes").join("m").join("x.nif");
    write(&mesh, b"mesh");
    write(&mods.join("textures").join("b.dds"), b"b");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p").join("plugins.txt");
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    std::fs::remove_dir_all(target.join("Data").join("meshes")).unwrap();
    std::fs::remove_file(&mesh).unwrap();
    let built = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(built.removed, 0, "{built:?}");
    assert_eq!(
        // The game's own spelling of the folder wins (Linux's file systems tell case apart).
        std::fs::read(target.join("Data").join("Textures").join("b.dds")).unwrap(),
        b"b"
    );
}

/// Folders made with their way (the plugin list's, a missing layer) are never made inside the
/// game's folder; a forbidden folder itself may be made when missing.
#[test]
fn makes_no_folder_in_the_game() {
    use crate::fsops::{act, Guard, Op};
    let dir = Scratch::new("make-all");
    let game = game(&dir.0);
    let layer = dir.0.join("mods");
    let guard = Guard::new(&dir.0.join("modgame"), &[game.clone(), layer.clone()], &[]).unwrap();
    assert!(act(&guard, Op::MakeDirAll(&game.join("Data").join("new"))).is_err());
    assert!(!game.join("Data").join("new").exists());
    act(&guard, Op::MakeDirAll(&layer)).unwrap();
    act(&guard, Op::MakeDirAll(&dir.0.join("p").join("q"))).unwrap();
    assert!(layer.is_dir() && dir.0.join("p").join("q").is_dir());
}

/// A layer that lies inside the game's folder (a package unpacked there) is never made, though
/// a forbidden folder itself may be; and a whole build with such a layer writes nothing there.
#[test]
fn a_layer_inside_the_game_folder_is_never_made() {
    use crate::fsops::{act, Guard, Op};
    let dir = Scratch::new("layer-in-game");
    let game = game(&dir.0);
    let layer = game.join("nv-rs").join("mods");
    let guard = Guard::new(&dir.0.join("modgame"), &[game.clone(), layer.clone()], &[]).unwrap();
    assert!(act(&guard, Op::MakeDirAll(&layer)).is_err());
    assert!(!game.join("nv-rs").exists());
    let plugins = dir.0.join("p.txt");
    assert!(build(
        &game,
        std::slice::from_ref(&layer),
        &dir.0.join("modgame"),
        &plugins
    )
    .is_err());
    assert!(!game.join("nv-rs").exists());
}

/// The real location of a path not there yet: its nearest existing folder's, with the rest after.
#[test]
fn a_missing_paths_real_location_starts_at_its_nearest_folder() {
    let dir = Scratch::new("real");
    let real = crate::fsops::real(&dir.0.join("missing").join("x")).unwrap();
    assert_eq!(
        real,
        std::fs::canonicalize(&dir.0)
            .unwrap()
            .join("missing")
            .join("x")
    );
}

/// A link leading nowhere on the way is not walked past: no real location, the change refused.
#[cfg(unix)]
#[test]
fn a_link_leading_nowhere_has_no_real_location() {
    use crate::fsops::{act, Guard, Op};
    let dir = Scratch::new("dangling");
    let root = dir.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(dir.0.join("nowhere"), root.join("gone")).unwrap();
    assert!(crate::fsops::real(&root.join("gone").join("x")).is_err());
    let guard = Guard::new(&root, &[], &[]).unwrap();
    assert!(act(&guard, Op::MakeDirAll(&root.join("gone").join("x"))).is_err());
    assert!(!dir.0.join("nowhere").exists());
}

/// The mod installs folder swapped for a link: no mod folder is removed through it.
#[test]
fn removing_needs_a_plain_installs_folder() {
    let dir = Scratch::new("installs-link");
    let real_installs = dir.0.join("elsewhere");
    write(&real_installs.join("01 Mod").join("a.txt"), b"keep");
    let installs = dir.0.join("mod installs");
    folder_link(&real_installs, &installs);
    let game = game(&dir.0);
    assert!(crate::fsops::remove_mod_folder(&installs, &installs.join("01 Mod"), &game).is_err());
    assert_eq!(
        std::fs::read(real_installs.join("01 Mod").join("a.txt")).unwrap(),
        b"keep"
    );
}

/// A link at the built folder's place leading to the game's folder itself: the link is removed
/// (never followed), the folder built again, the game untouched.
#[test]
fn a_link_at_the_folders_place_to_the_game_itself() {
    let dir = Scratch::new("target-game-link");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    write(&mods.join("Textures").join("a.dds"), b"mod's");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    folder_link(&game, &target);
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    let meta = std::fs::symlink_metadata(&target).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert_eq!(
        std::fs::read(target.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"mod's"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("Textures").join("a.dds")).unwrap(),
        b"game's"
    );
    assert!(game.join("FalloutNV.exe").exists());
    assert!(game.join("Data").join("FalloutNV.esm").exists());
}

/// The built folder inside the game's folder, the game's folder inside it, or a layer inside it:
/// refused before anything is changed.
#[test]
fn folders_inside_one_another_are_refused() {
    let dir = Scratch::new("nested");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    let plugins = dir.0.join("p").join("plugins.txt");
    for (target, layer) in [
        (game.join("modgame"), mods.clone()),
        (dir.0.clone(), mods.clone()),
        (dir.0.join("modgame"), dir.0.join("modgame").join("mods")),
    ] {
        let error = build(&game, std::slice::from_ref(&layer), &target, &plugins).unwrap_err();
        assert!(
            error.step.contains("inside one another"),
            "{}: {error:?}",
            target.display()
        );
    }
    assert!(!game.join("modgame").exists());
    assert!(!plugins.exists());
    assert_eq!(
        std::fs::read(game.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
}

/// Linux and macOS: a hand-placed name with a tab and a line break cannot forge a record line:
/// it is left out (said in `skipped`), and the file outside is untouched.
#[cfg(unix)]
#[test]
fn a_name_cannot_forge_a_record_line() {
    let dir = Scratch::new("forge-name");
    let game = game(&dir.0);
    let mods = dir.0.join("mods");
    let outside = dir.0.join("outside.txt");
    write(&outside, b"keep");
    let forged = format!("a\tb\nm\t4\t0\t{}\t../outside.txt", outside.display());
    write(&mods.join(&forged), b"x");
    let target = dir.0.join("modgame");
    let plugins = dir.0.join("p.txt");
    let built = build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(built.skipped.len(), 1, "{built:?}");
    assert!(!std::fs::read_to_string(state_path(&target))
        .unwrap()
        .contains("outside"));
    std::fs::remove_file(mods.join(&forged)).unwrap();
    build(&game, std::slice::from_ref(&mods), &target, &plugins).unwrap();
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep");
}

/// How long an unchanged start takes for 100,000 mod files (run by hand: `--ignored`, with
/// NV_BENCH_DIR naming a scratch folder, which it fills and empties).
#[test]
#[ignore]
fn an_unchanged_start_of_100000_files() {
    let Some(dir) = std::env::var_os("NV_BENCH_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir).join("nv-mod-install-bench");
    let _ = std::fs::remove_dir_all(&dir);
    let game = game(&dir);
    let mods = dir.join("mod installs").join("01 Big");
    for folder in 0..100 {
        let at = mods.join("Textures").join(format!("f{folder}"));
        std::fs::create_dir_all(&at).unwrap();
        for file in 0..1000 {
            std::fs::write(at.join(format!("t{file}.dds")), b"x").unwrap();
        }
    }
    let target = dir.join("modgame");
    let plugins = dir.join("p.txt");
    let layers = [mods.clone()];
    let started = std::time::Instant::now();
    let first = build(&game, &layers, &target, &plugins).unwrap();
    let first_time = started.elapsed();
    let started = std::time::Instant::now();
    let again = build(&game, &layers, &target, &plugins).unwrap();
    let again_time = started.elapsed();
    eprintln!(
        "100,000 files: first build {first_time:?} ({} made), unchanged start {again_time:?} ({} made)",
        first.made, again.made
    );
    assert_eq!(again.made, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------------------------------------
// Review of #750: the chooser's changes, guarded here

/// A killed install's staging folder (part of its files, no record) is neither listed nor laid
/// over the game, and the next start clears it; a link at a staging name is removed itself, never
/// followed.
#[test]
fn a_leftover_staging_folder_is_not_a_mod_and_is_cleared() {
    let dir = Scratch::new("staging-left");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    let folder = next_folder(&installs, "Big Mod").unwrap();
    let staging = make_staging(&installs, &folder, &game).unwrap();
    assert_eq!(staging, installs.join(".staging-01 Big Mod"));
    write(&staging.join("Textures").join("half.dds"), b"half");
    // An older leftover named like a mod but without its record: not a mod either.
    write(&installs.join("02 Old.part").join("x.esp"), b"x");
    assert!(installed(&installs).unwrap().is_empty());
    assert_eq!(
        layers(&installs, &loose).unwrap(),
        std::slice::from_ref(&loose)
    );
    let outside = dir.0.join("outside");
    write(&outside.join("keep.txt"), b"keep");
    folder_link(&outside, &installs.join(".staging-09 Link"));
    assert_eq!(clear_staging(&installs, &game).unwrap().cleared, 2);
    assert!(!staging.exists());
    assert!(std::fs::symlink_metadata(installs.join(".staging-09 Link")).is_err());
    assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"keep");
    let again = clear_staging(&installs, &game).unwrap();
    assert_eq!((again.cleared, again.left.len()), (0, 0));
    // The next install never takes the leftover's number.
    assert_eq!(
        next_folder(&installs, "New").unwrap(),
        installs.join("03 New")
    );
}

/// The whole guarded install: staging made, the record written whole, the folder renamed into
/// place (never over a folder there), then listed; the archive kept under a free name.
#[test]
fn an_install_finishes_through_the_guard() {
    let dir = Scratch::new("finish");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    let folder = next_folder(&installs, "Gun Pack").unwrap();
    let staging = make_staging(&installs, &folder, &game).unwrap();
    write(&staging.join("GunPack.esp"), b"TES4");
    let record = Record {
        name: "Gun Pack".into(),
        files: vec!["GunPack.esp".into()],
        ..Record::default()
    };
    // A folder already at the final name: refused, nothing replaced.
    std::fs::create_dir_all(&folder).unwrap();
    assert!(finish(&installs, &staging, &folder, &record, &game).is_err());
    std::fs::remove_dir(&folder).unwrap();
    finish(&installs, &staging, &folder, &record, &game).unwrap();
    assert!(!staging.exists());
    let listed = installed(&installs).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        Record::parse(&std::fs::read_to_string(folder.join(RECORD)).unwrap()),
        record
    );
    // No temporary name left.
    let names: Vec<String> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    write(&loose.join("Gun Pack.7z"), b"first");
    write(&loose.join(KEPT).join("Gun Pack.7z"), b"older");
    let kept = keep_archive(&loose, &loose.join("Gun Pack.7z"), &game).unwrap();
    assert_eq!(kept, loose.join(KEPT).join("Gun Pack (2).7z"));
    assert_eq!(std::fs::read(&kept).unwrap(), b"first");
    assert_eq!(
        std::fs::read(loose.join(KEPT).join("Gun Pack.7z")).unwrap(),
        b"older"
    );
    assert!(!loose.join("Gun Pack.7z").exists());
    // Only an archive directly in the mods folder.
    write(&dir.0.join("elsewhere.7z"), b"x");
    assert!(keep_archive(&loose, &dir.0.join("elsewhere.7z"), &game).is_err());
}

/// The installs folder or the mods folder inside the game's folder (a junction or symbolic link
/// to it): every change is refused and nothing is written in the game's folder.
#[test]
fn nothing_is_changed_inside_the_game() {
    let dir = Scratch::new("in-game");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    folder_link(&game.join("Data"), &installs);
    folder_link(&game.join("Data"), &loose);
    let folder = installs.join("01 Mod");
    assert!(make_staging(&installs, &folder, &game).is_err());
    write(
        &game.join("Data").join(".staging-01 Mod").join("a"),
        b"game's own",
    );
    let refused = clear_staging(&installs, &game).unwrap();
    assert_eq!((refused.cleared, refused.left.len()), (0, 1), "{refused:?}");
    assert!(game.join("Data").join(".staging-01 Mod").join("a").exists());
    write(
        &game.join("Data").join("01 Mod").join(RECORD),
        b"name: Mod\n",
    );
    assert!(remove(&installs, &folder, &game).is_err());
    assert!(game.join("Data").join("01 Mod").join(RECORD).exists());
    assert!(finish(
        &installs,
        &installs.join(".staging-01 Mod"),
        &installs.join("02 Mod"),
        &Record::default(),
        &game
    )
    .is_err());
    assert!(!game.join("Data").join("02 Mod").exists());
    write(&game.join("Data").join("a.7z"), b"x");
    assert!(keep_archive(&loose, &loose.join("a.7z"), &game).is_err());
    assert!(game.join("Data").join("a.7z").exists());
    assert!(!game.join("Data").join(KEPT).exists());
}

/// A rename never lands on a name that is there, not even a link leading nowhere, and never
/// leaves the guard's folder.
#[test]
fn a_rename_needs_a_free_name_inside() {
    use crate::fsops::{act, Guard, Op};
    let dir = Scratch::new("rename");
    let root = dir.0.join("root");
    write(&root.join("a.txt"), b"a");
    write(&root.join("b.txt"), b"b");
    let guard = Guard::new(&root, &[dir.0.join("game")], &[]).unwrap();
    assert!(act(
        &guard,
        Op::Rename {
            from: &root.join("a.txt"),
            to: &root.join("b.txt")
        }
    )
    .is_err());
    assert!(act(
        &guard,
        Op::Rename {
            from: &root.join("a.txt"),
            to: &dir.0.join("out.txt")
        }
    )
    .is_err());
    assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"b");
    act(
        &guard,
        Op::Rename {
            from: &root.join("a.txt"),
            to: &root.join("c.txt"),
        },
    )
    .unwrap();
    assert_eq!(std::fs::read(root.join("c.txt")).unwrap(), b"a");
}

/// One installer at a time: while one holds the lock a second is refused, and once it lets go (as
/// when its program ends, however it ends) the next takes it. The lock file is never a mod, never
/// a layer, never cleared as a leftover; it can't be taken in the game's folder.
#[test]
fn one_installer_holds_the_lock() {
    let dir = Scratch::new("installer-lock");
    let game = game(&dir.0);
    let installs = dir.0.join("mod installs");
    let loose = dir.0.join("mods");
    let first = hold_installer(&installs, &game).unwrap();
    assert!(first.is_some());
    assert!(hold_installer(&installs, &game).unwrap().is_none());
    drop(first);
    let again = hold_installer(&installs, &game).unwrap();
    assert!(again.is_some());
    assert!(installs.join(LOCK).is_file());
    assert!(installed(&installs).unwrap().is_empty());
    assert_eq!(
        layers(&installs, &loose).unwrap(),
        std::slice::from_ref(&loose)
    );
    assert_eq!(clear_staging(&installs, &game).unwrap().cleared, 0);
    assert!(installs.join(LOCK).is_file());
    assert_eq!(next_folder(&installs, "A").unwrap(), installs.join("01 A"));
    drop(again);
    // Inside the game's folder: refused, nothing made there.
    assert!(hold_installer(&game.join("Data").join("x"), &game).is_err());
    assert!(!game.join("Data").join("x").exists());
}
