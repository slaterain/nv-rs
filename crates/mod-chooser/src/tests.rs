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

/// Runs the chooser with these typed lines; what it said.
fn run_with(places: &Places, typed: &str) -> String {
    let mut input = Cursor::new(typed.as_bytes().to_vec());
    let mut out = Vec::new();
    run(&mut input, &mut out, places).unwrap();
    String::from_utf8(out).unwrap()
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

/// The whole way: pick the archive, a typo and a wrong number (asked again, nothing lost), read an
/// option, choose Loud, Enter for the recommended patches, yes: installed with its record, the
/// archive moved, the game folder untouched.
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
    assert!(said.contains("Installed Gun Pack"), "{said}");
    let installed = installs::installed(&places.installs).unwrap();
    assert_eq!(installed.len(), 1);
    let folder = &installed[0].folder;
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
        .join("installed archives")
        .join("Gun Pack 1.2.fomod")
        .exists());
    assert_eq!(
        std::fs::read(places.game.join("Data").join("FalloutNV.esm")).unwrap(),
        b"esm"
    );
}

/// q stops: nothing installed, no leftover folder, the archive still waiting.
#[test]
fn stopping_installs_nothing() {
    let dir = Scratch::new("stop");
    let places = dir.places();
    let archive = gun_pack(&places);
    let said = run_with(&places, "1\nq\n\n");
    assert!(said.contains("Stopped: nothing was installed."), "{said}");
    assert!(std::fs::read_dir(&places.installs).map_or(true, |mut d| d.next().is_none()));
    assert!(archive.exists());
}

/// Enter at the end of the input (the window closed) stops as q does.
#[test]
fn the_input_ending_stops() {
    let dir = Scratch::new("eof");
    let places = dir.places();
    gun_pack(&places);
    let said = run_with(&places, "1\n");
    assert!(said.contains("Stopped: nothing was installed."), "{said}");
}

/// An archive with no installer installs its Data layout after a yes; remove takes it away after
/// a yes.
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
    assert!(said.contains("Installed Retex (1 files)"), "{said}");
    let installed = installs::installed(&places.installs).unwrap();
    assert_eq!(
        std::fs::read(installed[0].folder.join("Textures").join("a.dds")).unwrap(),
        b"tex"
    );
    let said = run_with(&places, "remove 1\nno\nremove 1\nyes\n\n");
    assert!(said.contains("Kept Retex."), "{said}");
    assert!(said.contains("Removed Retex."), "{said}");
    assert!(installs::installed(&places.installs).unwrap().is_empty());
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
    assert!(std::fs::read_dir(&places.installs).map_or(true, |mut d| d.next().is_none()));
}

/// An empty mods folder says where to put mods.
#[test]
fn nothing_to_do_says_where_mods_go() {
    let dir = Scratch::new("empty");
    let places = dir.places();
    let said = run_with(&places, "");
    assert!(said.contains("Put a mod's archive"), "{said}");
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

/// A link left at the staging folder's place, leading to another folder: removed itself, the
/// folder it leads to untouched; the mod installs.
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
    folder_link(&outside, &places.installs.join("01 Retex.part"));
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("Installed Retex"), "{said}");
    assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"keep");
    assert!(!outside.join("Textures").exists());
}

/// An archive of the same name already kept: it stays, the new one is left where it was.
#[test]
fn a_kept_archive_is_never_replaced() {
    let dir = Scratch::new("kept-archive");
    let places = dir.places();
    make_zip(
        &places.loose.join("Retex.zip"),
        &[("Data/Textures/a.dds", b"new")],
    );
    let kept = places.loose.join("installed archives");
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::write(kept.join("Retex.zip"), b"older archive").unwrap();
    let said = run_with(&places, "1\nyes\n\n");
    assert!(said.contains("could not be moved"), "{said}");
    assert_eq!(
        std::fs::read(kept.join("Retex.zip")).unwrap(),
        b"older archive"
    );
    assert!(places.loose.join("Retex.zip").exists());
}

/// The files read for the installers' questions: past either cap the install stops in words,
/// never answering from part of the list.
#[test]
fn the_facts_caps_refuse() {
    let dir = Scratch::new("facts-caps");
    let places = dir.places();
    std::fs::create_dir_all(places.game.join("Data").join("Textures")).unwrap();
    std::fs::write(
        places.game.join("Data").join("Textures").join("a.dds"),
        b"t",
    )
    .unwrap();
    assert!(facts_with(&places, 100, 8).is_ok());
    let many = facts_with(&places, 1, 8).unwrap_err();
    assert!(many.contains("more than 1 files"), "{many}");
    let deep = facts_with(&places, 100, 0).unwrap_err();
    assert!(deep.contains("more than 0 folders deep"), "{deep}");
}
