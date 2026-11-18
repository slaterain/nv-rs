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
            ..Limits::default()
        };
        let total_limit = Limits {
            file: 1000,
            total: 59,
            ..Limits::default()
        };
        for limits in [file_limit, total_limit] {
            let error = extract(&archive, &wanted, &to, limits).unwrap_err();
            assert_eq!(error.problem, Problem::TooBig, "{limits:?}");
            assert!(!to.exists());
        }
        let fits = Limits {
            file: 30,
            total: 60,
            ..Limits::default()
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

// ------------------------------------------------------------------------------------------------
// Caps and indexes (review of #720, applied here): every count from the archive is capped with a
// refusal, every lookup indexed.

/// File lists past the caps are refused as not a mod's; at the caps they are taken.
#[test]
fn file_lists_past_the_caps_are_refused() {
    let entry = |name: String| (name, 1u64, true);
    let ok: Vec<(String, u64, bool)> = vec![
        entry("x".repeat(MAX_PATH_BYTES)),
        entry("d/".repeat(MAX_PATH_PARTS - 1) + "f"),
    ];
    assert!(check_entries(&ok).is_ok());
    for bad in [
        vec![entry("x".repeat(MAX_PATH_BYTES + 1))],
        vec![entry("d/".repeat(MAX_PATH_PARTS) + "f")],
        vec![entry("a".into()); MAX_ENTRIES + 1],
    ] {
        assert_eq!(check_entries(&bad).unwrap_err().problem, Problem::NotAMod);
    }
    assert!(stays_inside(&format!("{}/x", "n".repeat(MAX_NAME_BYTES))));
    assert!(!stays_inside(&"n".repeat(MAX_NAME_BYTES + 1)));
    assert!(!stays_inside(&"d/".repeat(MAX_PATH_PARTS).to_string()));
    assert!(!stays_inside(&"a/".repeat(MAX_PATH_BYTES / 2 + 1)));
    let dir = Scratch::new("wanted-cap");
    let archive = &archives(&dir.0)[0];
    let many = vec![("a".to_owned(), "a".to_owned()); MAX_ENTRIES + 1];
    let error = extract(archive, &many, &dir.0.join("out"), Limits::default()).unwrap_err();
    assert_eq!(error.problem, Problem::NotAMod);
}

/// 20,000 files into one folder: each placed with one look in that folder's names, not a reading
/// of the folder per file (which takes minutes at this size).
#[test]
fn many_files_in_one_folder_are_placed_by_index() {
    let dir = Scratch::new("many");
    let names: Vec<String> = (0..20_000)
        .map(|i| format!("Mod/Textures/t{i}.dds"))
        .collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
    let archive = dir.0.join("many.zip");
    make_zip(&archive, &files, zip::CompressionMethod::Stored);
    let wanted: Vec<(String, String)> = names
        .iter()
        .map(|n| (n.clone(), n.trim_start_matches("Mod/").to_owned()))
        .collect();
    let started = std::time::Instant::now();
    let done = extract(&archive, &wanted, &dir.0.join("out"), Limits::default()).unwrap();
    assert_eq!(done.written.len(), 20_000);
    assert!(
        started.elapsed().as_secs() < 60,
        "took {:?}",
        started.elapsed()
    );
}

/// A 7z with folder entries and an empty file among its files: files are matched by name.
#[test]
fn a_7z_with_folder_entries_unpacks_by_name() {
    let dir = Scratch::new("7z-folders");
    let path = dir.0.join("folders.7z");
    let mut writer = sevenz_rust2::ArchiveWriter::create(&path).unwrap();
    writer
        .push_archive_entry::<&[u8]>(sevenz_rust2::ArchiveEntry::new_directory("Mod"), None)
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/empty.txt"),
            Some(&b""[..]),
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_directory("Mod/Textures"),
            None::<&[u8]>,
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/Textures/a.dds"),
            Some(&b"aaa"[..]),
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/b.esp"),
            Some(&b"bb"[..]),
        )
        .unwrap();
    writer.finish().unwrap();
    let to = dir.0.join("out");
    let done = extract(
        &path,
        &[
            ("Mod/Textures/a.dds".into(), "Textures/a.dds".into()),
            ("Mod/b.esp".into(), "b.esp".into()),
        ],
        &to,
        Limits::default(),
    )
    .unwrap();
    assert!(done.missing.is_empty(), "{:?}", done.missing);
    assert_eq!(
        std::fs::read(to.join("Textures").join("a.dds")).unwrap(),
        b"aaa"
    );
    assert_eq!(std::fs::read(to.join("b.esp")).unwrap(), b"bb");
}

// ------------------------------------------------------------------------------------------------
// Review of #722

/// The 7z reader is opened with the memory limit: lowered here below what the archive's decoder
/// needs, unpacking is refused as too big instead of allocating.
#[test]
fn a_7z_is_opened_with_a_memory_limit() {
    let dir = Scratch::new("memory");
    let archive = &archives(&dir.0)[2];
    DECODER_MEMORY_KB.with(|kb| kb.set(4));
    let error = extract(
        archive,
        &[("Gun Pack/Textures/a.dds".into(), "a.dds".into())],
        &dir.0.join("out"),
        Limits::default(),
    )
    .unwrap_err();
    DECODER_MEMORY_KB.with(|kb| kb.set(MAX_DECODER_MEMORY / 1024));
    assert_eq!(error.problem, Problem::TooBig, "{error}");
    assert_eq!(
        sevenz_error(sevenz_rust2::Error::MaxMemLimited {
            max_kb: 1,
            actaul_kb: 2
        })
        .problem,
        Problem::TooBig
    );
}

/// A 7z whose big unwanted file comes first: a name it lacks is answered from the listing
/// without decoding, decoding past the limits is refused before it starts, and a wanted file in
/// a block of its own is reached without decoding the big one.
#[test]
fn a_7z_decodes_only_what_is_wanted() {
    let dir = Scratch::new("decode");
    let big = vec![0u8; 64 << 20];
    let solid = dir.0.join("solid.7z");
    make_7z(
        &solid,
        &[
            ("pad.bin", &big),
            ("Mod/fomod/ModuleConfig.xml", b"<config/>"),
        ],
    );
    let started = std::time::Instant::now();
    assert_eq!(read(&solid, "Mod/fomod/info.xml").unwrap(), None);
    assert!(
        started.elapsed().as_millis() < 500,
        "{:?}",
        started.elapsed()
    );
    let small = Limits {
        file: 1 << 20,
        total: 1 << 20,
        decode: 1 << 20,
    };
    let error = extract(
        &solid,
        &[(
            "Mod/fomod/ModuleConfig.xml".into(),
            "ModuleConfig.xml".into(),
        )],
        &dir.0.join("out-solid"),
        small,
    )
    .unwrap_err();
    assert_eq!(error.problem, Problem::TooBig, "{error}");
    assert!(error.detail.contains("to decode"), "{error}");
    // Non-solid: each file a block of its own; the wanted one is reached by skipping the other.
    let blocks = dir.0.join("blocks.7z");
    let mut writer = sevenz_rust2::ArchiveWriter::create(&blocks).unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("pad.bin"),
            Some(&big[..]),
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/b.esp"),
            Some(&b"bb"[..]),
        )
        .unwrap();
    writer.finish().unwrap();
    let to = dir.0.join("out-blocks");
    let done = extract(&blocks, &[("Mod/b.esp".into(), "b.esp".into())], &to, small).unwrap();
    assert_eq!(done.written.len(), 1);
    assert_eq!(std::fs::read(to.join("b.esp")).unwrap(), b"bb");
}

/// A bad file part-way: nothing partial is left, a file already there is not touched, and the
/// error says which complete files were written.
#[test]
fn a_failure_leaves_no_partial_file_and_says_what_was_written() {
    let dir = Scratch::new("partial");
    let archive = dir.0.join("bad.zip");
    make_zip(
        &archive,
        &[("a.esp", b"first file"), ("b.esp", b"second file, damaged")],
        zip::CompressionMethod::Stored,
    );
    // Damage the second file's stored bytes (its CRC no longer matches).
    let mut bytes = std::fs::read(&archive).unwrap();
    let at = bytes
        .windows(b"second file".len())
        .position(|w| w == b"second file")
        .unwrap();
    bytes[at] ^= 0xff;
    std::fs::write(&archive, bytes).unwrap();
    let to = dir.0.join("out");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("b.esp"), b"original").unwrap();
    let error = extract(
        &archive,
        &[
            ("a.esp".into(), "a.esp".into()),
            ("b.esp".into(), "b.esp".into()),
        ],
        &to,
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.problem, Problem::Damaged, "{error}");
    assert_eq!(error.written, [to.join("a.esp")]);
    assert_eq!(std::fs::read(to.join("b.esp")).unwrap(), b"original");
    let names: Vec<String> = std::fs::read_dir(&to)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.contains("nv-rs-part")), "{names:?}");
}

/// Password-locked archives get the password message: a zip whose entries say so (made here by
/// setting the flag), and 7-Zip's password method, which this build has no decoder for.
#[test]
fn locked_archives_say_so() {
    let dir = Scratch::new("locked");
    let archive = dir.0.join("locked.zip");
    make_zip(
        &archive,
        &[("a.esp", b"locked")],
        zip::CompressionMethod::Stored,
    );
    let mut bytes = std::fs::read(&archive).unwrap();
    // General-purpose bit 0 (encrypted) of a.esp's local header (offset 6, its name at 30) and
    // central one (offset 8, its name at 46); the test archive starts with a folder entry.
    let header = |bytes: &[u8], signature: &[u8], name_at: usize| {
        (0..bytes.len() - name_at - 5)
            .find(|&i| {
                bytes[i..].starts_with(signature) && bytes[i + name_at..].starts_with(b"a.esp")
            })
            .unwrap()
    };
    let local = header(&bytes, b"PK\x03\x04", 30);
    bytes[local + 6] |= 1;
    let central = header(&bytes, b"PK\x01\x02", 46);
    bytes[central + 8] |= 1;
    std::fs::write(&archive, bytes).unwrap();
    let error = extract(
        &archive,
        &[("a.esp".into(), "a.esp".into())],
        &dir.0.join("out"),
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.problem, Problem::Encrypted, "{error}");
    assert!(error.player_text().contains("password"));
    for e in [
        sevenz_rust2::Error::UnsupportedCompressionMethod("AES256_SHA256".into()),
        sevenz_rust2::Error::PasswordRequired,
    ] {
        assert_eq!(sevenz_error(e).problem, Problem::Encrypted);
    }
    assert_eq!(
        sevenz_error(sevenz_rust2::Error::UnsupportedCompressionMethod(
            "DEFLATE64".into()
        ))
        .problem,
        Problem::Unsupported
    );
}

/// Names no system holds safely are refused; dots and spaces at a new name's end are dropped as
/// Windows drops them; a name stored in another Unicode form is found.
#[test]
fn odd_names_and_unicode_forms() {
    for bad in [
        "a<b.dds",
        "a>b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "a\u{1}b",
        "PROGRA~1/x",
        "x/ABCDEF~2.DDS",
        "CONIN$",
        "conout$.txt",
        "CLOCK$",
    ] {
        assert!(!stays_inside(bad), "{bad}");
    }
    for good in ["Textures/a.dds", "a~b.dds", "COM10.dds", "aux_cable.nif"] {
        assert!(stays_inside(good), "{good}");
    }
    let dir = Scratch::new("unicode");
    let archive = dir.0.join("nfd.zip");
    // "Café" stored decomposed (e and a combining acute), asked for composed.
    make_zip(
        &archive,
        &[("Mod/Cafe\u{301}.esp", b"cafe")],
        zip::CompressionMethod::Stored,
    );
    let to = dir.0.join("out");
    let done = extract(
        &archive,
        &[("mod/caf\u{e9}.esp".into(), "Textures /Caf\u{e9}.esp".into())],
        &to,
        Limits::default(),
    )
    .unwrap();
    assert!(done.missing.is_empty(), "{:?}", done.missing);
    assert_eq!(
        std::fs::read(to.join("Textures").join("Caf\u{e9}.esp")).unwrap(),
        b"cafe"
    );
}

/// The placer gives the spelling already there, on every system whatever its file system does
/// with case.
#[test]
fn the_placer_keeps_the_spelling_already_there() {
    let dir = Scratch::new("placer");
    std::fs::create_dir_all(dir.0.join("textures")).unwrap();
    let mut placer = Placer::new(&dir.0);
    let placed = placer.place("Textures/a.dds").unwrap();
    assert_eq!(placed, dir.0.join("textures").join("a.dds"));
    // A new folder made with the spelling asked for, and kept for the next file.
    let first = placer.place("Meshes/x.nif").unwrap();
    let second = placer.place("meshes/y.nif").unwrap();
    assert_eq!(first.parent(), second.parent());
}

/// A 7z whose empty file has the name of a file with data, listed first: the first listed is the
/// one taken, by its index, and the unpacking does not fail.
#[test]
fn a_7z_pairs_entries_by_index() {
    let dir = Scratch::new("pairing");
    let path = dir.0.join("pair.7z");
    let mut writer = sevenz_rust2::ArchiveWriter::create(&path).unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/a.dds"),
            Some(&b""[..]),
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("mod/A.dds"),
            Some(&b"data"[..]),
        )
        .unwrap();
    writer
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file("Mod/b.dds"),
            Some(&b"bbb"[..]),
        )
        .unwrap();
    writer.finish().unwrap();
    let to = dir.0.join("out");
    let done = extract(
        &path,
        &[
            ("Mod/a.dds".into(), "a.dds".into()),
            ("Mod/b.dds".into(), "b.dds".into()),
        ],
        &to,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(done.written.len(), 2);
    assert_eq!(std::fs::read(to.join("a.dds")).unwrap(), b"");
    assert_eq!(std::fs::read(to.join("b.dds")).unwrap(), b"bbb");
}

/// Progress: reported after every file (one archive file going to two places counts twice), with
/// the totals known from the start and every megabyte inside a large file; the size shown before
/// unpacking ([`wanted_size`]) is what is written.
#[test]
fn progress_is_reported_as_it_goes() {
    let dir = Scratch::new("progress");
    let big = vec![7u8; (3 << 20) + 5];
    let files: &[(&str, &[u8])] = &[("Data/a.esp", b"plugin"), ("Data/big.bsa", &big)];
    let zip = dir.0.join("p.zip");
    let sevenz = dir.0.join("p.7z");
    make_zip(&zip, files, zip::CompressionMethod::Deflated);
    make_7z(&sevenz, files);
    let wanted = vec![
        ("data/A.esp".to_owned(), "a.esp".to_owned()),
        ("Data/big.bsa".to_owned(), "big.bsa".to_owned()),
        ("Data/a.esp".to_owned(), "copy/a.esp".to_owned()),
        ("Data/none.esp".to_owned(), "none.esp".to_owned()),
    ];
    for (i, archive) in [zip, sevenz].iter().enumerate() {
        let (files_wanted, bytes_wanted) = wanted_size(&list(archive).unwrap(), &wanted);
        assert_eq!(files_wanted, 3);
        assert_eq!(bytes_wanted, 6 + 6 + big.len() as u64);
        let mut seen = Vec::new();
        let to = dir.0.join(format!("out{i}"));
        extract_with(archive, &wanted, &to, Limits::default(), &mut |p| {
            seen.push(p)
        })
        .unwrap();
        let last = *seen.last().unwrap();
        assert_eq!(
            last,
            Progress {
                files: 3,
                files_total: 3,
                bytes: bytes_wanted,
                bytes_total: bytes_wanted,
                read: 0,
                saved: 3,
                saving: 3,
            }
        );
        assert_eq!(
            seen[0].files, 0,
            "a first report as the unpacking starts: {seen:?}"
        );
        assert!(seen
            .iter()
            .all(|p| p.files_total == 3 && p.bytes_total == bytes_wanted));
        // The plugin and its copy come first (the archive's order), then inside the large file
        // at least one report per megabyte before it is done.
        let inside = seen.iter().filter(|p| p.files == 2).count();
        assert!(inside >= 3, "{seen:?}");
        assert!(seen
            .windows(2)
            .all(|w| w[0].bytes <= w[1].bytes && w[0].files <= w[1].files));
    }
}

/// A full drive is said as a full drive, never as a damaged archive, whichever side of the copy
/// the system's "disk full" comes from.
#[test]
fn a_full_drive_is_not_a_damaged_archive() {
    struct Full;
    impl Read for Full {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::StorageFull))
        }
    }
    let dir = Scratch::new("full");
    let placer = Placer::new(&dir.0);
    let e = write_whole(&placer, &dir.0.join("a.bsa"), &mut Full, 10, "a.bsa").unwrap_err();
    assert_eq!(e.problem, Problem::NoRoom);
    assert!(e.player_text().contains("not enough room on the drive"));
    assert!(drive_full(&std::io::Error::from(
        std::io::ErrorKind::QuotaExceeded
    )));
    // The systems' own codes: ENOSPC (28) on Linux and macOS, ERROR_DISK_FULL (112) on Windows.
    #[cfg(unix)]
    assert!(drive_full(&std::io::Error::from_raw_os_error(28)));
    #[cfg(windows)]
    assert!(drive_full(&std::io::Error::from_raw_os_error(112)));
    assert_eq!(
        io(
            &dir.0,
            std::io::Error::from_raw_os_error(if cfg!(windows) { 112 } else { 28 })
        )
        .problem,
        Problem::NoRoom
    );
    // Nothing left behind, not even the temporary name.
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
}

/// What a 7z decodes is capped apart from what is written: a small file after a big one in a
/// solid block installs with the written total far below the decoded bytes (the front end lowers
/// only the written total, to the room on the drive), and only a low decode cap refuses it. The
/// big file read through on the way is reported as read past, before the first file is written.
#[test]
fn decoding_is_capped_apart_from_writing() {
    let dir = Scratch::new("decode-cap");
    let big = vec![0u8; 3 << 20];
    let solid = dir.0.join("solid.7z");
    make_7z(&solid, &[("pad.bin", &big), ("Data/a.esp", b"plugin")]);
    let wanted = vec![("Data/a.esp".to_owned(), "a.esp".to_owned())];
    let mut seen = Vec::new();
    let written = Limits {
        total: 100,
        ..Limits::default()
    };
    let done = extract_with(&solid, &wanted, &dir.0.join("out"), written, &mut |p| {
        seen.push(p)
    })
    .unwrap();
    assert_eq!(done.written.len(), 1);
    assert_eq!(
        std::fs::read(dir.0.join("out").join("a.esp")).unwrap(),
        b"plugin"
    );
    let before_first: Vec<&Progress> = seen.iter().filter(|p| p.files == 0).collect();
    assert!(before_first.len() >= 3, "{seen:?}");
    assert!(before_first.iter().any(|p| p.read >= 2 << 20), "{seen:?}");
    let decode = Limits {
        decode: 1 << 20,
        ..Limits::default()
    };
    let error = extract(&solid, &wanted, &dir.0.join("out2"), decode).unwrap_err();
    assert_eq!(error.problem, Problem::TooBig, "{error}");
    assert!(error.detail.contains("to decode"), "{error}");
    assert!(!dir.0.join("out2").exists());
}

/// The destination folder removed while unpacking (another window clearing it): never made again;
/// the unpacking stops with an error, and nothing is left where it was.
#[test]
fn a_destination_removed_while_unpacking_is_never_made_again() {
    let dir = Scratch::new("root-gone");
    let files: &[(&str, &[u8])] = &[
        ("Data/Textures/a.dds", b"a"),
        ("Data/Textures/b.dds", b"b"),
        ("Data/Meshes/c.nif", b"c"),
        ("Data/d.esp", b"d"),
    ];
    let zip = dir.0.join("m.zip");
    let sevenz = dir.0.join("m.7z");
    make_zip(&zip, files, zip::CompressionMethod::Stored);
    make_7z(&sevenz, files);
    let wanted: Vec<(String, String)> = files
        .iter()
        .map(|(n, _)| (n.to_string(), n.trim_start_matches("Data/").to_owned()))
        .collect();
    for (i, archive) in [zip, sevenz].iter().enumerate() {
        let to = dir.0.join(format!("out{i}"));
        std::fs::create_dir_all(&to).unwrap();
        let error = extract_with(archive, &wanted, &to, Limits::default(), &mut |p| {
            if p.files == 1 {
                let _ = std::fs::remove_dir_all(&to);
            }
        })
        .unwrap_err();
        assert!(!to.exists(), "{error}");
    }
}

/// What each way of syncing the unpacked files costs, on scratch files only: about 2,000 small
/// files and a few large ones, unpacked with each [`SyncMode`] (ABCDDCBA, three rounds). Run by
/// hand:
/// `cargo test -p mod-archive syncing_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn syncing_cost() {
    let dir = Scratch::new("sync-cost");
    let mut files: Vec<(String, Vec<u8>)> = (0..2000u32)
        .map(|i| {
            let size = 1024 + (i as usize * 37) % 15_000;
            (
                format!("Data/Textures/set{}/t{i}.dds", i % 20),
                (0..size).map(|b| (b as u32 ^ i) as u8).collect(),
            )
        })
        .collect();
    for i in 0..3u32 {
        files.push((
            format!("Data/big{i}.bsa"),
            (0..(64usize << 20))
                .map(|b| (b as u32 >> 3 ^ i) as u8)
                .collect(),
        ));
    }
    let named: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), &d[..])).collect();
    let archive = dir.0.join("cost.zip");
    make_zip(&archive, &named, zip::CompressionMethod::Stored);
    let wanted: Vec<(String, String)> = files
        .iter()
        .map(|(n, _)| (n.clone(), n.trim_start_matches("Data/").to_owned()))
        .collect();
    let bytes: usize = files.iter().map(|(_, d)| d.len()).sum();
    println!("{} files, {} MB", files.len(), bytes >> 20);
    let modes = [
        SyncMode::Off,
        SyncMode::EachFile,
        SyncMode::PassAll,
        SyncMode::PassBigAndPlugins,
    ];
    let mut times = vec![Vec::new(); modes.len()];
    for round in 0..3 {
        // Each mode twice a round, the second half in the reverse order (ABCDDCBA).
        let order: Vec<usize> = (0..modes.len()).chain((0..modes.len()).rev()).collect();
        for (n, &m) in order.iter().enumerate() {
            SYNC_MODE.with(|s| s.set(modes[m]));
            let to = dir.0.join(format!("out-{round}-{n}"));
            let _ = std::fs::remove_dir_all(&to);
            let started = std::time::Instant::now();
            extract(&archive, &wanted, &to, Limits::default()).unwrap();
            let took = started.elapsed().as_secs_f64();
            times[m].push(took);
            println!("round {round} {:?}: {took:.2} s", modes[m]);
            std::fs::remove_dir_all(&to).unwrap();
        }
    }
    SYNC_MODE.with(|s| s.set(SyncMode::USED));
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let base = mean(&times[0]);
    for (m, mode) in modes.iter().enumerate() {
        let t = mean(&times[m]);
        println!(
            "{mode:?}: mean {t:.2} s, {:+.0}% against Off ({:?})",
            (t / base - 1.0) * 100.0,
            times[m]
                .iter()
                .map(|t| format!("{t:.2}"))
                .collect::<Vec<_>>()
        );
    }
}

/// Which files the end-of-unpacking pass syncs: every one on Linux and macOS; on Windows the
/// plugins and archives whatever their size (any case) and every file of 1 MB and more.
#[test]
fn the_sync_pass_takes_plugins_and_big_files() {
    let dir = Scratch::new("sync-pick");
    let write = |name: &str, size: usize| {
        let path = dir.0.join(name);
        std::fs::write(&path, vec![1u8; size]).unwrap();
        path
    };
    let plugin = write("Small.ESP", 10);
    let master = write("m.esm", 10);
    let archive = write("a.Bsa", 10);
    let texture = write("t.dds", 10);
    let big = write("big.dds", SYNC_BIG as usize);
    let some = SyncMode::PassBigAndPlugins;
    for p in [&plugin, &master, &archive, &big] {
        assert!(pass_syncs(some, p), "{}", p.display());
    }
    assert!(!pass_syncs(some, &texture));
    assert!(pass_syncs(SyncMode::PassAll, &texture));
    assert!(!pass_syncs(SyncMode::Off, &plugin) && !pass_syncs(SyncMode::EachFile, &plugin));
    let used_on_this_system = if cfg!(windows) {
        SyncMode::PassBigAndPlugins
    } else {
        SyncMode::PassAll
    };
    assert_eq!(SyncMode::USED, used_on_this_system);
}
