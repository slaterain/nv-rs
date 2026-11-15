//! The `[GamePatches]` switches of the player's `Data\NVSE\plugins\jip_nvse.ini`, read as JIP
//! reads them in `InitGamePatches` (`internal/patches_game.h` (JIP LN NVSE)): Windows'
//! `GetPrivateProfileSectionA` on the relative path, then each line's key looked up in
//! `s_optionalHacks` (`jip_nvse.cpp`) and its value's first character tested.
//!
//! JIP's own defaults, with no section or an empty one (`lines <= 0`): switches 1, 4, 5, 6, 7, 8,
//! 9, 13 and 14 on. The `jip_nvse.ini` JIP's authors publish ("JIP LN Settings INI" 56.24, the
//! copy beside VNV's utilities: `ext-bin/vnv-utilities-2026-11-07/JIP_LN_Settings_INI-58277-56-24-1691190751`,
//! sha256 `db231f7c...2243`) holds exactly those on, the others 0 (`bFO3WpnDegradation` not
//! listed: off), and `uWMChancePerLevel=2`, `uWMChanceMin=10`, `uWMChanceMax=60` (JIP's code
//! starts the three at 0). The JIP 57.30 package itself ships no ini.
//!
//! How Windows answers the section (measured on scratch files, 2026-11-14,
//! `measure/jip/ini-section-2026-11-14.txt`): a relative path with a separator is in the current
//! directory (the game's folder), not the Windows directory; the section name has no case and only
//! the first section of a name is read; blank lines and lines whose first non-blank character is
//! `;` are left out (`#` is not a comment); a line with `=` comes back as its key and value each
//! trimmed, joined by `=` (quotes kept); a line without `=` comes back trimmed; repeated keys come
//! back each; a UTF-8 byte order mark spoils the first line (so a section heading there is not
//! found), a UTF-16 (little-endian) one is read as text; a heading is the text from its `[` up to
//! the first `]` or the line's end, trimmed (`[GamePatches] x`, `[ GamePatches ]`,
//! `[GamePatches`, `  [GamePatches]x=9` all open the section; the text after `]` is no line); a
//! CR alone ends a line as CR LF and LF do; a section longer than the buffer is cut at 65,534
//! bytes (the lines with their NULs). The count is 0 for a missing file or section and for a
//! section with only comments, so all three give JIP's defaults. Not measured, so read as plain
//! bytes (they find no section, JIP's defaults): a UTF-16 big-endian file. The weapon mod chances
//! stay 0 without the file (JIP's code).

use crate::containers::str_hash_ci;
use crate::utility::str_to_int;
use engine_core::extend::jip_patches;
use engine_core::prelude::*;

/// The file, as JIP names it (relative: the game's folder).
pub const PATH: &[u8] = b"Data\\NVSE\\plugins\\jip_nvse.ini";

/// `s_optionalHacks`: the keys and their numbers (1-19 switches, 20-22 the weapon mod chances).
pub const KEYS: [(&str, u32); 21] = [
    ("bIgnoreDTDRFix", 1),
    ("bEnableFO3Repair", 2),
    ("bEnableBigGunsSkill", 3),
    ("bProjImpactDmgFix", 4),
    ("bGameDaysPassedFix", 5),
    ("bHardcoreNeedsFix", 6),
    ("bNoFailedScriptLocks", 7),
    ("bDoublePrecision", 8),
    ("bQttSelectShortKeys", 9),
    ("bFO3WpnDegradation", 11),
    ("bLocalizedDTDR", 12),
    ("bVoiceModulationFix", 13),
    ("bSneakBoundingBoxFix", 14),
    ("bEnableNVACAlerts", 15),
    ("bLoadScreenFix", 16),
    ("bNPCWeaponMods", 17),
    ("uNPCPerks", 18),
    ("bCreatureSpreadFix", 19),
    ("uWMChancePerLevel", 20),
    ("uWMChanceMin", 21),
    ("uWMChanceMax", 22),
];

/// JIP's switches on with no `[GamePatches]` lines.
pub const DEFAULTS: [u32; 9] = [1, 4, 5, 6, 7, 8, 9, 13, 14];

/// What the section said.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Switches {
    /// The switches on, as bits of `engine_core::extend::jip_patches` (`1 << number`).
    pub on: u32,
    /// `uNPCPerks=2` (`s_NPCPerksAutoAdd`: NPCs get starting perks too).
    pub npc_perks_auto_add: bool,
    /// `s_uWMChancePerLevel`, `s_uWMChanceMin`, `s_uWMChanceMax`.
    pub weapon_mod_chance: [u32; 3],
}

/// Reads the player's file through the game's file calls (as the built-in xNVSE reads its files)
/// and answers its switches; JIP's defaults when it has none.
pub fn read(e: &mut Engine) -> Switches {
    let text = nvse::saveload::read_file(e, PATH);
    switches(&section(text.as_deref().unwrap_or(&[]), b"GamePatches"))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches (the
// GetPrivateProfileSection loop).
/// The switches from the section's lines as `GetPrivateProfileSectionA` returns them into JIP's
/// 0x10000-byte buffer ([`section_buffer`]: the lines each ended by a NUL, cut to 0xFFFE bytes).
/// None (a count of 0): JIP's [`DEFAULTS`]. Else JIP's walk: `GetNextToken(buffer, '=')` ends the
/// token at the first `=` or NUL and clears the run of `=` there; the token is looked up by
/// `StrHashCI` alone, as JIP's map finds it (20-22 take the value's number, `StrToInt`; a switch is
/// on unless its value starts with `0`, so an empty value or no `=` turns it on; `uNPCPerks`
/// starting with `2` also adds the starting perks); then the walk goes on at the byte after the
/// token's end, so the value itself (and one empty token per extra `=`) is looked up next as a key.
/// JIP never turns a switch off here.
pub fn switches(lines: &[Vec<u8>]) -> Switches {
    let mut out = Switches::default();
    let mut buffer = section_buffer(lines);
    let mut remaining = buffer.len();
    if remaining == 0 {
        for n in DEFAULTS {
            out.on |= jip_patches::optional(n);
        }
        return out;
    }
    // The two NULs Windows writes after the data.
    buffer.extend_from_slice(&[0, 0]);
    let mut at = 0;
    while remaining > 0 {
        let end = at
            + buffer[at..]
                .iter()
                .position(|&b| b == b'=' || b == 0)
                .unwrap_or(buffer.len() - at);
        let mut value = end;
        while buffer[value] == b'=' {
            buffer[value] = 0;
            value += 1;
        }
        let value_end = value + buffer[value..].iter().position(|&b| b == 0).unwrap_or(0);
        let (key, value) = (&buffer[at..end], &buffer[value..value_end]);
        let hash = str_hash_ci(key);
        if let Some(&(_, number)) = KEYS
            .iter()
            .find(|(name, _)| str_hash_ci(name.as_bytes()) == hash)
        {
            if number >= 20 {
                out.weapon_mod_chance[(number - 20) as usize] = str_to_int(value) as u32;
            } else if value.first() != Some(&b'0') {
                if number == 18 && value.first() == Some(&b'2') {
                    out.npc_perks_auto_add = true;
                }
                out.on |= jip_patches::optional(number);
            }
        }
        // `size = StrLen(buffer) + 1`: the token and its NUL.
        let size = end - at + 1;
        at += size;
        remaining = remaining.saturating_sub(size);
    }
    out
}

/// The most `GetPrivateProfileSectionA` returns into JIP's 0x10000-byte buffer: `nSize - 2`
/// (measured, 2026-11-14: a 84,000-byte section answers 65,534, the lines cut there).
pub const SECTION_CAP: usize = 0xfffe;

/// The section's lines as the call writes them: each ended by a NUL, cut to [`SECTION_CAP`] bytes.
pub fn section_buffer(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut buffer = Vec::new();
    for line in lines {
        buffer.extend_from_slice(line);
        buffer.push(0);
    }
    buffer.truncate(SECTION_CAP);
    buffer
}

// Measured from KERNEL32 GetPrivateProfileSectionA (scratch files, 2026-11-14).
/// The lines of section `name` in an INI file's bytes, as `GetPrivateProfileSectionA` gives them
/// (module doc): empty when the file has no such section or the section only blank and comment
/// lines.
pub fn section(file: &[u8], name: &[u8]) -> Vec<Vec<u8>> {
    let text = match file {
        [0xff, 0xfe, rest @ ..] => rest
            .chunks_exact(2)
            .map(|c| match u16::from_le_bytes([c[0], c[1]]) {
                u @ 0..=0x7f => u as u8,
                _ => b'?',
            })
            .collect(),
        _ => file.to_vec(),
    };
    let mut out = Vec::new();
    let mut inside = false;
    // A CR alone ends a line too (measured); CR LF leaves an empty line between, passed over.
    for line in text.split(|&b| b == b'\n' || b == b'\r') {
        let line = trim(line);
        if let Some(rest) = line.strip_prefix(b"[") {
            if inside {
                break;
            }
            let heading = rest.split(|&b| b == b']').next().unwrap_or(rest);
            inside = trim(heading).eq_ignore_ascii_case(name);
            continue;
        }
        if !inside || line.is_empty() || line.starts_with(b";") {
            continue;
        }
        match line.iter().position(|&b| b == b'=') {
            Some(at) => {
                let mut joined = trim(&line[..at]).to_vec();
                joined.push(b'=');
                joined.extend_from_slice(trim(&line[at + 1..]));
                out.push(joined);
            }
            None => out.push(line.to_vec()),
        }
    }
    out
}

/// ASCII white space taken off both ends (`<[u8]>::trim_ascii` is newer than the MSRV).
fn trim(text: &[u8]) -> &[u8] {
    let start = text
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &text[start..end]
}
