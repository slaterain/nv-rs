//! The Settings screen's rows, with no window in them. The main list: the twelve rows of the old
//! Settings.cmd (its distribution/Settings.ps1) by the same names, choices, defaults and keys (the
//! census in the launcher track's notes). Every other key of the host's table (`crates/host/data/
//! game_ini.tsv`) is a row of its own, made from the table's line, so a key added there shows
//! here with nothing else to change ([`List`]): the fix rows (`[nv-rs Fixes]`) in "Fixes one by
//! one", the rest (the new lighting, the background row) in "New lighting and more". All are read
//! from and written to the profile's own `Fallout.ini` and `FalloutPrefs.ini` through the shared
//! code the game's side uses (`game_profile`). A row that is changed writes only its own keys, as
//! Settings.ps1 saves only the options changed in it; "Game's setting" writes nothing. Our keys'
//! defaults are the host's table's, so what the screen shows for a key that is missing is what
//! the game does; a fix row follows the preset ("Game fixes") as the game does, until it is
//! chosen here, and choosing the preset's value hands it back. A table row shows a plain name and
//! a short text of what it changes in the game ([`PLAIN`]), never the table's note (written for
//! readers of Fallout.ini); a row that does nothing in play is not shown ([`HIDDEN`]).

use game_profile::game_ini::{set_key, unset_key, FOLLOWS_PRESET};
use game_profile::settings::ini_value;
use std::sync::OnceLock;

/// The host's table: section, key, variable, default, note.
const TABLE: &str = include_str!("../../host/data/game_ini.tsv");

/// The fix rows' section (the host's `game_ini::FIXES_SECTION`): each key turns one fix on or
/// off, over the preset.
pub const FIXES_SECTION: &str = "nv-rs Fixes";

/// "Game's setting": keeps what the game's own file says (Settings.ps1's `$own`).
pub const OWN: &str = "Game's setting";

/// The two files of a profile, as text one character a byte (every other byte stays).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Files {
    pub ini: String,
    pub prefs: String,
}

/// One line of the host's table.
#[derive(Debug, PartialEq, Eq)]
pub struct Key {
    pub section: &'static str,
    pub key: &'static str,
    /// The environment variable over it (tests and scripts), and its settings.ini line's name.
    pub variable: &'static str,
    /// The value a play run takes when the file says nothing (for a fix row: with the fixes
    /// preset).
    pub default: &'static str,
    /// The one note the generated sections write above the key.
    pub note: &'static str,
}

fn parse(text: &'static str) -> Vec<Key> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some(Key {
                section: f.next()?,
                key: f.next()?,
                variable: f.next()?,
                default: f.next()?,
                note: f.next().unwrap_or(""),
            })
        })
        .collect()
}

/// The host's table, line by line in its order.
pub fn table() -> &'static [Key] {
    static KEYS: OnceLock<Vec<Key>> = OnceLock::new();
    KEYS.get_or_init(|| parse(TABLE))
}

/// A row of the Settings screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Row {
    Renderer,
    DisplayMode,
    Resolution,
    AntiAliasing,
    Anisotropy,
    VSync,
    GameFixes,
    FrameLimit,
    MenuFrameLimit,
    FrameRateCounter,
    GameThreads,
    DrawingThread,
    /// A key of the host's table that none of the twelve reads: on or off for a `b` key, a number
    /// for an `f` key, its own value otherwise.
    Key(&'static Key),
}

/// The lists of the Settings screen: the main one, and the two it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    Main,
    /// The table's keys outside the fix rows that none of the twelve reads: the new lighting and
    /// the background row.
    More,
    /// The table's fix rows, every one.
    Fixes,
}

/// Keys that change nothing off Windows, so their rows show on Windows only: Windows' background
/// slowdown (`crates/qos`: `apply_at_start` does nothing on other systems), and the shaders made
/// ready in the menus (only the real Direct3D 9 renderer does that, `engine_platform::d3d9real`'s
/// `backend_of`, and it is a Windows API).
const WINDOWS_ONLY: [&str; 2] = ["bNotBackground", "bShaderWarm"];

/// Keys whose rows are not shown: they do nothing in a play run, or are the engineers' until their
/// checks pass. The world drawn on a second thread: the host refuses it in every play run
/// (`host::draw_thread::refusal` lets it in only with its own check variable, which a play run never
/// has). The world's still lit objects drawn from kept records (`sDrawItems`): off until its picture
/// proof and speed check pass.
const HIDDEN: [&str; 2] = ["bWorldDrawThread", "sDrawItems"];

impl List {
    /// Its rows on this system.
    pub fn rows(self) -> &'static [Row] {
        static MORE: OnceLock<Vec<Row>> = OnceLock::new();
        static FIXES: OnceLock<Vec<Row>> = OnceLock::new();
        match self {
            List::Main => &ROWS,
            List::More => MORE.get_or_init(|| self.rows_on(cfg!(windows))),
            List::Fixes => FIXES.get_or_init(|| self.rows_on(cfg!(windows))),
        }
    }

    /// [`List::rows`] as on Windows (`windows`) or not: for the two others, the table's keys in its
    /// order, but those [`HIDDEN`] and (off Windows) those [`WINDOWS_ONLY`].
    pub fn rows_on(self, windows: bool) -> Vec<Row> {
        let shown =
            |k: &Key| !HIDDEN.contains(&k.key) && (windows || !WINDOWS_ONLY.contains(&k.key));
        let mine = |k: &Key| match self {
            List::Main => false,
            List::More => {
                k.section != FIXES_SECTION
                    && !ROWS.iter().any(|r| r.table_key() == Some(k.key))
                    && shown(k)
            }
            List::Fixes => k.section == FIXES_SECTION && shown(k),
        };
        match self {
            List::Main => ROWS.to_vec(),
            _ => table().iter().filter(|k| mine(k)).map(Row::Key).collect(),
        }
    }

    /// Its name: the main list's entry that opens it (the start screen's for the main list).
    pub fn name(self) -> &'static str {
        match self {
            List::Main => "Settings",
            List::More => "New lighting and more",
            List::Fixes => "Fixes one by one",
        }
    }

    /// The lines above its rows, after the line that says where choices are saved.
    pub fn notes(self) -> Vec<String> {
        self.notes_on(cfg!(windows))
    }

    /// [`List::notes`] as on Windows (`windows`) or not: the new lighting is still being tried
    /// (ambient occlusion's labels have no room for [`EXPERIMENTAL_MARK`]), and it is drawn by the
    /// game's own renderer only, so on Windows the list says Direct3D 9 does not draw it
    /// (elsewhere there is no other renderer).
    pub fn notes_on(self, windows: bool) -> Vec<String> {
        let preset = Row::GameFixes.label();
        match self {
            List::Main => Vec::new(),
            List::More => {
                let mut notes = vec!["The new lighting is still being tried.".to_string()];
                if windows {
                    notes.push(format!("{} does not draw it.", without_note(DIRECT3D_9)));
                }
                notes
            }
            List::Fixes => vec![format!("Each follows {preset} until chosen here.")],
        }
    }
}

/// Each shown table row's plain name and what it changes in the game, for a player (the table's
/// notes are for readers of Fallout.ini): the key, the name ([`LABEL_MAX`] with any
/// [`EXPERIMENTAL`] mark), the text ([`ABOUT_MAX`]). No key names, values or engine words; a
/// row with none here gets [`plain_of`] its note.
const PLAIN: [(&str, &str, &str); 44] = [
    ("bAmbientOcclusion", "Ambient occlusion", "A soft shade in corners, creases and where things meet the ground. Costs a little speed."),
    ("fAmbientOcclusionStrength", "Ambient occlusion strength", "How strong the ambient occlusion's shade is, from 0.1 (faint) to 1.0 (all of it)."),
    ("bSunShadows", "Sun shadows", "Objects cast shadows on the ground and on each other from the sun, and faintly from the moon at night, out to about 140 m. Costs about 1 ms a frame."),
    ("bSunLightFromSky", "Shadows follow the sun and moon", "Shadows point away from the sun by day and the moon by night. Off: the original's sun light, low from the north."),
    ("bSunShadowsCharacters", "Characters' sun shadows", "You (in third person), people, creatures and bodies cast sun and moon shadows that move with them, in place of the game's own."),
    ("bNotBackground", "Full speed in the background", "Windows does not slow the game down while its window is not the active one. Off: Windows decides, saving power and heat."),
    ("bUncap", "Uncapped frame rate", "Frames are not held to the original game's pace. Frame rate limit sets the most frames a second."),
    ("bHighFrameRateLanding", "Landing at high frame rates", "At high frame rates, when you land while standing still you fall as far as at 60 frames a second."),
    ("bHighFrameRateSlowMotion", "Slow motion at high frame rates", "Slow motion, as in V.A.T.S., comes on and goes off in the same time at any frame rate."),
    ("bMenuCap", "Menus use their own limit", "Menus run at most at Frame rate limit in menus; loading screens are not limited."),
    ("bGrabPush", "Dragging at high frame rates", "Something you hold and drag is pulled as it is at 60 frames a second, whatever the frame rate."),
    ("bPhysicsBlend", "Smooth moving objects", "Dropped and thrown items, falling bodies and debris move smoothly instead of freezing on some frames."),
    ("bModFitWorn", "Safe weapon mod fitting", "Fitting a weapon mod can no longer stop the game when the menu closes."),
    ("bTopicIndex", "Faster loads with dialogue mods", "The game finds where a dialogue line belongs without searching every topic: the same dialogue, faster loads with dialogue mods."),
    ("bShaderWarm", "Shaders made ready in menus", "With Direct3D 9, the graphics driver makes the game's shaders ready in the menus, not at their first use in play, where a frame would wait."),
    ("bIOWait", "Shorter waits for loading", "The game's waits for loading (entering an area, a load, a save) end once the files are in, not up to 50 ms later."),
    ("bLoadSteps", "Smoother running between areas", "While you run between areas, loading is done in smaller steps each frame, so frame times jump less; far objects may appear a few frames later."),
    ("bDeadLightGuard", "Destroyed light crash fix", "A light destroyed while something still used it no longer stops the game (seen in fights with JIP's projectile lights)."),
    ("bThreadHeap", "Faster memory handling", "The game gets memory for its objects and texts faster: the pause menu opens sooner with mods and fights stutter less."),
    ("bHalfLoadedMove", "Fast running crash fix", "Running fast across areas no longer crashes the game when a character there changes how the game runs it while it is still loading."),
    ("bPlayerHighProcess", "Very fast movement crash fix", "Moving very fast across areas no longer crashes the game when you reach an area that is still being set up."),
    ("bTextureReadFirst", "Textures read ahead of drawing", "Texture files are read and unpacked first, so drawing does not wait for them."),
    ("bCellEntryWait", "No pause entering new areas", "Entering a new outdoor area no longer pauses for the areas loaded ahead; they load in the background afterwards."),
    ("bCellOffsets", "Faster area loads with mods", "Areas from mods (and from YUP or TTW) are found through a table made as the game loads, not by searching the file each time."),
    ("bFacialHairByRaceFolder", "Facial hair kept to its race", "Facial hair a race mod brings for its own heads is offered only to that race in the character menu."),
    ("bStripLodWater", "No hidden water under the Strip", "The distant water under the Strip's ground, which is never seen, is no longer made."),
    ("bDirectFiles", "More mods and larger files", "The game reads its files directly, without its limits of about 140 plugins and 2 GB for a file or an archive."),
    ("bCosaveKeep", "Other plugins' save data kept", "Save data of plugins not built in (JohnnyGuitar's, ShowOff's and others) is written back while the same files are loaded."),
    ("bJIPInstalled", "JIP LN NVSE built in", "Mods that need JIP LN NVSE (MCM, oHUD and others) find it built in, with its commands and game patches. Off turns JIP LN off: mods that need it then say so."),
    ("bJIPCosaveFix", "JIP save data fixes", "JIP LN's save data is written with the right sizes, as VNV's saves have it, and JIP's own damaged version is not read."),
    ("bJIPVanillaCmds", "JIP's versions of game commands", "Seven of the game's own script commands work as JIP LN changes them (for one, perk checks on followers)."),
    ("bJIPGamePatches", "JIP's changes to the game", "Some change play for everyone: JIP's hit damage, V.A.T.S. damage on you, vendors repairing to 100%, 2-second corner messages, Ctrl or Shift to take one or all."),
    ("bJGEditorIDs", "Forms keep their editor names", "Every form keeps its editor name, so the console and scripts made from text find things by name, as with JohnnyGuitar NVSE."),
    ("bUIO", "UI Organizer built in", "UI mods' HUD and menu additions (the lists in their uio folder) are put into the menus, as with the UIO plugin."),
    ("bNVSEErrorGuard", "Script error loop fix", "An error inside a mod script's own error handling is not sent back to it again, so it cannot go round forever."),
    ("bNVSEImodCheck", "Screen effect end check", "Mods hear that a screen effect ended only when it really ran out, not for depth of field or blur."),
    ("bNVSEInvRefCheck", "Script item and door checks", "A mod script asking about an item, door or container with parts missing gets 0 instead of stopping the game."),
    ("bNVSESizeCaps", "Script size limits", "Lists and texts a mod script makes stop at a safe size, and a bad text format no longer ends the game."),
    ("bNVSENullGuard", "Script missing list fix", "A mod script reading a list that does not exist, or a bad text, no longer stops the game."),
    ("bNVSECycleGuard", "Script list loop fix", "Copying, comparing or printing a script list that holds itself stops where it meets itself, instead of crashing."),
    ("bNVSEUISortPaths", "Mod menu list sorting fix", "Mods' menu lists sorted by something inside each item sort every item, not only the first."),
    ("bNVSEListGuard", "Script loops and long names fix", "Mod scripts that walk a list that loops back, scan too far, or pass a very long name no longer stop or hang the game."),
    ("bNVSEExtractArgs", "xNVSE expressions in scripts", "Mods' scripts written for xNVSE (expressions in the game's commands) run as with xNVSE; off, such a script can crash the game."),
    ("bHavokRefCounts", "No physics wait", "The game no longer waits for the physics step to end each time it takes up or lets go of a physics object, so frames stall less. Still being checked."),
];

/// The mark of a row still being tried, in its label.
pub const EXPERIMENTAL_MARK: &str = " (experimental)";

/// Rows still being tried outside the fix rows, marked in their labels: the sun shadows (the fix
/// rows' are the table's off in every preset, [`is_experimental`]). Ambient occlusion is being
/// tried too, but its name and the mark do not fit a row ([`LABEL_MAX`]): the list's note says it
/// ([`List::notes_on`]).
const EXPERIMENTAL: [&str; 1] = ["bSunShadows"];

/// Whether a table row is still being tried: one of [`EXPERIMENTAL`], or a fix row off in every
/// preset (the table's 0: the host's `OFF_IN_EVERY_PRESET`, which a host test holds to it).
fn is_experimental(k: &Key) -> bool {
    EXPERIMENTAL.contains(&k.key) || (k.section == FIXES_SECTION && k.default == "0")
}

/// The longest label a row of the table may have: its row, `"> "`, the label, `": "` and `Off`,
/// fits the highlight (695 wide) in the plain look, 18 a character (a test measures it).
pub const LABEL_MAX: usize = 31;

/// The longest text a row's choices screen shows under its first line: with that line it shows
/// whole in the plain look (a test checks every screen).
pub const ABOUT_MAX: usize = 160;

/// `value` without its note in brackets (`Vulkan (recommended)`: "Vulkan").
fn without_note(value: &str) -> &str {
    value.split(" (").next().unwrap_or(value)
}

/// Whether a word names a key or a value (`bUncap`, `iFrameLimit`, `NV_UNCAP`, `x=1`): never in
/// a text a player reads.
fn is_jargon(word: &str) -> bool {
    let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '=');
    let mut c = w.chars();
    let key = matches!(c.next(), Some('b' | 'f' | 'i' | 's'))
        && c.next().is_some_and(|c| c.is_ascii_uppercase())
        && w.chars().any(|c| c.is_ascii_lowercase());
    key || w.contains('=') || w.contains('_')
}

/// A short plain text from a table note, for a row with none of its own in [`PLAIN`]: what the
/// row does (after its name, or after `1 = `, up to `0 = `), its first sentence, without the parts
/// in brackets or a word that names a key or a value; at most [`ABOUT_MAX`] long, whole words.
fn plain_of(note: &str) -> String {
    let body = match note.strip_prefix("1 = ") {
        Some(on) => on.split("; 0 =").next().unwrap_or(on),
        None if name_in(note).is_some() => note.split_once(':').map_or(note, |(_, b)| b),
        None => note,
    };
    let mut text = String::new();
    let mut depth = 0usize;
    for c in body.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => text.push(c),
            _ => {}
        }
    }
    let first = text.split(". ").next().unwrap_or(&text);
    let first = first.split("; ").next().unwrap_or(first);
    let mut out = String::new();
    for word in first.split_whitespace().filter(|w| !is_jargon(w)) {
        let next = if out.is_empty() {
            word.to_string()
        } else if word.starts_with([',', '.', ';', ':']) {
            format!("{out}{word}")
        } else {
            format!("{out} {word}")
        };
        if next.chars().count() > ABOUT_MAX - 1 {
            break;
        }
        out = next;
    }
    let out = out.trim_end_matches([',', '.', ';', ':', ' ']);
    let mut chars = out.chars();
    match chars.next() {
        Some(c) => format!("{}{}.", c.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// The name a note starts with (`Uncapped frame rate: ...`), without a part in brackets (`JIP LN
/// NVSE built in (on in the fixes preset, ...): ...`); none for a note that says what a value
/// does (`1 = ...`) or has no name.
fn name_in(note: &str) -> Option<&str> {
    let (name, _) = note.split_once(':')?;
    let name = name.split(" (").next().unwrap_or(name).trim();
    (!name.is_empty() && !name.contains('=')).then_some(name)
}

/// A key's own words (`bCellEntryWait`: "Cell entry wait"), its type letter left out, at most
/// `max` long (whole words).
fn words_of(key: &str, max: usize) -> String {
    let chars: Vec<char> = key.chars().skip(1).collect();
    let mut words: Vec<String> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        let after_lower = i > 0 && chars[i - 1].is_ascii_lowercase();
        let before_lower = i > 0 && chars.get(i + 1).is_some_and(char::is_ascii_lowercase);
        if words.is_empty() || (c.is_ascii_uppercase() && (after_lower || before_lower)) {
            words.push(String::new());
        }
        if let Some(w) = words.last_mut() {
            w.push(c);
        }
    }
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        // A word in capitals (JIP, NVSE) stays as it is; the others in small letters after the first.
        let caps = w.chars().filter(char::is_ascii_uppercase).count() > 1;
        let w = if i == 0 || caps {
            w.clone()
        } else {
            w.to_ascii_lowercase()
        };
        let next = if out.is_empty() {
            w
        } else {
            format!("{out} {w}")
        };
        if next.chars().count() > max {
            break;
        }
        out = next;
    }
    out
}

/// Each table key's label: [`PLAIN`]'s name, else its note's name when it fits and no other key's
/// note has the same name, else its own words; with [`EXPERIMENTAL_MARK`] for a row still being
/// tried, all of it at most [`LABEL_MAX`] long.
fn labels_of(keys: &[Key]) -> Vec<String> {
    keys.iter()
        .map(|k| {
            let mark = if is_experimental(k) {
                EXPERIMENTAL_MARK
            } else {
                ""
            };
            let room = LABEL_MAX - mark.len();
            let name = match PLAIN.iter().find(|(key, _, _)| *key == k.key) {
                Some((_, label, _)) => label.to_string(),
                None => match name_in(k.note) {
                    Some(name)
                        if name.chars().count() <= room
                            && keys
                                .iter()
                                .filter(|o| name_in(o.note) == Some(name))
                                .count()
                                == 1 =>
                    {
                        name.to_string()
                    }
                    _ => words_of(k.key, room),
                },
            };
            format!("{name}{mark}")
        })
        .collect()
}

/// [`labels_of`] the host's table, in its order.
fn labels() -> &'static [String] {
    static LABELS_NOW: OnceLock<Vec<String>> = OnceLock::new();
    LABELS_NOW.get_or_init(|| labels_of(table()))
}

/// Rows that do something only while another row has one value: the row's key, the other row's
/// key (one of the twelve's, or a table row's) and the value it must show. The frame rate limits
/// and the menus' limit are read only by the uncapped step (host `fixes::Choice::frame_limit`,
/// `uncap::MENU_FIX`); JIP's commands and game patches take effect only once JIP is built in
/// (host `fixes::JIP_VANILLA_COMMANDS`, `JIP_GAME_PATCHES`); the new lighting is drawn by the
/// game's own renderer only, its strength and the sun rows used only with their switch on (host
/// `fixes::Choice`); the shaders are made ready by the real Direct3D 9 renderer only.
const NEEDS: [(&str, &str, &str); 15] = [
    ("iFrameLimit", "bUncap", "On"),
    ("iMenuFrameLimit", "bUncap", "On"),
    ("iMenuFrameLimit", "bMenuCap", "On"),
    ("bMenuCap", "bUncap", "On"),
    ("bShaderWarm", "sRenderer", DIRECT3D_9),
    ("bJIPVanillaCmds", "bJIPInstalled", "On"),
    ("bJIPGamePatches", "bJIPInstalled", "On"),
    ("bAmbientOcclusion", "sRenderer", OWN_RENDERER),
    ("fAmbientOcclusionStrength", "sRenderer", OWN_RENDERER),
    ("fAmbientOcclusionStrength", "bAmbientOcclusion", "On"),
    ("bSunShadows", "sRenderer", OWN_RENDERER),
    ("bSunLightFromSky", "sRenderer", OWN_RENDERER),
    ("bSunLightFromSky", "bSunShadows", "On"),
    ("bSunShadowsCharacters", "sRenderer", OWN_RENDERER),
    ("bSunShadowsCharacters", "bSunShadows", "On"),
];

impl Row {
    /// One of the twelve's key in the host's table, for the ones that write one.
    fn table_key(self) -> Option<&'static str> {
        Some(match self {
            Row::Renderer => "sRenderer",
            Row::DisplayMode => "bBorderless",
            Row::GameFixes => "sPreset",
            Row::FrameLimit => "iFrameLimit",
            Row::MenuFrameLimit => "iMenuFrameLimit",
            Row::FrameRateCounter => "bShowFrameRate",
            Row::GameThreads => "bSteppedThreads",
            Row::DrawingThread => "bRenderThread",
            _ => return None,
        })
    }

    /// The row that reads a key of the host's table: one of the twelve, else the key's own row.
    pub fn of(key: &str) -> Option<Row> {
        ROWS.iter()
            .copied()
            .find(|r| r.table_key() == Some(key))
            .or_else(|| table().iter().find(|k| k.key == key).map(Row::Key))
    }

    /// Its key in the host's table, for the rows that have one.
    fn key(self) -> Option<&'static str> {
        match self {
            Row::Key(k) => Some(k.key),
            _ => self.table_key(),
        }
    }

    /// A table row's variable (its settings.ini line's name).
    pub fn variable(self) -> Option<&'static str> {
        match self {
            Row::Key(k) => Some(k.variable),
            _ => None,
        }
    }

    /// What a table row changes in the game, for a player: [`PLAIN`]'s text, else a plain short
    /// form of the table's note ([`plain_of`]).
    pub fn about(self) -> Option<String> {
        let Row::Key(k) = self else {
            return None;
        };
        Some(match PLAIN.iter().find(|(key, _, _)| *key == k.key) {
            Some((_, _, text)) => text.to_string(),
            None => plain_of(k.note),
        })
    }

    /// Why the row does nothing with `files`, when it does nothing: the first row of [`NEEDS`]
    /// that does not show the value it needs ("Does nothing while Uncapped frame rate is Off.").
    pub fn idle_in(self, files: &Files) -> Option<String> {
        let key = self.key()?;
        NEEDS
            .iter()
            .filter(|(k, _, _)| *k == key)
            .find_map(|(_, parent, wanted)| {
                let parent = Row::of(parent)?;
                let now = parent.current(files);
                (now != *wanted).then(|| {
                    format!(
                        "Does nothing while {} is {}.",
                        without_note(parent.label()),
                        without_note(&now)
                    )
                })
            })
    }
}

/// Whether a table key is on or off (a `b` key).
fn is_flag(k: &Key) -> bool {
    k.key.starts_with('b')
}

/// A table key's value as the game plays it when the file gives none it can use: a fix row's
/// follows the preset (off under `sPreset=vanilla`, the host's rule for a fix row without its
/// line), any other the host table's default.
fn key_default(files: &Files, k: &Key) -> &'static str {
    if k.section == FIXES_SECTION
        && ours(files, "nv-rs General", "sPreset").eq_ignore_ascii_case("vanilla")
    {
        "0"
    } else {
        k.default
    }
}

/// A table key's value in the profile's Fallout.ini when the game can use it (a `b` key 0 or 1,
/// an `f` key a finite number, anything not empty otherwise), as the host reads it.
fn key_value<'a>(files: &'a Files, k: &Key) -> Option<&'a str> {
    let raw = ini_value(&files.ini, k.section, k.key)
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    match k.key.as_bytes().first() {
        Some(b'b') => ["0", "1"].contains(&raw).then_some(raw),
        Some(b'f') => raw.parse::<f32>().is_ok_and(f32::is_finite).then_some(raw),
        _ => Some(raw),
    }
}

/// The value of a table key the game plays with: the file's, else [`key_default`]; a number row's
/// held to its range as the game holds it, and shown as the step it equals ([`NUMBERS`]).
fn key_now(files: &Files, k: &Key) -> String {
    let now = key_value(files, k).unwrap_or_else(|| key_default(files, k));
    let Some((_, steps, (low, high))) = NUMBERS.iter().find(|(key, _, _)| *key == k.key) else {
        return now.to_string();
    };
    let Ok(n) = now.parse::<f32>() else {
        return now.to_string();
    };
    let n = n.clamp(*low, *high);
    steps
        .iter()
        .find(|s| s.parse::<f32>() == Ok(n))
        .map_or_else(|| n.to_string(), |s| s.to_string())
}

/// "On" for 1, "Off" for anything else.
fn on_off(flag: &str) -> &'static str {
    if flag == "1" {
        "On"
    } else {
        "Off"
    }
}

/// The number rows' choices and the range the game holds each to: the lighting strength's is the
/// host's (`d3d9_wgpu::scene_light::STRENGTH_RANGE`, 0.1 to 1.0, which `clamp_strength` holds a
/// value to) in four steps, its default (`STRENGTH`, 0.7) among them.
#[allow(clippy::type_complexity)]
const NUMBERS: [(&str, &[&str], (f32, f32)); 1] = [(
    "fAmbientOcclusionStrength",
    &["0.1", "0.4", "0.7", "1.0"],
    (0.1, 1.0),
)];

/// The Renderer row's choice for the game's own renderer (wgpu): on Windows Vulkan, recommended
/// over the original's Direct3D 9; on Linux Vulkan and on macOS Metal, the only renderer there.
pub const OWN_RENDERER: &str = if cfg!(windows) {
    "Vulkan (recommended)"
} else if cfg!(target_os = "macos") {
    "Metal"
} else {
    "Vulkan"
};

/// The original's renderer, a choice on Windows only (the real Direct3D 9 is a Windows API).
pub const DIRECT3D_9: &str = "Direct3D 9 (the original's)";

/// The rows in Settings.ps1's order.
pub const ROWS: [Row; 12] = [
    Row::Renderer,
    Row::DisplayMode,
    Row::Resolution,
    Row::AntiAliasing,
    Row::Anisotropy,
    Row::VSync,
    Row::GameFixes,
    Row::FrameLimit,
    Row::MenuFrameLimit,
    Row::FrameRateCounter,
    Row::GameThreads,
    Row::DrawingThread,
];

/// The host table's default for one of our keys.
fn default_of(key: &str) -> &'static str {
    table()
        .iter()
        .find(|k| k.key == key)
        .map_or("", |k| k.default)
}

/// Our key's value in the profile's Fallout.ini, else the table's default.
fn ours(files: &Files, section: &str, key: &str) -> String {
    ini_value(&files.ini, section, key)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default_of(key))
        .trim()
        .to_string()
}

/// "2x" for a sample count, "Off" for 0, the game's own for anything else.
fn times(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some("0") => "Off".into(),
        Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => format!("{n}x"),
        _ => OWN.into(),
    }
}

impl Row {
    /// Its name on the screen (Settings.ps1's labels).
    pub fn label(self) -> &'static str {
        match self {
            Row::Renderer => "Renderer",
            Row::DisplayMode => "Display mode",
            Row::Resolution => "Resolution",
            Row::AntiAliasing => "Anti-aliasing",
            Row::Anisotropy => "Anisotropic filtering",
            Row::VSync => "V-sync",
            Row::GameFixes => "Game fixes",
            Row::FrameLimit => "Frame rate limit",
            Row::MenuFrameLimit => "Frame rate limit in menus",
            Row::FrameRateCounter => "Frame rate counter",
            Row::GameThreads => "Game threads",
            Row::DrawingThread => "Drawing thread",
            // A table row's label ([`labels_of`]); the key itself for one not in the table.
            Row::Key(k) => table()
                .iter()
                .position(|t| std::ptr::eq(t, k))
                .and_then(|i| labels().get(i))
                .map_or(k.key, String::as_str),
        }
    }

    /// A fix row's value when the file chooses it here (its key there, with a value the game can
    /// use), and so does not follow the preset; none for one that follows it, and for any other row.
    pub fn chosen_here(self, files: &Files) -> Option<&'static str> {
        match self {
            Row::Key(k) if k.section == FIXES_SECTION => {
                key_value(files, k).map(|v| if v == "1" { "1" } else { "0" })
            }
            _ => None,
        }
    }

    /// The choices screen's first line: what the row is now, for a fix row whether that comes
    /// from the preset or was chosen here (and then the choice that follows the preset again,
    /// [`Row::hands_back`]), and why the row does nothing when it does nothing ([`Row::idle_in`]).
    pub fn now_line(self, files: &Files) -> String {
        let now = self.current(files);
        let preset = Row::GameFixes.label();
        let mut more = Vec::new();
        let line = match self {
            Row::Key(k) => match self.chosen_here(files) {
                None if k.section == FIXES_SECTION => format!("Now: {now}, from {preset}"),
                None => format!("Now: {now}"),
                Some(_) => {
                    more.push(format!(
                        "Choose {} to follow {preset}.",
                        on_off(key_default(files, k))
                    ));
                    format!("Now: {now}, chosen here")
                }
            },
            _ => format!("Now: {now}"),
        };
        more.extend(self.idle_in(files));
        if more.is_empty() {
            line
        } else {
            format!("{line}. {}", more.join(" "))
        }
    }

    /// The settings.ini line that carries this table row's choice in `chosen` (what the files
    /// say once it is made) while a profile is not ready yet. A fix row has one only when it is
    /// chosen here, or when `held` (a Fallout.ini this settings.ini reaches has the key, which
    /// only a line can change): then the line hands it back to the preset
    /// ([`game_profile::game_ini::FOLLOWS_PRESET`]); with none, a fix follows the preset.
    pub fn pending_line(self, chosen: &Files, held: bool) -> Option<String> {
        let Row::Key(k) = self else {
            return None;
        };
        let value = match self.chosen_here(chosen) {
            Some(v) => v.to_string(),
            None if k.section == FIXES_SECTION => {
                return held.then(|| format!("{}={FOLLOWS_PRESET}", k.variable));
            }
            None if is_flag(k) => (if self.current(chosen) == "On" {
                "1"
            } else {
                "0"
            })
            .to_string(),
            None => self.current(chosen),
        };
        Some(format!("{}={value}", k.variable))
    }

    /// Whether `choice` on this row hands a fix chosen here back to the preset: the file has its
    /// own line, and the choice is the value the preset gives (which may be the line's own).
    fn hands_back(self, files: &Files, choice: &str) -> bool {
        match self {
            Row::Key(k) if k.section == FIXES_SECTION && is_flag(k) => {
                self.chosen_here(files).is_some()
                    && (if choice == "On" { "1" } else { "0" }) == key_default(files, k)
            }
            _ => false,
        }
    }

    /// Its choices, the recommended one first where Settings.ps1 marks one; `desktop` is the
    /// main monitor's size for the resolution row. A value the list lacks (written by hand) is
    /// kept as a choice of its own, as Settings.ps1 keeps it.
    pub fn choices(self, files: &Files, desktop: (u32, u32)) -> Vec<String> {
        self.choices_on(files, desktop, cfg!(windows))
    }

    /// [`Row::choices`] as on Windows (`windows`) or not.
    pub fn choices_on(self, files: &Files, desktop: (u32, u32), windows: bool) -> Vec<String> {
        let fixed: &[&str] = match self {
            Row::Renderer if windows => &[OWN_RENDERER, DIRECT3D_9],
            // Off Windows the game's own renderer only: the real Direct3D 9 is a Windows API.
            Row::Renderer => &[OWN_RENDERER],
            Row::DisplayMode => &["Borderless fullscreen", "Window"],
            Row::Resolution => &[],
            Row::AntiAliasing => &[OWN, "Off", "2x", "4x", "8x"],
            Row::Anisotropy => &[OWN, "Off", "2x", "4x", "8x", "15x"],
            Row::VSync => &[OWN, "On", "Off"],
            Row::GameFixes => &["Fixes (recommended)", "Vanilla (the original game)"],
            Row::FrameLimit => &["No limit", "60", "90", "120", "144", "165", "240"],
            Row::MenuFrameLimit => &["No limit", "30", "60", "90", "120", "144", "165", "240"],
            Row::FrameRateCounter => &["Hidden (F11 shows it)", "Shown (F11 hides it)"],
            Row::GameThreads => &["Side by side (recommended)", "One at a time (slower)"],
            Row::DrawingThread => &["Own thread (recommended)", "Main thread (slower)"],
            // A table row on or off: the value the game plays with when nothing chooses it first
            // (a fix row's preset value).
            Row::Key(k) if is_flag(k) => {
                if key_default(files, k) == "1" {
                    &["On", "Off"]
                } else {
                    &["Off", "On"]
                }
            }
            // A number or a word: its listed steps, else its default (the file's own value is
            // added below).
            Row::Key(k) => NUMBERS
                .iter()
                .find(|(key, _, _)| *key == k.key)
                .map_or(std::slice::from_ref(&k.default), |(_, steps, _)| *steps),
        };
        let mut list: Vec<String> = if self == Row::Resolution {
            // The monitor's own size first, when it is known (never 0x0).
            let native =
                (desktop.0 > 0 && desktop.1 > 0).then(|| format!("{}x{}", desktop.0, desktop.1));
            let mut l = vec![OWN.to_string()];
            for s in [
                native.as_deref().unwrap_or("3840x2160"),
                "3840x2160",
                "2560x1440",
                "1920x1080",
                "1600x900",
                "1280x720",
            ] {
                if !l.iter().any(|x| x == s) {
                    l.push(s.to_string());
                }
            }
            l
        } else {
            fixed.iter().map(|s| s.to_string()).collect()
        };
        let now = self.current(files);
        // A profile that holds Direct3D 9 off Windows shows it as its value, never as a choice.
        let refused = self == Row::Renderer && now == DIRECT3D_9 && !windows;
        if !list.contains(&now) && !refused {
            list.push(now);
        }
        list
    }

    /// The choice the files say now.
    pub fn current(self, files: &Files) -> String {
        let display = |key: &str| ini_value(&files.prefs, "Display", key);
        match self {
            Row::Renderer => {
                if ours(files, "nv-rs Display", "sRenderer").eq_ignore_ascii_case("d3d9") {
                    DIRECT3D_9.into()
                } else {
                    OWN_RENDERER.into()
                }
            }
            Row::DisplayMode => {
                if ours(files, "nv-rs Display", "bBorderless") == "0" {
                    "Window".into()
                } else {
                    "Borderless fullscreen".into()
                }
            }
            Row::Resolution => match (display("iSize W"), display("iSize H")) {
                (Some(w), Some(h))
                    if [w, h]
                        .iter()
                        .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())) =>
                {
                    format!("{w}x{h}")
                }
                _ => OWN.into(),
            },
            Row::AntiAliasing => times(display("iMultiSample")),
            Row::Anisotropy => times(display("iMaxAnisotropy")),
            Row::VSync => match ini_value(&files.ini, "Display", "iPresentInterval").map(str::trim)
            {
                Some("1") => "On".into(),
                Some("0") => "Off".into(),
                _ => OWN.into(),
            },
            Row::GameFixes => {
                if ours(files, "nv-rs General", "sPreset").eq_ignore_ascii_case("vanilla") {
                    "Vanilla (the original game)".into()
                } else {
                    "Fixes (recommended)".into()
                }
            }
            Row::FrameLimit => match ours(files, "nv-rs Display", "iFrameLimit").as_str() {
                "0" => "No limit".into(),
                v => v.into(),
            },
            Row::MenuFrameLimit => match ours(files, "nv-rs Display", "iMenuFrameLimit").as_str() {
                "0" => "No limit".into(),
                v => v.into(),
            },
            Row::FrameRateCounter => {
                if ours(files, "nv-rs Display", "bShowFrameRate") == "1" {
                    "Shown (F11 hides it)".into()
                } else {
                    "Hidden (F11 shows it)".into()
                }
            }
            Row::GameThreads => {
                if ours(files, "nv-rs Threads", "bSteppedThreads") == "1" {
                    "One at a time (slower)".into()
                } else {
                    "Side by side (recommended)".into()
                }
            }
            Row::DrawingThread => {
                if ours(files, "nv-rs Threads", "bRenderThread") == "0" {
                    "Main thread (slower)".into()
                } else {
                    "Own thread (recommended)".into()
                }
            }
            Row::Key(k) if is_flag(k) => on_off(&key_now(files, k)).into(),
            Row::Key(k) => key_now(files, k),
        }
    }

    /// Whether `choice` may be saved for this row on this system: never Direct3D 9 off Windows, and
    /// never a resolution with a side of 0 (or one that is not a number).
    pub fn allows(self, choice: &str) -> bool {
        self.allows_on(choice, cfg!(windows))
    }

    /// [`Row::allows`] as on Windows (`windows`) or not.
    pub fn allows_on(self, choice: &str, windows: bool) -> bool {
        match self {
            Row::Renderer => windows || !choice.starts_with("Direct3D 9"),
            Row::Resolution => choice
                .split_once('x')
                .is_some_and(|(w, h)| [w, h].iter().all(|v| v.parse::<u32>().is_ok_and(|n| n > 0))),
            // A table row: a value the game can read there ([`key_value`]'s rule).
            Row::Key(k) => match k.key.as_bytes().first() {
                Some(b'b') => choice == "On" || choice == "Off",
                Some(b'f') => choice.trim().parse::<f32>().is_ok_and(f32::is_finite),
                Some(b'i') => choice.trim().parse::<u32>().is_ok(),
                _ => !choice.trim().is_empty() && !choice.chars().any(char::is_control),
            },
            _ => true,
        }
    }

    /// The files with `choice` written into this row's keys (and nothing else changed); the
    /// files as they are for "Game's setting", the choice already in them (but a fix chosen here
    /// given the preset's value, which hands it back: [`Row::hands_back`]), or one this system
    /// does not allow ([`Row::allows`]).
    pub fn apply(self, files: &Files, choice: &str) -> Files {
        if choice == OWN
            || !self.allows(choice)
            || (choice == self.current(files) && !self.hands_back(files, choice))
        {
            return files.clone();
        }
        let mut out = files.clone();
        let mut ini = |section: &str, key: &str, value: &str| {
            out.ini = set_key(&out.ini, section, key, value);
        };
        let first = |s: &str| s.split_whitespace().next().unwrap_or("").to_string();
        match self {
            Row::Renderer => ini(
                "nv-rs Display",
                "sRenderer",
                if choice.starts_with("Direct3D 9") {
                    "d3d9"
                } else {
                    "vulkan"
                },
            ),
            Row::DisplayMode => ini(
                "nv-rs Display",
                "bBorderless",
                if choice == "Window" { "0" } else { "1" },
            ),
            Row::VSync => ini(
                "Display",
                "iPresentInterval",
                if choice == "On" { "1" } else { "0" },
            ),
            Row::GameFixes => ini(
                "nv-rs General",
                "sPreset",
                if choice.starts_with("Vanilla") {
                    "vanilla"
                } else {
                    "fixes"
                },
            ),
            Row::FrameLimit | Row::MenuFrameLimit => {
                let key = if self == Row::FrameLimit {
                    "iFrameLimit"
                } else {
                    "iMenuFrameLimit"
                };
                let value = if choice == "No limit" {
                    "0".to_string()
                } else {
                    first(choice)
                };
                ini("nv-rs Display", key, &value)
            }
            Row::FrameRateCounter => ini(
                "nv-rs Display",
                "bShowFrameRate",
                if choice.starts_with("Shown") {
                    "1"
                } else {
                    "0"
                },
            ),
            Row::GameThreads => ini(
                "nv-rs Threads",
                "bSteppedThreads",
                if choice.starts_with("One at a time") {
                    "1"
                } else {
                    "0"
                },
            ),
            Row::DrawingThread => ini(
                "nv-rs Threads",
                "bRenderThread",
                if choice.starts_with("Main thread") {
                    "0"
                } else {
                    "1"
                },
            ),
            Row::Resolution => {
                if let Some((w, h)) = choice.split_once('x') {
                    out.prefs = set_key(&out.prefs, "Display", "iSize W", w);
                    out.prefs = set_key(&out.prefs, "Display", "iSize H", h);
                }
            }
            Row::Key(k) => {
                let value = match (is_flag(k), choice) {
                    (true, "On") => "1",
                    (true, _) => "0",
                    (false, c) => c.trim(),
                };
                // A fix row given the preset's value follows the preset again: its line back
                // to the generated note, so a later change of "Game fixes" moves it too.
                if k.section == FIXES_SECTION && value == key_default(files, k) {
                    out.ini = unset_key(&out.ini, k.section, k.key, k.default);
                } else {
                    ini(k.section, k.key, value);
                }
            }
            Row::AntiAliasing | Row::Anisotropy => {
                let key = if self == Row::AntiAliasing {
                    "iMultiSample"
                } else {
                    "iMaxAnisotropy"
                };
                let value = if choice == "Off" {
                    "0"
                } else {
                    choice.trim_end_matches('x')
                };
                out.prefs = set_key(&out.prefs, "Display", key, value);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(ini: &str, prefs: &str) -> Files {
        Files {
            ini: ini.into(),
            prefs: prefs.into(),
        }
    }

    /// With none of our keys, each row shows the host table's default, as the game plays it.
    #[test]
    fn missing_keys_show_the_tables_defaults() {
        let f = files("", "");
        let shown: Vec<String> = ROWS.iter().map(|r| r.current(&f)).collect();
        assert_eq!(
            shown,
            [
                OWN_RENDERER,
                "Borderless fullscreen",
                OWN,
                OWN,
                OWN,
                OWN,
                "Fixes (recommended)",
                "No limit",
                "120",
                "Hidden (F11 shows it)",
                "Side by side (recommended)",
                "Own thread (recommended)",
            ]
        );
    }

    /// A change writes only that row's keys, in place; every other byte stays.
    #[test]
    fn a_change_writes_its_own_keys_only() {
        let f = files(
            "[Display]\r\niPresentInterval=1\r\n[nv-rs Display]\r\n;note\r\nsRenderer=vulkan\r\niFrameLimit=0\r\n",
            "[Display]\r\niSize W=1920\r\niSize H=1080\r\niMultiSample=4\r\n",
        );
        let r = Row::Renderer.apply(&f, DIRECT3D_9);
        if cfg!(windows) {
            assert_eq!(r.ini, f.ini.replace("sRenderer=vulkan", "sRenderer=d3d9"));
        } else {
            assert_eq!(r, f, "no Direct3D 9 off Windows");
        }
        assert_eq!(r.prefs, f.prefs);
        let r = Row::FrameLimit.apply(&f, "144");
        assert_eq!(r.ini, f.ini.replace("iFrameLimit=0", "iFrameLimit=144"));
        let r = Row::VSync.apply(&f, "Off");
        assert_eq!(
            r.ini,
            f.ini.replace("iPresentInterval=1", "iPresentInterval=0")
        );
        let r = Row::Resolution.apply(&f, "2560x1440");
        assert_eq!(
            r.prefs,
            f.prefs.replace("1920", "2560").replace("1080", "1440")
        );
        let r = Row::AntiAliasing.apply(&f, "Off");
        assert_eq!(r.prefs, f.prefs.replace("iMultiSample=4", "iMultiSample=0"));
        assert_eq!(Row::AntiAliasing.current(&r), "Off");
        // "Game's setting", and the value already there, write nothing.
        assert_eq!(Row::Anisotropy.apply(&f, OWN), f);
        assert_eq!(Row::Renderer.apply(&f, OWN_RENDERER), f);
        // A resolution with a side of 0 is never written.
        assert_eq!(Row::Resolution.apply(&f, "0x0"), f);
        assert_eq!(Row::Resolution.apply(&f, "1280x0"), f);
    }

    /// A value written by hand is kept as a choice of its own.
    #[test]
    fn a_hand_written_value_is_a_choice() {
        let f = files(
            "[nv-rs Display]\nsRenderer=VULKAN\niFrameLimit=75\n",
            "[Display]\niSize W=1366\niSize H=768\n",
        );
        assert!(Row::FrameLimit
            .choices(&f, (2560, 1440))
            .contains(&"75".to_string()));
        let res = Row::Resolution.choices(&f, (2560, 1440));
        assert_eq!(res[0], OWN);
        assert_eq!(res[1], "2560x1440");
        assert_eq!(res.last().map(String::as_str), Some("1366x768"));
        assert_eq!(Row::Renderer.current(&f), OWN_RENDERER);
        // No monitor size known: no 0x0 among the choices.
        let res = Row::Resolution.choices(&f, (0, 0));
        assert!(
            res.iter()
                .all(|r| !r.starts_with("0x") && !r.ends_with("x0")),
            "{res:?}"
        );
    }

    /// The Renderer row offers the original's Direct3D 9 on Windows only; elsewhere the game's own
    /// renderer by its system's name (Vulkan, Metal on macOS) is the one choice, and a profile that
    /// already holds d3d9 there shows it as its value, never offered, and choosing the one choice
    /// moves it off.
    #[test]
    fn direct3d_9_is_a_choice_on_windows_only() {
        let plain = files("[nv-rs Display]\r\nsRenderer=vulkan\r\n", "");
        let held = files("[nv-rs Display]\r\nsRenderer=d3d9\r\n", "");
        let size = (1920, 1080);
        assert_eq!(Row::Renderer.current(&held), DIRECT3D_9);
        // Windows' rule, and every other system's, checked on any system.
        assert_eq!(
            Row::Renderer.choices_on(&plain, size, true),
            [OWN_RENDERER, DIRECT3D_9]
        );
        assert!(Row::Renderer.allows_on(DIRECT3D_9, true));
        assert_eq!(
            Row::Renderer.choices_on(&plain, size, false),
            [OWN_RENDERER]
        );
        assert_eq!(Row::Renderer.choices_on(&held, size, false), [OWN_RENDERER]);
        assert!(!Row::Renderer.allows_on(DIRECT3D_9, false));
        assert!(Row::Renderer.allows_on(OWN_RENDERER, false));
        let moved = Row::Renderer.apply(&held, OWN_RENDERER);
        assert_eq!(Row::Renderer.current(&moved), OWN_RENDERER);
        if cfg!(windows) {
            assert_eq!(OWN_RENDERER, "Vulkan (recommended)");
        } else {
            assert_eq!(Row::Renderer.apply(&plain, DIRECT3D_9), plain);
            let name = if cfg!(target_os = "macos") {
                "Metal"
            } else {
                "Vulkan"
            };
            assert_eq!(OWN_RENDERER, name);
        }
    }

    /// A fresh profile's Fallout.ini: the generated sections (the fix lines notes, so the preset
    /// decides) above a line of the game's own.
    fn generated(preset: &str) -> Files {
        let text = game_profile::game_ini::with_sections("[Display]\r\niPresentInterval=1\r\n")
            .unwrap()
            .text;
        files(&set_key(&text, "nv-rs General", "sPreset", preset), "")
    }

    /// The table row of `key`.
    fn row(key: &str) -> Row {
        Row::of(key).unwrap_or_else(|| panic!("{key} is not in the host's table"))
    }

    /// Every line of the host's table is one row, in one list: one of the twelve reads it, or
    /// it is a fix row (Fixes one by one), or it is in New lighting and more, in the table's
    /// order; only a key that does nothing in play is left out, and off Windows a key that
    /// changes nothing there. The launcher's own lists name only keys the table has.
    #[test]
    fn every_line_of_the_table_is_one_row() {
        let keys: Vec<&str> = TABLE
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.split('\t').nth(1).unwrap())
            .collect();
        assert_eq!(keys.len(), table().len());
        assert!(keys.len() > 40, "{}", keys.len());
        for windows in [true, false] {
            let more = List::More.rows_on(windows);
            let fixes = List::Fixes.rows_on(windows);
            let mut seen: Vec<&str> = ROWS.iter().filter_map(|r| r.table_key()).collect();
            for r in more.iter().chain(&fixes) {
                let Row::Key(k) = r else {
                    panic!("{r:?} is not a table row");
                };
                seen.push(k.key);
            }
            let mut left: Vec<&str> = keys.clone();
            left.retain(|k| !HIDDEN.contains(k));
            if !windows {
                left.retain(|k| !WINDOWS_ONLY.contains(k));
            }
            let mut sorted = seen.clone();
            sorted.sort_unstable();
            left.sort_unstable();
            assert_eq!(sorted, left, "windows {windows}");
            // In the table's order, fix rows only in their own list.
            let order = |rows: &[Row]| -> Vec<usize> {
                rows.iter()
                    .map(|r| match r {
                        Row::Key(k) => keys.iter().position(|x| x == &k.key).unwrap(),
                        _ => unreachable!(),
                    })
                    .collect()
            };
            assert!(order(&more).is_sorted() && order(&fixes).is_sorted());
            assert!(fixes
                .iter()
                .all(|r| matches!(r, Row::Key(k) if k.section == FIXES_SECTION)));
            assert!(more
                .iter()
                .all(|r| matches!(r, Row::Key(k) if k.section != FIXES_SECTION)));
        }
        let fix_lines = TABLE
            .lines()
            .filter(|l| l.starts_with("nv-rs Fixes\t"))
            .filter(|l| !HIDDEN.iter().any(|k| l.contains(&format!("\t{k}\t"))))
            .filter(|l| {
                cfg!(windows) || !WINDOWS_ONLY.iter().any(|k| l.contains(&format!("\t{k}\t")))
            })
            .count();
        assert_eq!(List::Fixes.rows().len(), fix_lines);
        assert_eq!(
            List::More.rows().len(),
            List::More.rows_on(cfg!(windows)).len()
        );
        assert_eq!(List::Main.rows(), ROWS);
        // The world drawn on a second thread does nothing in play: in no list on any system.
        for windows in [true, false] {
            assert!(!List::Fixes
                .rows_on(windows)
                .contains(&row("bWorldDrawThread")));
        }
        assert!(!List::Fixes.rows_on(false).contains(&row("bShaderWarm")));
        assert!(List::Fixes.rows_on(true).contains(&row("bShaderWarm")));
        for key in PLAIN
            .iter()
            .map(|(k, _, _)| *k)
            .chain(NUMBERS.iter().map(|(k, _, _)| *k))
            .chain(WINDOWS_ONLY)
            .chain(HIDDEN)
            .chain(EXPERIMENTAL)
            .chain(NEEDS.iter().flat_map(|(k, p, _)| [*k, *p]))
        {
            assert!(keys.contains(&key), "{key} is not in the host's table");
        }
        // The new lighting's switches and the background row are there (the last on Windows only).
        let more_keys: Vec<&str> = List::More
            .rows_on(true)
            .iter()
            .filter_map(|r| match r {
                Row::Key(k) => Some(k.key),
                _ => None,
            })
            .collect();
        for key in [
            "bAmbientOcclusion",
            "fAmbientOcclusionStrength",
            "bSunShadows",
            "bSunLightFromSky",
            "bSunShadowsCharacters",
            "bNotBackground",
        ] {
            assert!(more_keys.contains(&key), "{key}: {more_keys:?}");
        }
        assert!(!List::More.rows_on(false).contains(&row("bNotBackground")));
    }

    /// Each table row reads its own key as the game plays it and writes that key alone: with
    /// none, the table's default (a fix row's preset value, off under vanilla), offered first;
    /// the other choice written in place, everything else byte for byte; and a fix row chosen
    /// back to the preset's value is the generated note again, the file as it was.
    #[test]
    fn each_table_row_reads_and_writes_its_own_key() {
        for preset in ["fixes", "vanilla"] {
            let fresh = generated(preset);
            for &r in List::More.rows_on(true).iter().chain(List::Fixes.rows()) {
                let Row::Key(k) = r else { unreachable!() };
                assert_eq!(r.variable(), Some(k.variable));
                assert_ne!(r.about().as_deref(), Some(k.note));
                let fix = k.section == FIXES_SECTION;
                let plays = if fix && preset == "vanilla" {
                    "0"
                } else {
                    k.default
                };
                let shown = r.current(&fresh);
                let choices = r.choices_on(&fresh, (1920, 1080), true);
                if !is_flag(k) {
                    assert_eq!(shown, plays, "{}", k.key);
                    assert!(choices.contains(&shown), "{}: {choices:?}", k.key);
                    continue;
                }
                assert_eq!(shown, on_off(plays), "{} under {preset}", k.key);
                assert_eq!(choices[0], shown, "{}: the default first", k.key);
                assert_eq!(choices.len(), 2);
                let other = &choices[1];
                let changed = r.apply(&fresh, other);
                assert_eq!(r.current(&changed), *other, "{}", k.key);
                let value = if other == "On" { "1" } else { "0" };
                assert_eq!(
                    changed.ini,
                    set_key(&fresh.ini, k.section, k.key, value),
                    "{}: its key alone",
                    k.key
                );
                assert_eq!(changed.prefs, fresh.prefs);
                assert_eq!(r.chosen_here(&changed), fix.then_some(value));
                // Back to the value the game played: a fix row follows the preset again.
                let back = r.apply(&changed, &shown);
                assert_eq!(r.current(&back), shown, "{}", k.key);
                if fix {
                    assert_eq!(back, fresh, "{}: the generated note again", k.key);
                    assert_eq!(r.chosen_here(&back), None);
                } else {
                    assert_eq!(
                        back.ini,
                        set_key(&fresh.ini, k.section, k.key, plays),
                        "{}",
                        k.key
                    );
                }
                // The value already there, "Game's setting" and a word the row cannot take
                // write nothing.
                assert_eq!(r.apply(&fresh, &shown), fresh);
                assert_eq!(r.apply(&fresh, OWN), fresh);
                assert_eq!(r.apply(&fresh, "Maybe"), fresh);
            }
        }
    }

    /// A fix row follows "Game fixes" while the file has no usable line of its own: a vanilla
    /// preset turns it off, a value written here wins over either preset (as the host's
    /// `Choice::is_on` takes the line first), and the choices screen's first line says which.
    #[test]
    fn a_fix_row_follows_the_preset_until_chosen_here() {
        let uncap = row("bUncap");
        let fixes = generated("fixes");
        let vanilla = generated("vanilla");
        assert_eq!(uncap.current(&fixes), "On");
        assert_eq!(uncap.current(&vanilla), "Off");
        assert_eq!(uncap.now_line(&fixes), "Now: On, from Game fixes");
        let off = uncap.apply(&fixes, "Off");
        assert_eq!(
            uncap.now_line(&off),
            "Now: Off, chosen here. Choose On to follow Game fixes."
        );
        // The preset changed afterwards: the choice made here stays.
        let off_vanilla = Row::GameFixes.apply(&off, "Vanilla (the original game)");
        assert_eq!(uncap.current(&off_vanilla), "Off");
        assert_eq!(
            uncap.now_line(&off_vanilla),
            "Now: Off, chosen here. Choose Off to follow Game fixes."
        );
        // Choosing Off there, the value it shows and the one Vanilla gives, hands it back: the
        // generated note again, so the next preset change moves it.
        let back = uncap.apply(&off_vanilla, "Off");
        assert_eq!(back, vanilla, "the generated note again");
        assert_eq!(uncap.now_line(&back), "Now: Off, from Game fixes");
        let fixes_again = Row::GameFixes.apply(&back, "Fixes (recommended)");
        assert_eq!(fixes_again, fixes);
        assert_eq!(uncap.current(&fixes_again), "On");
        let on_vanilla = uncap.apply(&vanilla, "On");
        assert_eq!(
            uncap.now_line(&on_vanilla),
            "Now: On, chosen here. Choose Off to follow Game fixes."
        );
        let on_fixes = Row::GameFixes.apply(&on_vanilla, "Fixes (recommended)");
        assert_eq!(uncap.current(&on_fixes), "On");
        assert_eq!(
            uncap.now_line(&on_fixes),
            "Now: On, chosen here. Choose On to follow Game fixes."
        );
        assert_eq!(
            uncap.apply(&on_fixes, "On"),
            fixes,
            "handed back under Fixes too"
        );
        // A line that is not the preset's value, chosen again, stays as it is.
        assert_eq!(uncap.apply(&off, "Off"), off);
        // The preset's word in any case, as the host reads it.
        let shouted = files("[nv-rs General]\r\nsPreset=VANILLA\r\n", "");
        assert_eq!(uncap.current(&shouted), "Off");
        // A line the game cannot use (2, empty, a word) is no choice: the preset's value.
        for bad in ["2", "", "yes"] {
            let f = files(
                &format!("[nv-rs General]\r\nsPreset=vanilla\r\n[nv-rs Fixes]\r\nbUncap={bad}\r\n"),
                "",
            );
            assert_eq!(uncap.current(&f), "Off", "{bad:?}");
            assert_eq!(uncap.chosen_here(&f), None);
        }
        // A row off in every preset (the table's 0) is off with either.
        let havok = row("bHavokRefCounts");
        assert_eq!(havok.current(&fixes), "Off");
        assert_eq!(havok.choices(&fixes, (1, 1)), ["Off", "On"]);
        // Rows outside the fixes keep the table's default under vanilla: no preset of theirs.
        let sky = row("bSunLightFromSky");
        assert_eq!(sky.current(&vanilla), "On");
        assert_eq!(
            sky.now_line(&vanilla),
            "Now: On. Does nothing while Sun shadows is Off."
        );
        assert_eq!(sky.chosen_here(&sky.apply(&vanilla, "Off")), None);
        let lit = row("bSunShadows").apply(&vanilla, "On");
        assert_eq!(sky.now_line(&lit), "Now: On");
    }

    /// A row that works only with another says so on its first line while that one does not
    /// have its value: the frame rate limits and the menus' own without the uncapped frame rate,
    /// JIP's commands and patches without JIP, the new lighting with Direct3D 9 (on Windows), the
    /// shaders made ready with Vulkan; with the other row's value, nothing is said.
    #[test]
    fn a_row_that_needs_another_says_so() {
        let fixes = generated("fixes");
        let uncap_off = row("bUncap").apply(&fixes, "Off");
        let menu = row("bMenuCap");
        assert_eq!(menu.idle_in(&fixes), None);
        assert_eq!(
            menu.now_line(&uncap_off),
            "Now: On, from Game fixes. Does nothing while Uncapped frame rate is Off."
        );
        assert_eq!(
            Row::FrameLimit.now_line(&uncap_off),
            "Now: No limit. Does nothing while Uncapped frame rate is Off."
        );
        let no_menu_cap = menu.apply(&fixes, "Off");
        assert_eq!(
            Row::MenuFrameLimit.idle_in(&no_menu_cap).as_deref(),
            Some("Does nothing while Menus use their own limit is Off.")
        );
        let jip_off = row("bJIPInstalled").apply(&fixes, "Off");
        for key in ["bJIPVanillaCmds", "bJIPGamePatches"] {
            assert_eq!(
                row(key).idle_in(&jip_off).as_deref(),
                Some("Does nothing while JIP LN NVSE built in is Off."),
                "{key}"
            );
            assert_eq!(row(key).idle_in(&fixes), None, "{key}");
        }
        // JIP's co-save fixes act on its save data with JIP off too.
        assert_eq!(row("bJIPCosaveFix").idle_in(&jip_off), None);
        let strength = row("fAmbientOcclusionStrength");
        assert_eq!(
            strength.idle_in(&fixes).as_deref(),
            Some("Does nothing while Ambient occlusion is Off.")
        );
        let ao = row("bAmbientOcclusion").apply(&fixes, "On");
        assert_eq!(strength.idle_in(&ao), None);
        let warm = row("bShaderWarm");
        assert_eq!(
            warm.idle_in(&fixes).as_deref(),
            Some(
                format!(
                    "Does nothing while Renderer is {}.",
                    without_note(OWN_RENDERER)
                )
                .as_str()
            )
        );
        if cfg!(windows) {
            let d3d9 = Row::Renderer.apply(&ao, DIRECT3D_9);
            assert_eq!(warm.idle_in(&d3d9), None);
            assert_eq!(
                strength.idle_in(&d3d9).as_deref(),
                Some("Does nothing while Renderer is Direct3D 9.")
            );
        }
        // A row that needs nothing never says so.
        assert_eq!(row("bUncap").idle_in(&uncap_off), None);
        assert_eq!(Row::Resolution.now_line(&fixes), format!("Now: {OWN}"));
    }

    /// The lighting strength is a number: four steps of the host's range with its default, the
    /// file's own number kept as a choice, a word the game cannot read shown as the default.
    #[test]
    fn the_lighting_strength_is_a_number() {
        let s = row("fAmbientOcclusionStrength");
        let f = generated("fixes");
        assert_eq!(s.current(&f), "0.7");
        assert_eq!(s.choices(&f, (1, 1)), ["0.1", "0.4", "0.7", "1.0"]);
        let r = s.apply(&f, "0.4");
        assert_eq!(
            r.ini,
            f.ini.replace(
                "fAmbientOcclusionStrength=0.7",
                "fAmbientOcclusionStrength=0.4"
            )
        );
        assert_eq!(s.current(&r), "0.4");
        assert_eq!(s.apply(&f, "strong"), f);
        assert_eq!(s.apply(&f, "inf"), f);
        let own = s.apply(&f, "0.65");
        assert_eq!(s.current(&own), "0.65");
        assert_eq!(
            s.choices(&own, (1, 1)).last().map(String::as_str),
            Some("0.65")
        );
        let word = files(
            "[nv-rs Display]\r\nfAmbientOcclusionStrength=strong\r\n",
            "",
        );
        assert_eq!(s.current(&word), "0.7");
        // A number written by hand shows as the game plays it: held to 0.1..1.0, as the step it
        // equals (no fifth choice for 1 beside 1.0); one that is not finite is the default.
        let by_hand = |v: &str| {
            files(
                &format!("[nv-rs Display]\r\nfAmbientOcclusionStrength={v}\r\n"),
                "",
            )
        };
        for (v, shown) in [
            ("5", "1.0"),
            ("1", "1.0"),
            ("0.05", "0.1"),
            ("-2", "0.1"),
            ("inf", "0.7"),
        ] {
            assert_eq!(s.current(&by_hand(v)), shown, "{v}");
            assert_eq!(s.choices(&by_hand(v), (1, 1)).len(), 4, "{v}");
        }
        assert_eq!(s.current(&by_hand("0.650")), "0.65");
        assert_eq!(
            s.pending_line(&r, false).as_deref(),
            Some("NV_AO_STRENGTH=0.4")
        );
    }

    /// A shown row's label and text are the player's: [`PLAIN`]'s, no key names, values or
    /// engine words, each label unique in its list and fitting its row, a row still being tried
    /// marked; a row with none of its own takes its note's name when that fits and is no other
    /// row's, else its key's words, and a plain short form of its note.
    #[test]
    fn labels_and_texts_are_plain() {
        assert_eq!(
            name_in("Uncapped frame rate: the game's frame step"),
            Some("Uncapped frame rate")
        );
        assert_eq!(
            name_in("JIP LN NVSE built in (on in the fixes preset): mods"),
            Some("JIP LN NVSE built in")
        );
        assert_eq!(name_in("1 = new lighting: ambient occlusion"), None);
        assert_eq!(
            name_in("How strong the shade is; 0.7 is the default."),
            None
        );
        assert_eq!(words_of("bCellEntryWait", LABEL_MAX), "Cell entry wait");
        assert_eq!(words_of("bNVSEListGuard", LABEL_MAX), "NVSE list guard");
        assert_eq!(
            words_of("bTheVeryLongKeyNameThatGoesOnAndOnPastTheRow", LABEL_MAX),
            "The very long key name that"
        );
        // Two notes with one name, a name too long, a row off in every preset: each takes its
        // key's words (the last with the mark, in the room left); and a plain text of its note.
        let made_up = parse(concat!(
            "nv-rs Fixes\tbCrashFixInventoryUse\tNV_A\t1\tCrash fix: a click does nothing.\n",
            "nv-rs Fixes\tbCrashFixQuantityDrop\tNV_B\t1\tCrash fix: a drop drops nothing (bQuantity=1).\n",
            "nv-rs Fixes\tbShort\tNV_C\t1\tA short name: yes; with bUncap only.\n",
            "nv-rs Fixes\tbLongOneOfManyKeys\tNV_D\t0\tA name far too long to fit in its row at all: no.\n",
            "nv-rs Display\tbNew\tNV_E\t0\t1 = new: a shade (Vulkan only); 0 = none.\n",
        ));
        assert_eq!(
            labels_of(&made_up),
            [
                "Crash fix inventory use",
                "Crash fix quantity drop",
                "A short name",
                "Long one of many (experimental)",
                "New",
            ]
        );
        let texts: Vec<String> = made_up.iter().map(|k| plain_of(k.note)).collect();
        assert_eq!(
            texts,
            [
                "A click does nothing.",
                "A drop drops nothing.",
                "Yes.",
                "No.",
                "New: a shade.",
            ]
        );
        assert!(is_jargon("bUncap") && is_jargon("iMenuFrameLimit,") && is_jargon("NV_UNCAP"));
        assert!(is_jargon("x=1") && !is_jargon("bodies") && !is_jargon("JIP's"));
        assert_eq!(row("bUncap").label(), "Uncapped frame rate");
        assert_eq!(row("bJIPInstalled").label(), "JIP LN NVSE built in");
        // Ambient occlusion by the name players know; with the mark it would not fit its row, so
        // the list's note says the new lighting is still being tried.
        assert_eq!(row("bAmbientOcclusion").label(), "Ambient occlusion");
        assert!(format!("Ambient occlusion{EXPERIMENTAL_MARK}").len() > LABEL_MAX);
        assert_eq!(
            row("fAmbientOcclusionStrength").label(),
            "Ambient occlusion strength"
        );
        assert_eq!(row("bSunShadows").label(), "Sun shadows (experimental)");
        for windows in [true, false] {
            let notes = List::More.notes_on(windows);
            assert_eq!(notes[0], "The new lighting is still being tried.");
            assert_eq!(notes.len(), if windows { 2 } else { 1 });
            assert!(notes
                .iter()
                .all(|n| !n.contains("Vulkan") && !n.contains("Metal")));
        }
        assert_eq!(List::More.notes_on(true)[1], "Direct3D 9 does not draw it.");
        assert_eq!(
            row("bHavokRefCounts").label(),
            "No physics wait (experimental)"
        );
        assert!(row("bJIPInstalled")
            .about()
            .is_some_and(|t| t.contains("Off turns JIP LN off")));
        assert_eq!(Row::FrameLimit.about(), None);
        for (key, label, text) in PLAIN {
            for word in label.split_whitespace().chain(text.split_whitespace()) {
                assert!(!is_jargon(word), "{key}: {word}");
            }
            assert!(text.chars().count() <= ABOUT_MAX, "{key}: {}", text.len());
            assert!(text.ends_with('.'), "{key}");
            assert!(!text.contains(" = ") && !text.contains("Vulkan"), "{key}");
        }
        for list in [List::More, List::Fixes] {
            let rows = list.rows_on(true);
            let labels: Vec<&str> = rows.iter().map(|r| r.label()).collect();
            for (i, l) in labels.iter().enumerate() {
                assert!(!l.is_empty() && l.chars().count() <= LABEL_MAX, "{l:?}");
                assert!(!labels[..i].contains(l), "{l:?} twice in {list:?}");
                assert!(!ROWS.iter().any(|r| r.label() == *l), "{l:?}");
            }
            for r in rows {
                let Row::Key(k) = r else { unreachable!() };
                assert_eq!(
                    r.label().ends_with(EXPERIMENTAL_MARK),
                    is_experimental(k),
                    "{}",
                    k.key
                );
                // The text a row without its own would get is plain too.
                let fallback = plain_of(k.note);
                assert!(
                    !fallback.is_empty() && fallback.chars().count() <= ABOUT_MAX,
                    "{}",
                    k.key
                );
                assert!(
                    fallback.split_whitespace().all(|w| !is_jargon(w)),
                    "{}: {fallback}",
                    k.key
                );
            }
        }
    }

    /// While a profile is not ready, a table row's choice is its settings.ini line: always for a
    /// row outside the fixes, for a fix row only when chosen here or when a profile's own file
    /// holds the key (no line follows the preset).
    #[test]
    fn a_pending_line_carries_a_table_rows_choice() {
        let fixes = generated("fixes");
        let ao = row("bAmbientOcclusion");
        assert_eq!(ao.pending_line(&fixes, false).as_deref(), Some("NV_AO=0"));
        assert_eq!(
            ao.pending_line(&ao.apply(&fixes, "On"), false).as_deref(),
            Some("NV_AO=1")
        );
        let uncap = row("bUncap");
        assert_eq!(uncap.pending_line(&fixes, false), None);
        // Held in a file the line reaches: the line hands it back to the preset (a 0 or 1 would
        // pin it).
        assert_eq!(
            uncap.pending_line(&fixes, true).as_deref(),
            Some("NV_UNCAP=preset")
        );
        assert_eq!(
            uncap
                .pending_line(&uncap.apply(&fixes, "Off"), false)
                .as_deref(),
            Some("NV_UNCAP=0")
        );
        assert_eq!(Row::FrameLimit.pending_line(&fixes, true), None);
    }
}
