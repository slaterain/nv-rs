//! The devices a game pad reader leaves out, as SDL does, on every system: Steam's two lists for
//! the games it starts (`SDL_GAMECONTROLLER_IGNORE_DEVICES` and `..._EXCEPT`, [`SteamLists`]),
//! and SDL's own lists of devices that look like game controllers but are not, or are not to be
//! read ([`DeviceLists`], from SDL3's `src/joystick/SDL_joystick.c`, zlib: `blacklist_devices`,
//! `rog_gamepad_mice`, `SDL_IsJoystickUSBChargingOnly` and `SDL_ShouldIgnoreJoystick`). Pure
//! code; the readers ask with a pad's vendor and product ids.

use super::evdev::STEAM_VIRTUAL;

/// The pads Steam leaves out of the games it starts, as SDL reads the two lists
/// (`SDL_GAMECONTROLLER_IGNORE_DEVICES` and `..._EXCEPT`): each a list of `0xVVVV/0xPPPP` vendor
/// and product ids, or `@` and a file's path, the list being the file's text.
/// `allow_virtual`: `SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD`, which Steam sets for the
/// games it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SteamLists {
    pub ignore: Vec<(u16, u16)>,
    pub except: Vec<(u16, u16)>,
    pub allow_virtual: bool,
}

impl SteamLists {
    pub fn from_env() -> SteamLists {
        SteamLists {
            ignore: hint(std::env::var("SDL_GAMECONTROLLER_IGNORE_DEVICES").ok()),
            except: hint(std::env::var("SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT").ok()),
            allow_virtual: yes(
                std::env::var("SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD")
                    .ok()
                    .as_deref(),
                false,
            ),
        }
    }

    /// Whether the pad `id` is left out: never Steam's virtual pad; with an except list (one with
    /// an id in it), every pad not in it; otherwise the pads in the ignore list.
    pub fn leave_out(&self, id: (u16, u16)) -> bool {
        if id == STEAM_VIRTUAL {
            return false;
        }
        if !self.except.is_empty() {
            return !self.except.contains(&id);
        }
        self.ignore.contains(&id)
    }
}

/// A variable's yes or no by SDL's rules (`SDL_GetStringBoolean`): unset or empty is `default`; a
/// value starting with `0`, or `false` in any case, is no; anything else yes.
pub fn yes(value: Option<&str>, default: bool) -> bool {
    match value {
        None | Some("") => default,
        Some(v) => !(v.starts_with('0') || v.eq_ignore_ascii_case("false")),
    }
}

/// A list's text, or the file `@path` names.
pub fn hint(value: Option<String>) -> Vec<(u16, u16)> {
    match value {
        Some(v) => match v.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .map(|text| ids(&text))
                .unwrap_or_default(),
            None => ids(&v),
        },
        None => Vec::new(),
    }
}

/// The ids in a list's text as SDL finds them: each number written after `0x`, in hexadecimal,
/// taken in pairs (vendor, product), whatever lies between.
pub fn ids(text: &str) -> Vec<(u16, u16)> {
    let mut numbers = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("0x") {
        rest = &rest[at + 2..];
        let end = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());
        numbers.push(u32::from_str_radix(&rest[..end], 16).unwrap_or(0));
        rest = &rest[end..];
    }
    numbers
        .chunks_exact(2)
        .filter_map(|pair| Some((u16::try_from(pair[0]).ok()?, u16::try_from(pair[1]).ok()?)))
        .collect()
}

/// `initial_blacklist_devices`: keyboards, mice, tablets and other devices that present a
/// joystick collection without being one.
const BLACKLIST: &[(u16, u16)] = &[
    (0x045e, 0x009d),
    (0x045e, 0x00b0),
    (0x045e, 0x00b4),
    (0x045e, 0x0730),
    (0x045e, 0x0745),
    (0x045e, 0x0748),
    (0x045e, 0x0750),
    (0x045e, 0x0768),
    (0x045e, 0x0773),
    (0x045e, 0x07a5),
    (0x045e, 0x07b2),
    (0x045e, 0x0800),
    (0x046d, 0xc30a),
    (0x04d9, 0xa0df),
    (0x056a, 0x0010),
    (0x056a, 0x0011),
    (0x056a, 0x0012),
    (0x056a, 0x0013),
    (0x056a, 0x0014),
    (0x056a, 0x0015),
    (0x056a, 0x0016),
    (0x056a, 0x0017),
    (0x056a, 0x0018),
    (0x056a, 0x0019),
    (0x056a, 0x00d1),
    (0x056a, 0x030e),
    (0x09da, 0x054f),
    (0x09da, 0x1410),
    (0x09da, 0x3043),
    (0x09da, 0x31b5),
    (0x09da, 0x3997),
    (0x09da, 0x3f8b),
    (0x09da, 0x51f4),
    (0x09da, 0x5589),
    (0x09da, 0x7b22),
    (0x09da, 0x7f2d),
    (0x09da, 0x8090),
    (0x09da, 0x9033),
    (0x09da, 0x9066),
    (0x09da, 0x9090),
    (0x09da, 0x90c0),
    (0x09da, 0xf012),
    (0x09da, 0xf32a),
    (0x09da, 0xf613),
    (0x09da, 0xf624),
    (0x1b1c, 0x1b3c),
    (0x1d57, 0xad03),
    (0x1e7d, 0x2e4a),
    (0x20a0, 0x422d),
    (0x2516, 0x001f),
    (0x2516, 0x0028),
    (0x04d9, 0x8008),
    (0x04d9, 0x8009),
    (0x04d9, 0xa292),
    (0x04d9, 0xa293),
    (0x04f2, 0xa13c),
    (0x1532, 0x0266),
    (0x1532, 0x0282),
    (0x256c, 0x006d),
    (0x256c, 0x006e),
    (0x26ce, 0x01a2),
    (0x3297, 0x1969),
    (0x3434, 0x0121),
    (0x3434, 0x0163),
    (0x3434, 0x0211),
    (0x3434, 0x02a0),
    (0x3434, 0x0353),
    (0x3434, 0xd030),
];

/// `initial_rog_gamepad_mice`: ASUS ROG mice whose joystick collection SDL reads only when asked
/// (`SDL_JOYSTICK_ROG_CHAKRAM`).
const ROG_MICE: &[(u16, u16)] = &[
    (0x0b05, 0x18e3),
    (0x0b05, 0x18e5),
    (0x0b05, 0x1906),
    (0x0b05, 0x1958),
    (0x0b05, 0x1a18),
    (0x0b05, 0x1a1a),
    (0x0b05, 0x1a1c),
];

/// `SDL_IsJoystickUSBChargingOnly`: wireless pads whose USB connection only charges them, with
/// the same ids over Bluetooth. SDL leaves them out when their GUID says USB, which its
/// DirectInput GUIDs always do, so a DirectInput pad with these ids is never read.
const USB_CHARGING_ONLY: &[(u16, u16)] = &[
    (0x0e6f, 0x0186),
    (0x0e6f, 0x018a),
    (0x0e6f, 0x0193),
    (0x0e6f, 0x0195),
    (0x20d6, 0x0002),
];

/// One of SDL's lists with the player's own changes: the built-in ids and those the variable
/// `name` adds, less those `name_EXCLUDED` takes out (`SDL_LoadVIDPIDList`, `SDL_VIDPIDInList`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VidPidList {
    pub included: Vec<(u16, u16)>,
    pub excluded: Vec<(u16, u16)>,
}

impl VidPidList {
    fn new(built_in: &[(u16, u16)], added: Option<String>, excluded: Option<String>) -> VidPidList {
        let mut included = built_in.to_vec();
        included.extend(hint(added));
        VidPidList {
            included,
            excluded: hint(excluded),
        }
    }

    pub fn contains(&self, id: (u16, u16)) -> bool {
        self.included.contains(&id) && !self.excluded.contains(&id)
    }
}

/// SDL's own lists of devices a DirectInput or HID reader leaves out, and Steam's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceLists {
    pub blacklist: VidPidList,
    pub rog_mice: VidPidList,
    /// `SDL_JOYSTICK_ROG_CHAKRAM`: read the ROG mice's joystick collection after all.
    pub rog_chakram: bool,
    pub steam: SteamLists,
}

impl DeviceLists {
    /// The lists with the variables `get` answers for (`std::env::var` in the game).
    pub fn with(get: impl Fn(&str) -> Option<String>) -> DeviceLists {
        DeviceLists {
            blacklist: VidPidList::new(
                BLACKLIST,
                get("SDL_JOYSTICK_BLACKLIST_DEVICES"),
                get("SDL_JOYSTICK_BLACKLIST_DEVICES_EXCLUDED"),
            ),
            rog_mice: VidPidList::new(
                ROG_MICE,
                get("SDL_ROG_GAMEPAD_MICE"),
                get("SDL_ROG_GAMEPAD_MICE_EXCLUDED"),
            ),
            rog_chakram: yes(get("SDL_JOYSTICK_ROG_CHAKRAM").as_deref(), false),
            steam: SteamLists {
                ignore: hint(get("SDL_GAMECONTROLLER_IGNORE_DEVICES")),
                except: hint(get("SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT")),
                allow_virtual: yes(
                    get("SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD").as_deref(),
                    false,
                ),
            },
        }
    }

    pub fn from_env() -> DeviceLists {
        DeviceLists::with(|name| std::env::var(name).ok())
    }

    /// Whether a DirectInput-read pad `id` is left out, as SDL leaves it out: a USB-charging-only
    /// pad, a device of the blacklist, a ROG mouse unless asked for, or one Steam's lists leave
    /// out.
    pub fn leave_out(&self, id: (u16, u16)) -> bool {
        USB_CHARGING_ONLY.contains(&id)
            || self.blacklist.contains(id)
            || (!self.rog_chakram && self.rog_mice.contains(id))
            || self.steam.leave_out(id)
    }
}

/// The XUSB device interface class (`{EC87F1E3-C13B-4100-B5F7-8B84D54260CB}`), which an XInput
/// pad's path can carry.
const XUSB_INTERFACE: &str = "{EC87F1E3-C13B-4100-B5F7-8B84D54260CB}";

/// Whether XInput reads the device (`SDL_IsXInputDevice`): an `IG_` or the XUSB interface class
/// in its path (in any case), the `xinputhid` filter driver on it (`xinputhid`: Xbox pads over
/// Bluetooth and other makers' XInputHID pads, which carry neither), an id SDL takes for an Xbox
/// 360 or Xbox One pad ([`XBOX`]), or Steam's virtual pad.
pub fn is_xinput_device(id: (u16, u16), path: &str, xinputhid: bool) -> bool {
    let path = path.to_ascii_uppercase();
    path.contains("IG_")
        || path.contains(XUSB_INTERFACE)
        || xinputhid
        || XBOX.binary_search(&id).is_ok()
        || id == STEAM_VIRTUAL
}

/// Which of the system's HID pad lists a device joins ([`joins`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Joins {
    /// The PlayStation pads read by nv-rs itself (the first list after XInput's own pads).
    Sony,
    /// The pads read through the controller database (after the PlayStation pads), if its line
    /// is there and fits.
    Database,
    /// Neither: left out by SDL's or Steam's lists, not a game controller, or read by XInput.
    Neither,
}

/// Which list a probed HID device joins, in the finder's order (review of #804): SDL's and
/// Steam's lists leave it out first, a PlayStation pad then goes to its own list (so it stays
/// ahead of any database pad), and a game controller (`game_controller`: its top usage is a
/// joystick, gamepad or multi-axis controller) that XInput does not read (`xinput`, asked only
/// then) is a database pad.
pub fn joins(
    id: (u16, u16),
    sony: bool,
    game_controller: bool,
    xinput: impl FnOnce() -> bool,
    lists: &DeviceLists,
) -> Joins {
    if lists.leave_out(id) {
        Joins::Neither
    } else if sony {
        Joins::Sony
    } else if !game_controller || xinput() {
        Joins::Neither
    } else {
        Joins::Database
    }
}

/// The ids SDL3's `src/joystick/controller_list.h` (Valve, zlib) gives an Xbox 360, Xbox One or
/// Xbox Elite type, sorted.
pub const XBOX: &[(u16, u16)] = &[
    (0x0000, 0x6686),
    (0x0079, 0x1832),
    (0x0079, 0x1874),
    (0x0079, 0x187c),
    (0x0079, 0x187f),
    (0x0079, 0x1883),
    (0x0079, 0x188e),
    (0x0079, 0x189c),
    (0x0079, 0x18a1),
    (0x0079, 0x18c2),
    (0x0079, 0x18c8),
    (0x0079, 0x18cf),
    (0x0079, 0x18d3),
    (0x0079, 0x18d4),
    (0x0351, 0x1000),
    (0x0351, 0x2000),
    (0x03eb, 0xff01),
    (0x03eb, 0xff02),
    (0x03f0, 0x0495),
    (0x044f, 0xb326),
    (0x044f, 0xd012),
    (0x045e, 0x028e),
    (0x045e, 0x028f),
    (0x045e, 0x0291),
    (0x045e, 0x02a0),
    (0x045e, 0x02a1),
    (0x045e, 0x02a2),
    (0x045e, 0x02a9),
    (0x045e, 0x02d1),
    (0x045e, 0x02dd),
    (0x045e, 0x02e0),
    (0x045e, 0x02e3),
    (0x045e, 0x02ea),
    (0x045e, 0x02fd),
    (0x045e, 0x02ff),
    (0x045e, 0x0719),
    (0x045e, 0x0867),
    (0x045e, 0x0b00),
    (0x045e, 0x0b05),
    (0x045e, 0x0b0a),
    (0x045e, 0x0b0c),
    (0x045e, 0x0b12),
    (0x045e, 0x0b13),
    (0x045e, 0x0b20),
    (0x045e, 0x0b21),
    (0x045e, 0x0b22),
    (0x046d, 0x0000),
    (0x046d, 0x0291),
    (0x046d, 0x0301),
    (0x046d, 0x0401),
    (0x046d, 0x1000),
    (0x046d, 0x1004),
    (0x046d, 0x1007),
    (0x046d, 0x1008),
    (0x046d, 0xc21d),
    (0x046d, 0xc21e),
    (0x046d, 0xc21f),
    (0x046d, 0xc242),
    (0x046d, 0xc261),
    (0x046d, 0xcaa3),
    (0x046d, 0xf301),
    (0x056e, 0x2004),
    (0x056e, 0x2012),
    (0x06a3, 0xf51a),
    (0x0738, 0x02a0),
    (0x0738, 0x02a6),
    (0x0738, 0x02ab),
    (0x0738, 0x4716),
    (0x0738, 0x4718),
    (0x0738, 0x4726),
    (0x0738, 0x4728),
    (0x0738, 0x4736),
    (0x0738, 0x4738),
    (0x0738, 0x4740),
    (0x0738, 0x4a01),
    (0x0738, 0x7263),
    (0x0738, 0x9806),
    (0x0738, 0xb726),
    (0x0738, 0xb738),
    (0x0738, 0xbeef),
    (0x0738, 0xcb02),
    (0x0738, 0xcb03),
    (0x0738, 0xcb29),
    (0x0738, 0xf401),
    (0x0738, 0xf738),
    (0x0955, 0x7210),
    (0x0955, 0xb400),
    (0x0b05, 0x1b4c),
    (0x0c12, 0x0e17),
    (0x0c12, 0x0e1c),
    (0x0c12, 0x0e22),
    (0x0c12, 0x0e30),
    (0x0c12, 0x0ef8),
    (0x0d62, 0x9a1a),
    (0x0d62, 0x9a1b),
    (0x0e00, 0x0e00),
    (0x0e6f, 0x0105),
    (0x0e6f, 0x0113),
    (0x0e6f, 0x011f),
    (0x0e6f, 0x0125),
    (0x0e6f, 0x0127),
    (0x0e6f, 0x012a),
    (0x0e6f, 0x0131),
    (0x0e6f, 0x0133),
    (0x0e6f, 0x0139),
    (0x0e6f, 0x013a),
    (0x0e6f, 0x013b),
    (0x0e6f, 0x0143),
    (0x0e6f, 0x0145),
    (0x0e6f, 0x0146),
    (0x0e6f, 0x0147),
    (0x0e6f, 0x0152),
    (0x0e6f, 0x0159),
    (0x0e6f, 0x015b),
    (0x0e6f, 0x015c),
    (0x0e6f, 0x015d),
    (0x0e6f, 0x015f),
    (0x0e6f, 0x0160),
    (0x0e6f, 0x0161),
    (0x0e6f, 0x0162),
    (0x0e6f, 0x0163),
    (0x0e6f, 0x0164),
    (0x0e6f, 0x0165),
    (0x0e6f, 0x0166),
    (0x0e6f, 0x0167),
    (0x0e6f, 0x0201),
    (0x0e6f, 0x0205),
    (0x0e6f, 0x0206),
    (0x0e6f, 0x0213),
    (0x0e6f, 0x021f),
    (0x0e6f, 0x0246),
    (0x0e6f, 0x0261),
    (0x0e6f, 0x0262),
    (0x0e6f, 0x02a0),
    (0x0e6f, 0x02a1),
    (0x0e6f, 0x02a2),
    (0x0e6f, 0x02a3),
    (0x0e6f, 0x02a4),
    (0x0e6f, 0x02a5),
    (0x0e6f, 0x02a6),
    (0x0e6f, 0x02a7),
    (0x0e6f, 0x02a8),
    (0x0e6f, 0x02a9),
    (0x0e6f, 0x02aa),
    (0x0e6f, 0x02ab),
    (0x0e6f, 0x02ac),
    (0x0e6f, 0x02ad),
    (0x0e6f, 0x02ae),
    (0x0e6f, 0x02af),
    (0x0e6f, 0x02b0),
    (0x0e6f, 0x02b1),
    (0x0e6f, 0x02b2),
    (0x0e6f, 0x02b3),
    (0x0e6f, 0x02b5),
    (0x0e6f, 0x02b6),
    (0x0e6f, 0x02b8),
    (0x0e6f, 0x02bd),
    (0x0e6f, 0x02be),
    (0x0e6f, 0x02bf),
    (0x0e6f, 0x02c0),
    (0x0e6f, 0x02c1),
    (0x0e6f, 0x02c2),
    (0x0e6f, 0x02c3),
    (0x0e6f, 0x02c4),
    (0x0e6f, 0x02c5),
    (0x0e6f, 0x02c6),
    (0x0e6f, 0x02c7),
    (0x0e6f, 0x02c8),
    (0x0e6f, 0x02c9),
    (0x0e6f, 0x02ca),
    (0x0e6f, 0x02cb),
    (0x0e6f, 0x02cd),
    (0x0e6f, 0x02ce),
    (0x0e6f, 0x02cf),
    (0x0e6f, 0x02d5),
    (0x0e6f, 0x02d6),
    (0x0e6f, 0x02d9),
    (0x0e6f, 0x02da),
    (0x0e6f, 0x0301),
    (0x0e6f, 0x0313),
    (0x0e6f, 0x0314),
    (0x0e6f, 0x0338),
    (0x0e6f, 0x0346),
    (0x0e6f, 0x0401),
    (0x0e6f, 0x0413),
    (0x0e6f, 0x0446),
    (0x0e6f, 0x0501),
    (0x0e6f, 0x1414),
    (0x0e6f, 0xf501),
    (0x0e6f, 0xf900),
    (0x0f0d, 0x000a),
    (0x0f0d, 0x000c),
    (0x0f0d, 0x000d),
    (0x0f0d, 0x0016),
    (0x0f0d, 0x001b),
    (0x0f0d, 0x0063),
    (0x0f0d, 0x0067),
    (0x0f0d, 0x006d),
    (0x0f0d, 0x0078),
    (0x0f0d, 0x008c),
    (0x0f0d, 0x0097),
    (0x0f0d, 0x00a4),
    (0x0f0d, 0x00ae),
    (0x0f0d, 0x00b1),
    (0x0f0d, 0x00ba),
    (0x0f0d, 0x00c0),
    (0x0f0d, 0x00c5),
    (0x0f0d, 0x00d8),
    (0x0f0d, 0x00db),
    (0x0f0d, 0x00ed),
    (0x0f0d, 0x011e),
    (0x0f0d, 0x0150),
    (0x0f0d, 0x01b1),
    (0x0fff, 0x02a1),
    (0x1038, 0x1430),
    (0x1038, 0x1431),
    (0x1038, 0xb360),
    (0x10f5, 0x7009),
    (0x10f5, 0x7013),
    (0x11c9, 0x55f0),
    (0x11ff, 0x0511),
    (0x12ab, 0x0004),
    (0x12ab, 0x0301),
    (0x12ab, 0x0303),
    (0x12ab, 0x0304),
    (0x1345, 0x6006),
    (0x1430, 0x0291),
    (0x1430, 0x02a0),
    (0x1430, 0x02a7),
    (0x1430, 0x02a8),
    (0x1430, 0x02a9),
    (0x1430, 0x0705),
    (0x1430, 0x070b),
    (0x1430, 0x0719),
    (0x1430, 0x0805),
    (0x1430, 0x1705),
    (0x1430, 0x1715),
    (0x1430, 0x4734),
    (0x1430, 0x4748),
    (0x1430, 0xf801),
    (0x146b, 0x0601),
    (0x146b, 0x0602),
    (0x146b, 0x0611),
    (0x1532, 0x0037),
    (0x1532, 0x0a00),
    (0x1532, 0x0a03),
    (0x1532, 0x0a14),
    (0x1532, 0x0a15),
    (0x15e4, 0x3f00),
    (0x15e4, 0x3f0a),
    (0x15e4, 0x3f10),
    (0x162e, 0xbeef),
    (0x1689, 0xfd00),
    (0x1689, 0xfd01),
    (0x1689, 0xfe00),
    (0x16d0, 0x0f3f),
    (0x1949, 0x041a),
    (0x1bad, 0x0002),
    (0x1bad, 0x0003),
    (0x1bad, 0x0130),
    (0x1bad, 0x028e),
    (0x1bad, 0x02a0),
    (0x1bad, 0x02a6),
    (0x1bad, 0x02ab),
    (0x1bad, 0x1330),
    (0x1bad, 0x1338),
    (0x1bad, 0x1430),
    (0x1bad, 0x1438),
    (0x1bad, 0x1530),
    (0x1bad, 0x1538),
    (0x1bad, 0x5500),
    (0x1bad, 0xf016),
    (0x1bad, 0xf018),
    (0x1bad, 0xf019),
    (0x1bad, 0xf021),
    (0x1bad, 0xf023),
    (0x1bad, 0xf025),
    (0x1bad, 0xf027),
    (0x1bad, 0xf028),
    (0x1bad, 0xf02e),
    (0x1bad, 0xf036),
    (0x1bad, 0xf038),
    (0x1bad, 0xf039),
    (0x1bad, 0xf03a),
    (0x1bad, 0xf03d),
    (0x1bad, 0xf03e),
    (0x1bad, 0xf03f),
    (0x1bad, 0xf042),
    (0x1bad, 0xf080),
    (0x1bad, 0xf501),
    (0x1bad, 0xf502),
    (0x1bad, 0xf503),
    (0x1bad, 0xf504),
    (0x1bad, 0xf505),
    (0x1bad, 0xf506),
    (0x1bad, 0xf900),
    (0x1bad, 0xf901),
    (0x1bad, 0xf902),
    (0x1bad, 0xf903),
    (0x1bad, 0xf904),
    (0x1bad, 0xf906),
    (0x1bad, 0xfa01),
    (0x1bad, 0xfd00),
    (0x1bad, 0xfd01),
    (0x2068, 0x0001),
    (0x2068, 0x0002),
    (0x20ab, 0x55ef),
    (0x20d6, 0x2001),
    (0x20d6, 0x2002),
    (0x20d6, 0x2003),
    (0x20d6, 0x2004),
    (0x20d6, 0x2005),
    (0x20d6, 0x2006),
    (0x20d6, 0x2009),
    (0x20d6, 0x200a),
    (0x20d6, 0x200b),
    (0x20d6, 0x200c),
    (0x20d6, 0x200d),
    (0x20d6, 0x200e),
    (0x20d6, 0x200f),
    (0x20d6, 0x2011),
    (0x20d6, 0x2012),
    (0x20d6, 0x2015),
    (0x20d6, 0x2016),
    (0x20d6, 0x2017),
    (0x20d6, 0x2018),
    (0x20d6, 0x2019),
    (0x20d6, 0x201a),
    (0x20d6, 0x4001),
    (0x20d6, 0x4002),
    (0x20d6, 0x890b),
    (0x24c6, 0x5000),
    (0x24c6, 0x5300),
    (0x24c6, 0x5303),
    (0x24c6, 0x530a),
    (0x24c6, 0x531a),
    (0x24c6, 0x5397),
    (0x24c6, 0x541a),
    (0x24c6, 0x542a),
    (0x24c6, 0x543a),
    (0x24c6, 0x5500),
    (0x24c6, 0x5501),
    (0x24c6, 0x5502),
    (0x24c6, 0x5503),
    (0x24c6, 0x5506),
    (0x24c6, 0x5508),
    (0x24c6, 0x5509),
    (0x24c6, 0x550d),
    (0x24c6, 0x550e),
    (0x24c6, 0x5510),
    (0x24c6, 0x551a),
    (0x24c6, 0x561a),
    (0x24c6, 0x581a),
    (0x24c6, 0x591a),
    (0x24c6, 0x592a),
    (0x24c6, 0x5b00),
    (0x24c6, 0x5b02),
    (0x24c6, 0x5b03),
    (0x24c6, 0x5d04),
    (0x24c6, 0x791a),
    (0x24c6, 0xfafa),
    (0x24c6, 0xfafb),
    (0x24c6, 0xfafc),
    (0x24c6, 0xfafd),
    (0x24c6, 0xfafe),
    (0x24c6, 0xfaff),
    (0x2516, 0x0069),
    (0x25b1, 0x0360),
    (0x2c22, 0x2203),
    (0x2dc8, 0x2002),
    (0x2dc8, 0x200a),
    (0x2dc8, 0x201e),
    (0x2dc8, 0x3106),
    (0x2dc8, 0x310a),
    (0x2e24, 0x0423),
    (0x2e24, 0x0652),
    (0x2e24, 0x072e),
    (0x2e24, 0x1618),
    (0x2e24, 0x1688),
    (0x2f24, 0x0011),
    (0x2f24, 0x002e),
    (0x2f24, 0x0050),
    (0x2f24, 0x0053),
    (0x2f24, 0x008f),
    (0x2f24, 0x0091),
    (0x2f24, 0x00b7),
    (0x3537, 0x103e),
    (0x3537, 0x109c),
    (0x3651, 0x1000),
    (0x3651, 0x6000),
    (0xd2d2, 0xd2d2),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn lists(vars: &'static [(&'static str, &'static str)]) -> DeviceLists {
        DeviceLists::with(|name| {
            vars.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        })
    }

    /// ROG Chakram mice are not pads unless `SDL_JOYSTICK_ROG_CHAKRAM` says yes (review of #804);
    /// the hint's own lists add to and take from SDL's.
    #[test]
    fn rog_mice_left_out_unless_asked() {
        let default = lists(&[]);
        for id in [(0x0b05, 0x18e3), (0x0b05, 0x1958), (0x0b05, 0x1a1c)] {
            assert!(default.leave_out(id), "{id:04x?}");
        }
        assert!(!lists(&[("SDL_JOYSTICK_ROG_CHAKRAM", "1")]).leave_out((0x0b05, 0x18e3)));
        let fewer = lists(&[("SDL_ROG_GAMEPAD_MICE_EXCLUDED", "0x0b05/0x18e3")]);
        assert!(!fewer.leave_out((0x0b05, 0x18e3)));
        assert!(fewer.leave_out((0x0b05, 0x18e5)));
        let more = lists(&[("SDL_ROG_GAMEPAD_MICE", "0x0b05/0x1234")]);
        assert!(more.leave_out((0x0b05, 0x1234)));
    }

    /// SDL's blacklist (keyboards, mice, tablets) and its USB-charging-only pads, with the
    /// blacklist's own variables; an ordinary pad passes.
    #[test]
    fn sdls_other_lists() {
        let default = lists(&[]);
        assert!(default.leave_out((0x056a, 0x030e)));
        assert!(default.leave_out((0x3434, 0xd030)));
        assert!(default.leave_out((0x0e6f, 0x0186)));
        assert!(default.leave_out((0x20d6, 0x0002)));
        assert!(!default.leave_out((0x2dc8, 0x6006)));
        let added = lists(&[("SDL_JOYSTICK_BLACKLIST_DEVICES", "0x2dc8/0x6006")]);
        assert!(added.leave_out((0x2dc8, 0x6006)));
        let excluded = lists(&[("SDL_JOYSTICK_BLACKLIST_DEVICES_EXCLUDED", "0x056a/0x030e")]);
        assert!(!excluded.leave_out((0x056a, 0x030e)));
    }

    /// Steam's lists, read on every system now: an ignore list, and an except list that leaves
    /// out every other pad; never Steam's virtual pad.
    #[test]
    fn steams_lists_on_every_system() {
        let ignore = lists(&[("SDL_GAMECONTROLLER_IGNORE_DEVICES", "0x2dc8/0x6006")]);
        assert!(ignore.leave_out((0x2dc8, 0x6006)));
        assert!(!ignore.leave_out((0x046d, 0xc216)));
        let except = lists(&[("SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT", "0x046d/0xc216")]);
        assert!(!except.leave_out((0x046d, 0xc216)));
        assert!(except.leave_out((0x2dc8, 0x6006)));
        assert!(!except.leave_out(STEAM_VIRTUAL));
        let allow = lists(&[("SDL_GAMECONTROLLER_ALLOW_STEAM_VIRTUAL_GAMEPAD", "1")]);
        assert!(allow.steam.allow_virtual);
    }

    /// XInput's pads as SDL tells them (review of #804): the path's marks, the xinputhid filter,
    /// an Xbox type's ids, Steam's virtual pad; a DirectInput pad is none of them.
    #[test]
    fn xinput_devices_as_sdl_tells_them() {
        let path = r"\?\HID#VID_2DC8&PID_6006#7&2";
        assert!(!is_xinput_device((0x2dc8, 0x6006), path, false));
        assert!(is_xinput_device(
            (0x2dc8, 0x6006),
            r"\?\hid#vid_045e&pid_028e&ig_00#3",
            false
        ));
        assert!(is_xinput_device(
            (0x2dc8, 0x6006),
            r"\?\USB#VID_045E&PID_028E#1#{ec87f1e3-c13b-4100-b5f7-8b84d54260cb}",
            false
        ));
        assert!(is_xinput_device((0x2dc8, 0x6006), path, true));
        // An Xbox One S pad over Bluetooth (045e:02e0) and an Xbox 360 pad's ids.
        assert!(is_xinput_device((0x045e, 0x02e0), path, false));
        assert!(is_xinput_device((0x045e, 0x028e), path, false));
        assert!(is_xinput_device(STEAM_VIRTUAL, path, false));
        assert!(XBOX.windows(2).all(|w| w[0] < w[1]), "sorted, each once");
        assert_eq!(XBOX.len(), 391);
    }

    /// Which list a probed device joins (review of #804): SDL's lists first, then a PlayStation pad
    /// to its own list (ahead of any database pad), then a game controller XInput does not read to
    /// the database's; XInput is asked only for a game controller that is not a PlayStation pad.
    #[test]
    fn which_list_a_device_joins() {
        let none = lists(&[]);
        let never = || -> bool { panic!("XInput asked") };
        let dualsense = (0x054c, 0x0ce6);
        assert_eq!(joins(dualsense, true, true, never, &none), Joins::Sony);
        assert_eq!(
            joins((0x2dc8, 0x6006), false, true, || false, &none),
            Joins::Database
        );
        assert_eq!(
            joins((0x2dc8, 0x6006), false, true, || true, &none),
            Joins::Neither
        );
        assert_eq!(
            joins((0x2dc8, 0x6006), false, false, never, &none),
            Joins::Neither
        );
        // Left out by SDL's lists, a PlayStation pad among them, before anything is asked.
        let ignored = lists(&[("SDL_GAMECONTROLLER_IGNORE_DEVICES", "0x054c/0x0ce6")]);
        assert_eq!(
            joins(dualsense, true, true, never, &ignored),
            Joins::Neither
        );
        assert_eq!(
            joins((0x0b05, 0x18e3), false, true, never, &none),
            Joins::Neither
        );
    }
}
