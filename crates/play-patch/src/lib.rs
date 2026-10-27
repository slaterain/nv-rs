//! The play loop's patch crate.
//!
//! A stop in a play-build run is usually one function translated wrong or not at all. Fixing it
//! in its home file rebuilds that engine crate and everything after it (engine-fallout at
//! opt-level 3 takes many minutes). A fix written here instead rebuilds only this crate, the
//! host and the link: [`funcs`] is registered over the engine's own table at start-up
//! (`host::Host::start`, right after the exe is mapped), so an entry here replaces the one with
//! the same address.
//!
//! Rules: the same as any engine file (translated from the exe, address marked, tests in this
//! file). A patch reaches only calls made through `Engine::call`: a call made directly within
//! its home unit file still runs the old code, so patch that caller too when it matters. The
//! next gated collection moves every entry to its home file (replacing the old translation)
//! and empties [`funcs`]; main never carries patches.

#[allow(unused_imports)]
use engine::prelude::*;
use engine::{args, entry};

/// `SetLastError`'s import slot (`00fdf1dc`), for the platform imports patched here.
const SET_LAST_ERROR: u32 = 0x00fd_f1dc;
/// The path separator ``.
const SEP: u8 = 0x5c;
/// `ERROR_INVALID_NAME`.
const ERROR_INVALID_NAME: u32 = 123;

/// The parts of a path after its root, `.` and `..` resolved, empty parts (repeated separators)
/// dropped; `None` for a shape not measured (a part other than `.`/`..` that ends in a dot or a
/// space before the last part).
fn resolve_parts(parts: Vec<&[u8]>) -> Option<Vec<Vec<u8>>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let count = parts.len();
    for (index, part) in parts.into_iter().enumerate() {
        match part {
            b"" | b"." => {}
            b".." => {
                out.pop();
            }
            _ => {
                let last = index + 1 == count;
                let mut part = part.to_vec();
                if last {
                    while matches!(part.last(), Some(b'.') | Some(b' ')) {
                        part.pop();
                    }
                } else if matches!(part.last(), Some(b'.') | Some(b' ')) {
                    return None;
                }
                if !part.is_empty() {
                    out.push(part);
                }
            }
        }
    }
    Some(out)
}

// Platform: replaces KERNEL32.DLL GetFullPathNameA (import slot 00fdf194)
/// `GetFullPathNameA(name, size, buffer, file part)` as measured on this machine
/// (`build43-scratch/measure/fullpath`, 2026-10-27): a relative name is joined to the current
/// directory (`GetCurrentDirectoryA`'s), a name starting with a separator to its drive, and a
/// drive-relative name of the current drive (`C:x`) to the current directory; `/` reads as `\`,
/// repeated separators collapse, `.` parts go and `..` goes up (never past the root), and the
/// last part loses its trailing dots and spaces. A name that ends in a separator keeps it and
/// has no file part (null); otherwise the file part points at the last part. The answer is the
/// length; with too small a buffer, the length with its terminator, the buffer untouched. The
/// last error is left as it was, except an empty name: 0 and `ERROR_INVALID_NAME`. Other drives,
/// UNC and device names, and trailing dots or spaces inside the path, are not provided.
pub fn get_full_path_name_a(
    e: &mut Engine,
    name: Ptr,
    size: u32,
    buffer: Ptr,
    part_out: Ptr,
) -> u32 {
    let text = e.mem.cstr(name.addr());
    if text.is_empty() {
        e.call(SET_LAST_ERROR, &args![ERROR_INVALID_NAME]);
        return 0;
    }
    let text: Vec<u8> = text
        .iter()
        .map(|&c| if c == b'/' { SEP } else { c })
        .collect();
    let current = e.kernel.current_directory.clone();
    let drive = current[..2].to_vec();
    assert!(
        drive[1] == b':' && !text.starts_with(br"\\"),
        "GetFullPathNameA({}): not provided (UNC or device name)",
        String::from_utf8_lossy(&text)
    );
    let (root, rest): (Vec<u8>, Vec<u8>) = if text.len() >= 2 && text[1] == b':' {
        assert!(
            text[0].eq_ignore_ascii_case(&drive[0]),
            "GetFullPathNameA({}): another drive is not provided",
            String::from_utf8_lossy(&text)
        );
        if text.get(2) == Some(&SEP) {
            (text[..2].to_vec(), text[2..].to_vec())
        } else {
            let mut joined = current[2..].to_vec();
            joined.push(SEP);
            joined.extend_from_slice(&text[2..]);
            (drive.clone(), joined)
        }
    } else if text[0] == SEP {
        (drive.clone(), text.clone())
    } else {
        let mut joined = current[2..].to_vec();
        joined.push(SEP);
        joined.extend_from_slice(&text);
        (drive.clone(), joined)
    };
    let ends_with_separator = text.last() == Some(&SEP);
    let parts: Vec<&[u8]> = rest.split(|&c| c == SEP).collect();
    let resolved = resolve_parts(parts).unwrap_or_else(|| {
        panic!(
            "GetFullPathNameA({}): a part ending in a dot or space is not provided",
            String::from_utf8_lossy(&text)
        )
    });
    let mut full = root;
    let mut file_part = 0usize;
    for part in &resolved {
        full.push(SEP);
        file_part = full.len();
        full.extend_from_slice(part);
    }
    if resolved.is_empty() || ends_with_separator {
        full.push(SEP);
    }
    let length = full.len() as u32;
    if size <= length {
        return length + 1;
    }
    e.mem.set_cstr(buffer.addr(), &full);
    if !part_out.is_null() {
        let part = if ends_with_separator || resolved.is_empty() {
            0
        } else {
            buffer.addr() + file_part as u32
        };
        e.mem.set_u32(part_out.addr(), part);
    }
    length
}

/// The patched functions, by exe address. Empty between play loops.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00fd_f194, get_full_path_name_a(Ptr, u32, Ptr, Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(e: &mut Engine, name: &str, size: u32) -> (u32, String, u32) {
        let at = e.mem.alloc(0x400);
        e.mem.set_cstr(at, name.as_bytes());
        let buffer = at + 0x100;
        let part = at + 0x300;
        e.mem.set_u32(part, 0xcccc_cccc);
        let n = get_full_path_name_a(e, Ptr::new(at), size, Ptr::new(buffer), Ptr::new(part));
        let text = String::from_utf8(e.mem.cstr(buffer).to_vec()).unwrap_or_default();
        let p = e.mem.u32(part);
        (
            n,
            text,
            if p == 0 || p == 0xcccc_cccc {
                p
            } else {
                p - buffer
            },
        )
    }

    /// The measured answers (`build43-scratch/measure/fullpath`, current directory `C:\G\F`).
    #[test]
    fn full_path_names_follow_the_measured_rules() {
        let mut e = Engine::new();
        e.register(SET_LAST_ERROR, |_, _| Ret::default());
        e.kernel.current_directory = br"C:\G\F".to_vec();
        assert_eq!(
            full(&mut e, r"Data\Video\FNVIntro.bik", 260),
            (30, r"C:\G\F\Data\Video\FNVIntro.bik".into(), 18)
        );
        assert_eq!(
            full(&mut e, "Data/Video/a.bik", 260).1,
            r"C:\G\F\Data\Video\a.bik"
        );
        assert_eq!(full(&mut e, r"..\x\y", 260).1, r"C:\G\x\y");
        assert_eq!(full(&mut e, r"\root\x", 260), (9, r"C:\root\x".into(), 8));
        assert_eq!(full(&mut e, "C:x", 260).1, r"C:\G\F\x");
        assert_eq!(full(&mut e, r"a\b", 260).1, r"C:\G\F\a\b");
        assert_eq!(full(&mut e, "x. ", 260).1, r"C:\G\F\x");
        assert_eq!(
            full(&mut e, r"Data\Video\", 260),
            (18, r"C:\G\F\Data\Video\".into(), 0)
        );
        assert_eq!(full(&mut e, r"C:\a\..\..\..\b", 260).1, r"C:\b");
        assert_eq!(
            full(&mut e, r"C:\a\b", 4).0,
            7,
            "too small: the length with its terminator"
        );
        assert_eq!(full(&mut e, "", 260).0, 0);
    }
}
