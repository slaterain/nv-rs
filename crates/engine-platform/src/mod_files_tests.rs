//! Tests of [`super`]: the names kept inside the game's folder, the order reads look in, the port's
//! folder written and the game's never (the INI text rules are the core's tests).

use super::*;
use engine_core::mod_files::{profile_string, write_profile_string};

fn engine(game: &Path) -> Engine {
    let mut e = Engine::new();
    e.kernel.current_directory = game.to_string_lossy().replace('/', "\\").into_bytes();
    e
}

/// A scratch folder of this test's own (removed first).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv-mod-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Names stay inside the game's folder: `..`, drives, roots and other full paths are refused.
#[test]
fn names_outside_the_game_folder_are_refused() {
    let e = engine(Path::new(r"C:\Games\FNV"));
    assert_eq!(
        relative(&e, br"Data\Config\MCM.ini").unwrap(),
        br"Data\Config\MCM.ini"
    );
    assert_eq!(
        relative(&e, b"./Data//Config/MCM.ini").unwrap(),
        br"Data\Config\MCM.ini"
    );
    assert_eq!(
        relative(&e, br"c:\games\fnv\Data\Config\MCM.ini").unwrap(),
        br"Data\Config\MCM.ini"
    );
    for bad in [
        &br"..\x.ini"[..],
        br"Data\..\..\x.ini",
        br"C:\Games\FNVx\a.ini",
        br"C:\Other\a.ini",
        br"\x.ini",
        br"\\server\share\x.ini",
        br"C:x.ini",
        br"Data\Config\",
        b"",
        b".",
    ] {
        assert_eq!(
            relative(&e, bad),
            None,
            "{:?}",
            String::from_utf8_lossy(bad)
        );
    }
}

/// Reads look at what this run wrote, then the port's folder, then the game's; writes go to the
/// run and the port's folder, never the game's folder.
#[test]
fn reads_prefer_the_ports_folder_and_writes_never_reach_the_game() {
    let game = scratch("game");
    let kept = scratch("kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::write(
        game.join(r"Data\Config\MCM.ini"),
        b"[List]\r\nMaxWidth=350\r\n",
    )
    .unwrap();
    let mut e = engine(&game);
    let name = br"Data\Config\MCM.ini";
    // Without a folder: the game's file.
    assert_eq!(
        profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"350"
    );
    // A folder with a copy from an earlier session wins.
    std::fs::create_dir_all(kept.join(r"Data\Config")).unwrap();
    std::fs::write(
        kept.join(r"Data\Config\MCM.ini"),
        b"[List]\r\nMaxWidth=200\r\n",
    )
    .unwrap();
    set_folder(&mut e, Some(kept.clone()));
    assert_eq!(
        profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"200"
    );
    assert!(exists(&e, name));
    // A write: into the port's folder and the run; the game's file untouched.
    assert!(write_profile_string(
        &mut e,
        name,
        b"List",
        b"MaxWidth",
        b"100.0000"
    ));
    assert_eq!(
        std::fs::read(kept.join(r"Data\Config\MCM.ini")).unwrap(),
        b"[List]\r\nMaxWidth=100.0000\r\n"
    );
    assert!(!kept.join(r"Data\Config\MCM.ini.part").exists());
    assert_eq!(
        std::fs::read(game.join(r"Data\Config\MCM.ini")).unwrap(),
        b"[List]\r\nMaxWidth=350\r\n"
    );
    assert_eq!(
        profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"100.0000"
    );
    // A name outside the game's folder is written nowhere.
    let outside = game.parent().unwrap().join("nv-mod-files-outside.ini");
    let outside_name = outside.to_string_lossy().replace('/', "\\").into_bytes();
    assert!(!write(&mut e, &outside_name, b"x"));
    assert!(!write(&mut e, br"..\escape.ini", b"x"));
    assert!(!outside.exists());
    assert!(!game.parent().unwrap().join("escape.ini").exists());
    // A file in a folder that exists nowhere: not written, as Windows answers.
    assert!(!write_profile_string(
        &mut e,
        br"Data\NoSuch\x.ini",
        b"A",
        b"x",
        b"1"
    ));
    assert!(!exists(&e, br"Data\NoSuch\x.ini"));
    // Once made, it can be.
    assert!(create_directory(&mut e, br"Data\NoSuch"));
    assert!(write_profile_string(
        &mut e,
        br"Data\NoSuch\x.ini",
        b"A",
        b"x",
        b"1"
    ));
    assert_eq!(
        std::fs::read(kept.join(r"Data\NoSuch\x.ini")).unwrap(),
        b"[A]\r\nx=1\r\n"
    );
    assert!(!game.join(r"Data\NoSuch").exists());
    let _ = std::fs::remove_dir_all(&game);
    let _ = std::fs::remove_dir_all(&kept);
}

/// Without a folder a write lasts for the run only: read back in it, on disk nowhere.
#[test]
fn without_a_folder_writes_last_for_the_session() {
    let game = scratch("session");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let mut e = engine(&game);
    assert!(write(&mut e, br"Data\Config\New.ini", b"\x08"));
    assert!(write_profile_string(
        &mut e,
        br"Data\Config\New.ini",
        b"S",
        b"K",
        b"1.0000"
    ));
    assert_eq!(
        read(&e, br"data\config\new.ini").unwrap(),
        b"[S]\r\nK=1.0000\r\n"
    );
    assert!(!game.join(r"Data\Config\New.ini").exists());
    let _ = std::fs::remove_dir_all(&game);
}
