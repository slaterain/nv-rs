//! What the launcher reads from the player's installed game to look like its terminal: files from
//! the game's Data folder (a loose file first, then the archive that holds it, as the game's own
//! file finder prefers loose files), the terminal's settings strings from the player's
//! `FalloutNV.exe`, and the terminal colour from the game's INI. Everything is only read; nothing
//! of the game is copied anywhere.
use std::path::{Path, PathBuf};

/// The player's installed game, found or named.
pub struct Game {
    pub folder: PathBuf,
    pub data: PathBuf,
}

impl Game {
    /// The game at `folder` (its installation folder, the one holding Data and FalloutNV.exe).
    pub fn at(folder: &Path) -> Option<Game> {
        let data = child(folder, "Data")?;
        child(&data, "FalloutNV.esm")?;
        Some(Game {
            folder: folder.to_path_buf(),
            data,
        })
    }

    /// The bytes of `path` (a Data-relative path such as `textures\fonts\x.fnt`): the loose file
    /// when there is one, else from `archive` (a file name in Data).
    pub fn read(&self, path: &str, archive: &str) -> Option<Vec<u8>> {
        self.loose(path).or_else(|| {
            let bsa = bsa::Archive::open(child(&self.data, archive)?).ok()?;
            let entry = bsa.find(path)?;
            bsa.read(entry).ok()
        })
    }

    fn loose(&self, path: &str) -> Option<Vec<u8>> {
        let mut at = self.data.clone();
        for part in path.split(['\\', '/']) {
            at = child(&at, part)?;
        }
        std::fs::read(&at).ok()
    }

    /// The default text of the game setting `name` as the player's `FalloutNV.exe` holds it: the
    /// exe's static initializer pushes the value's address and then the name's
    /// (`PUSH value; PUSH name; MOV ECX, setting; CALL`), so the name's address is found in the
    /// exe's data and the push before it in its code. None when the exe is not the one expected.
    pub fn exe_setting_text(&self, name: &str) -> Option<String> {
        let exe = std::fs::read(child(&self.folder, "FalloutNV.exe")?).ok()?;
        let pe = Pe::parse(&exe)?;
        let mut needle = name.as_bytes().to_vec();
        needle.push(0);
        let at = find(&exe, &needle, 0)?;
        // The name starts the string: the byte before it ends another string or is padding.
        let name_va = pe.va_of(at)?;
        let mut pattern = vec![0x68];
        pattern.extend_from_slice(&name_va.to_le_bytes());
        let mut from = 0;
        while let Some(i) = find(&exe, &pattern, from) {
            from = i + 1;
            if i >= 5 && exe[i - 5] == 0x68 {
                let value_va = u32::from_le_bytes(exe[i - 4..i].try_into().ok()?);
                let off = pe.offset_of(value_va)?;
                let end = exe[off..].iter().position(|&b| b == 0)?;
                // The game's text is Windows-1252; the terminal strings are plain ASCII.
                return Some(exe[off..off + end].iter().map(|&b| b as char).collect());
            }
        }
        None
    }

    /// The terminal colour (`[Interface] iSystemColorTerminalRed`, `Green`, `Blue`): the first
    /// of `inis` (the profile's own files, then the game's `Fallout_default.ini`) that has all
    /// three.
    pub fn terminal_colour(&self, inis: &[PathBuf]) -> Option<[u8; 3]> {
        let default = child(&self.folder, "Fallout_default.ini");
        for path in inis.iter().cloned().chain(default) {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let text: String = bytes.iter().map(|&b| b as char).collect();
            let get = |key: &str| -> Option<u8> {
                ini_value(&text, "Interface", key)?.trim().parse::<u8>().ok()
            };
            if let (Some(r), Some(g), Some(b)) = (
                get("iSystemColorTerminalRed"),
                get("iSystemColorTerminalGreen"),
                get("iSystemColorTerminalBlue"),
            ) {
                return Some([r, g, b]);
            }
        }
        None
    }
}

/// `dir`'s entry named `name`, any case (the exact name first): the game's folders are read on
/// systems whose names have case.
pub fn child(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        e.file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| e.path())
    })
}

/// The value of `key` in `[section]`, the first section of that name, names without case.
fn ini_value<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut inside = false;
    for line in text.split(['\n', '\r']) {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('[') {
            if inside {
                return None;
            }
            inside = rest
                .split(']')
                .next()
                .unwrap_or(rest)
                .trim()
                .eq_ignore_ascii_case(section);
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((name, value)) = line.split_once('=') {
            if name.trim().eq_ignore_ascii_case(key) {
                return Some(value.trim());
            }
        }
    }
    None
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// The sections of a 32-bit PE file: enough to turn file offsets into addresses and back.
struct Pe {
    base: u32,
    /// (virtual address, virtual size, file offset, file size) of each section.
    sections: Vec<(u32, u32, u32, u32)>,
}

impl Pe {
    fn parse(b: &[u8]) -> Option<Pe> {
        let u16_at = |o: usize| Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?));
        let u32_at = |o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?));
        let pe = u32_at(0x3c)? as usize;
        if b.get(pe..pe + 4)? != b"PE\0\0" {
            return None;
        }
        let count = u16_at(pe + 6)? as usize;
        let optional = u16_at(pe + 20)? as usize;
        let base = u32_at(pe + 24 + 28)?;
        let table = pe + 24 + optional;
        let sections = (0..count)
            .map(|i| {
                let s = table + i * 40;
                Some((u32_at(s + 12)?, u32_at(s + 8)?, u32_at(s + 20)?, u32_at(s + 16)?))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Pe { base, sections })
    }

    fn va_of(&self, offset: usize) -> Option<u32> {
        let offset = offset as u32;
        self.sections.iter().find_map(|&(va, _, raw, size)| {
            (offset >= raw && offset < raw + size).then(|| self.base + va + (offset - raw))
        })
    }

    fn offset_of(&self, address: u32) -> Option<usize> {
        let rva = address.checked_sub(self.base)?;
        self.sections.iter().find_map(|&(va, vsize, raw, size)| {
            (rva >= va && rva < va + vsize.max(size) && rva - va < size)
                .then(|| (raw + (rva - va)) as usize)
        })
    }
}
