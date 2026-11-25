//! The differential test of every typed function of the archives family (`engine_typed::archives`)
//! against its translation: here because the translations live in the part crates, and this crate
//! registers them all. Each test builds the archive list in guest memory (its first-node word at
//! `011f8160`, `(archive, next)` nodes, archives with their reference counts, type bits and
//! `BSArchive` directory tables, each directory's block of 16-byte file entries) and asks which
//! archive owns a file entry. With `NV_DATA` set, the opt-in test lays out the directory tables
//! of every BSA in that Data folder (the game's and any mod's), in name order.
//!
//! What stands in: the archive list's critical section and the interlocked imports (each logging
//! its word; the counts are kept in guest memory as the imports keep them) and an archive's
//! `DeleteThis` (logging `this`). Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_core::typed::MAIN_THREAD;
use engine_typed::archives::{self, lookup};

/// The archive list's first-node word (its critical section, `011f8170`, is on the same page).
const LIST: u32 = 0x011f_8160;
/// The imports' slots.
const ENTER: u32 = 0x00fd_f05c;
const LEAVE: u32 = 0x00fd_f100;
const INCREMENT: u32 = 0x00fd_f1c0;
const DECREMENT: u32 = 0x00fd_f1c4;
/// `DeleteThis`, where the archives' vtables point (past `.text`).
const DELETE_THIS: u32 = 0x00ff_1000;
/// The log the stand-ins write: a count, then `(tag, word)` pairs.
const LOG: u32 = 0x0300_0000;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

/// The exe's data page and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x1000);
    e.map(LIST & !0xfff, 0x1000);
    e.register(ENTER, |e, a| {
        log(e, 1, a[0]);
        ret(0)
    });
    e.register(LEAVE, |e, a| {
        log(e, 2, a[0]);
        ret(0)
    });
    e.register(INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        log(e, 3, a[0]);
        ret(n)
    });
    e.register(DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        log(e, 4, a[0]);
        ret(n)
    });
    e.register(DELETE_THIS, |e, a| {
        log(e, 5, a[0]);
        ret(0)
    });
}

/// One archive: its type bits, its reference count and its directories' file counts. Answers the
/// archive and each directory's first file entry. With `entries`, the file entries are one block
/// there, each directory's after the one before, as a real table's are, with no memory behind them:
/// the walk only compares addresses with each directory's range. Without, each directory's entries
/// are a block of their own (so one past a directory's end is in no directory).
fn archive(
    e: &mut Engine,
    types: u16,
    refs: u32,
    counts: &[u32],
    entries: Option<u32>,
) -> (u32, Vec<u32>) {
    let a = e.mem.alloc(0x1d0);
    let vtable = e.mem.alloc(8);
    e.mem.set_u32(vtable + 4, DELETE_THIS);
    e.mem.set_u32(a + 0x158, vtable);
    e.mem.set_u32(a + 0x15c, refs);
    e.mem.set_u16(a + 0x180, types);
    let n = counts.len() as u32;
    let table = e.mem.alloc((16 * n).max(16));
    e.mem.set_u32(a + 0x170, n);
    e.mem.set_u32(a + 0x184, table);
    let mut next = entries.unwrap_or(0);
    let mut starts = Vec::new();
    for (i, &count) in counts.iter().enumerate() {
        let block = if entries.is_some() {
            let block = next;
            next += 16 * count;
            block
        } else {
            e.mem.alloc((16 * count).max(16))
        };
        let directory = table + 16 * i as u32;
        e.mem.set_u32(directory, 0x1000 + i as u32);
        e.mem.set_u32(directory + 8, count);
        e.mem.set_u32(directory + 0xc, block);
        starts.push(block);
    }
    (a, starts)
}

/// The list of `archives`, in order (a null archive where one is 0).
fn list(e: &mut Engine, archives: &[u32]) {
    let mut next = 0;
    for &a in archives.iter().rev() {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, a);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    e.mem.set_u32(LIST, next);
}

/// The test world: five archives (meshes, textures, a second meshes one, one whose count is 0, so
/// asking it deletes it, and one with no type bits, which a walk over any type finds only by its test
/// of the mask itself) and a null node; answers the file entries `WHICH` names.
fn world(e: &mut Engine) -> Vec<u32> {
    stand_ins(e);
    let (meshes, m) = archive(e, 1, 1, &[3, 1, 0, 5], None);
    let (textures, t) = archive(e, 2, 1, &[2, 4], None);
    let (meshes2, m2) = archive(e, 1, 2, &[1], None);
    let (dying, d) = archive(e, 4, 0, &[2], None);
    let (untyped, u) = archive(e, 0, 1, &[2], None);
    list(e, &[meshes, 0, textures, dying, meshes2, untyped]);
    vec![
        m[0],                 // the first archive's first entry
        m[0] + 2 * 16,        // its last entry of the first directory
        m[0] + 3 * 16,        // one past it: the end is not inside
        m[0].wrapping_sub(1), // one before the start
        m[2],                 // a directory with no files: never inside
        m[3] + 4 * 16 + 15,   // the last byte of the fourth directory's last entry
        t[1] + 16,            // the second archive, second directory
        m2[0],                // the third
        d[0] + 16,            // the archive that is deleted when asked
        0,                    // null
        0x7fff_0000,          // in no archive
        u[0] + 16,            // the archive with no type bits
    ]
}

/// Which file entry ([`world`]'s list).
const WHICH: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
/// The type masks: any, each type, none, two types and a mask with bits past 16.
const TYPES: &[u32] = &[0xffff, 1, 2, 4, 0, 3, 0x1_0002];

/// `00af6910` over the list: every entry under every mask, the list's lock, the references and the
/// deletion included.
#[test]
fn owner_of() {
    Diff::new(entry!(0x00af_6910, lookup::owner_of(Ptr, u32) -> u32))
        .family(&archives::ARCHIVES)
        .inputs(&[Input::Of(WHICH), Input::Of(TYPES)])
        .setup(|e, case| {
            let entries = world(e);
            vec![entries[case.word(0) as usize], case.word(1)]
        })
        .run()
        .assert_same();
}

/// `00afdc00` on each archive of the list's world, for every entry.
#[test]
fn archive_holds() {
    Diff::new(entry!(0x00af_dc00, lookup::archive_holds(Ptr, u32) -> bool))
        .family(&archives::ARCHIVES)
        .inputs(&[Input::Of(WHICH), Input::Of(&[0, 2, 3, 4, 5])])
        .setup(|e, case| {
            let entries = world(e);
            let mut node = e.mem.u32(LIST);
            for _ in 0..case.word(1) {
                node = e.mem.u32(node + 4);
            }
            let archive = e.mem.u32(node);
            vec![archive + 0x160, entries[case.word(0) as usize]]
        })
        .run()
        .assert_same();
}

/// `00afdbc0` around a directory's block: below, at, inside, at the end and past it, with counts
/// that make the end wrap.
#[test]
fn directory_holds() {
    const STARTS: &[u32] = &[0x0010_0000, 0xffff_ff00, 0];
    const COUNTS: &[u32] = &[0, 1, 3, 0x1000_0000, 0x0fff_fff0];
    const OFFSETS: &[u32] = &[0xffff_ffff, 0, 15, 16, 47, 48, 0x100, 0x8000_0000];
    Diff::new(entry!(0x00af_dbc0, lookup::directory_holds(u32, u32) -> bool))
        .family(&archives::ARCHIVES)
        .inputs(&[Input::Of(STARTS), Input::Of(COUNTS), Input::Of(OFFSETS)])
        .setup(|e, case| {
            stand_ins(e);
            let directory = e.mem.alloc(16);
            e.mem.set_u32(directory + 8, case.word(1));
            e.mem.set_u32(directory + 0xc, case.word(0));
            vec![directory, case.word(0).wrapping_add(case.word(2))]
        })
        .run()
        .assert_same();
}

/// The directory tables of every BSA in `NV_DATA`: each archive's folder records, version 104 (an
/// 8-byte name hash, a file count, an offset), with its type bits (the header's file flags). Only the
/// 36-byte header and the folder records are read.
fn real_archives(data: &std::path::Path) -> Vec<(String, u16, Vec<u32>)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut names: Vec<std::path::PathBuf> = std::fs::read_dir(data)
        .expect("NV_DATA")
        .filter_map(|d| d.ok().map(|d| d.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bsa")))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for path in names {
        let mut file = std::fs::File::open(&path).expect("a BSA");
        let mut header = [0u8; 36];
        if file.read_exact(&mut header).is_err() {
            continue;
        }
        let word =
            |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
        if header[0..4] != [b'B', b'S', b'A', 0] || word(&header, 4) != 104 {
            continue;
        }
        let (records, folders) = (word(&header, 8), word(&header, 16) as usize);
        let types = u16::from_le_bytes([header[32], header[33]]);
        let mut table = vec![0u8; 16 * folders];
        file.seek(SeekFrom::Start(u64::from(records)))
            .expect("the folder records");
        file.read_exact(&mut table).expect("the folder records");
        let counts = (0..folders).map(|i| word(&table, 16 * i + 8)).collect();
        let name = path
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into());
        out.push((name, types, counts));
    }
    out
}

/// [`owner_of_real_archives`]'s world: every archive of `all` in one list, each one's file entries 16
/// MB apart above the memory the test maps. Answers, for each archive, its first directory's first
/// entry, its middle one's last entry and the end of its last one, then an entry in no archive.
fn real_world(e: &mut Engine, all: &[(String, u16, Vec<u32>)]) -> Vec<u32> {
    stand_ins(e);
    let mut archives = Vec::new();
    let mut entries = Vec::new();
    for (i, (_, types, counts)) in all.iter().enumerate() {
        let entries_at = 0x4000_0000 + 0x0100_0000 * i as u32;
        let (a, starts) = archive(e, *types, 1, counts, Some(entries_at));
        archives.push(a);
        let mid = counts.len() / 2;
        entries.push(starts.first().copied().unwrap_or(0));
        entries.push(
            starts
                .get(mid)
                .map_or(0, |&s| s + 16 * counts[mid].saturating_sub(1)),
        );
        entries.push(
            starts
                .last()
                .map_or(0, |&s| s + 16 * counts[counts.len() - 1]),
        );
    }
    entries.push(0x7fff_0000);
    list(e, &archives);
    entries
}

/// Opt-in (`NV_DATA`): every BSA's real directory table in one list, name order. Under any type, for
/// each archive its first directory's first entry, its middle one's last entry and the end of its
/// last one (in no archive: the whole list walked); then the entry in no archive under the meshes
/// bit and 0x10 (both passes walked). One control word: the walk is integers only, and the small
/// tests run both. The harness hashes all mapped memory at each call, and a whole walk makes one
/// call a directory, so the cases are few and each group's time is printed.
#[test]
#[ignore = "needs a Data folder with BSAs (set NV_DATA)"]
fn owner_of_real_archives() {
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let all: &'static [(String, u16, Vec<u32>)] = real_archives(&data).leak();
    assert!(all.len() >= 2, "at least two BSAs in {}", data.display());
    eprintln!(
        "{} archives, {} directories",
        all.len(),
        all.iter().map(|a| a.2.len()).sum::<usize>()
    );
    let n = all.len();
    let none = 3 * n;
    let groups: [(&str, Vec<(usize, u32)>); 4] = [
        ("first entries", (0..n).map(|i| (3 * i, 0xffff)).collect()),
        (
            "middle entries",
            (0..n).map(|i| (3 * i + 1, 0xffff)).collect(),
        ),
        ("ends", (0..n).map(|i| (3 * i + 2, 0xffff)).collect()),
        ("in no archive", vec![(none, 1), (none, 0x10)]),
    ];
    for (name, picked) in groups {
        let start = std::time::Instant::now();
        let picked: &'static [(usize, u32)] = picked.leak();
        let indices: &'static [u32] = (0..picked.len() as u32).collect::<Vec<_>>().leak();
        Diff::new(entry!(0x00af_6910, lookup::owner_of(Ptr, u32) -> u32))
            .family(&archives::ARCHIVES)
            .control_words(&[MAIN_THREAD])
            .inputs(&[Input::Of(indices)])
            .max_cases(indices.len())
            .setup(move |e, case| {
                let entries = real_world(e, all);
                let (which, types) = picked[case.word(0) as usize];
                vec![entries[which], types]
            })
            .run()
            .assert_same();
        eprintln!("{name}: {} cases in {:?}", picked.len(), start.elapsed());
    }
}
