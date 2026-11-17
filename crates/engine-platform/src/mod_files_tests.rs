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
#[cfg(windows)]
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
    // `..` takes off the part before it while the name stays inside the game's folder
    // (REVIEW-642: JIP's `..\NVSE\Plugins\x.ini` from the config folder is Data's).
    ok(br"Data\Config\..\x.ini", br"Data\x.ini");
    ok(
        br"data\config\..\NVSE\Plugins\SomePlugin.ini",
        br"data\NVSE\Plugins\SomePlugin.ini",
    );
    ok(br"Data\a\b\..\..\x.ini", br"Data\x.ini");
    ok(br"c:\games\fnv\Data\..\Data\x.ini", br"Data\x.ini");
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
        // The superscripts in UTF-8, as the file layer would read them (REVIEW-642b).
        b"Data\\COM\xc2\xb9.ini",
        b"Data\\lpt\xc2\xb2",
        b"Data\\Com\xc2\xb3 .txt",
        // A device is refused before any `..` is weighed (REVIEW-642): never opened, inside or
        // outside, and also in a part a later `..` takes off.
        br"..\con.ini",
        br"Data\Config\..\..\..\nul",
        br"Data\CON\..\x.ini",
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
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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

/// A name the game's folder does not hold reads nothing and is not there, even when a file of
/// that name exists on disk (`..`, the full path of a file beside the game's folder, another
/// drive, a UNC path, the folder above).
// Runs on every system: the refusal is name logic alone (only its last read is Windows-only).
#[test]
fn a_refused_name_reads_nothing() {
    let root = scratch("refused");
    let game = root.join("game");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    let above: &Path = &root;
    let leaf = "beside.ini";
    let beside = above.join(leaf);
    std::fs::write(&beside, b"[A]\r\nx=1\r\n").unwrap();
    let e = engine(&game);
    let beside_name = beside.to_string_lossy().replace('/', "\\").into_bytes();
    let above_name = above.to_string_lossy().replace('/', "\\").into_bytes();
    let dotdot = format!(r"..\{leaf}").into_bytes();
    let deeper = format!(r"Data\..\..\{leaf}").into_bytes();
    let other_drive = format!(r"D:\{leaf}").into_bytes();
    let unc = format!(r"\\server\share\{leaf}").into_bytes();
    let rooted = format!(r"\{leaf}").into_bytes();
    for name in [
        &dotdot[..],
        &deeper,
        &beside_name,
        &other_drive,
        &unc,
        &rooted,
    ] {
        let shown = String::from_utf8_lossy(name).into_owned();
        // As a file that is not there (the stated rule; REVIEW-642).
        assert_eq!(
            seam::read_or_error(&e, name),
            Err(ERROR_FILE_NOT_FOUND),
            "{shown}"
        );
        assert_eq!(seam::profile_string(&e, name, b"A", b"x"), None, "{shown}");
        assert!(!seam::exists(&e, name), "{shown}");
    }
    for folder in [&b".."[..], br"..\", &above_name, br"\\server\share\"] {
        assert!(
            !seam::directory_exists(&e, folder),
            "{}",
            String::from_utf8_lossy(folder)
        );
    }
    // The same file through the game's folder is read. Windows only: the game's backslash in
    // `Data\in.ini` is a separator for the platform's file layer on Windows alone
    // (`files::real_path` keeps it as part of a name elsewhere).
    #[cfg(windows)]
    {
        std::fs::write(game.join(r"Data\in.ini"), b"[A]\r\nx=2\r\n").unwrap();
        assert_eq!(
            seam::profile_string(&e, br"Data\in.ini", b"A", b"x").unwrap(),
            b"2"
        );
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
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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

/// A folder of the port's folder written into once and then swapped for a junction to another
/// folder (a scratch "game" folder here) is checked again: the next write is refused and nothing
/// lands in that folder (REVIEW-642b: no folder is trusted for the session).
#[cfg(windows)]
#[test]
fn a_folder_swapped_for_a_junction_after_a_write_is_refused() {
    let game = scratch("swap-game");
    let kept = scratch("swap-kept");
    let away = scratch("swap-away");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"Data\Config\A.ini";
    assert_eq!(seam::write_or_error(&mut e, name, b"first"), Ok(()));
    let config = kept.join(r"Data\Config");
    std::fs::remove_dir_all(&config).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&config)
        .arg(&*away)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let second = seam::write_or_error(&mut e, name, b"second");
    let away_tree = tree(&away);
    let _ = std::fs::remove_dir(&config);
    assert_eq!(second, Err(ERROR_ACCESS_DENIED));
    assert!(
        away_tree.is_empty(),
        "nothing written through the junction: {away_tree:?}"
    );
}

/// MCM's `SetModINI` sequence on the real calls (`GetFileAttributesA("Data\Config\")`,
/// `CreateDirectoryA`, `fopen("r")`, `fopen("w")` with one backspace byte,
/// `WritePrivateProfileStringA`): with and without the game's `Data\Config`, with and without a
/// port folder; the file Windows would make, and the game's folder untouched.
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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
// Windows only: it builds the game's names with backslashes and drive letters, which the platform's
// file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
// as part of a name elsewhere; starting the game there is refused today).
#[cfg(windows)]
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
    // The writes too (REVIEW-629b): each reaches the platform's state with the lock held (its
    // calls assert it in debug builds), and gives it back.
    engine_core::real_threads::without_kernel_lock_mut(&mut e, |e| {
        assert!(!engine_core::real_threads::kernel_lock_held(e));
        assert_eq!(seam::create_directory_or_error(e, br"Data\New"), Ok(()));
        assert_eq!(
            seam::write_or_error(e, br"Data\New\w.ini", b"[A]\r\n"),
            Ok(())
        );
        assert_eq!(
            seam::write_profile_string(e, br"Data\New\w.ini", b"A", b"k", b"1"),
            Ok(())
        );
        assert!(
            !engine_core::real_threads::kernel_lock_held(e),
            "given back"
        );
        assert_eq!(
            seam::profile_string(e, br"Data\New\w.ini", b"A", b"k").unwrap(),
            b"1"
        );
        // The section writes, each holding the lock over its read and write (REVIEW-642b: the
        // core checks the hold between them in debug builds).
        let w = br"Data\New\w.ini";
        assert_eq!(
            seam::write_profile_section(e, w, b"B", &[b"z=3".to_vec()]),
            Ok(())
        );
        assert_eq!(seam::remove_profile_key(e, w, b"A", b"k"), Ok(()));
        assert_eq!(seam::remove_profile_section(e, w, b"B"), Ok(()));
        assert!(
            !engine_core::real_threads::kernel_lock_held(e),
            "given back"
        );
        assert_eq!(seam::read(e, w).unwrap(), b"[A]\r\n");
    });
}

/// A name that goes through `..` and stays inside the game's folder is written under the name it
/// resolves to, in the port's folder, and read back through either name (REVIEW-642: JIP's
/// `..\NVSE\Plugins\x.ini` from `Data\Config`).
// Windows only: the game's names with backslashes, which the platform's file layer maps to host
// paths on Windows alone (`files::real_path` keeps a backslash as part of a name elsewhere).
#[cfg(windows)]
#[test]
fn a_dotdot_name_inside_the_game_folder_is_written() {
    let game = scratch("dotdot-game");
    let kept = scratch("dotdot-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::create_dir_all(game.join(r"Data\NVSE\Plugins")).unwrap();
    let before = tree(&game);
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"data\config\..\NVSE\Plugins\SomePlugin.ini";
    seam::write_profile_string(&mut e, name, b"Main", b"bFoo", b"1").unwrap();
    assert_eq!(
        std::fs::read(kept.join(r"data\NVSE\Plugins\SomePlugin.ini")).unwrap(),
        b"[Main]\r\nbFoo=1\r\n"
    );
    assert_eq!(
        seam::profile_string(&e, br"Data\NVSE\Plugins\SomePlugin.ini", b"Main", b"bFoo").unwrap(),
        b"1"
    );
    assert_eq!(
        seam::profile_string(&e, name, b"Main", b"bFoo").unwrap(),
        b"1"
    );
    assert!(seam::exists(&e, name));
    assert_eq!(tree(&game), before, "the game's folder is as it was");
}

/// The `.part` a write goes through is made new: a plain one an earlier write left is replaced;
/// a folder of that name refuses the write and stays; a hard link to a file elsewhere is taken
/// away, never written through (REVIEW-642).
// Windows only: the game's names with backslashes, which the platform's file layer maps to host
// paths on Windows alone (`files::real_path` keeps a backslash as part of a name elsewhere).
#[cfg(windows)]
#[test]
fn a_part_entry_is_never_written_through() {
    let game = scratch("part-game");
    let kept = scratch("part-kept");
    let away = scratch("part-away");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::create_dir_all(kept.join(r"Data\Config")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let part = kept.join(r"Data\Config\A.ini.part");
    // A plain one left over.
    std::fs::write(&part, b"stale").unwrap();
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\Config\A.ini", b"new"),
        Ok(())
    );
    assert_eq!(
        std::fs::read(kept.join(r"Data\Config\A.ini")).unwrap(),
        b"new"
    );
    assert!(!part.exists());
    // A hard link to a file elsewhere.
    let outside = away.join("game-file.ini");
    std::fs::write(&outside, b"keep me").unwrap();
    std::fs::hard_link(&outside, &part).unwrap();
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\Config\A.ini", b"newer"),
        Ok(())
    );
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep me");
    assert_eq!(
        std::fs::read(kept.join(r"Data\Config\A.ini")).unwrap(),
        b"newer"
    );
    // A folder of that name.
    std::fs::create_dir(&part).unwrap();
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\Config\A.ini", b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
    assert!(part.is_dir(), "left as it is");
    assert_eq!(
        std::fs::read(kept.join(r"Data\Config\A.ini")).unwrap(),
        b"newer"
    );
    std::fs::remove_dir(&part).unwrap();
    // A symbolic link, where this machine lets a test make one.
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(&outside, &part).is_ok();
    #[cfg(not(windows))]
    let linked = std::os::unix::fs::symlink(&outside, &part).is_ok();
    if linked {
        assert_eq!(
            seam::write_or_error(&mut e, br"Data\Config\A.ini", b"x"),
            Err(ERROR_ACCESS_DENIED)
        );
        assert_eq!(std::fs::read(&outside).unwrap(), b"keep me");
        let _ = std::fs::remove_file(&part);
    }
}

/// A write whose rename fails (another program holds the kept file without delete sharing)
/// fails, keeps the kept bytes and leaves no `.part` behind (REVIEW-629b).
#[cfg(windows)]
#[test]
fn a_failed_rename_leaves_no_part() {
    use std::os::windows::fs::OpenOptionsExt;
    let game = scratch("rename-game");
    let kept = scratch("rename-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::create_dir_all(kept.join(r"Data\Config")).unwrap();
    let file = kept.join(r"Data\Config\R.ini");
    std::fs::write(&file, b"old").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let result = {
        // FILE_SHARE_READ | FILE_SHARE_WRITE, no FILE_SHARE_DELETE: the rename onto it fails.
        let _held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&file)
            .unwrap();
        seam::write_or_error(&mut e, br"Data\Config\R.ini", b"new")
    };
    assert!(result.is_err(), "{result:?}");
    assert_eq!(std::fs::read(&file).unwrap(), b"old");
    assert!(!kept.join(r"Data\Config\R.ini.part").exists());
    assert_eq!(seam::read(&e, br"Data\Config\R.ini").unwrap(), b"old");
}

/// The section calls on the real platform: a key or a section removed from a file that is not
/// there makes it empty (measured, `ini4-2026-11-16.txt`, `DELKEY missing file` and `DELSEC
/// missing file`); a UTF-16 LE file is changed as text and stays UTF-16 LE (`ini6-...`); a file
/// that is only its mark gets a line end first from a key write (`ini7-...`, `bom only`) and is
/// refused by the section writes; a file with a 0 byte that is not UTF-16 LE is never written.
// Windows only: the game's names with backslashes, which the platform's file layer maps to host
// paths on Windows alone (`files::real_path` keeps a backslash as part of a name elsewhere).
#[cfg(windows)]
#[test]
fn section_calls_on_the_platform_and_encodings() {
    let game = scratch("enc-game");
    let kept = scratch("enc-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let at = |n: &str| kept.join(format!(r"Data\Config\{n}"));
    assert_eq!(
        seam::remove_profile_key(&mut e, br"Data\Config\K.ini", b"A", b"x"),
        Ok(())
    );
    assert_eq!(std::fs::read(at("K.ini")).unwrap(), b"");
    assert_eq!(
        seam::remove_profile_section(&mut e, br"Data\Config\S.ini", b"A"),
        Ok(())
    );
    assert_eq!(std::fs::read(at("S.ini")).unwrap(), b"");
    let u16le = |t: &str| -> Vec<u8> {
        [0xff, 0xfe]
            .into_iter()
            .chain(t.encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect()
    };
    std::fs::write(
        game.join(r"Data\Config\U.ini"),
        u16le("[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n"),
    )
    .unwrap();
    let u = br"Data\Config\U.ini";
    assert_eq!(seam::profile_string(&e, u, b"A", b"y").unwrap(), b"2");
    assert_eq!(
        seam::profile_section(&e, u, b"A").unwrap(),
        [b"x=1".to_vec(), b"y=2".to_vec()]
    );
    seam::write_profile_string(&mut e, u, b"A", b"x", b"\xe4").unwrap();
    assert_eq!(
        std::fs::read(at("U.ini")).unwrap(),
        u16le("[A]\r\nx=\u{e4}\r\ny=2\r\n[B]\r\nz=3\r\n")
    );
    seam::write_profile_section(&mut e, u, b"A", &[b"p=1".to_vec(), b"q=2".to_vec()]).unwrap();
    assert_eq!(
        std::fs::read(at("U.ini")).unwrap(),
        u16le("[A]\r\np=1\r\nq=2\r\n[B]\r\nz=3\r\n")
    );
    seam::remove_profile_section(&mut e, u, b"A").unwrap();
    assert_eq!(std::fs::read(at("U.ini")).unwrap(), u16le("[B]\r\nz=3\r\n"));
    // Only the mark.
    std::fs::write(game.join(r"Data\Config\M.ini"), [0xff, 0xfe]).unwrap();
    let m = br"Data\Config\M.ini";
    assert_eq!(
        seam::write_profile_section(&mut e, m, b"A", &[b"x=5".to_vec()]),
        Err(seam::ERROR_NOT_SUPPORTED)
    );
    seam::write_profile_string(&mut e, m, b"A", b"x", b"5").unwrap();
    assert_eq!(
        std::fs::read(at("M.ini")).unwrap(),
        u16le("\r\n[A]\r\nx=5\r\n")
    );
    // UTF-16 BE: never written, the game's copy and the port's folder as they were.
    let be: Vec<u8> = [0xfe, 0xff]
        .into_iter()
        .chain(
            "[A]\r\nx=1\r\n"
                .encode_utf16()
                .flat_map(|u| u.to_be_bytes()),
        )
        .collect();
    std::fs::write(game.join(r"Data\Config\BE.ini"), &be).unwrap();
    let b = br"Data\Config\BE.ini";
    assert_eq!(
        seam::write_profile_string(&mut e, b, b"A", b"x", b"5"),
        Err(seam::ERROR_NOT_SUPPORTED)
    );
    assert_eq!(
        seam::remove_profile_key(&mut e, b, b"A", b"x"),
        Err(seam::ERROR_NOT_SUPPORTED)
    );
    assert!(!at("BE.ini").exists());
    assert_eq!(std::fs::read(game.join(r"Data\Config\BE.ini")).unwrap(), be);
}

// ---- REVIEW-643: appends, bounded reads, names Windows folds, `.part` names ----

/// A part ending in a dot or a space (which Windows takes off) is refused in every call, so one
/// disk file never has two names in this run's memory.
#[test]
fn names_windows_would_fold_are_refused() {
    let e = engine(Path::new(r"C:\Games\FNV"));
    for folded in [
        &br"Data\a.txt."[..],
        br"Data\a.txt ",
        br"Data\a.txt. .",
        br"Data.\a.txt",
        br"Data \a.txt",
        br"Data\...\a.txt",
        br"Data\x.\..\a.txt",
    ] {
        assert_eq!(
            relative(&e, folded),
            Err(NotMine::Folded),
            "{:?}",
            String::from_utf8_lossy(folded)
        );
    }
    assert_eq!(relative(&e, br"Data\.\a.txt").unwrap(), br"Data\a.txt");
    assert_eq!(relative(&e, br"Data\.a").unwrap(), br"Data\.a");
}

/// Writes, appends and new folders refuse a last part ending in `.part` (any case), and without
/// a port folder a name with a byte above 0x7F (with one, such a name is taken); reads are not
/// refused for those.
#[test]
fn names_writes_refuse() {
    let mut e = engine(Path::new(r"C:\Games\FNV"));
    set_folder(&mut e, Some(PathBuf::from(r"C:\Kept")));
    for accented in [&b"Data\\\xc4.txt"[..], b"Data\\\xc3\xa9t\xc3\xa9.txt"] {
        assert_eq!(writable_as(&e, accented, false).unwrap(), accented);
    }
    assert!(!remembered(b"Data\\\xc4.txt"));
    assert!(remembered(br"Data\a.txt"));
    assert_eq!(
        writable_as(&e, br"Data\x.txt.part", false),
        Err(ERROR_ACCESS_DENIED)
    );
    set_folder(&mut e, None);
    assert_eq!(
        writable_as(&e, br"Data\a.txt", false).unwrap(),
        br"Data\a.txt"
    );
    assert_eq!(
        writable_as(&e, br"Data\a.part\b.txt", false).unwrap(),
        br"Data\a.part\b.txt"
    );
    for bad in [
        &b"Data\\\xc4.txt"[..],
        b"Data\\\xc3\xa9t\xc3\xa9.txt",
        b"\xc4\\a.txt",
        br"Data\x.txt.part",
        br"Data\x.TXT.Part",
        br"Data\.part",
        br"Data\a.txt.",
        br"..\a.txt",
    ] {
        assert_eq!(
            writable_as(&e, bad, false),
            Err(ERROR_ACCESS_DENIED),
            "{:?}",
            String::from_utf8_lossy(bad)
        );
    }
    assert!(relative(&e, br"Data\x.txt.part").is_ok());
    assert!(relative(&e, b"Data\\\xc4.txt").is_ok());
}

/// Appends add only the new bytes to the port folder's copy (made once, whole, from the game's
/// copy when the folder has none); the game's copy is never changed; this run's memory keeps no
/// whole copy of an appended file; reads answer the disk's bytes.
// Windows only: the game's names with backslashes, which the platform's file layer maps to host
// paths on Windows alone (`files::real_path` keeps a backslash as part of a name elsewhere).
#[cfg(windows)]
#[test]
fn appends_add_only_the_new_bytes() {
    let game = scratch("append-game");
    let kept = scratch("append-kept");
    std::fs::create_dir_all(game.join("Logs")).unwrap();
    std::fs::write(game.join(r"Logs\L.txt"), b"game").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"Logs\L.txt";
    // The first append makes the folder's copy from the game's.
    assert_eq!(seam::append_or_error(&mut e, name, b"\nA"), Ok(()));
    let file = kept.join(r"Logs\L.txt");
    assert_eq!(std::fs::read(&file).unwrap(), b"game\nA");
    // Later ones add their bytes only: a file put in its place shows it is not rewritten from
    // this run's memory.
    std::fs::write(&file, b"disk").unwrap();
    assert_eq!(seam::append_or_error(&mut e, name, b"\nB"), Ok(()));
    assert_eq!(std::fs::read(&file).unwrap(), b"disk\nB");
    assert!(!state_ref!(e).mod_files.written.contains_key(&key(name)));
    assert_eq!(seam::read(&e, name).unwrap(), b"disk\nB");
    assert_eq!(seam::read_head(&e, name, 3).unwrap(), b"dis");
    assert!(seam::exists(&e, name));
    assert_eq!(std::fs::read(game.join(r"Logs\L.txt")).unwrap(), b"game");
    // After a whole write, an append still adds only its bytes, and reads agree with the disk.
    assert_eq!(seam::write_or_error(&mut e, name, b"new"), Ok(()));
    assert_eq!(seam::append_or_error(&mut e, name, b"+"), Ok(()));
    assert_eq!(std::fs::read(&file).unwrap(), b"new+");
    assert_eq!(seam::read(&e, name).unwrap(), b"new+");
    // A file that is not there anywhere is made (`fopen("ab")`), its folder needed.
    assert_eq!(seam::append_or_error(&mut e, br"Logs\M.txt", b"m"), Ok(()));
    assert_eq!(std::fs::read(kept.join(r"Logs\M.txt")).unwrap(), b"m");
    assert_eq!(
        seam::append_or_error(&mut e, br"Nowhere\M.txt", b"m"),
        Err(ERROR_PATH_NOT_FOUND)
    );
    assert!(!kept.join(r"Logs\L.txt.part").exists());
    // Without a folder: the run's copy grows, seeded from the game's once.
    let mut e = engine(&game);
    assert_eq!(seam::append_or_error(&mut e, name, b"1"), Ok(()));
    assert_eq!(seam::append_or_error(&mut e, name, b"2"), Ok(()));
    assert_eq!(seam::read(&e, name).unwrap(), b"game12");
    assert_eq!(std::fs::read(game.join(r"Logs\L.txt")).unwrap(), b"game");
}

/// An append is refused for every name a write is refused for, with the same code, and nothing
/// is written anywhere.
// Windows only: the game's names with backslashes (see above).
#[cfg(windows)]
#[test]
fn appends_refuse_what_writes_refuse() {
    let game = scratch("append-names-game");
    let kept = scratch("append-names-kept");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    let before = tree(&game);
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let outside = game.parent().unwrap().join(format!(
        "nv-mod-files-append-out-{}.txt",
        std::process::id()
    ));
    let outside_name = outside.to_string_lossy().replace('/', "\\").into_bytes();
    for bad in [
        &br"..\escape.txt"[..],
        &outside_name,
        br"\\server\share\x.txt",
        br"Data\CON.txt",
        br"Data\nul",
        br"Data\x.txt.part",
        br"Data\a.txt.",
        br"Data\a.txt ",
        br"Data\Config",
        br"Data\x.txt:stream",
    ] {
        let wrote = seam::write_or_error(&mut e, bad, b"x");
        let appended = seam::append_or_error(&mut e, bad, b"x");
        assert!(wrote.is_err(), "{:?}", String::from_utf8_lossy(bad));
        assert_eq!(appended, wrote, "{:?}", String::from_utf8_lossy(bad));
    }
    assert!(!outside.exists());
    assert!(!game.parent().unwrap().join("escape.txt").exists());
    assert_eq!(tree(&game), before);
    assert!(
        tree(&kept).iter().all(|(_, bytes)| bytes.is_none()),
        "no file made"
    );
}

/// A folder of the port's folder appended to and then swapped for a junction to another folder
/// is checked again: the next append is refused and nothing lands there.
#[cfg(windows)]
#[test]
fn a_folder_swapped_for_a_junction_refuses_appends() {
    let game = scratch("append-swap-game");
    let kept = scratch("append-swap-kept");
    let away = scratch("append-swap-away");
    std::fs::create_dir_all(game.join(r"Data\Config")).unwrap();
    std::fs::write(away.join("A.txt"), b"away").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"Data\Config\A.txt";
    assert_eq!(seam::append_or_error(&mut e, name, b"first"), Ok(()));
    assert_eq!(seam::append_or_error(&mut e, name, b"+"), Ok(()));
    let config = kept.join(r"Data\Config");
    std::fs::remove_dir_all(&config).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&config)
        .arg(&*away)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let second = seam::append_or_error(&mut e, name, b"second");
    let away_bytes = std::fs::read(away.join("A.txt")).unwrap();
    let away_tree = tree(&away);
    let _ = std::fs::remove_dir(&config);
    assert_eq!(second, Err(ERROR_ACCESS_DENIED));
    assert_eq!(away_bytes, b"away", "nothing appended through the junction");
    assert_eq!(away_tree.len(), 1, "nothing made there: {away_tree:?}");
}

/// A script's `x.txt.part` is never the name a write of `x.txt` goes through: the script's own
/// is refused, and a mod's `x.txt.part` in the game's folder stays there and reads as it was
/// after a write of `x.txt`.
#[cfg(windows)]
#[test]
fn a_part_name_and_its_file_both_stay() {
    let game = scratch("partname-game");
    let kept = scratch("partname-kept");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::write(game.join(r"Data\x.txt.part"), b"mod's").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    assert_eq!(
        seam::write_or_error(&mut e, br"Data\x.txt.part", b"script"),
        Err(ERROR_ACCESS_DENIED)
    );
    assert_eq!(seam::write_or_error(&mut e, br"Data\x.txt", b"x"), Ok(()));
    assert_eq!(seam::append_or_error(&mut e, br"Data\x.txt", b"y"), Ok(()));
    assert_eq!(seam::read(&e, br"Data\x.txt.part").unwrap(), b"mod's");
    assert_eq!(seam::read(&e, br"Data\x.txt").unwrap(), b"xy");
    assert_eq!(
        std::fs::read(game.join(r"Data\x.txt.part")).unwrap(),
        b"mod's"
    );
    assert!(!kept.join(r"Data\x.txt.part").exists());
}

/// Write `a.txt`, then `a.txt.` (one disk file to Windows): the second is refused, and every
/// read agrees with the disk.
#[cfg(windows)]
#[test]
fn a_folded_spelling_never_holds_other_bytes() {
    let game = scratch("fold-game");
    let kept = scratch("fold-kept");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    assert_eq!(seam::write_or_error(&mut e, br"Data\a.txt", b"one"), Ok(()));
    for other in [&br"Data\a.txt."[..], br"Data\a.txt ", br"Data\A.TXT. "] {
        assert_eq!(
            seam::write_or_error(&mut e, other, b"two"),
            Err(ERROR_ACCESS_DENIED)
        );
        assert_eq!(
            seam::append_or_error(&mut e, other, b"two"),
            Err(ERROR_ACCESS_DENIED)
        );
        assert_eq!(seam::read_or_error(&e, other), Err(ERROR_FILE_NOT_FOUND));
        assert!(!seam::exists(&e, other));
    }
    let disk = std::fs::read(kept.join(r"Data\a.txt")).unwrap();
    assert_eq!(disk, b"one");
    assert_eq!(seam::read(&e, br"Data\a.txt").unwrap(), disk);
    assert_eq!(seam::read(&e, br"data\A.TXT").unwrap(), disk);
    assert_eq!(tree(&kept).len(), 2, "Data and a.txt only");
}

/// An accented or Cyrillic name (a mod's plugin or INI name) is written and appended to in the
/// port's folder, and read back by another case spelling as Windows folds it: this run's memory
/// keeps no copy of it, so the disk answers every spelling. Without a port folder it is refused.
#[cfg(windows)]
#[test]
fn non_ascii_names_live_on_the_disk() {
    let game = scratch("accent-game");
    let kept = scratch("accent-kept");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::create_dir_all(game.join("Mod Logs")).unwrap();
    std::fs::write(game.join("Data").join("Été.txt"), b"game's").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = "Data\\Été.txt".as_bytes();
    let other = "data\\éTÉ.TXT".as_bytes();
    assert_eq!(seam::read(&e, other).unwrap(), b"game's");
    assert_eq!(seam::write_or_error(&mut e, name, b"one"), Ok(()));
    assert!(state_ref!(e).mod_files.written.is_empty());
    assert_eq!(seam::read(&e, other).unwrap(), b"one");
    assert_eq!(seam::append_or_error(&mut e, other, b"+two"), Ok(()));
    assert_eq!(
        std::fs::read(kept.join("Data").join("Été.txt")).unwrap(),
        b"one+two"
    );
    assert_eq!(seam::read(&e, name).unwrap(), b"one+two");
    assert_eq!(seam::read_head(&e, other, 3).unwrap(), b"one");
    assert!(seam::exists(&e, other));
    // Cyrillic, a folder and a file in it.
    let folder = "Mod Logs\\Мой".as_bytes();
    assert_eq!(seam::create_directory_or_error(&mut e, folder), Ok(()));
    assert!(state_ref!(e).mod_files.folders.is_empty());
    assert!(seam::directory_exists(&e, "mod logs\\мОЙ".as_bytes()));
    assert_eq!(
        seam::append_or_error(&mut e, "Mod Logs\\Мой\\Мод.log".as_bytes(), b"x"),
        Ok(())
    );
    assert_eq!(
        seam::read(&e, "MOD LOGS\\МОЙ\\мод.LOG".as_bytes()).unwrap(),
        b"x"
    );
    assert_eq!(
        std::fs::read(game.join("Data").join("Été.txt")).unwrap(),
        b"game's"
    );
    // Without a port folder: refused (the memory would be the only copy).
    let mut e = engine(&game);
    assert_eq!(
        seam::write_or_error(&mut e, name, b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
    assert_eq!(
        seam::append_or_error(&mut e, name, b"x"),
        Err(ERROR_ACCESS_DENIED)
    );
    assert!(!seam::create_directory(&mut e, "Data\\Мой".as_bytes()));
}

/// An append adds to the same file, never a new one put in its place: a reader that holds the
/// folder's copy open across the append (sharing read, write and delete) sees the new bytes. A
/// write through a `.part` and a rename would leave that reader on the old, replaced file.
#[cfg(windows)]
#[test]
fn an_append_grows_the_same_file() {
    use std::io::{Read as _, Seek as _};
    use std::os::windows::fs::OpenOptionsExt as _;
    let game = scratch("same-game");
    let kept = scratch("same-kept");
    std::fs::create_dir_all(game.join("Logs")).unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    let name = br"Logs\S.txt";
    assert_eq!(seam::write_or_error(&mut e, name, b"one"), Ok(()));
    let mut held = std::fs::OpenOptions::new()
        .read(true)
        // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
        .share_mode(7)
        .open(kept.join(r"Logs\S.txt"))
        .unwrap();
    assert_eq!(seam::append_or_error(&mut e, name, b"+two"), Ok(()));
    let mut seen = Vec::new();
    held.seek(std::io::SeekFrom::Start(0)).unwrap();
    held.read_to_end(&mut seen).unwrap();
    drop(held);
    assert_eq!(seen, b"one+two", "the file held open grew");
}

/// A write that fails part way through an append is cut back: the file keeps its old length
/// and bytes, and the write's code is the answer.
#[test]
fn a_failed_append_is_cut_back() {
    let dir = scratch("cut");
    let file = dir.join("c.txt");
    std::fs::write(&file, b"old").unwrap();
    let failed = append_with(&file, b"new line", |f, bytes| {
        use std::io::Write as _;
        f.write_all(&bytes[..3])?;
        f.flush()?;
        Err(std::io::Error::from_raw_os_error(112))
    });
    assert_eq!(failed, Err(112));
    assert_eq!(std::fs::read(&file).unwrap(), b"old");
    assert_eq!(append_to(&file, b"+ok"), Ok(()));
    assert_eq!(std::fs::read(&file).unwrap(), b"old+ok");
}

/// `read_head` reads only the first bytes, from this run's write, the folder's copy or the
/// game's, with `read`'s answers for a file that is not there or a refused name.
#[cfg(windows)]
#[test]
fn bounded_reads() {
    let game = scratch("head-game");
    let kept = scratch("head-kept");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::create_dir_all(kept.join("Data")).unwrap();
    std::fs::write(game.join(r"Data\g.txt"), b"game file").unwrap();
    std::fs::write(kept.join(r"Data\k.txt"), b"kept file").unwrap();
    let mut e = engine(&game);
    set_folder(&mut e, Some(kept.to_path_buf()));
    assert_eq!(seam::read_head(&e, br"Data\g.txt", 4).unwrap(), b"game");
    assert_eq!(seam::read_head(&e, br"Data\k.txt", 4).unwrap(), b"kept");
    assert_eq!(
        seam::read_head(&e, br"Data\k.txt", 100).unwrap(),
        b"kept file"
    );
    seam::write_or_error(&mut e, br"Data\w.txt", b"written").unwrap();
    assert_eq!(seam::read_head(&e, br"Data\w.txt", 5).unwrap(), b"writt");
    assert_eq!(
        seam::read_head_or_error(&e, br"Data\none.txt", 5),
        Err(ERROR_FILE_NOT_FOUND)
    );
    assert_eq!(
        seam::read_head_or_error(&e, br"..\g.txt", 5),
        Err(ERROR_FILE_NOT_FOUND)
    );
    assert!(seam::read_head_or_error(&e, b"Data", 5).is_err());
}
