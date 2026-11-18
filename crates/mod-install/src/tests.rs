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
    let list = plugin_list(&[
        (
            "b.esp".into(),
            plugin(&["FalloutNV.esm", "lonesomeroad.esm"]),
        ),
        ("A.ESP".into(), plugin(&["FalloutNV.esm"])),
        ("Own.esm".into(), plugin(&["DeadMoney.esm"])),
    ]);
    assert_eq!(
        list,
        "DeadMoney.esm\r\nLonesomeRoad.esm\r\nOwn.esm\r\nA.ESP\r\nb.esp\r\n"
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
        (3, 2, false)
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
    assert_eq!(built.mod_files, 6);
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
        assert_eq!(classify(Some(1), true), Failure::Denied);
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
    assert!(text(Failure::NoLinks).contains("NTFS"));
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
    for name in ["10 Ten", "2 Two", "02 Also two", "junk", "x Not numbered"] {
        std::fs::create_dir_all(installs.join(name)).unwrap();
    }
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
        installs.join("11 Weapon Mod")
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
    assert!(remove(&installs, &installs).is_err());
    assert!(remove(&installs, &dir.0).is_err());
    assert!(remove(&installs, &installs.join("01 One").join("Textures")).is_err());
    assert!(remove(&installs, &installs.join("..")).is_err());
    assert!(one.exists());
    remove(&installs, &one).unwrap();
    assert!(!one.exists());
    assert!(dir.0.join("keep.txt").exists());
}
