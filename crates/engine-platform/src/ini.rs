//! The profile (INI) file imports, `GetPrivateProfileIntA`, `GetPrivateProfileStringA` and
//! `WritePrivateProfileStringA`: they read the files the game made in this run ([`files`]) and
//! the Windows directory of the player's table ([`d3d9::windows_directory`]), so they are in the
//! platform layer rather than the core (moved from `engine-core`'s `units/platform.rs`).

#[allow(unused_imports)]
use crate::prelude::*;

use super::{d3d9, files};

/// The value of a key as `GetPrivateProfileStringA` reads it: the core's (`engine_core::mod_files`,
/// which the extenders' mod files share); its measured answers are this file's tests.
pub use engine_core::mod_files::ini_value;

/// Where the profile functions look for the file `name` names (measured on scratch files,
/// 2026-11-16, `build43-scratch/measure/mcm/ini-measure/ini-2026-11-16.txt`): a bare name (no `\`
/// or `/`) is in the Windows directory (`d3d9::windows_directory`, from the player's table); a
/// full path is itself; any other name with a folder part (`Data\Config\MCM.ini`,
/// `Data/Config/MCM.ini`, `.\name.ini`) is relative to the current directory. Before 2026-11-16
/// every name that was not a full path was looked for in the Windows directory.
fn profile_path(e: &mut Engine, name: &[u8]) -> Vec<u8> {
    if files::is_full_path(name) {
        name.to_vec()
    } else if name.iter().any(|&b| b == b'\\' || b == b'/') {
        files::full_path(e, name)
    } else {
        let mut path = d3d9::windows_directory(e);
        path.push(b'\\');
        path.extend_from_slice(name);
        path
    }
}

/// The text of the profile file `file` names ([`profile_path`]): the file as the game made it
/// in this run when it did (`files`), else the player's file, read only. `None` when there is
/// none.
fn profile_text(e: &mut Engine, file: Ptr) -> Option<Vec<u8>> {
    let name = e.mem.cstr(file.addr());
    let path = profile_path(e, &name);
    files::read_whole(e, &path)
}

// Platform: replaces KERNEL32.DLL GetPrivateProfileIntA (import)
/// `GetPrivateProfileIntA(section, key, default, file)`: the key's value as an integer, or
/// `default` when the file, the section or the key isn't there ([`profile_text`]). The value is
/// read as [`profile_int`] does, which follows Windows' answers measured on the player's machine
/// (2026-10-21, 38 values).
///
/// One key answers 1 whatever the file says: `[General] bUseMyGamesDirectory`
/// ([`MY_GAMES_KEY`]), which the game reads at start-up from `.\Custom.ini` and `.\FALLOUT.INI`
/// in its own folder (`00876ad0`, call sites `00876b13` and `00876b55`): 0 there would put its INI
/// files, `plugins.txt` and saves in the game's folder, where nv-rs never writes. Before 2026-11-16
/// those names were looked for in the Windows directory, found nothing and gave the default, 1;
/// with names now read from the current directory as Windows reads them, the key is pinned
/// instead: a deliberate difference, said once in the log when a file holds 0.
pub fn get_private_profile_int_a(
    e: &mut Engine,
    section: Ptr,
    key: Ptr,
    default: u32,
    file: Ptr,
) -> u32 {
    let section = e.mem.cstr(section.addr());
    let key = e.mem.cstr(key.addr());
    let value = profile_text(e, file).and_then(|text| ini_value(&text, &section, &key));
    let trim = engine_core::mod_files::trim_spaces;
    if MY_GAMES_KEY.0.eq_ignore_ascii_case(trim(&section))
        && MY_GAMES_KEY.1.eq_ignore_ascii_case(trim(&key))
    {
        if value.as_deref().map(profile_int) == Some(0) {
            static SAID: std::sync::Once = std::sync::Once::new();
            let name = String::from_utf8_lossy(&e.mem.cstr(file.addr())).into_owned();
            SAID.call_once(|| {
                println!(
                    "{name}: bUseMyGamesDirectory=0 is set in the game folder's settings; nv-rs keeps settings and saves in its own folder"
                )
            });
        }
        return 1;
    }
    let Some(value) = value else {
        return default;
    };
    profile_int(&value)
}

/// The section and key [`get_private_profile_int_a`] answers 1 for.
pub const MY_GAMES_KEY: (&[u8], &[u8]) = (b"General", b"bUseMyGamesDirectory");

/// A profile value as Windows' `GetPrivateProfileIntA` reads it (measured): leading spaces and
/// tabs, then one optional `+` or `-`, then an optional lower-case base prefix `0x` (16), `0o`
/// (8) or `0b` (2), else base 10; digits of that base up to the first other character, kept
/// modulo 2^32; a `-` negates the result modulo 2^32. No digits (`-`, `0x`, `0b2`, `x5`) give 0;
/// upper-case prefixes are not prefixes (`0X1F` is 0).
fn profile_int(value: &[u8]) -> u32 {
    let mut rest = value;
    while let [b' ' | b'\t', tail @ ..] = rest {
        rest = tail;
    }
    let negative = match rest {
        [b'-', tail @ ..] => {
            rest = tail;
            true
        }
        [b'+', tail @ ..] => {
            rest = tail;
            false
        }
        _ => false,
    };
    let base = match rest {
        [b'0', b'x', tail @ ..] => {
            rest = tail;
            16
        }
        [b'0', b'o', tail @ ..] => {
            rest = tail;
            8
        }
        [b'0', b'b', tail @ ..] => {
            rest = tail;
            2
        }
        _ => 10,
    };
    let mut n = 0u32;
    for &c in rest {
        let Some(digit) = char::from(c).to_digit(base) else {
            break;
        };
        n = n.wrapping_mul(base).wrapping_add(digit);
    }
    if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

// Platform: replaces KERNEL32.DLL GetPrivateProfileStringA (import)
/// The value of `key` in `section` of the INI file `file` ([`profile_text`]), or
/// `default` (none: empty) when the file, the section or the key isn't there: copied into
/// `buffer`, cut to `size - 1` bytes and zero-terminated; answers the bytes copied. Listing a
/// file's sections or keys (a null section or key) is not provided.
pub fn get_private_profile_string_a(
    e: &mut Engine,
    section: Ptr,
    key: Ptr,
    default: Ptr,
    buffer: Ptr,
    size: u32,
    file: Ptr,
) -> u32 {
    if section.is_null() || key.is_null() {
        panic!("GetPrivateProfileStringA with a null section or key is not provided");
    }
    let section = e.mem.cstr(section.addr());
    let key = e.mem.cstr(key.addr());
    let found = profile_text(e, file).and_then(|text| ini_value(&text, &section, &key));
    let value = match found {
        Some(value) => value,
        None if default.is_null() => Vec::new(),
        None => e.mem.cstr(default.addr()),
    };
    if size == 0 {
        return 0;
    }
    let copied = value.len().min(size as usize - 1);
    e.mem.set_cstr(buffer.addr(), &value[..copied]);
    copied as u32
}

// Platform: replaces KERNEL32.DLL WritePrivateProfileStringA (import)
/// Answers success without writing: the platform never changes the player's files.
pub fn write_private_profile_string_a(
    _e: &mut Engine,
    _section: Ptr,
    _key: Ptr,
    _value: Ptr,
    _file: Ptr,
) -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.mem.map(0x1000, 0x1000);
        e
    }

    #[test]
    fn ini_values_are_found_by_section_and_key() {
        let text = b"; comment\r\n[General]\r\n sLanguage = ENGLISH \r\n[Controls]\r\nForward=\"0011FF00\"\r\nBack=1F\r\n";
        assert_eq!(
            ini_value(text, b"controls", b"forward").unwrap(),
            b"0011FF00"
        );
        assert_eq!(ini_value(text, b"Controls", b"Back").unwrap(), b"1F");
        assert_eq!(
            ini_value(text, b"General", b"sLanguage").unwrap(),
            b"ENGLISH"
        );
        assert_eq!(ini_value(text, b"General", b"Back"), None);
    }

    /// A `;` line is never found: a key nv-rs's `Fallout.ini` writes commented out
    /// (`distribution/nv-rs-sections.ini`) or a player commented out is not the key, and the
    /// `[nv-rs ...]` sections are sections like any other.
    #[test]
    fn a_commented_key_is_never_found() {
        let text = b"[nv-rs Fixes]\r\n; Uncapped frame rate: a note\r\n;bUncap=1\r\n\
                     [Display]\r\n;iPresentInterval=0\r\niPresentInterval=1\r\n";
        assert_eq!(ini_value(text, b"nv-rs Fixes", b"bUncap"), None);
        assert_eq!(
            ini_value(text, b"Display", b"iPresentInterval").unwrap(),
            b"1"
        );
    }

    /// A file that starts with a UTF-8 byte order mark, as Windows' `GetPrivateProfileStringA`
    /// reads it (measured on scratch files, 2026-11-09, measure/ini/DESIGN.md): a heading glued
    /// to the mark is no heading, so the keys under it are in no section and not found, and the
    /// sections after it read as usual. Play.exe puts nv-rs's block at the end of such a file
    /// (scripts/launcher `game_ini::with_sections`), where it changes none of the player's keys.
    #[test]
    fn a_heading_glued_to_a_byte_order_mark_is_no_heading() {
        let marked = b"\xef\xbb\xbf[General]\r\nsA=1\r\nsB=two\r\n[Display]\r\nfB=2\r\n\
                       \r\n[nv-rs General]\r\nsPreset=fixes\r\n";
        assert_eq!(ini_value(marked, b"General", b"sA"), None);
        assert_eq!(ini_value(marked, b"General", b"sB"), None);
        assert_eq!(ini_value(marked, b"Display", b"fB").unwrap(), b"2");
        assert_eq!(
            ini_value(marked, b"nv-rs General", b"sPreset").unwrap(),
            b"fixes"
        );
        // The mark alone on the first line: the next line is a heading.
        let alone = b"\xef\xbb\xbf\r\n[General]\r\nsA=1\r\n";
        assert_eq!(ini_value(alone, b"General", b"sA").unwrap(), b"1");
    }

    /// Windows' answers on a file with a repeated section name, measured on scratch files on the
    /// player's machine (2026-11-09; `GetPrivateProfileStringA` and `GetPrivateProfileIntA` alike):
    /// only the first section of a name is read, the name compared without case and with the
    /// spaces in the brackets and in the caller's name trimmed. An empty first section hides a
    /// later one; an empty key in the first one is found, empty.
    #[test]
    fn only_the_first_section_of_a_name_is_read() {
        let text = b"[A]\r\nx=1\r\n[B]\r\nz=3\r\n[a]\r\ny=2\r\nx=9\r\n[ A ]\r\nw=6\r\n";
        assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        assert_eq!(ini_value(text, b"A", b"y"), None);
        assert_eq!(ini_value(text, b"A", b"w"), None);
        assert_eq!(ini_value(text, b"B", b"z").unwrap(), b"3");
        assert_eq!(ini_value(text, b" A ", b"x").unwrap(), b"1");
        let text = b"[a]\r\nx=1\r\n[A]\r\ny=2\r\n[C]\r\nq=1\r\n[C]\r\nq=2\r\nr=3\r\n";
        assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        assert_eq!(ini_value(text, b"A", b"y"), None);
        assert_eq!(ini_value(text, b"C", b"q").unwrap(), b"1");
        assert_eq!(ini_value(text, b"C", b"r"), None);
        assert_eq!(ini_value(b"[A]\r\n[A]\r\nx=1\r\n", b"A", b"x"), None);
        assert_eq!(
            ini_value(b"[A]\r\n; note\r\n\r\n[A]\r\nx=1\r\n", b"A", b"x"),
            None
        );
        assert_eq!(
            ini_value(b"[A]\r\nx=\r\n[A]\r\nx=1\r\n", b"A", b"x").unwrap(),
            b""
        );
    }

    /// Three more of Windows' `GetPrivateProfileStringA` answers, measured on scratch files on
    /// the player's machine (2026-11-10): a key repeated in one section gives its first line,
    /// whatever its case and spaces; a heading is the text up to its first `]` (text after it, or
    /// no `]` at all, changes nothing, and `[A]x=9` is a heading, not a key); a key before any
    /// heading is in no section, not even the one named "", which `[]` opens.
    #[test]
    fn repeated_keys_headings_and_keys_before_any_section_read_as_windows_does() {
        assert_eq!(
            ini_value(b"[A]\r\nx=1\r\nx=2\r\n", b"A", b"x").unwrap(),
            b"1"
        );
        assert_eq!(
            ini_value(b"[A]\r\nX=1\r\n x =2\r\n", b"A", b"x").unwrap(),
            b"1"
        );
        let headings: [&[u8]; 6] = [
            b"[A] trailing\r\nx=1\r\n",
            b"[A]; note\r\nx=1\r\n",
            b"[A]]\r\nx=1\r\n",
            b"[A\r\nx=1\r\n",
            b"[A]x=9\r\nx=1\r\n",
            b" [A] \r\nx=1\r\n",
        ];
        for text in headings {
            assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        }
        assert_eq!(ini_value(b"[A]]\r\nx=1\r\n", b"A]", b"x"), None);
        assert_eq!(ini_value(b"x=1\r\n[A]\r\ny=2\r\n", b"", b"x"), None);
        assert_eq!(ini_value(b"x=1\r\n[A]\r\ny=2\r\n", b"A", b"x"), None);
        assert_eq!(ini_value(b"x=1\r\n[]\r\nz=3\r\n", b"", b"z").unwrap(), b"3");
        assert_eq!(ini_value(b"x=1\r\n[]\r\nz=3\r\n", b"", b"x"), None);
    }

    /// Windows' `GetPrivateProfileIntA` answers, measured on the player's machine (2026-10-21).
    #[test]
    fn profile_ints_read_as_windows_does() {
        let measured: &[(&[u8], u32)] = &[
            (b"-5", 0xffff_fffb),
            (b"-0", 0),
            (b"0x10", 16),
            (b"0X1F", 0),
            (b"-0x10", 0xffff_fff0),
            (b"12abc", 12),
            (b"  7", 7),
            (b"+3", 3),
            (b"4294967295", 0xffff_ffff),
            (b"4294967296", 0),
            (b"99999999999", 1_215_752_191),
            (b"1e3", 1),
            (b"0", 0),
            (b"08", 8),
            (b"010", 10),
            (b"-", 0),
            (b"x5", 0),
            (b"0b101", 5),
            (b"0o17", 15),
            (b"0x", 0),
            (b"0xg", 0),
            (b"-  5", 0),
            (b" -5", 0xffff_fffb),
            (b"0b2", 0),
            (b"0x10abc", 0x10abc),
            (b"0xFFFFFFFF1", 0xffff_fff1),
            (b"0B11", 0),
            (b"0O7", 0),
            (b"\t9", 9),
            (b"--5", 0),
            (b"+-5", 0),
            (b"- 5", 0),
            (b"00x10", 0),
            (b"0x-5", 0),
            (b"0b", 0),
            (b"0o8", 0),
            (b"5 6", 5),
        ];
        for &(value, want) in measured {
            assert_eq!(profile_int(value), want, "{:?}", std::str::from_utf8(value));
        }
    }

    /// Where a name is looked for, as Windows looked on scratch files (2026-11-16): a name with a
    /// folder part from the current directory (both separators, `.\` too), a bare name from the
    /// Windows directory, a full path as it is.
    #[test]
    fn a_name_with_a_folder_part_is_relative_to_the_current_directory() {
        let mut e = engine();
        e.kernel.current_directory = br"C:\Games\FNV".to_vec();
        assert_eq!(
            profile_path(&mut e, br"Data\Config\MCM.ini"),
            br"C:\Games\FNV\Data\Config\MCM.ini"
        );
        assert_eq!(
            profile_path(&mut e, b"Data/Config/MCM.ini"),
            b"C:\\Games\\FNV\\Data/Config/MCM.ini"
        );
        assert_eq!(
            profile_path(&mut e, br".\bare.ini"),
            br"C:\Games\FNV\bare.ini"
        );
        assert_eq!(profile_path(&mut e, br"D:\x\y.ini"), br"D:\x\y.ini");
        // A bare name: the Windows directory, from the machine's table.
        windows_table(&mut e);
        assert_eq!(
            profile_path(&mut e, b"bare-nvrs-measure.ini"),
            br"C:\WINDOWS\bare-nvrs-measure.ini"
        );
    }

    /// A machine table with only its Windows directory, `C:\WINDOWS`.
    fn windows_table(e: &mut Engine) {
        let hex: String = br"C:\WINDOWS".iter().map(|b| format!("{b:02x}")).collect();
        d3d9::load_adapter_table(e, &format!("GetWindowsDirectoryA\t-\t0\t{hex}\n"));
    }

    /// A scratch folder removed when the test ends, however it ends.
    #[cfg(windows)]
    struct Scratch(std::path::PathBuf);
    #[cfg(windows)]
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[cfg(windows)]
    fn scratch(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("nv-ini-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    /// `GetPrivateProfileIntA` through a name with a folder part (`.\FALLOUT.INI`, as the game
    /// reads it at start-up) reads the current directory's file; `[General]
    /// bUseMyGamesDirectory` answers 1 whatever the file says (the game's folder keeps nothing of
    /// nv-rs's), other keys their values; a bare name reads the Windows directory.
    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
    #[test]
    fn profile_ints_read_relative_names_and_pin_the_my_games_key() {
        let game = scratch("game");
        std::fs::write(
            game.0.join("FALLOUT.INI"),
            b"[General]\r\nbUseMyGamesDirectory=0\r\niOther=7\r\n",
        )
        .unwrap();
        let mut e = engine();
        windows_table(&mut e);
        e.kernel.current_directory = game.0.to_string_lossy().replace('/', "\\").into_bytes();
        e.mem.set_cstr(0x1000, b"General");
        e.mem.set_cstr(0x1020, b"bUseMyGamesDirectory");
        e.mem.set_cstr(0x1040, br".\FALLOUT.INI");
        e.mem.set_cstr(0x1060, b"iOther");
        e.mem.set_cstr(0x1080, b"FALLOUT.INI");
        let int = |e: &mut Engine, key: u32, file: u32| {
            get_private_profile_int_a(e, Ptr::new(0x1000), Ptr::new(key), 5, Ptr::new(file))
        };
        assert_eq!(int(&mut e, 0x1020, 0x1040), 1, "pinned, the file says 0");
        assert_eq!(int(&mut e, 0x1060, 0x1040), 7, "another key reads the file");
        assert_eq!(
            int(&mut e, 0x1060, 0x1080),
            5,
            "a bare name: the Windows directory"
        );
        std::fs::write(game.0.join("FALLOUT.INI"), b"[General]\r\niOther=7\r\n").unwrap();
        assert_eq!(int(&mut e, 0x1020, 0x1040), 1, "no key: 1 too");
    }

    /// Quotes around a value, as Windows read them on scratch files (2026-11-16,
    /// `build43-scratch/measure/mcm/ini-measure/ini3-2026-11-16.txt`).
    #[test]
    fn quotes_around_a_value_read_as_windows_does() {
        let measured: &[(&[u8], &[u8])] = &[
            (b"x='1'", b"1"),
            (b"x=\"1\"", b"1"),
            (b"x=\"1'", b"\"1'"),
            (b"x=\"", b"\""),
            (b"x=\"\"", b""),
            (b"x= \"1\" ", b"1"),
            (b"x=''", b""),
            (b"x='", b"'"),
            (b"x=\"a\"b\"", b"a\"b"),
            (b"x=\"1\" ;c", b"\"1\" ;c"),
            (b"x=\"1\"\"", b"1\""),
        ];
        for &(line, want) in measured {
            let text = [b"[A]\r\n", line, b"\r\n"].concat();
            assert_eq!(
                ini_value(&text, b"A", b"x").unwrap(),
                want,
                "{}",
                String::from_utf8_lossy(line)
            );
        }
    }

    /// The game's own profile reads do not look in the mods' folder: a value a mod's command
    /// wrote (`engine_core::mod_files`) is not what the import reads; the game's file is.
    // Windows only: it builds the game's names with backslashes and drive letters, which the platform's
    // file layer maps to host paths on Windows alone (`files::real_path` keeps a backslash
    // as part of a name elsewhere; starting the game there is refused today).
    #[cfg(windows)]
    #[test]
    fn the_games_profile_import_does_not_read_mod_files() {
        let game = scratch("own");
        std::fs::create_dir_all(game.0.join(r"Data\Config")).unwrap();
        std::fs::write(game.0.join(r"Data\Config\X.ini"), b"[A]\r\nx=game\r\n").unwrap();
        let mut e = engine();
        e.kernel.current_directory = game.0.to_string_lossy().replace('/', "\\").into_bytes();
        engine_core::mod_files::write_profile_string(
            &mut e,
            br"Data\Config\X.ini",
            b"A",
            b"x",
            b"mod",
        )
        .unwrap();
        assert_eq!(
            engine_core::mod_files::profile_string(&e, br"Data\Config\X.ini", b"A", b"x").unwrap(),
            b"mod"
        );
        e.mem.set_cstr(0x1000, b"A");
        e.mem.set_cstr(0x1020, b"x");
        e.mem.set_cstr(0x1040, br"Data\Config\X.ini");
        let n = get_private_profile_string_a(
            &mut e,
            Ptr::new(0x1000),
            Ptr::new(0x1020),
            Ptr::NULL,
            Ptr::new(0x1100),
            0x40,
            Ptr::new(0x1040),
        );
        assert_eq!((n, e.mem.cstr(0x1100)), (4, b"game".to_vec()));
    }

    /// A missing file gives the default, cut to the buffer; writing changes nothing and answers 1.
    #[test]
    fn profile_strings_fall_back_to_the_default() {
        let mut e = engine();
        e.mem.set_cstr(0x1000, b"Controls");
        e.mem.set_cstr(0x1020, b"Forward");
        e.mem.set_cstr(0x1040, b"default");
        e.mem.set_cstr(0x1060, br"Z:\no\such\file.ini");
        let n = get_private_profile_string_a(
            &mut e,
            Ptr::new(0x1000),
            Ptr::new(0x1020),
            Ptr::new(0x1040),
            Ptr::new(0x1100),
            4,
            Ptr::new(0x1060),
        );
        assert_eq!(n, 3);
        assert_eq!(e.mem.cstr(0x1100), b"def");
        let n = get_private_profile_string_a(
            &mut e,
            Ptr::new(0x1000),
            Ptr::new(0x1020),
            Ptr::NULL,
            Ptr::new(0x1100),
            0x40,
            Ptr::new(0x1060),
        );
        assert_eq!((n, e.mem.u8(0x1100)), (0, 0));
        assert_eq!(
            write_private_profile_string_a(
                &mut e,
                Ptr::new(0x1000),
                Ptr::new(0x1020),
                Ptr::new(0x1040),
                Ptr::new(0x1060)
            ),
            1
        );
    }
}
