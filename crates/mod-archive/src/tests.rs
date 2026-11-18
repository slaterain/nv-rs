//! Tests on archives the tests make themselves, in a scratch folder removed afterwards.

use super::*;
use std::io::{Cursor, Write};

/// A scratch folder, removed when the test ends however it ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("nv-mod-archive-{name}-{}", std::process::id()));
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

const FILES: &[(&str, &[u8])] = &[
    ("Gun Pack\\fomod\\ModuleConfig.xml", b"<config/>"),
    ("Gun Pack/Main/GunPack.esp", b"TES4 plugin"),
    (
        "Gun Pack/Main/Meshes/gun.nif",
        b"a mesh, a mesh, a mesh, a mesh",
    ),
    ("Gun Pack/Textures/a.dds", b"texture one"),
];

fn make_zip(path: &Path, files: &[(&str, &[u8])], method: zip::CompressionMethod) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    writer.add_directory("Gun Pack/", options).unwrap();
    for (name, data) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

/// One solid LZMA2 block holding every file.
fn make_7z(path: &Path, files: &[(&str, &[u8])]) {
    let mut writer = sevenz_rust2::ArchiveWriter::create(path).unwrap();
    let entries = files
        .iter()
        .map(|(n, _)| sevenz_rust2::ArchiveEntry::new_file(n))
        .collect();
    let readers = files
        .iter()
        .map(|(_, d)| sevenz_rust2::SourceReader::new(Cursor::new(d.to_vec())))
        .collect();
    writer.push_archive_entries(entries, readers).unwrap();
    writer.finish().unwrap();
    let made = open_7z(path).unwrap();
    assert_eq!(made.archive().blocks.len(), 1, "one solid block");
    assert!(made.archive().is_solid);
}

fn archives(dir: &Path) -> Vec<PathBuf> {
    let zip = dir.join("mod.zip");
    let stored = dir.join("stored.fomod");
    let sevenz = dir.join("mod.fomod");
    make_zip(&zip, FILES, zip::CompressionMethod::Deflated);
    make_zip(&stored, FILES, zip::CompressionMethod::Stored);
    make_7z(&sevenz, FILES);
    vec![zip, stored, sevenz]
}

#[test]
fn formats_are_told_by_their_first_bytes() {
    assert_eq!(format_of(b"PK\x03\x04rest"), Format::Zip);
    assert_eq!(format_of(b"PK\x05\x06"), Format::Zip);
    assert_eq!(format_of(b"7z\xBC\xAF\x27\x1C\x00\x04"), Format::SevenZip);
    assert_eq!(format_of(b"Rar!\x1A\x07\x00"), Format::Rar);
    assert_eq!(format_of(b"Rar!\x1A\x07\x01\x00"), Format::Rar);
    assert_eq!(format_of(b"<config>"), Format::Unknown);
    assert_eq!(format_of(b""), Format::Unknown);
}

#[test]
fn zip_and_solid_7z_list_alike() {
    let dir = Scratch::new("list");
    for archive in archives(&dir.0) {
        let listed = list(&archive).unwrap();
        let names: Vec<(String, u64)> = listed.iter().map(|e| (key(&e.path), e.size)).collect();
        let expected: Vec<(String, u64)> = FILES
            .iter()
            .map(|(n, d)| (key(n), d.len() as u64))
            .collect();
        assert_eq!(names, expected, "{}", archive.display());
    }
}

#[test]
fn one_file_is_read_by_any_spelling() {
    let dir = Scratch::new("read");
    for archive in archives(&dir.0) {
        assert_eq!(
            read(&archive, "gun pack/FOMOD/moduleconfig.xml")
                .unwrap()
                .as_deref(),
            Some(&b"<config/>"[..]),
            "{}",
            archive.display()
        );
        assert_eq!(read(&archive, "Gun Pack/nothing.txt").unwrap(), None);
    }
}

#[test]
fn wanted_files_are_unpacked_to_their_places() {
    let dir = Scratch::new("extract");
    for (i, archive) in archives(&dir.0).iter().enumerate() {
        let to = dir.0.join(format!("out{i}"));
        // A folder already there under another case is used, on every system.
        std::fs::create_dir_all(to.join("textures")).unwrap();
        let wanted = vec![
            (
                "gun pack\\main\\gunpack.esp".to_owned(),
                "GunPack.esp".to_owned(),
            ),
            (
                "Gun Pack/Main/Meshes/gun.nif".to_owned(),
                "Meshes/Weapons/gun.nif".to_owned(),
            ),
            (
                "Gun Pack/Textures/a.dds".to_owned(),
                "Textures/a.dds".to_owned(),
            ),
            (
                "Gun Pack/Textures/a.dds".to_owned(),
                "Textures/copy/b.dds".to_owned(),
            ),
            ("Gun Pack/missing.esp".to_owned(), "missing.esp".to_owned()),
        ];
        let done = extract(archive, &wanted, &to, Limits::default()).unwrap();
        assert_eq!(done.missing, ["Gun Pack/missing.esp"]);
        assert_eq!(done.written.len(), 4, "{:?}", done.written);
        assert_eq!(
            std::fs::read(to.join("GunPack.esp")).unwrap(),
            b"TES4 plugin"
        );
        assert_eq!(
            std::fs::read(to.join("Meshes").join("Weapons").join("gun.nif")).unwrap(),
            b"a mesh, a mesh, a mesh, a mesh"
        );
        assert_eq!(
            std::fs::read(to.join("textures").join("a.dds")).unwrap(),
            b"texture one"
        );
        assert_eq!(
            std::fs::read(to.join("textures").join("copy").join("b.dds")).unwrap(),
            b"texture one"
        );
        let top: Vec<String> = std::fs::read_dir(&to)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.eq_ignore_ascii_case("textures"))
            .collect();
        assert_eq!(top, ["textures"], "{}", archive.display());
        assert!(!to.join("missing.esp").exists());
    }
}

#[test]
fn a_destination_outside_the_folder_writes_nothing() {
    let dir = Scratch::new("outside");
    let archive = &archives(&dir.0)[0];
    let to = dir.0.join("out");
    for bad in [
        "../escape.esp",
        "a/../../escape.esp",
        "/root.esp",
        "\\root.esp",
        "C:\\x.esp",
        "a//b",
        "x:stream",
        "",
    ] {
        let wanted = vec![
            (
                "Gun Pack/Main/GunPack.esp".to_owned(),
                "fine.esp".to_owned(),
            ),
            ("Gun Pack/Main/GunPack.esp".to_owned(), bad.to_owned()),
        ];
        let error = extract(archive, &wanted, &to, Limits::default()).unwrap_err();
        assert_eq!(error.problem, Problem::Outside, "{bad}");
        assert!(!to.join("fine.esp").exists(), "{bad}");
    }
    assert!(!dir.0.join("escape.esp").exists());
}

#[test]
fn what_cannot_be_read_says_what_to_do() {
    let dir = Scratch::new("refused");
    let rar = dir.0.join("mod.rar");
    std::fs::write(&rar, b"Rar!\x1A\x07\x01\x00 the rest").unwrap();
    let error = list(&rar).unwrap_err();
    assert_eq!(error.problem, Problem::Rar);
    assert!(error.player_text().contains(".7z or .zip"));
    let text = dir.0.join("readme.fomod");
    std::fs::write(&text, b"just text").unwrap();
    assert_eq!(list(&text).unwrap_err().problem, Problem::NotAnArchive);
    // Cut short: the first half of a real archive.
    for archive in archives(&dir.0) {
        let bytes = std::fs::read(&archive).unwrap();
        let cut = dir.0.join("cut");
        std::fs::write(&cut, &bytes[..bytes.len() / 2]).unwrap();
        let error = list(&cut)
            .and_then(|_| {
                extract(
                    &cut,
                    &[("Gun Pack/Textures/a.dds".into(), "a.dds".into())],
                    &dir.0.join("cut-out"),
                    Limits::default(),
                )
            })
            .unwrap_err();
        assert_eq!(
            error.problem,
            Problem::Damaged,
            "{}: {error}",
            archive.display()
        );
        assert!(error.player_text().contains("Download it again"));
    }
    let gone = dir.0.join("no such file.7z");
    assert_eq!(list(&gone).unwrap_err().problem, Problem::Io);
}

// ------------------------------------------------------------------------------------------------
// Hostile archives

/// A link inside the archive is never unpacked: it is not listed, and asking for it finds nothing.
#[test]
fn links_in_an_archive_are_never_unpacked() {
    let dir = Scratch::new("links");
    let archive = dir.0.join("links.zip");
    let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    writer
        .add_symlink("Mod/escape.dds", "../../../outside.dds", options)
        .unwrap();
    writer.start_file("Mod/real.dds", options).unwrap();
    writer.write_all(b"real").unwrap();
    writer.finish().unwrap();
    let names: Vec<String> = list(&archive)
        .unwrap()
        .into_iter()
        .map(|e| e.path)
        .collect();
    assert_eq!(names, ["Mod/real.dds"]);
    let to = dir.0.join("out");
    let done = extract(
        &archive,
        &[
            ("Mod/escape.dds".into(), "escape.dds".into()),
            ("Mod/real.dds".into(), "real.dds".into()),
        ],
        &to,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(done.missing, ["Mod/escape.dds"]);
    assert!(!to.join("escape.dds").exists());
    assert_eq!(std::fs::read(to.join("real.dds")).unwrap(), b"real");
    assert_eq!(read(&archive, "Mod/escape.dds").unwrap(), None);
}

/// Two entries whose names differ only in case (or by a dot at the end): the first is taken.
#[test]
fn of_names_differing_in_case_the_first_is_taken() {
    let dir = Scratch::new("case");
    let zip_path = dir.0.join("case.zip");
    let sevenz_path = dir.0.join("case.7z");
    let files: &[(&str, &[u8])] = &[("Mod/A.dds", b"first"), ("mod/a.DDS.", b"second")];
    make_zip(&zip_path, files, zip::CompressionMethod::Deflated);
    make_7z(&sevenz_path, files);
    for archive in [zip_path, sevenz_path] {
        let to = dir.0.join(format!(
            "out-{}",
            archive.extension().unwrap().to_string_lossy()
        ));
        let done = extract(
            &archive,
            &[("MOD/a.dds".into(), "a.dds".into())],
            &to,
            Limits::default(),
        )
        .unwrap();
        assert_eq!(done.written.len(), 1);
        assert_eq!(std::fs::read(to.join("a.dds")).unwrap(), b"first");
        assert_eq!(read(&archive, "mod/A.DDS").unwrap().unwrap(), b"first");
    }
}

/// Destinations no system holds safely are refused on every system, before anything is written.
#[test]
fn odd_destinations_are_refused_everywhere() {
    let dir = Scratch::new("odd");
    let archive = &archives(&dir.0)[0];
    let to = dir.0.join("out");
    for bad in [
        "CON",
        "sub/nul.txt",
        "Com1.dds",
        "LPT9",
        "a/.../b",
        "x/ ..",
        "a:b",
    ] {
        let error = extract(
            archive,
            &[("Gun Pack/Main/GunPack.esp".into(), bad.into())],
            &to,
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error.problem, Problem::Outside, "{bad}");
    }
    assert!(!to.exists());
}

/// Sizes beyond the limits are refused before anything is written.
#[test]
fn what_is_too_big_is_refused_first() {
    let dir = Scratch::new("big");
    for archive in archives(&dir.0) {
        let to = dir.0.join("out");
        let wanted = vec![
            (
                "Gun Pack/Main/Meshes/gun.nif".to_owned(),
                "a.nif".to_owned(),
            ),
            (
                "Gun Pack/Main/Meshes/gun.nif".to_owned(),
                "b.nif".to_owned(),
            ),
        ];
        // gun.nif is 30 bytes.
        let file_limit = Limits {
            file: 29,
            total: 1000,
        };
        let total_limit = Limits {
            file: 1000,
            total: 59,
        };
        for limits in [file_limit, total_limit] {
            let error = extract(&archive, &wanted, &to, limits).unwrap_err();
            assert_eq!(error.problem, Problem::TooBig, "{limits:?}");
            assert!(!to.exists());
        }
        let fits = Limits {
            file: 30,
            total: 60,
        };
        assert_eq!(
            extract(&archive, &wanted, &to, fits).unwrap().written.len(),
            2
        );
        std::fs::remove_dir_all(&to).unwrap();
    }
}

/// A link already in the folder is never written through: the unpacking stops, and nothing
/// lands where it points.
#[test]
fn a_link_in_the_folder_is_not_followed() {
    let dir = Scratch::new("follow");
    let archive = &archives(&dir.0)[2];
    let to = dir.0.join("out");
    let elsewhere = dir.0.join("elsewhere");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let link = to.join("textures");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
    #[cfg(windows)]
    {
        // A junction: made without rights a player lacks.
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&elsewhere)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }
    let error = extract(
        archive,
        &[("Gun Pack/Textures/a.dds".into(), "Textures/a.dds".into())],
        &to,
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.problem, Problem::Outside, "{error}");
    assert!(error.detail.contains("is a link"), "{error}");
    assert!(!elsewhere.join("a.dds").exists());
    #[cfg(windows)]
    std::fs::remove_dir(&link).unwrap();
}

/// A link where a file is to go (made by something else) is not written through either.
#[cfg(unix)]
#[test]
fn a_link_at_the_file_is_not_followed() {
    let dir = Scratch::new("follow-file");
    let archive = &archives(&dir.0)[0];
    let to = dir.0.join("out");
    std::fs::create_dir_all(&to).unwrap();
    let outside = dir.0.join("outside.esp");
    std::fs::write(&outside, b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, to.join("GunPack.esp")).unwrap();
    let error = extract(
        archive,
        &[("Gun Pack/Main/GunPack.esp".into(), "GunPack.esp".into())],
        &to,
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.problem, Problem::Outside, "{error}");
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep");
}
