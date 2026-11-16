//! Tests of [`super`]: the names kept inside the game's folder, the order reads look in, the port's
//! folder written and the game's never, the failures Windows reports, the kernel lock, and MCM's
//! whole `SetModINI` sequence on the real calls (the INI text rules are the core's tests).

use super::*;
use engine_core::mod_files as seam;

fn engine(game: &Path) -> Engine {
    let mut e = Engine::new();
    e.kernel.current_directory = game.to_string_lossy().replace('/', "\\").into_bytes();
    e
}

/// A scratch folder of a test's own, removed when the test ends, however it ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("nv-mod-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// Every file and folder under `root`, with each file's bytes.
fn tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push((path.clone(), None));
                stack.push(path);
            } else {
                out.push((path.clone(), std::fs::read(&path).ok()));
            }
        }
    }
    out.sort();
    out
}

/// Names stay inside the game's folder; devices are refused in any part; the directory calls
/// take one trailing separator.
#[test]
fn names_outside_the_game_folder_and_devices_are_refused() {
    let e = engine(Path::new(r"C:\Games\FNV"));
    let ok = |name: &[u8], want: &[u8]| assert_eq!(relative(&e, name).unwrap(), want);
    ok(br"Data\Config\MCM.ini", br"Data\Config\MCM.ini");
    ok(b"./Data//Config/MCM.ini", br"Data\Config\MCM.ini");
    ok(
        br"c:\games\fnv\Data\Config\MCM.ini",
        br"Data\Config\MCM.ini",
    );
    ok(br"Data\Console.ini", br"Data\Console.ini");
    ok(br"Data\COM10.ini", br"Data\COM10.ini");
    for bad in [
        &br"..\x.ini"[..],
        br"Data\..\..\x.ini",
        br"C:\Games\FNVx\a.ini",
        br"C:\Other\a.ini",
        br"\x.ini",
        br"\\server\share\x.ini",
        br"\\?\C:\Games\FNV\x.ini",
        br"C:x.ini",
        br"Data\x.ini:stream",
        br"Data\Config\",
        b"",
        b".",
    ] {
        assert_eq!(
            relative(&e, bad),
            Err(NotMine::Outside),
            "{:?}",
            String::from_utf8_lossy(bad)
        );
    }
    for device in [
        &br"Data\Config\CON.ini"[..],
        br"CON",
        br"Data\nul",
        br"Data\Aux.txt\x.ini",
        br"Data\com1.ini",
        br"Data\LPT9",
        br"Data\PRN.part",
        br"Data\CONIN$",
        br"Data\conout$.ini",
        br"Data\NUL .ini",
        b"Data\\COM\xb9.ini",
    ] {
        assert_eq!(
            relative(&e, device),
            Err(NotMine::Device),
            "{:?}",
            String::from_utf8_lossy(device)
        );
    }
    assert_eq!(
        relative_as(&e, br"Data\Config\", true).unwrap(),
        br"Data\Config"
    );
    assert_eq!(
        relative_as(&e, br"Data\Config\\", true),
        Err(NotMine::Outside)
    );
}

/// Reads look at this run's writes, then the port's folder, then the game's; a write goes to the
/// folder and the run, never the game's folder; names outside it are written nowhere.
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
    assert_eq!(
        seam::profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"350"
    );
    std::fs::create_dir_all(kept.join(r"Data\Config")).unwrap();
    std::fs::write(
        kept.join(r"Data\Config\MCM.ini"),
        b"[List]\r\nMaxWidth=200\r\n",
    )
    .unwrap();
    set_folder(&mut e, Some(kept.to_path_buf()));
    assert_eq!(
        seam::profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"200"
    );
    assert!(seam::exists(&e, name));
    seam::write_profile_string(&mut e, name, b"List", b"MaxWidth", b"100.0000").unwrap();
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
        seam::profile_string(&e, name, b"List", b"MaxWidth").unwrap(),
        b"100.0000"
    );
    // Outside the game's folder: written nowhere, made nowhere.
    let outside = game
        .parent()
        .unwrap()
        .join(format!("nv-mod-files-out-{}.ini", std::process::id()));
    let outside_name = outside.to_string_lossy().replace('/', "\\").into_bytes();
    assert_eq!(
        seam::write_or_error(&mut e, &outside_name, b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
    assert!(!seam::write(&mut e, br"..\escape.ini", b"x"));
    assert!(!seam::create_directory(&mut e, br"..\made"));
    assert!(!seam::create_directory(&mut e, &outside_name));
    assert!(!outside.exists());
    for place in [game.parent().unwrap(), kept.parent().unwrap()] {
        assert!(!place.join("escape.ini").exists());
        assert!(!place.join("made").exists());
    }
}

/// A kept file that is there but cannot be read (another program holds it) fails the read: the
/// game's shipped copy is not used in its place, and a write leaves the kept bytes as they are.
#[cfg(windows)]
#[test]
fn an_unreadable_kept_file_is_never_replaced() {
    use std::os::windows::fs::OpenOptionsExt;
    let game = scratch("held-game");
    let kept = scratch("held-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::write(game.join(r"Data\Config\MCM.ini"), b"[A]\r\nx=shipped\r\n").unwrap();
    std::fs::create_dir_all(kept.join(r"Data\Config")).unwrap();
    let file = kept.join(r"Data\Config\MCM.ini");
    std::fs::write(&file, b"[A]\r\nx=mine\r\n").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"Data\Config\MCM.ini";
    {
        let _held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&file)
            .unwrap();
        assert_eq!(seam::read_or_error(&e, name), Err(32), "sharing violation");
        assert_eq!(seam::profile_string(&e, name, b"A", b"x"), None);
        assert!(seam::exists(&e, name), "from attributes, not the bytes");
        assert_eq!(
            seam::write_profile_string(&mut e, name, b"A", b"x", b"new"),
            Err(32)
        );
    }
    assert_eq!(std::fs::read(&file).unwrap(), b"[A]\r\nx=mine\r\n");
    // A folder in the file's place: the read fails too.
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert_eq!(seam::read_or_error(&e, name), Err(ERROR_ACCESS_DENIED));
    assert!(!seam::exists(&e, name));
    assert_eq!(
        seam::write_or_error(&mut e, name, b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
}

/// `exists` answers from attributes: a game file another program holds without sharing (so its
/// bytes cannot be read) exists; a folder does not; a device does not.
#[cfg(windows)]
#[test]
fn exists_does_not_read_the_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let game = scratch("exists");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    let big = game.join(r"Data\Big.esm");
    std::fs::write(&big, b"TES4").unwrap();
    let e = engine(&game);
    let _held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&big)
        .unwrap();
    assert!(seam::read_or_error(&e, br"Data\Big.esm").is_err());
    assert!(seam::exists(&e, br"Data\Big.esm"));
    assert!(!seam::exists(&e, b"Data"));
    assert!(!seam::exists(&e, br"Data\Nope.esm"));
    assert!(!seam::exists(&e, br"Data\NUL"));
}

/// Without a folder, writes and folders live in the run's memory: nothing on any disk (the game's
/// folder, the process's current folder, the temporary folder), read back in the run.
#[test]
fn without_a_folder_nothing_touches_the_disk() {
    let game = scratch("session");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let before = tree(&game);
    let mut e = engine(&game);
    assert!(seam::create_directory(&mut e, br"Data\New\"));
    assert!(seam::directory_exists(&e, br"Data\New"));
    assert!(seam::write(&mut e, br"Data\New\x.ini", b"\x08"));
    seam::write_profile_string(&mut e, br"Data\New\x.ini", b"S", b"K", b"1.0000").unwrap();
    assert_eq!(
        seam::read(&e, br"data\new\X.INI").unwrap(),
        b"[S]\r\nK=1.0000\r\n"
    );
    assert_eq!(tree(&game), before, "the game's folder is as it was");
    for root in [std::env::current_dir().unwrap(), std::env::temp_dir()] {
        assert!(!root.join(r"Data\New").exists(), "{}", root.display());
    }
}

/// A write into an empty port folder makes the folders its name says (when the game has them)
/// and puts the bytes on disk; a folder that exists nowhere fails as Windows does.
#[test]
fn a_write_into_an_empty_port_folder_makes_its_folders() {
    let game = scratch("empty-game");
    let kept = scratch("empty-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    seam::write_profile_string(&mut e, br"Data\Config\New.ini", b"A", b"x", b"1").unwrap();
    assert_eq!(
        std::fs::read(kept.join(r"Data\Config\New.ini")).unwrap(),
        b"[A]\r\nx=1\r\n"
    );
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\NoSuch\x.ini", b"x"),
        Err(ERROR_PATH_NOT_FOUND)
    );
    assert!(!kept.join(r"Data\NoSuch").exists());
    assert_eq!(
        seam::create_directory_or_error(&mut e, br"Data\Gone\Deep\"),
        Err(ERROR_PATH_NOT_FOUND)
    );
    assert_eq!(
        seam::create_directory_or_error(&mut e, br"Data\Config\"),
        Err(ERROR_ALREADY_EXISTS)
    );
    assert!(seam::create_directory(&mut e, br"Data\Made\"));
    assert!(kept.join(r"Data\Made").is_dir());
    assert!(!game.join(r"Data\Made").exists());
}

/// A port folder that cannot be written fails the call and changes nothing: no `.part`, no copy
/// in memory; nor does a name that is a folder.
#[test]
fn a_failed_folder_write_is_a_failure() {
    let game = scratch("fail-game");
    let base = scratch("fail-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    // The port's folder is a file: nothing can be made under it.
    let kept = base.join("kept");
    std::fs::write(&kept, b"a file").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.clone()));
    assert!(seam::write_or_error(&mut e, br"Data\Config\A.ini", b"x").is_err());
    assert!(!seam::exists(&e, br"Data\Config\A.ini"));
    assert!(!seam::create_directory(&mut e, br"Data\New"));
    assert!(!seam::directory_exists(&e, br"Data\New"));
    assert_eq!(std::fs::read(&kept).unwrap(), b"a file");
    // A name that is a folder.
    let mut e = engine(&game);
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\Config", b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
}

/// A junction inside the port's folder that leads out is not written through.
#[cfg(windows)]
#[test]
fn a_junction_out_of_the_port_folder_is_refused() {
    let game = scratch("link-game");
    let kept = scratch("link-kept");
    let away = scratch("link-away");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(kept.join("Data"))
        .arg(&*away)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let refused = seam::write_or_error(&mut e, br"Data\Config\MCM.ini", b"x");
    let made_dir = seam::create_directory(&mut e, br"Data\Other");
    let away_tree = tree(&away);
    let _ = std::fs::remove_dir(kept.join("Data"));
    assert_eq!(refused, Err(ERROR_ACCESS_DENIED));
    assert!(!made_dir);
    assert!(away_tree.is_empty(), "nothing made through the junction");
}

/// MCM's `SetModINI` sequence on the real calls (`GetFileAttributesA("Data\Config\")`,
/// `CreateDirectoryA`, `fopen("r")`, `fopen("w")` with one backspace byte,
/// `WritePrivateProfileStringA`): with and without the game's `Data\Config`, with and without a
/// port folder; the file Windows would make, and the game's folder untouched.
#[test]
fn mcms_set_mod_ini_sequence() {
    for (game_has_config, with_folder) in
        [(true, true), (true, false), (false, true), (false, false)]
    {
        let game = scratch(&format!("seq-game-{game_has_config}-{with_folder}"));
        let kept = scratch(&format!("seq-kept-{game_has_config}-{with_folder}"));
        std::fs::create_dir_all(game.join("Data")).unwrap();
        if game_has_config {
            std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
        }
        let before = tree(&game);
        let mut e = engine(&game);
        if with_folder {
            set_folder(&mut e, Some(kept.to_path_buf()));
        }
        let config = br"Data\Config\";
        assert_eq!(seam::directory_exists(&e, config), game_has_config);
        if !seam::directory_exists(&e, config) {
            assert!(seam::create_directory(&mut e, config));
        }
        let file = br"Data\Config\SeqTest.ini";
        assert!(!seam::exists(&e, file));
        assert!(seam::write(&mut e, file, b"\x08"));
        seam::write_profile_string(&mut e, file, b"Messages", b"Initial", b"1.0000").unwrap();
        let want = b"[Messages]\r\nInitial=1.0000\r\n";
        assert_eq!(seam::read(&e, file).unwrap(), want);
        assert_eq!(
            std::fs::read(kept.join(r"Data\Config\SeqTest.ini")).ok(),
            with_folder.then(|| want.to_vec())
        );
        assert_eq!(tree(&game), before, "{game_has_config} {with_folder}");
    }
}

/// With real threads the seam holds the kernel lock while it reaches the platform's state: a
/// thread that does not hold it reads through the seam, and the calls' check passes (they
/// assert the lock in debug builds).
#[test]
fn the_seam_takes_the_kernel_lock() {
    let game = scratch("threads");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::write(game.join(r"Data\x.ini"), b"[A]\r\nx=1\r\n").unwrap();
    let mut e = engine(&game);
    engine_core::real_threads::start(&mut e).expect("real threads on");
    engine_core::real_threads::without_kernel_lock(&e, || {
        assert!(!engine_core::real_threads::kernel_lock_held(&e));
        assert_eq!(
            seam::profile_string(&e, br"Data\x.ini", b"A", b"x").unwrap(),
            b"1"
        );
        assert!(seam::exists(&e, br"Data\x.ini"));
        assert!(seam::directory_exists(&e, b"Data"));
        assert!(
            !engine_core::real_threads::kernel_lock_held(&e),
            "given back"
        );
    });
}
