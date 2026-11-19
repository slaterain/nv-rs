//! The chooser driven by typed answers, on archives and folders the tests make themselves.

use super::*;
use std::io::Cursor;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("nv-mod-chooser-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn places(&self) -> Places {
        let game = self.0.join("game");
        std::fs::create_dir_all(game.join("Data")).unwrap();
        std::fs::write(game.join("Data").join("FalloutNV.esm"), b"esm").unwrap();
        std::fs::write(game.join("Data").join("DeadMoney.esm"), b"esm").unwrap();
        Places {
            game,
            installs: self.0.join("mod installs"),
            loose: self.0.join("mods"),
            extenders: vec![("nvse".into(), "6.4.9".into())],
            play_with_mods: "play with mods".into(),
            room: |_| Some(1 << 40),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    // Stored, so a test can damage a file's bytes where they lie.
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

/// Runs the chooser with these typed bytes; what it said.
fn run_bytes(places: &Places, typed: &[u8]) -> String {
    let mut input = Cursor::new(typed.to_vec());
    let mut out = Vec::new();
    run(&mut input, &mut out, places).unwrap();
    String::from_utf8(out).unwrap()
}

/// Runs the chooser with these typed lines; what it said.
fn run_with(places: &Places, typed: &str) -> String {
    run_bytes(places, typed.as_bytes())
}

/// The chooser as nv-rs's program runs it: what it said, the exit code, and what was left unread.
fn run_window(places: Result<Places, String>, typed: &str) -> (String, i32, String) {
    let mut input = Cursor::new(typed.as_bytes().to_vec());
    let mut out = Vec::new();
    let code = run_in_window(&mut input, &mut out, places);
    let mut rest = String::new();
    input.read_to_string(&mut rest).unwrap();
    (String::from_utf8(out).unwrap(), code, rest)
}

const CONFIG: &str = r#"<config><moduleName>Gun Pack</moduleName>
  <requiredInstallFiles><file source="Main/GunPack.esp" destination="GunPack.esp"/></requiredInstallFiles>
  <installSteps order="Explicit"><installStep name="Options"><optionalFileGroups order="Explicit">
    <group name="Sounds" type="SelectExactlyOne"><plugins order="Explicit">
      <plugin name="Quiet"><description>Softer shots.</description><files><file source="Sounds/Quiet/shot.wav" destination="Sound/shot.wav"/></files><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
      <plugin name="Loud"><description>Louder shots.</description><files><file source="Sounds/Loud/shot.wav" destination="Sound/shot.wav"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    </plugins></group>
    <group name="Patches" type="SelectAny"><plugins order="Explicit">
      <plugin name="Dead Money"><description/><files><file source="Patches/DM.esp" destination=""/></files><typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns><pattern><dependencies><fileDependency file="DeadMoney.esm" state="Active"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
      <plugin name="Lonesome Road"><description/><files><file source="Patches/LR.esp" destination=""/></files><typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns><pattern><dependencies><fileDependency file="LonesomeRoad.esm" state="Active"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
    </plugins></group>
  </optionalFileGroups></installStep></installSteps></config>"#;

fn gun_pack(places: &Places) -> PathBuf {
    let archive = places.loose.join("Gun Pack 1.2.fomod");
    make_zip(
        &archive,
        &[
            ("Gun Pack/fomod/ModuleConfig.xml", CONFIG.as_bytes()),
            (
                "Gun Pack/fomod/info.xml",
                b"<fomod><Name>Gun Pack</Name><Version>1.2</Version><Author>Someone</Author></fomod>",
            ),
            ("Gun Pack/Main/GunPack.esp", b"TES4 gun pack"),
            ("Gun Pack/Sounds/Quiet/shot.wav", b"quiet"),
            ("Gun Pack/Sounds/Loud/shot.wav", b"loud"),
            ("Gun Pack/Patches/DM.esp", b"TES4 dm"),
            ("Gun Pack/Patches/LR.esp", b"TES4 lr"),
        ],
    );
    archive
}

/// Four steps, every group type, and two steps shown only for an earlier answer: "Outfits for A"
/// when Type A is the body, "Outfits for B" when Type B is.
const BODIES: &str = r#"<config><moduleName>Bodies</moduleName>
  <installSteps order="Explicit">
    <installStep name="Body"><optionalFileGroups order="Explicit">
      <group name="Body type" type="SelectExactlyOne"><plugins order="Explicit">
        <plugin name="Type A"><description>The A body.</description><files><file source="A/body.nif" destination="meshes/body.nif"/></files><conditionFlags><flag name="body">A</flag></conditionFlags><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
        <plugin name="Type B"><description>The B body.</description><files><file source="B/body.nif" destination="meshes/body.nif"/></files><conditionFlags><flag name="body">B</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group>
      <group name="Hands" type="SelectAtMostOne"><plugins order="Explicit">
        <plugin name="Gloves"><description/><files><file source="X/gloves.nif" destination="meshes/gloves.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="Rings"><description/><files><file source="X/rings.nif" destination="meshes/rings.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group>
    </optionalFileGroups></installStep>
    <installStep name="Outfits for A"><visible><flagDependency flag="body" value="A"/></visible><optionalFileGroups order="Explicit">
      <group name="Outfits A" type="SelectAny"><plugins order="Explicit">
        <plugin name="Suit A"><description/><files><file source="A/suit.nif" destination="meshes/suit.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="Hat A"><description/><files><file source="A/hat.nif" destination="meshes/hat.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group>
    </optionalFileGroups></installStep>
    <installStep name="Outfits for B"><visible><flagDependency flag="body" value="B"/></visible><optionalFileGroups order="Explicit">
      <group name="Outfits B" type="SelectAtLeastOne"><plugins order="Explicit">
        <plugin name="Suit B"><description/><files><file source="B/suit.nif" destination="meshes/suit.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="Hat B"><description/><files><file source="B/hat.nif" destination="meshes/hat.nif"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group>
    </optionalFileGroups></installStep>
    <installStep name="Core"><optionalFileGroups order="Explicit">
      <group name="Core files" type="SelectAll"><plugins order="Explicit">
        <plugin name="Main plugin"><description/><files><file source="Main/Bodies.esp" destination="Bodies.esp"/></files><typeDescriptor><type name="Required"/></typeDescriptor></plugin>
      </plugins></group>
    </optionalFileGroups></installStep>
  </installSteps></config>"#;

fn bodies(places: &Places) -> PathBuf {
    let archive = places.loose.join("Bodies.zip");
    make_zip(
        &archive,
        &[
            ("fomod/ModuleConfig.xml", BODIES.as_bytes()),
            ("A/body.nif", b"body A"),
            ("B/body.nif", b"body B"),
            ("A/suit.nif", b"suit A"),
            ("A/hat.nif", b"hat A"),
            ("B/suit.nif", b"suit B"),
            ("B/hat.nif", b"hat B"),
            ("X/gloves.nif", b"gloves"),
            ("X/rings.nif", b"rings"),
            ("Main/Bodies.esp", b"TES4 bodies"),
        ],
    );
    archive
}

/// The one installed mod's folder.
fn only_installed(places: &Places) -> PathBuf {
    let installed = installs::installed(&places.installs).unwrap();
    assert_eq!(installed.len(), 1, "{installed:?}");
    installed[0].folder.clone()
}

/// The names in the installs folder, the installer's lock file left out.
fn names_in_installs(places: &Places) -> Vec<String> {
    std::fs::read_dir(&places.installs).map_or_else(
        |_| vec![],
        |d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| n != installs::LOCK)
                .collect()
        },
    )
}

fn nothing_installed(places: &Places) -> bool {
    names_in_installs(places).is_empty()
}

/// The whole way: pick the archive, a typo and a wrong number (asked again, nothing lost), read an
/// option, choose Loud, Enter for the recommended patches, yes: installed with its record, the
/// archive moved, the game folder untouched; the size, the room and the progress said.
#[test]
fn a_fomod_installs_from_typed_answers() {
    let dir = Scratch::new("install");
    let places = dir.places();
    let archive = gun_pack(&places);
    let said = run_with(&places, "1\nloud\n7\n?2\n2\n\nyes\n\n");
    assert!(
        said.contains("\"loud\" is not one of the numbers shown"),
        "{said}"
    );
    assert!(
        said.contains("\"7\" is not one of the numbers shown"),
        "{said}"
    );
    assert!(said.contains("Louder shots."), "{said}");
    assert!(said.contains("(can't be chosen here)"), "{said}");
    assert!(
        said.contains("3 files will be installed: 24 bytes unpacked."),
        "{said}"
    );
    assert!(said.contains("Free on the drive: 1024.0 GB."), "{said}");
    assert!(said.contains("several minutes"), "{said}");
    assert!(
        said.contains("3 of 3 files, 24 bytes of 24 bytes"),
        "{said}"
    );
    assert!(said.contains("Installed Gun Pack"), "{said}");
    assert!(said.contains("next time you play with mods"), "{said}");
    let folder = only_installed(&places);
    assert_eq!(
        std::fs::read(folder.join("GunPack.esp")).unwrap(),
        b"TES4 gun pack"
    );
    assert_eq!(
        std::fs::read(folder.join("Sound").join("shot.wav")).unwrap(),
        b"loud"
    );
    assert_eq!(std::fs::read(folder.join("DM.esp")).unwrap(), b"TES4 dm");
    assert!(!folder.join("LR.esp").exists());
    let record = Record::parse(&std::fs::read_to_string(folder.join(mod_install::RECORD)).unwrap());
    assert_eq!(record.name, "Gun Pack");
    assert_eq!(record.version, "1.2");
    assert!(record
        .choices
        .contains(&"Options / Sounds / Loud".to_owned()));
    assert_eq!(record.files.len(), 3);
    assert!(!archive.exists());
    assert!(places
        .loose
        .join(installs::KEPT)
        .join("Gun Pack 1.2.fomod")
        .exists());
    assert_eq!(
        std::fs::read(places.game.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
    // Nothing but the mod's folder in the installs folder (no staging folder left).
    assert_eq!(names_in_installs(&places).len(), 1);
}

/// q stops: nothing installed, no leftover folder, the archive still waiting.
#[test]
fn stopping_installs_nothing() {
    let dir = Scratch::new("stop");
    let places = dir.places();
    let archive = gun_pack(&places);
    let said = run_with(&places, "1\nq\n\n");
    assert!(said.contains("Stopped: nothing was installed."), "{said}");
    assert!(nothing_installed(&places));
    assert!(archive.exists());
}

/// The input ending (the window closed) anywhere stops cleanly: at the list, at a step, at the
/// summary, at a second-copy question; never a loop, nothing installed.
#[test]
fn the_input_ending_stops_anywhere() {
    let dir = Scratch::new("eof");
    let places = dir.places();
    gun_pack(&places);
    for typed in [
        "",
        "1\n",
        "1\n2\n",
        "1\n2\n\n",
        "1\n2\n\nb\n",
        "1\n2\n\n\n\n",
    ] {
        let said = run_with(&places, typed);
        assert!(nothing_installed(&places), "{typed:?}: {said}");
        if !typed.is_empty() {
            assert!(
                said.contains("Stopped: nothing was installed."),
                "{typed:?}: {said}"
            );
        }
    }
}

/// An archive with no installer installs its Data layout after a yes (its size said first);
/// remove takes it away after a yes, in any case, as remove, uninstall or delete.
#[test]
fn a_plain_archive_installs_and_is_removed() {
    let dir = Scratch::new("plain");
    let places = dir.places();
    make_zip(
        &places.loose.join("Retex.zip"),
        &[
            ("Retex/Data/Textures/a.dds", b"tex"),
            ("Retex/readme.txt", b"read me"),
        ],
    );
    let said = run_with(&places, "1\nyes\n\n");
    assert!(
        said.contains("1 files will be installed: 3 bytes unpacked"),
        "{said}"
    );
    assert!(said.contains("Installed Retex (1 files)"), "{said}");
    assert_eq!(
        std::fs::read(only_installed(&places).join("Textures").join("a.dds")).unwrap(),
        b"tex"
    );
    let said = run_with(&places, "Remove 1\nno\nUNINSTALL 1\n\nDelete 1\nYES\n\n");
    assert!(said.contains("Kept Retex."), "{said}");
    assert!(said.contains("Removed Retex."), "{said}");
    assert!(installs::installed(&places.installs).unwrap().is_empty());
}

/// Lines the list does not understand say what is understood, and the list is asked again: a
/// word that is not a command, remove without a number, several numbers, bytes that are not
/// UTF-8.
#[test]
fn the_list_explains_what_it_understands() {
    let dir = Scratch::new("list-words");
    let places = dir.places();
    gun_pack(&places);
    let said = run_bytes(&places, b"install 1\nremove\n1 1\n\xff\xfe\nremove 1\nq\n");
    assert!(
        said.contains("\"install\" is not something this understands"),
        "{said}"
    );
    assert!(said.contains("Type the number after \"remove\""), "{said}");
    assert!(said.contains("Type one number from 1 to 1."), "{said}");
    assert!(said.contains("is not something this understands"), "{said}");
    assert!(said.contains("That one is not installed"), "{said}");
    assert!(said.contains("Done."), "{said}");
    assert!(nothing_installed(&places));
}

/// A RAR and a damaged archive say what to do; a failed unpacking leaves no folder behind.
#[test]
fn what_cannot_be_installed_says_why() {
    let dir = Scratch::new("refused");
    let places = dir.places();
    std::fs::create_dir_all(&places.loose).unwrap();
    std::fs::write(places.loose.join("a.rar"), b"Rar!\x1A\x07\x01\x00 rest").unwrap();
    let said = run_with(&places, "1\n\n");
    assert!(said.contains(".7z or .zip"), "{said}");
    // A zip whose file is damaged after the listing: the unpacking fails, nothing is left.
    let damaged = places.loose.join("b.zip");
    make_zip(&damaged, &[("Data/Textures/b.dds", b"0123456789")]);
    let mut bytes = std::fs::read(&damaged).unwrap();
    let at = bytes.windows(10).position(|w| w == b"0123456789").unwrap();
    bytes[at] ^= 0xff;
    std::fs::write(&damaged, bytes).unwrap();
    let said = run_with(&places, "2\nyes\n\n");
    assert!(said.contains("Nothing was installed."), "{said}");
    assert!(nothing_installed(&places));
}

/// Too little room on the drive: said before anything is unpacked, as "X needed, Y free", never as
/// a damaged archive; nothing installed, the archive still waiting.
#[test]
fn too_little_room_is_said_before_unpacking() {
    let dir = Scratch::new("room");
    let mut places = dir.places();
    places.room = |_| Some(100 << 20);
    let archive = places.loose.join("Retex.zip");
    make_zip(&archive, &[("Data/Textures/a.dds", b"tex")]);
    // Joined back where it was wrapped.
    let said = run_with(&places, "1\nyes\n\n").replace("\n    ", " ");
    assert!(said.contains("Free on the drive: 100.0 MB."), "{said}");
    assert!(
        said.contains(
            "Not enough room on the drive: the mod unpacks to 3 bytes, and nv-rs keeps 256.0 MB more free"
        ),
        "{said}"
    );
    assert!(
        said.contains("100.0 MB is free: 156.0 MB more is needed."),
        "{said}"
    );
    assert!(!said.contains("damaged"), "{said}");
    assert!(nothing_installed(&places));
    assert!(archive.exists());
    // Unknown room: said, and the install goes on with the default limits.
    places.room = |_| None;
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("could not be read"), "{said}");
    assert!(said.contains("Installed Retex"), "{said}");
    // The limits passed down: the room less the spare room.
    let limits = room_limits(10, 1, Some(1 << 30)).unwrap();
    assert_eq!(limits.total, (1 << 30) - spare_for(1));
    assert_eq!(
        limits.decode,
        Limits::default().decode,
        "decoding is never lowered for room"
    );
    assert!(room_limits(10, 1, Some(spare_for(1))).is_err());
    assert_eq!(room_limits(10, 1, None).unwrap(), Limits::default());
}

/// The room on the drive is read on this system (the scratch folder's drive has some).
#[test]
fn the_room_on_the_drive_is_read() {
    let dir = Scratch::new("free");
    assert!(room_on_drive(&dir.0).is_some_and(|free| free > 0));
    // A folder not made yet: its nearest folder's drive.
    assert!(room_on_drive(&dir.0.join("not").join("yet")).is_some());
}

/// An empty mods folder says where to put mods, the folder's full path on a line of its own; then
/// "Press Enter to close." waits for one line.
#[test]
fn nothing_to_do_says_where_mods_go_and_waits() {
    let dir = Scratch::new("empty");
    let places = dir.places();
    let (said, code, rest) = run_window(Ok(places.clone()), "\nleft\n");
    assert!(said.contains("Put a mod's archive"), "{said}");
    let full = full_path(&places.loose);
    assert!(
        said.lines().any(|l| l == full.display().to_string()),
        "{said}"
    );
    assert!(said.trim_end().ends_with("Press Enter to close."), "{said}");
    assert_eq!(code, 0);
    assert_eq!(rest, "left\n", "one line read, no more");
}

/// Every way the window's chooser ends finishes with "Press Enter to close.": finished, stopped by
/// the input ending, started without its folders, refused (inside the game), and an error.
#[test]
fn every_end_waits_for_enter() {
    let dir = Scratch::new("window");
    let places = dir.places();
    gun_pack(&places);
    let ends = |said: &str| said.trim_end().ends_with("Press Enter to close.");
    let (said, code, _) = run_window(Ok(places.clone()), "\n\n");
    assert!(said.contains("Done.") && ends(&said) && code == 0, "{said}");
    let (said, code, _) = run_window(Ok(places.clone()), "1\n");
    assert!(
        said.contains("Stopped") && ends(&said) && code == 0,
        "{said}"
    );
    let (said, code, _) = run_window(Err("started without its folders".into()), "");
    assert!(
        said.contains("without its folders") && ends(&said) && code == 2,
        "{said}"
    );
    let mut inside = places.clone();
    inside.loose = places.game.join("Data");
    let (said, code, _) = run_window(Ok(inside), "\n");
    assert!(
        said.contains("inside the game's folder") && ends(&said),
        "{said}"
    );
    assert_eq!(code, 2);
    // The input failing (not ending): said in words, then the same last line.
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("the console went away"))
        }
    }
    impl BufRead for Broken {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Err(std::io::Error::other("the console went away"))
        }
        fn consume(&mut self, _: usize) {}
    }
    let mut out = Vec::new();
    let code = run_in_window(&mut Broken, &mut out, Ok(places));
    let said = String::from_utf8(out).unwrap();
    assert!(
        said.contains("the console went away") && ends(&said),
        "{said}"
    );
    assert_eq!(code, 1);
}

/// The installs folder or the mods folder inside the game's folder, plainly or through a link
/// (a junction on Windows): the chooser refuses to start, in words, and changes nothing.
#[test]
fn folders_inside_the_game_are_refused() {
    let dir = Scratch::new("in-game");
    let places = dir.places();
    let mut plain = places.clone();
    plain.installs = places.game.join("mod installs");
    let said = run_with(&plain, "");
    assert!(
        said.contains("The mod installs folder is inside the game's folder"),
        "{said}"
    );
    assert!(!plain.installs.exists());
    let mut linked = places.clone();
    linked.loose = dir.0.join("linked mods");
    folder_link(&places.game.join("Data"), &linked.loose);
    make_zip(
        &places.game.join("Data").join("Retex.zip"),
        &[("Data/Textures/a.dds", b"tex")],
    );
    let said = run_with(&linked, "1\nyes\n\n");
    assert!(
        said.contains("The mods folder is inside the game's folder"),
        "{said}"
    );
    assert!(!said.contains("Installed"), "{said}");
    assert!(places.game.join("Data").join("Retex.zip").exists());
    assert!(!places.game.join("Data").join(installs::KEPT).exists());
    assert!(nothing_installed(&places));
}

/// A folder link to `to` at `link`: a symbolic link on Linux and macOS, a junction on Windows.
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

/// A killed install's staging folder: not listed as a mod, cleared at the next start (said), and
/// the mod installs from scratch.
#[test]
fn a_stopped_install_is_cleared_at_the_next_start() {
    let dir = Scratch::new("leftover");
    let places = dir.places();
    make_zip(
        &places.loose.join("Retex.zip"),
        &[("Data/Textures/a.dds", b"tex")],
    );
    let staging = places
        .installs
        .join(format!("{}01 Retex", installs::STAGING));
    std::fs::create_dir_all(staging.join("Textures")).unwrap();
    std::fs::write(staging.join("Textures").join("half.dds"), b"half").unwrap();
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("They were cleared"), "{said}");
    let before = &said[..said.find("Installing").unwrap()];
    assert!(!before.contains("Installed with nv-rs"), "{said}");
    assert!(!staging.exists());
    let folder = only_installed(&places);
    assert_eq!(
        folder.file_name().unwrap().to_string_lossy(),
        "01 Retex",
        "{said}"
    );
    assert!(!folder.join("Textures").join("half.dds").exists());
}

/// A link left at a staging folder's place, leading to another folder: removed itself, the folder
/// it leads to untouched; the mod installs.
#[test]
fn a_link_at_the_staging_place_is_never_followed() {
    let dir = Scratch::new("staging-link");
    let places = dir.places();
    make_zip(
        &places.loose.join("Retex.zip"),
        &[("Data/Textures/a.dds", b"tex")],
    );
    let outside = dir.0.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), b"keep").unwrap();
    std::fs::create_dir_all(&places.installs).unwrap();
    folder_link(
        &outside,
        &places
            .installs
            .join(format!("{}01 Retex", installs::STAGING)),
    );
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("Installed Retex"), "{said}");
    assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"keep");
    assert!(!outside.join("Textures").exists());
}

/// An archive of the same name already kept: it stays, the new one is kept beside it under a free
/// name.
#[test]
fn a_kept_archive_is_never_replaced() {
    let dir = Scratch::new("kept-archive");
    let places = dir.places();
    make_zip(
        &places.loose.join("Retex.zip"),
        &[("Data/Textures/a.dds", b"new")],
    );
    let kept = places.loose.join(installs::KEPT);
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::write(kept.join("Retex.zip"), b"older archive").unwrap();
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("The archive was moved to"), "{said}");
    assert_eq!(
        std::fs::read(kept.join("Retex.zip")).unwrap(),
        b"older archive"
    );
    assert!(kept.join("Retex (2).zip").exists());
    assert!(!places.loose.join("Retex.zip").exists());
}

/// An archive installed already (its name and size in a record): marked in the list, and a second
/// copy only after a yes.
#[test]
fn an_archive_installed_already_is_said() {
    let dir = Scratch::new("twice");
    let places = dir.places();
    let archive = places.loose.join("Retex.zip");
    make_zip(&archive, &[("Data/Textures/a.dds", b"tex")]);
    let bytes = std::fs::read(&archive).unwrap();
    run_with(&places, "1\nyes\n\n");
    std::fs::write(&archive, &bytes).unwrap();
    let said = run_with(&places, "2\n\n\n");
    assert!(
        said.contains("Retex.zip (installed already, as Retex)"),
        "{said}"
    );
    assert!(said.contains("Install a second copy"), "{said}");
    assert!(said.contains("Left as it is"), "{said}");
    assert_eq!(installs::installed(&places.installs).unwrap().len(), 1);
    let said = run_with(&places, "2\nyes\nyes\n\n");
    assert!(said.contains("Installed Retex"), "{said}");
    assert_eq!(installs::installed(&places.installs).unwrap().len(), 2);
}

/// The steps: a step shown only for an earlier answer, b going back a group and a step with the
/// earlier marks kept, a changed answer changing which step comes next, 0 for none, each group
/// type's own words, upper-case commands.
#[test]
fn the_steps_follow_the_answers_and_go_back() {
    let dir = Scratch::new("bodies");
    let places = dir.places();
    bodies(&places);
    // Body: Type B, Hands: none, then at "Outfits B": back, back (Hands, then Body type with
    // Type B marked), Type A, Hands kept (none), "Outfits A": none, Core: Enter, yes.
    let said = run_with(&places, "1\n2\n0\nB\nb\n1\n\n0\n\nyes\n\n");
    assert!(said.contains("Choose one: type its number"), "{said}");
    assert!(
        said.contains("Choose one, or none: type its number, or 0 for none"),
        "{said}"
    );
    assert!(said.contains("Choose at least one:"), "{said}");
    assert!(said.contains("Choose any:"), "{said}");
    assert!(said.contains("All of these are installed"), "{said}");
    assert!(
        said.contains("2) [x] Type B"),
        "the earlier answer kept: {said}"
    );
    let first_b = said.find("Step 2: Outfits for B").unwrap();
    let then_a = said.find("Step 2: Outfits for A").unwrap();
    assert!(first_b < then_a, "{said}");
    assert!(said[then_a..].find("Outfits for B").is_none(), "{said}");
    let folder = only_installed(&places);
    let read = |p: &str| std::fs::read(folder.join(p)).ok();
    assert_eq!(read("meshes/body.nif").unwrap(), b"body A");
    assert_eq!(read("Bodies.esp").unwrap(), b"TES4 bodies");
    assert!(read("meshes/suit.nif").is_none());
    assert!(read("meshes/gloves.nif").is_none());
    assert!(read("meshes/rings.nif").is_none());
}

/// Several numbers in a choose-one group are refused and the same group asked again; b at the
/// first question says so and asks it again; ?N reads an option; Q stops.
#[test]
fn one_choice_groups_take_one_number() {
    let dir = Scratch::new("one-choice");
    let places = dir.places();
    bodies(&places);
    let said = run_with(&places, "1\n1 2\nb\n?2\n2\n1 2\nQ\n");
    assert!(said.contains("Only one can be chosen here"), "{said}");
    assert!(said.contains("This is the first question"), "{said}");
    assert!(said.contains("The B body."), "{said}");
    // Body type shown four times (first, after the refused answer, after b, after ?2), Hands
    // twice.
    assert_eq!(said.matches("\n  Body type\n").count(), 4, "{said}");
    assert_eq!(said.matches("\n  Hands\n").count(), 2, "{said}");
    assert!(said.contains("Stopped: nothing was installed."), "{said}");
    assert!(nothing_installed(&places));
}

/// A choose-at-least-one group (its first option marked to start with, as the installers'
/// managers do): 0 is refused and a number not shown too, the group asked again each time; Enter
/// at the summary goes back to the last step, a word that is not yes or q says so.
#[test]
fn at_least_one_and_the_summary() {
    let dir = Scratch::new("at-least");
    let places = dir.places();
    bodies(&places);
    // Body B, Hands Enter; Outfits B: 0 and 3 refused, 2; Core Enter; summary Enter (back to
    // Core), Core Enter; summary "ok" (back), Core Enter; summary yes; list Enter.
    let said = run_with(&places, "1\n2\n\n0\n3\n2\n\n\n\nok\n\nyes\n\n");
    assert_eq!(said.matches("Not that:").count(), 1, "{said}");
    assert!(
        said.contains("\"3\" is not one of the numbers shown"),
        "{said}"
    );
    assert_eq!(said.matches("\n  Outfits B\n").count(), 3, "{said}");
    assert!(
        said.contains("\"ok\" is not yes or q: back to the choices."),
        "{said}"
    );
    assert_eq!(said.matches("Step 3: Core").count(), 3, "{said}");
    let folder = only_installed(&places);
    assert_eq!(
        std::fs::read(folder.join("meshes").join("hat.nif")).unwrap(),
        b"hat B"
    );
    assert_eq!(
        std::fs::read(folder.join("meshes").join("body.nif")).unwrap(),
        b"body B"
    );
}

/// An installer with no step shown: straight to the summary; Enter there has nothing to go back
/// to and asks again; the input ending stops; yes installs.
#[test]
fn an_installer_with_no_steps() {
    let dir = Scratch::new("no-steps");
    let places = dir.places();
    let config = r#"<config><moduleName>Small</moduleName>
      <requiredInstallFiles><file source="Small.esp"/></requiredInstallFiles>
      <installSteps><installStep name="Hidden"><visible><flagDependency flag="never" value="x"/></visible><optionalFileGroups>
        <group name="G" type="SelectAny"><plugins><plugin name="P"><description/><files><file source="p.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group>
      </optionalFileGroups></installStep></installSteps></config>"#;
    make_zip(
        &places.loose.join("Small.zip"),
        &[
            ("fomod/ModuleConfig.xml", config.as_bytes()),
            ("Small.esp", b"TES4 small"),
            ("p.esp", b"TES4 p"),
        ],
    );
    let said = run_with(&places, "1\n\n");
    assert!(
        said.contains("This installer has no choices to make."),
        "{said}"
    );
    assert!(said.contains("no choices to go back to"), "{said}");
    assert_eq!(said.matches("files will be installed").count(), 2, "{said}");
    assert!(said.contains("Stopped: nothing was installed."), "{said}");
    assert!(nothing_installed(&places));
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("Installed Small (1 files)"), "{said}");
}

/// The files read for the installers' questions: each folder counted on its own (two under the
/// cap that are over it together are fine), past either cap the install stops in words; the loose
/// mods folder's files count, its archives and READ ME do not.
#[test]
fn the_facts_caps_refuse_per_folder() {
    let dir = Scratch::new("facts-caps");
    let places = dir.places();
    std::fs::create_dir_all(places.game.join("Data").join("Textures")).unwrap();
    std::fs::write(
        places.game.join("Data").join("Textures").join("a.dds"),
        b"t",
    )
    .unwrap();
    let one = places.installs.join("01 One");
    std::fs::create_dir_all(&one).unwrap();
    std::fs::write(one.join(mod_install::RECORD), b"name: One\n").unwrap();
    std::fs::write(one.join("One.esp"), b"TES4").unwrap();
    std::fs::write(one.join("Two.esp"), b"TES4").unwrap();
    make_zip(&places.loose.join("Retex.zip"), &[("a.dds", b"x")]);
    std::fs::write(places.loose.join("READ ME.txt"), b"x").unwrap();
    std::fs::write(places.loose.join("Mine.esp"), b"TES4").unwrap();
    // Data: 4 entries (two masters, Textures, a.dds); the mod: 3; the loose folder: 3. Each under
    // a cap of 4, all together far over it.
    let facts = facts_with(&places, 4, 8).unwrap();
    let names: Vec<&str> = facts.plugins.iter().map(|(p, _)| p.as_str()).collect();
    for wanted in [
        "FalloutNV.esm",
        "Textures/a.dds",
        "One.esp",
        "Two.esp",
        "Mine.esp",
    ] {
        assert!(names.contains(&wanted), "{names:?}");
    }
    for not in ["Retex.zip", "READ ME.txt", mod_install::RECORD] {
        assert!(!names.contains(&not), "{names:?}");
    }
    let many = facts_with(&places, 3, 8).unwrap_err();
    assert!(many.contains("more than 3 files"), "{many}");
    let deep = facts_with(&places, 100, 0).unwrap_err();
    assert!(deep.contains("more than 0 folders deep"), "{deep}");
}

/// A folder that can't be read stops the install in words, never answering from part of the list
/// (Linux and macOS, where a folder's rights can be taken away; skipped when they are not
/// enforced, as for the superuser).
#[cfg(unix)]
#[test]
fn an_unreadable_folder_refuses() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Scratch::new("facts-unreadable");
    let places = dir.places();
    let locked = places.game.join("Data").join("Locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let enforced = std::fs::read_dir(&locked).is_err();
    let result = facts(&places);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if enforced {
        let why = result.unwrap_err();
        assert!(why.contains("could not be read"), "{why}");
    }
}

/// Text is wrapped at 78 columns on spaces with a hanging indent; a word longer than that (a
/// path) stays whole.
#[test]
fn text_is_wrapped_for_an_80_column_window() {
    let long = format!("    1) [ ] {}", "word ".repeat(40));
    let wrapped = wrap(&long);
    let lines: Vec<&str> = wrapped.lines().collect();
    assert!(lines.len() > 1);
    assert!(
        lines.iter().all(|l| l.chars().count() <= WIDTH),
        "{wrapped}"
    );
    assert!(lines[0].starts_with("    1) [ ] word"));
    assert!(
        lines[1..].iter().all(|l| l.starts_with("        word")),
        "{wrapped}"
    );
    let path = format!("C:\\{}", "x".repeat(100));
    assert_eq!(
        wrap(&format!("at {path} here")),
        format!("at\n    {path}\n    here\n")
    );
    assert_eq!(wrap("a\n\nb"), "a\n\nb\n");
}

/// Sizes as a player reads them.
#[test]
fn sizes_in_words() {
    assert_eq!(size_text(24), "24 bytes");
    assert_eq!(size_text(1536), "1.5 KB");
    assert_eq!(size_text(256 << 20), "256.0 MB");
    assert_eq!(size_text(5 << 30), "5.0 GB");
}

/// A typed line longer than the cap keeps its start, the rest dropped, and the next line read as
/// the next answer.
#[test]
fn a_long_line_is_capped() {
    let mut typed = vec![b'1'; MAX_LINE * 3];
    typed.extend_from_slice(b"\nnext\n");
    let mut input = Cursor::new(typed);
    assert_eq!(read_answer(&mut input).unwrap().unwrap().len(), MAX_LINE);
    assert_eq!(read_answer(&mut input).unwrap().unwrap(), "next");
    assert_eq!(read_answer(&mut input).unwrap(), None);
}

thread_local! {
    /// What a test does at each progress report of an install (with its staging folder).
    static PROGRESS_HOOK: std::cell::RefCell<Option<ProgressHook>> = std::cell::RefCell::new(None);
}

type ProgressHook = Box<dyn FnMut(&Path, mod_archive::Progress)>;

/// Called by [`install`] at each progress report.
pub(super) fn on_progress(staging: &Path, p: mod_archive::Progress) {
    PROGRESS_HOOK.with(|h| {
        if let Some(hook) = h.borrow_mut().as_mut() {
            hook(staging, p)
        }
    });
}

/// Runs the chooser with `hook` called at each progress report.
fn run_hooked(
    places: &Places,
    typed: &str,
    hook: impl FnMut(&Path, mod_archive::Progress) + 'static,
) -> String {
    PROGRESS_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
    let said = run_with(places, typed);
    PROGRESS_HOOK.with(|h| *h.borrow_mut() = None);
    said
}

fn three_files(places: &Places) -> PathBuf {
    let archive = places.loose.join("Retex.zip");
    make_zip(
        &archive,
        &[
            ("Data/Textures/a.dds", b"a"),
            ("Data/Textures/b.dds", b"b"),
            ("Data/Meshes/c.nif", b"c"),
        ],
    );
    archive
}

/// A second installer window while one is open: refused in words (exit 2) before anything is
/// changed, a staging folder (the first window's install in progress) untouched; once the first
/// lets go (its window closed, however), the next start runs.
#[test]
fn a_second_installer_window_is_refused() {
    let dir = Scratch::new("second-window");
    let places = dir.places();
    three_files(&places);
    let held = installs::hold_installer(&places.installs, &places.game)
        .unwrap()
        .unwrap();
    let busy = places
        .installs
        .join(format!("{}01 Retex", installs::STAGING));
    std::fs::create_dir_all(busy.join("Textures")).unwrap();
    std::fs::write(busy.join("Textures").join("a.dds"), b"a").unwrap();
    let (said, code, _) = run_window(Ok(places.clone()), "1\nyes\n\n");
    assert!(
        said.contains("The mod installer is already open in another window."),
        "{said}"
    );
    assert_eq!(code, 2);
    assert!(busy.join("Textures").join("a.dds").exists(), "{said}");
    assert!(!said.contains("Installed"), "{said}");
    drop(held);
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("They were cleared"), "{said}");
    assert!(said.contains("Installed Retex"), "{said}");
}

/// The staging folder removed while unpacking (as another window clearing it would): never made
/// again, "Nothing was installed", no mod listed, the archive still waiting.
#[test]
fn the_staging_folder_removed_while_unpacking_installs_nothing() {
    let dir = Scratch::new("staging-gone");
    let places = dir.places();
    let archive = three_files(&places);
    let said = run_hooked(&places, "1\nyes\n\n", |staging, p| {
        if p.files == 1 {
            let _ = std::fs::remove_dir_all(staging);
        }
    });
    assert!(said.contains("Nothing was installed"), "{said}");
    assert!(!said.contains("Installed Retex"), "{said}");
    assert!(installs::installed(&places.installs).unwrap().is_empty());
    assert!(
        nothing_installed(&places),
        "{:?}",
        names_in_installs(&places)
    );
    assert!(archive.exists());
}

/// A file removed after it was unpacked and before the record: the record would list a file the
/// folder lacks, so nothing is installed and the path is said on a line of its own.
#[test]
fn a_file_gone_before_the_record_installs_nothing() {
    let dir = Scratch::new("file-gone");
    let places = dir.places();
    three_files(&places);
    let said = run_hooked(&places, "1\nyes\n\n", |staging, p| {
        // When the last file is written.
        if p.files_total > 0 && p.files == p.files_total && p.saving == 0 {
            let _ = std::fs::remove_file(staging.join("Textures").join("a.dds"));
        }
    });
    assert!(said.contains("Nothing was installed"), "{said}");
    assert!(
        said.lines().any(|l| path_line(l) && l.contains("a.dds")),
        "{said}"
    );
    // Windows syncs no small texture, so the check before the record finds it gone; on Linux and
    // macOS the pass that syncs every file meets it first.
    if cfg!(windows) {
        assert!(said.contains("a file it had unpacked is gone"), "{said}");
    }
    assert!(
        nothing_installed(&places),
        "{:?}",
        names_in_installs(&places)
    );
}

/// A solid 7z whose chosen file comes after a large one not chosen: the room on the drive is
/// between what is written and what is decoded, and it installs (decoding writes nothing); the
/// large file read through is shown in the progress.
#[test]
fn a_solid_7z_decoding_more_than_the_room_installs() {
    let dir = Scratch::new("solid-room");
    let mut places = dir.places();
    // Room for the chosen file and the spare room, and 1 MB more: far less than the 4 MB decoded.
    places.room = |_| Some((256 << 20) + 4096 + 3 + (1 << 20));
    let archive = places.loose.join("Retex.7z");
    std::fs::create_dir_all(&places.loose).unwrap();
    let pad = vec![0u8; 4 << 20];
    let mut writer = sevenz_rust2::ArchiveWriter::create(&archive).unwrap();
    writer
        .push_archive_entries(
            vec![
                sevenz_rust2::ArchiveEntry::new_file("Extras/pad.bin"),
                sevenz_rust2::ArchiveEntry::new_file("Data/Textures/a.dds"),
            ],
            vec![
                sevenz_rust2::SourceReader::new(Cursor::new(pad)),
                sevenz_rust2::SourceReader::new(Cursor::new(b"tex".to_vec())),
            ],
        )
        .unwrap();
    writer.finish().unwrap();
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("Installed Retex"), "{said}");
    assert!(!said.contains("Not enough room"), "{said}");
    assert!(said.contains("of other files read through"), "{said}");
    assert_eq!(
        std::fs::read(only_installed(&places).join("Textures").join("a.dds")).unwrap(),
        b"tex"
    );
}

/// A leftover that can't be cleared: how many were cleared and which could not be (its path on a
/// line of its own) are said, and the installer carries on. The check is skipped where the system
/// lets it be removed anyway (the superuser on Linux and macOS).
#[test]
fn a_partial_clear_is_said_and_the_installer_carries_on() {
    let dir = Scratch::new("partial-clear");
    let places = dir.places();
    three_files(&places);
    let gone = places
        .installs
        .join(format!("{}01 Gone", installs::STAGING));
    let stuck = places
        .installs
        .join(format!("{}02 Stuck", installs::STAGING));
    for leftover in [&gone, &stuck] {
        std::fs::create_dir_all(leftover).unwrap();
        std::fs::write(leftover.join("half.dds"), b"half").unwrap();
    }
    #[cfg(windows)]
    let open = {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(stuck.join("half.dds"))
            .unwrap()
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stuck, std::fs::Permissions::from_mode(0o555)).unwrap();
    }
    let said = run_with(&places, "1\nyes\n\n");
    #[cfg(windows)]
    drop(open);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&stuck, std::fs::Permissions::from_mode(0o755));
    }
    assert!(!gone.exists(), "{said}");
    if stuck.join("half.dds").exists() {
        assert!(
            said.replace("\n    ", " ")
                .contains("1 of 2 were cleared; these could not be"),
            "{said}"
        );
        let full = full_path(&stuck);
        assert!(
            said.lines().any(|l| l == full.display().to_string()),
            "{said}"
        );
    }
    assert!(said.contains("Installed Retex"), "{said}");
}

/// A line that is a path is never wrapped, spaces and all.
#[test]
fn a_path_line_is_never_wrapped() {
    let path = format!(
        "C:\\Program Files\\{}\\a folder with spaces",
        "Some Long Name ".repeat(6)
    );
    assert_eq!(wrap(&format!("Gone:\n{path}")), format!("Gone:\n{path}\n"));
    let unix = format!("/home/someone/{}/mods", "a b ".repeat(30));
    assert_eq!(wrap(&unix), format!("{unix}\n"));
    assert!(path_line("\\\\server\\share\\x") && path_line("~/nv-rs") && !path_line("at C:\\x"));
}
