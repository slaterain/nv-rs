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

    /// [`Game::read`] with the archive already open (`archive`; None: loose files only).
    pub fn read_in(&self, path: &str, archive: Option<&bsa::Archive>) -> Option<Vec<u8>> {
        self.loose(path).or_else(|| {
            let bsa = archive?;
            bsa.read(bsa.find(path)?).ok()
        })
    }

    /// The files of the Data folder `folder` (a Data-relative path) whose names end in `ext`
    /// (lower case), as Data-relative paths: the loose ones first (by name), then the archive's
    /// that are not loose (in its order). The game's own list is its file finder's
    /// (`FileFinder::BuildFileList`); which file is picked is random either way.
    pub fn list(&self, folder: &str, ext: &str, archive: Option<&bsa::Archive>) -> Vec<String> {
        let folder = bsa::normalize_path(folder)
            .trim_end_matches('\\')
            .to_string();
        let mut names: Vec<String> = Vec::new();
        let mut at = Some(self.data.clone());
        for part in folder.split('\\').filter(|p| !p.is_empty()) {
            at = at.and_then(|a| child(&a, part));
        }
        if let Some(entries) = at.and_then(|dir| std::fs::read_dir(dir).ok()) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                if name.ends_with(ext) && e.path().is_file() {
                    names.push(name);
                }
            }
        }
        names.sort();
        if let Some(bsa) = archive {
            if let Some(f) = bsa.folders().iter().find(|f| f.name == folder) {
                for e in &bsa.files()[f.files.clone()] {
                    if e.name.ends_with(ext) && !names.contains(&e.name) {
                        names.push(e.name.clone());
                    }
                }
            }
        }
        names
            .into_iter()
            .map(|n| format!("{folder}\\{n}"))
            .collect()
    }

    fn loose(&self, path: &str) -> Option<Vec<u8>> {
        let mut at = self.data.clone();
        for part in path.split(['\\', '/']) {
            at = child(&at, part)?;
        }
        std::fs::read(&at).ok()
    }

    /// The default text of the game setting `name` as the player's `FalloutNV.exe` 1.4.0.525
    /// holds it ([`SETTINGS`]: the addresses its static initializer pushes, `PUSH value; PUSH
    /// name; MOV ECX, setting; CALL 0040c150`). Read from the exe's `.rdata`, which the Steam
    /// exe keeps unpacked (only its code is encrypted; `engine_core::exe`). The name must stand
    /// at its address, else the exe is not 1.4.0.525 and None is the answer.
    pub fn exe_setting_text(&self, name: &str) -> Option<String> {
        let &(_, name_va, value_va) = SETTINGS.iter().find(|(n, _, _)| *n == name)?;
        let exe = std::fs::read(child(&self.folder, "FalloutNV.exe")?).ok()?;
        let pe = Pe::parse(&exe)?;
        let text_at = |va: u32| -> Option<String> {
            let off = pe.offset_of(va)?;
            let end = exe.get(off..)?.iter().position(|&b| b == 0)?;
            // The game's text is Windows-1252, one character a byte.
            Some(exe[off..off + end].iter().map(|&b| b as char).collect())
        };
        (text_at(name_va)? == name).then(|| text_at(value_va))?
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
                ini_value(&text, "Interface", key)?
                    .trim()
                    .parse::<u8>()
                    .ok()
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

/// The terminal's game settings with text: name, the name's address and the default value's
/// address in FalloutNV.exe 1.4.0.525 (their static initializers 00f6daa0, 00f6dad0, 00f6dc80,
/// 00f6dc20 and 00f6df20 push them). FalloutNV.esm has no GMST for any of them.
const SETTINGS: [(&str, u32, u32); 5] = [
    ("sComputersHeader1", 0x0105_a540, 0x0105_a554),
    ("sComputersHeader2", 0x0105_a580, 0x0105_a594),
    ("sHackingIntro01", 0x0105_a6c8, 0x0105_a6d8),
    ("sComputersLogon", 0x0105_a680, 0x0105_a690),
    ("sHackingHeader2", 0x0105_a930, 0x0105_a940),
];

/// The sections of a 32-bit PE file: enough to turn addresses into file offsets.
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
                Some((
                    u32_at(s + 12)?,
                    u32_at(s + 8)?,
                    u32_at(s + 20)?,
                    u32_at(s + 16)?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Pe { base, sections })
    }

    fn offset_of(&self, address: u32) -> Option<usize> {
        let rva = address.checked_sub(self.base)?;
        self.sections.iter().find_map(|&(va, vsize, raw, size)| {
            (rva >= va && rva < va + vsize.max(size) && rva - va < size)
                .then(|| (raw + (rva - va)) as usize)
        })
    }
}
