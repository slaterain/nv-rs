//! zlib's inflate on the host (on unless `NV_NATIVE_INFLATE=0`): FalloutNV.exe's own zlib 1.2.1
//! (`00b43fe0` `inflateInit_`, `00b44000` `inflate`, `00b45db0` `inflateEnd`) decodes the
//! compressed BSA files (`CompressedArchiveFile`, `00afc0e0`) and records (`DecompressCurrentForm`);
//! the translation of its decoding loop (`00b49070`, zlib's `inflate_fast`) was about half of the
//! IO thread's time in a cell load (measure/stutter/REAL-THREADS.md). Here `inflate` runs on
//! zlib-rs (a Rust zlib), one decoder per stream, with the stream's fields (`next_in` +0,
//! `avail_in` +4, `total_in` +8, `next_out` +0xc, `avail_out` +0x10, `total_out` +0x14) moved as
//! zlib moves them and zlib's answers: the bytes out are the format's, the same as the exe's.
//! The exe's zlib reads the trailer's checksum but never compares it, and leaves the stream's
//! checksum word (+0x30) 0 once output was counted: a trailer that does not match ends the stream
//! here too, and the word is left 0. `inflateInit_` and `inflateEnd` still run their translations
//! (the stream's own state block stays what the exe makes) and start or drop the host decoder.

use engine::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use zlib_rs::{Inflate, InflateError, InflateFlush, Status};

const INFLATE_INIT: u32 = 0x00b4_3fe0;
const INFLATE: u32 = 0x00b4_4000;
const INFLATE_END: u32 = 0x00b4_5db0;

const NEXT_IN: u32 = 0x0;
const AVAIL_IN: u32 = 0x4;
const TOTAL_IN: u32 = 0x8;
const NEXT_OUT: u32 = 0xc;
const AVAIL_OUT: u32 = 0x10;
const TOTAL_OUT: u32 = 0x14;
const ADLER: u32 = 0x30;

/// One stream's host decoder, with its buffers kept between calls.
struct Decoder {
    inflate: Inflate,
    /// The stream ended (a call answered 1): later calls answer 1 again, as zlib's done mode does.
    ended: bool,
    input: Vec<u8>,
    output: Vec<u8>,
}

impl Decoder {
    fn new() -> Box<Self> {
        // A zlib header, a window of up to 32 KB: the exe's `inflateInit_` (window bits 15).
        Box::new(Decoder {
            inflate: Inflate::new(true, 15),
            ended: false,
            input: Vec::new(),
            output: Vec::new(),
        })
    }
}

/// Decoders by (kernel table, stream).
type Decoders = HashMap<(usize, u32), Box<Decoder>>;

/// The run's decoders by (kernel table, stream): every thread's engine context of a run shares them.
fn decoders() -> std::sync::MutexGuard<'static, Decoders> {
    static DECODERS: std::sync::OnceLock<Mutex<Decoders>> = std::sync::OnceLock::new();
    DECODERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether the host decoder is on: unless `NV_NATIVE_INFLATE=0` (the exe's zlib, its translation,
/// for a comparison). The bytes out are the same (`the_host_decoder_matches_the_exes_zlib_over_the_bsas`).
pub fn wanted() -> bool {
    std::env::var("NV_NATIVE_INFLATE").map_or(true, |v| v.trim() != "0")
}

/// Registers the three entry points over their translations.
pub fn install(e: &mut Engine) {
    e.register(INFLATE_INIT, inflate_init_entry);
    e.register(INFLATE, inflate_entry);
    e.register(INFLATE_END, inflate_end_entry);
}

fn inflate_init_entry(e: &mut Engine, a: &[u32]) -> Ret {
    let ret = engine::units::unplaced::run_00b43db0::fn_00b43fe0(e, a[0], a[1], a[2]);
    if ret == 0 {
        decoders().insert((e.kernel.id(), a[0]), Decoder::new());
    }
    Ret {
        eax: ret as u32,
        ..Ret::default()
    }
}

fn inflate_end_entry(e: &mut Engine, a: &[u32]) -> Ret {
    decoders().remove(&(e.kernel.id(), a[0]));
    let ret = engine::units::unplaced::run_00b45c00::fn_00b45db0(e, a[0]);
    Ret {
        eax: ret as u32,
        ..Ret::default()
    }
}

fn inflate_entry(e: &mut Engine, a: &[u32]) -> Ret {
    let (stream, flush) = (a[0], a[1] as i32);
    Ret {
        eax: inflate(e, stream, flush) as u32,
        ..Ret::default()
    }
}

/// `inflate(stream, flush)`: the stream's decoder is taken out of the run's table while it works,
/// so two threads' streams decode at the same time. A stream the host did not start (its
/// `inflateInit_` ran before the host decoder was installed) keeps the translation.
fn inflate(e: &mut Engine, stream: u32, flush: i32) -> i32 {
    let key = (e.kernel.id(), stream);
    let Some(mut d) = decoders().remove(&key) else {
        return engine::units::unplaced::run_00b45c00::inflate(e, stream, flush);
    };
    let answer = decode(e, stream, flush, &mut d);
    decoders().insert(key, d);
    answer
}

fn decode(e: &mut Engine, stream: u32, flush: i32, d: &mut Decoder) -> i32 {
    // zlib's stream checks: no output, or input missing that is said to be there.
    if stream == 0
        || e.mem.u32(stream + NEXT_OUT) == 0
        || (e.mem.u32(stream + NEXT_IN) == 0 && e.mem.u32(stream + AVAIL_IN) != 0)
    {
        return -2;
    }
    if d.ended {
        return 1;
    }
    let (next_in, avail_in) = (e.mem.u32(stream + NEXT_IN), e.mem.u32(stream + AVAIL_IN));
    let (next_out, avail_out) = (e.mem.u32(stream + NEXT_OUT), e.mem.u32(stream + AVAIL_OUT));
    if d.input.len() < avail_in as usize {
        d.input.resize(avail_in as usize, 0);
    }
    if d.output.len() < avail_out as usize {
        d.output.resize(avail_out as usize, 0);
    }
    let input = &mut d.input[..avail_in as usize];
    e.mem.read(next_in, input);
    let flush = match flush {
        4 => InflateFlush::Finish,
        2 | 3 => InflateFlush::SyncFlush,
        5 => InflateFlush::Block,
        _ => InflateFlush::NoFlush,
    };
    let (in_before, out_before) = (d.inflate.total_in(), d.inflate.total_out());
    let result = d
        .inflate
        .decompress(input, &mut d.output[..avail_out as usize], flush);
    let read = (d.inflate.total_in() - in_before) as u32;
    let written = (d.inflate.total_out() - out_before) as u32;
    e.mem.write(next_out, &d.output[..written as usize]);
    e.mem.set_u32(stream + NEXT_IN, next_in.wrapping_add(read));
    e.mem.set_u32(stream + AVAIL_IN, avail_in - read);
    e.mem.set_u32(
        stream + TOTAL_IN,
        e.mem.u32(stream + TOTAL_IN).wrapping_add(read),
    );
    e.mem
        .set_u32(stream + NEXT_OUT, next_out.wrapping_add(written));
    e.mem.set_u32(stream + AVAIL_OUT, avail_out - written);
    e.mem.set_u32(
        stream + TOTAL_OUT,
        e.mem.u32(stream + TOTAL_OUT).wrapping_add(written),
    );
    e.mem.set_u32(stream + ADLER, 0);
    match result {
        Ok(Status::Ok) => 0,
        Ok(Status::BufError) => -5,
        Ok(Status::StreamEnd) => {
            d.ended = true;
            1
        }
        // The exe's zlib reads the trailer's checksum and ends without comparing it.
        Err(InflateError::DataError)
            if d.inflate.error_message() == Some("incorrect data check") =>
        {
            d.ended = true;
            1
        }
        Err(InflateError::NeedDict { .. }) => 2,
        Err(InflateError::DataError) => -3,
        Err(InflateError::MemError) => -4,
        Err(InflateError::StreamError) => -2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream block as the archive makes it: the 0x38-byte stream, its state block after it.
    fn stream_block(e: &mut Engine) -> u32 {
        let block = e.mem.alloc(0x1be0);
        e.mem.set_u32(block + 0x1c, block + 0x38);
        decoders().insert((e.kernel.id(), block), Decoder::new());
        block
    }

    /// `data` as a zlib stream (level 6).
    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; zlib_rs::compress_bound(data.len())];
        let (packed, code) =
            zlib_rs::compress_slice(&mut out, data, zlib_rs::DeflateConfig::new(6));
        assert_eq!(code, zlib_rs::ReturnCode::Ok);
        packed.to_vec()
    }

    /// A trailer whose checksum does not match ends the stream with every byte out, as the exe's
    /// zlib (it reads the checksum and never compares it), and later calls answer the end again.
    #[test]
    fn a_wrong_checksum_still_ends_the_stream() {
        let mut e = Engine::new();
        let data: Vec<u8> = (0..5000u32).map(|i| (i % 13) as u8).collect();
        let mut packed = zlib(&data);
        let last = packed.len() - 1;
        packed[last] ^= 0x55;
        let stream = stream_block(&mut e);
        let input = e.mem.alloc(packed.len() as u32);
        e.mem.write(input, &packed);
        let output = e.mem.alloc(8192);
        e.mem.set_u32(stream + NEXT_IN, input);
        e.mem.set_u32(stream + AVAIL_IN, packed.len() as u32);
        e.mem.set_u32(stream + NEXT_OUT, output);
        e.mem.set_u32(stream + AVAIL_OUT, 8192);
        assert_eq!(inflate(&mut e, stream, 2), 1);
        assert_eq!(e.mem.bytes(output, data.len() as u32), data);
        assert_eq!(e.mem.u32(stream + AVAIL_IN), 0);
        assert_eq!(inflate(&mut e, stream, 2), 1);
        decoders().remove(&(e.kernel.id(), stream));
    }

    /// A zlib stream fed in 300-byte pieces into a 1000-byte output, as the archive's reads do:
    /// every byte comes out, the fields move as zlib moves them, the end answers 1 and the
    /// checksum word stays 0.
    #[test]
    fn a_stream_in_pieces_decodes_whole_with_zlibs_answers() {
        let mut e = Engine::new();
        let data: Vec<u8> = (0..20_000u32)
            .map(|i| (i * 7 % 251) as u8 ^ (i >> 9) as u8)
            .collect();
        let packed = zlib(&data);
        let stream = stream_block(&mut e);
        let input = e.mem.alloc(packed.len() as u32);
        e.mem.write(input, &packed);
        let output = e.mem.alloc(1000);
        let mut out = Vec::new();
        let mut fed = 0u32;
        e.mem.set_u32(stream + ADLER, 0x1234_5678);
        loop {
            if e.mem.u32(stream + AVAIL_IN) == 0 && (fed as usize) < packed.len() {
                let piece = 300.min(packed.len() as u32 - fed);
                e.mem.set_u32(stream + NEXT_IN, input + fed);
                e.mem.set_u32(stream + AVAIL_IN, piece);
                fed += piece;
            }
            e.mem.set_u32(stream + NEXT_OUT, output);
            e.mem.set_u32(stream + AVAIL_OUT, 1000);
            let answer = inflate(&mut e, stream, 2);
            let made = 1000 - e.mem.u32(stream + AVAIL_OUT);
            out.extend(e.mem.bytes(output, made));
            assert_eq!(e.mem.u32(stream + ADLER), 0);
            assert!(matches!(answer, 0 | 1 | -5), "answer {answer}");
            if answer == 1 {
                break;
            }
        }
        assert_eq!(out, data);
        assert_eq!(e.mem.u32(stream + TOTAL_OUT), data.len() as u32);
        assert_eq!(e.mem.u32(stream + TOTAL_IN), packed.len() as u32);
        decoders().remove(&(e.kernel.id(), stream));
    }

    /// Bad data is zlib's data error (-3).
    #[test]
    fn bad_data_is_a_data_error() {
        let mut e = Engine::new();
        let stream = stream_block(&mut e);
        let input = e.mem.alloc(8);
        e.mem
            .write(input, &[0x78, 0x9c, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        let output = e.mem.alloc(64);
        e.mem.set_u32(stream + NEXT_IN, input);
        e.mem.set_u32(stream + AVAIL_IN, 8);
        e.mem.set_u32(stream + NEXT_OUT, output);
        e.mem.set_u32(stream + AVAIL_OUT, 64);
        assert_eq!(inflate(&mut e, stream, 2), -3);
        decoders().remove(&(e.kernel.id(), stream));
    }
    const TEST_ZALLOC: u32 = 0x7d01_0000;
    const TEST_ZFREE: u32 = 0x7d01_0010;

    /// One compressed file through a fresh stream, fed as the archive feeds it (64 KB of input at
    /// a time, 256 KB of output), by the host decoder or by the exe's zlib (the translation, which
    /// this engine keeps registered): the bytes out and the last answer.
    fn decode(e: &mut Engine, packed: &[u8], expected: u32, host: bool) -> (Vec<u8>, i32) {
        const VERSION: u32 = 0x010a_4590; // "1.2.1", as `CompressedArchiveFile` passes it.
        const PIECE: u32 = 0x1_0000;
        const ROOM: u32 = 0x4_0000;
        let stream = e.mem.alloc(0x1be0);
        for at in (0..0x38).step_by(4) {
            e.mem.set_u32(stream + at, 0);
        }
        e.mem.set_u32(stream + 0x1c, stream + 0x38);
        // zlib's allocator pair (the window): the test's own, since this bare engine has no C runtime heap;
        // the bytes out don't depend on it.
        e.register(TEST_ZALLOC, |e, a| Ret {
            eax: e.mem.alloc(a[1].wrapping_mul(a[2])),
            ..Ret::default()
        });
        e.register(TEST_ZFREE, |e, a| {
            e.mem.free(a[1]);
            Ret::default()
        });
        e.mem.set_u32(stream + 0x20, TEST_ZALLOC);
        e.mem.set_u32(stream + 0x24, TEST_ZFREE);
        let init = [stream, VERSION, 0x38];
        let started = if host {
            inflate_init_entry(e, &init).eax
        } else {
            e.call(INFLATE_INIT, &init).eax
        };
        assert_eq!(started, 0);
        let input = e.mem.alloc(packed.len().max(1) as u32);
        e.mem.write(input, packed);
        let output = e.mem.alloc(ROOM);
        let (mut out, mut fed) = (Vec::with_capacity(expected as usize), 0u32);
        let last = loop {
            if e.mem.u32(stream + AVAIL_IN) == 0 && fed < packed.len() as u32 {
                let piece = PIECE.min(packed.len() as u32 - fed);
                e.mem.set_u32(stream + NEXT_IN, input + fed);
                e.mem.set_u32(stream + AVAIL_IN, piece);
                fed += piece;
            }
            e.mem.set_u32(stream + NEXT_OUT, output);
            e.mem.set_u32(stream + AVAIL_OUT, ROOM);
            let answer = if host {
                inflate(e, stream, 2)
            } else {
                e.call(INFLATE, &[stream, 2]).eax as i32
            };
            let made = ROOM - e.mem.u32(stream + AVAIL_OUT);
            out.extend(e.mem.bytes(output, made));
            // As the archive's reader stops: the stream's end, the whole file out, an error, or no input left
            // and no progress.
            if answer == 1 || out.len() >= expected as usize || !(answer == 0 || answer == -5) {
                break answer;
            }
            if answer == -5 && fed >= packed.len() as u32 && made == 0 {
                break answer;
            }
        };
        if host {
            inflate_end_entry(e, &[stream]);
        } else {
            e.call(INFLATE_END, &[stream]);
        }
        for block in [stream, input, output] {
            e.mem.free(block);
        }
        (out, last)
    }

    /// With `NV_EXE` set to an installed FalloutNV.exe 1.4.0.525: the host decoder's bytes are the
    /// exe's zlib's (its translation) for the compressed files of every BSA in the Data folder and the
    /// compressed records of every plugin there, every `NV_INFLATE_EVERY`-th of each (default 25; 1
    /// for all of them), in `NV_INFLATE_PART` parts.
    #[test]
    fn the_host_decoder_matches_the_exes_zlib_over_the_bsas() {
        use std::io::{Read, Seek, SeekFrom};
        let Some(path) = std::env::var("NV_EXE").ok().filter(|p| !p.is_empty()) else {
            return;
        };
        let exe = std::path::PathBuf::from(path);
        let data = exe.parent().unwrap().join("Data");
        let every: usize = std::env::var("NV_INFLATE_EVERY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        // `NV_INFLATE_PART=K/N`: only the K-th of N parts of what `every` picks (each item's place among
        // them modulo N), so the whole set can be checked in pieces.
        let (part, parts): (usize, usize) = std::env::var("NV_INFLATE_PART")
            .ok()
            .and_then(|v| {
                let (k, n) = v.split_once('/')?;
                Some((k.trim().parse().ok()?, n.trim().parse().ok()?))
            })
            .unwrap_or((0, 1));
        let picked = |i: usize| i % every == 0 && (i / every) % parts == part;
        let mut e = Engine::with_exe(&exe).unwrap();
        let mut bsas: Vec<_> = std::fs::read_dir(&data)
            .unwrap()
            .flatten()
            .map(|d| d.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bsa")))
            .collect();
        bsas.sort();
        let (mut files, mut bytes, mut short) = (0usize, 0usize, Vec::new());
        for bsa_path in &bsas {
            let archive = bsa::Archive::open(bsa_path).unwrap();
            let named = archive.header().version == 104
                && archive
                    .header()
                    .has(bsa::archive_flags::EMBEDDED_FILE_NAMES);
            let mut raw_file = std::fs::File::open(bsa_path).unwrap();
            for (i, entry) in archive.files().iter().enumerate() {
                if !entry.compressed || !picked(i) {
                    continue;
                }
                let mut raw = vec![0u8; entry.stored_size as usize];
                raw_file
                    .seek(SeekFrom::Start(u64::from(entry.offset)))
                    .unwrap();
                raw_file.read_exact(&mut raw).unwrap();
                let mut stored = &raw[..];
                if named {
                    stored = &stored[1 + usize::from(stored[0])..];
                }
                let expected = u32::from_le_bytes([stored[0], stored[1], stored[2], stored[3]]);
                let packed = &stored[4..];
                bytes += same(&mut e, packed, expected, &entry.path, &mut short);
                files += 1;
            }
        }
        // The plugins' compressed records (`DecompressCurrentForm`): the same, every `every`-th.
        let mut plugins: Vec<_> = std::fs::read_dir(&data)
            .unwrap()
            .flatten()
            .map(|d| d.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("esm") || x.eq_ignore_ascii_case("esp"))
            })
            .collect();
        plugins.sort();
        let (mut records, mut record_bytes) = (0usize, 0usize);
        for plugin in &plugins {
            let file = std::fs::read(plugin).unwrap();
            for (i, (form, packed, expected)) in compressed_records(&file).into_iter().enumerate() {
                if !picked(i) {
                    continue;
                }
                let name = format!("{} record {form:08x}", plugin.display());
                record_bytes += same(&mut e, packed, expected, &name, &mut short);
                records += 1;
            }
        }
        eprintln!(
            "host inflate = the exe's zlib over {files} files ({bytes} bytes) of {} BSAs and \
             {records} records ({record_bytes} bytes) of {} plugins",
            bsas.len(),
            plugins.len()
        );
        // Data the exe's zlib itself does not decode whole (the host gave the same bytes and answer).
        for s in &short {
            eprintln!("  short in the exe's zlib too: {s}");
        }
        assert!(files + records > 0);
    }

    /// One compressed item through both decoders: the same bytes and the same last answer (`name`
    /// in the failure); its length, and `name` noted in `short` when the exe's zlib gave fewer bytes
    /// than the item's stated length.
    fn same(
        e: &mut Engine,
        packed: &[u8],
        expected: u32,
        name: &str,
        short: &mut Vec<String>,
    ) -> usize {
        let (guest, guest_answer) = decode(e, packed, expected, false);
        let (host, host_answer) = decode(e, packed, expected, true);
        assert_eq!(host.len(), guest.len(), "{name}: the lengths differ");
        assert!(host == guest, "{name}: the bytes differ");
        assert_eq!(host_answer, guest_answer, "{name}: the answers differ");
        if guest.len() != expected as usize {
            short.push(format!(
                "{name}: {} of {expected} bytes, answer {guest_answer}",
                guest.len()
            ));
        }
        host.len()
    }

    /// A plugin's compressed records (24-byte record and group headers; a record's flag
    /// 0x0004_0000: its data is the decoded length, then zlib): form id, the zlib bytes, the length.
    fn compressed_records(file: &[u8]) -> Vec<(u32, &[u8], u32)> {
        let word =
            |at: usize| u32::from_le_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]);
        let (mut out, mut at) = (Vec::new(), 0usize);
        while at + 24 <= file.len() {
            let size = word(at + 4) as usize;
            if &file[at..at + 4] == b"GRUP" {
                // A group's size counts its header: its records follow it.
                at += 24;
                continue;
            }
            if word(at + 8) & 0x0004_0000 != 0 && size >= 4 {
                let body = &file[at + 24..at + 24 + size];
                out.push((word(at + 12), &body[4..], word(at + 24)));
            }
            at += 24 + size;
        }
        out
    }
}
