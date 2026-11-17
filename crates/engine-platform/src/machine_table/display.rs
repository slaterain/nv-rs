//! The display and graphics-card half of the machine table off Windows: what the game's Direct3D 9
//! asks about the screen and the card (`super::super::d3d9`), from what Linux and macOS report.
//!
//! - **The monitors** (winit, on the window layer's thread: `super::super::d3d9real::monitors`):
//!   `GetSystemMetrics` 0 and 1 (the primary monitor's size in the system's scaled pixels, as
//!   Windows gives a program without a DPI-awareness manifest, which FalloutNV.exe is), 0x50 (the
//!   number of monitors); `GetAdapterDisplayMode` (the primary monitor's real pixels and refresh
//!   rate, as Direct3D 9 answers whatever the program's DPI awareness); the modes of
//!   `D3DFMT_X8R8G8B8` ([`modes`]); `EnumDisplayDevicesA` (one device per monitor, the primary
//!   one flagged). Where winit names no primary monitor (Wayland) the first one is taken, and the
//!   game's borderless window is put on that same monitor ([`described`]). Not known here: a
//!   rotated output on Wayland reads unrotated (winit 0.30 reports the mode before the output's
//!   transform and gives no transform), so a portrait monitor there reads as landscape.
//! - **The card** (wgpu's adapter, `d3d9_wgpu::device::card_facts`): `GetAdapterIdentifier`'s
//!   description (the card's name), driver (wgpu's driver name), vendor and device numbers
//!   ([`identifier`]).
//! - **What a modern card answers** ([`CARD`], `data/machine/d3d9-card.tsv`): the Direct3D 9
//!   capabilities (`GetDeviceCaps`), every `Check*` answer over `scripts/d3d9-capture`'s domain and
//!   `CreateQuery`'s, as captured on Windows 11 on 2026-11-17 from an NVIDIA GeForce RTX card
//!   (vendor 10de, device 2f58; `d3d9-capture` and `d3d9-capture device queries`). These are the
//!   answers the Vulkan renderer (`d3d9_wgpu`) is already given on such a PC on Windows, where the
//!   game runs on them: the formats, shader model 3, the queries. They describe no person and no
//!   machine beyond the card's kind. One change follows the card actually there: a multisample
//!   count wgpu's adapter cannot draw with is answered as not available ([`honour_samples`]).
//!
//! Windows' own table is made by `scripts/d3d9-capture` from Windows' Direct3D 9 and is unchanged.
// Only Linux and macOS use this in a run; the tests run everywhere.
#![cfg_attr(windows, allow(dead_code))]

use super::hex;

/// What a modern card answers (see the module's documentation).
pub const CARD: &str = include_str!("../../data/machine/d3d9-card.tsv");

/// `D3DFMT_X8R8G8B8`: the format the desktop's modes are listed in.
pub const X8R8G8B8: u32 = 22;

/// The formats `scripts/d3d9-capture` asks `GetAdapterModeCount` about (d3d9types.h values and the
/// FourCCs games probe for): every one but [`X8R8G8B8`] answers 0 modes here.
pub const MODE_FORMATS: &[u32] = &[
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    29,
    30,
    31,
    32,
    33,
    34,
    35,
    36,
    40,
    41,
    50,
    51,
    52,
    60,
    61,
    62,
    63,
    64,
    67,
    70,
    71,
    73,
    75,
    77,
    79,
    80,
    81,
    82,
    83,
    84,
    85,
    100,
    101,
    102,
    110,
    111,
    112,
    113,
    114,
    115,
    116,
    117,
    118,
    119,
    199,
    0x3154_5844,
    0x3254_5844,
    0x3354_5844,
    0x3454_5844,
    0x3554_5844,
    0x5956_5955,
    0x3259_5559,
    0x4742_5247,
    0x4247_5247,
    0x3154_454d,
    0x5a54_4e49,
    0x5a57_4152,
    0x3631_4644,
    0x3432_4644,
    0x4c4c_554e,
    0x3149_5441,
    0x3249_5441,
    0x5453_4e49,
    0x5a53_4552,
    0x434f_5441,
    0x4141_5353,
    0x4244_564e,
    0x4256_3252,
    0x3631_4c41,
];

/// The sizes Windows' driver lists besides the monitor's own, drawn by scaling (measured with
/// `d3d9-capture`, 2026-11-17, an NVIDIA card on a 2560x1440 monitor: these 22 sizes, each at the
/// monitor's rates). The port draws any size the game asks for into its window, scaled, so each
/// one that fits on the monitor is listed here too.
pub const SCALED_SIZES: [(u32, u32); 22] = [
    (640, 480),
    (720, 480),
    (800, 600),
    (1024, 768),
    (1152, 864),
    (1176, 664),
    (1280, 720),
    (1280, 800),
    (1280, 960),
    (1280, 1024),
    (1360, 765),
    (1366, 768),
    (1400, 1050),
    (1440, 900),
    (1440, 1080),
    (1600, 900),
    (1600, 1024),
    (1600, 1200),
    (1680, 1050),
    (1920, 1080),
    (1920, 1200),
    (1920, 1440),
];

/// A mode a monitor can show, as winit reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u16,
    pub refresh_millihertz: u32,
}

/// A monitor as winit reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// The size in real pixels.
    pub size: (u32, u32),
    /// The system's scale (2.0: two real pixels to a scaled one).
    pub scale: f64,
    pub refresh_millihertz: Option<u32>,
    pub modes: Vec<Mode>,
    /// The system's primary monitor (none is on Wayland: then the first).
    pub primary: bool,
}

/// The graphics card as wgpu reports it, and the multisample counts it draws with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
    pub driver: String,
    pub driver_info: String,
    /// The sample counts the device will really make a surface with (`d3d9_wgpu`'s rule: every
    /// format of the multisampled family, the back buffer's and the occlusion count's agree).
    pub sample_counts: Vec<u32>,
    /// Whether the card takes block-compressed textures (`TEXTURE_COMPRESSION_BC`): the game's
    /// textures are DXT, so without it the start is refused.
    pub bc: bool,
    /// Whether the card blends 32-bit float targets (`FLOAT32_BLENDABLE`).
    pub float32_blendable: bool,
}

impl Card {
    /// Why the game cannot draw on this card, if it cannot.
    pub fn refusal(&self) -> Option<String> {
        (!self.bc).then(|| {
            format!(
                "the graphics card {} has no block-compressed (DXT) textures in its Vulkan driver, \
                 and every texture of the game is one",
                self.name
            )
        })
    }
}

/// Whole hertz as Direct3D 9 gives them: rounded down (Windows lists 59.94 Hz as 59, beside 60),
/// with 0.01 Hz allowed for a system that reports an exact rate a little under (59.999 Hz: 60).
pub fn hertz(millihertz: u32) -> u32 {
    (millihertz + 10) / 1000
}

/// The monitor the game's screen metrics and display mode describe, and the one the window layer
/// puts the game's borderless window on ([`described`]): the system's primary monitor, else the
/// first one (Wayland names none).
fn primary(monitors: &[Monitor]) -> Option<&Monitor> {
    described(monitors.iter().map(|m| m.primary)).map(|i| &monitors[i])
}

/// Which of the monitors (given by whether each is the system's primary one) the table describes
/// and the borderless window is shown on: the primary one, else the first.
pub fn described(primary: impl IntoIterator<Item = bool>) -> Option<usize> {
    let mut first = None;
    for (i, p) in primary.into_iter().enumerate() {
        if p {
            return Some(i);
        }
        first.get_or_insert(i);
    }
    first
}

/// The monitor's current rate in hertz: the one it reports, else the highest rate of a mode of
/// its own size, else 60 (no monitor answers 0 Hz to the game).
pub fn current_hertz(monitor: &Monitor) -> u32 {
    if let Some(mhz) = monitor.refresh_millihertz.filter(|&m| hertz(m) > 0) {
        return hertz(mhz);
    }
    monitor
        .modes
        .iter()
        .filter(|m| (m.width, m.height) == monitor.size)
        .map(|m| hertz(m.refresh_millihertz))
        .max()
        .filter(|&h| h > 0)
        .unwrap_or(60)
}

/// The modes of [`X8R8G8B8`] on `monitor`: each of its own modes of 24 or 32 bits (or of a depth
/// not reported) and each of [`SCALED_SIZES`] that fits on it at each rate of its own size, as
/// (width, height, hertz), without repeats, by size and then rate (Windows: the driver's order).
/// Its own size at its current rate is always among them.
pub fn modes(monitor: &Monitor) -> Vec<(u32, u32, u32)> {
    let (width, height) = monitor.size;
    let current = current_hertz(monitor);
    let mut list: Vec<(u32, u32, u32)> = monitor
        .modes
        .iter()
        .filter(|m| matches!(m.bit_depth, 0 | 24 | 32) && hertz(m.refresh_millihertz) > 0)
        .map(|m| (m.width, m.height, hertz(m.refresh_millihertz)))
        .collect();
    list.push((width, height, current));
    let mut rates: Vec<u32> = list
        .iter()
        .filter(|&&(w, h, _)| (w, h) == (width, height))
        .map(|&(_, _, r)| r)
        .collect();
    rates.sort_unstable();
    rates.dedup();
    for (w, h) in SCALED_SIZES {
        if w <= width && h <= height {
            list.extend(rates.iter().map(|&r| (w, h, r)));
        }
    }
    list.sort_unstable();
    list.dedup();
    list
}

/// A `D3DDISPLAYMODE`: width, height, refresh rate, format.
fn display_mode((width, height, hertz): (u32, u32, u32)) -> String {
    let mut b = Vec::with_capacity(16);
    for w in [width, height, hertz, X8R8G8B8] {
        b.extend_from_slice(&w.to_le_bytes());
    }
    hex(&b)
}

/// `text` as the game's ANSI bytes in a field of `size` bytes (cut to leave the terminating zero;
/// a character outside ASCII as `?`).
fn field(text: &str, size: usize) -> Vec<u8> {
    let mut b: Vec<u8> = text
        .chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .take(size - 1)
        .collect();
    b.resize(size, 0);
    b
}

/// `DISPLAY_DEVICEA` (0x1a8 bytes): `cb`, the device's name, its description (the card's), the
/// state flags (`DISPLAY_DEVICE_ATTACHED_TO_DESKTOP` 1, `_PRIMARY_DEVICE` 4), no device id or key.
fn display_device(index: usize, card: &str, primary: bool) -> Vec<u8> {
    let mut b = 0x1a8u32.to_le_bytes().to_vec();
    b.extend(field(&format!("\\\\.\\DISPLAY{}", index + 1), 32));
    b.extend(field(card, 128));
    b.extend_from_slice(&(if primary { 5u32 } else { 1 }).to_le_bytes());
    b.resize(0x1a8, 0);
    b
}

/// The display lines of the table: the screen metrics, the display mode, the mode counts and modes,
/// the display devices (named after `card`, the card's name, when it is known). An error when
/// there is no monitor.
pub fn display_lines(monitors: &[Monitor], card: &str) -> Result<String, String> {
    let primary = primary(monitors).ok_or("the system reports no monitor")?;
    let scaled = |real: u32| (f64::from(real) / primary.scale.max(1e-3)).round() as u32;
    let mut t = format!(
        "GetSystemMetrics\t0\t{:x}\nGetSystemMetrics\t1\t{:x}\nGetSystemMetrics\t50\t{:x}\n",
        scaled(primary.size.0),
        scaled(primary.size.1),
        monitors.len()
    );
    let current = (primary.size.0, primary.size.1, current_hertz(primary));
    t += &format!("GetAdapterDisplayMode\t0\t0\t{}\n", display_mode(current));
    let list = modes(primary);
    for &format in MODE_FORMATS {
        let n = if format == X8R8G8B8 { list.len() } else { 0 };
        t += &format!("GetAdapterModeCount\t0 {format:x}\t{n:x}\n");
    }
    for (i, &mode) in list.iter().enumerate() {
        t += &format!(
            "EnumAdapterModes\t0 {X8R8G8B8:x} {i:x}\t0\t{}\n",
            display_mode(mode)
        );
    }
    for (i, m) in monitors.iter().enumerate() {
        let is_primary = std::ptr::eq(m, primary);
        t += &format!(
            "EnumDisplayDevicesA\t{i:x} 0\t1\t{}\n",
            hex(&display_device(i, card, is_primary))
        );
    }
    let mut end = 0x1a8u32.to_le_bytes().to_vec();
    end.resize(0x1a8, 0);
    t += &format!(
        "EnumDisplayDevicesA\t{:x} 0\t0\t{}\n",
        monitors.len(),
        hex(&end)
    );
    Ok(t)
}

/// A number fixed for this card and driver, for the identifier's `DeviceIdentifier` (Windows'
/// changes when the card or its driver does; the game keeps it in its settings,
/// `uVideoDeviceIdentifierPart1..4`, to notice a new card): FNV-1a over the card's facts, twice.
fn device_identifier(card: &Card) -> [u8; 16] {
    let text = format!(
        "{}\0{:x}\0{:x}\0{}\0{}",
        card.name, card.vendor, card.device, card.driver, card.driver_info
    );
    let fnv = |seed: u64| {
        text.bytes().fold(seed, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    };
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&fnv(0xcbf2_9ce4_8422_2325).to_le_bytes());
    out[8..].copy_from_slice(&fnv(0x6c62_272e_07bb_0142).to_le_bytes());
    out
}

/// `D3DADAPTER_IDENTIFIER9` (1100 bytes) for `flags` (0, or `D3DENUM_WHQL_LEVEL` 2): the driver's
/// name (wgpu's), the card's name, the device name of the primary display, no driver version, the
/// vendor and device numbers, no subsystem or revision, [`device_identifier`], and the WHQL level
/// (asked with flag 2: 1, a signed driver, as measured; else 0).
pub fn identifier(card: &Card, flags: u32) -> Vec<u8> {
    let mut b = field(&card.driver, 512);
    b.extend(field(&card.name, 512));
    b.extend(field("\\\\.\\DISPLAY1", 32));
    b.extend_from_slice(&0u64.to_le_bytes());
    for w in [card.vendor, card.device, 0, 0] {
        b.extend_from_slice(&w.to_le_bytes());
    }
    b.extend_from_slice(&device_identifier(card));
    b.extend_from_slice(&u32::from(flags & 2 != 0).to_le_bytes());
    debug_assert_eq!(b.len(), 1100);
    b
}

/// The card's lines: `GetAdapterIdentifier` with flags 0 and 2.
pub fn card_lines(card: &Card) -> String {
    [0u32, 2]
        .iter()
        .map(|&flags| {
            format!(
                "GetAdapterIdentifier\t0 {flags:x}\t0\t{}\n",
                hex(&identifier(card, flags))
            )
        })
        .collect()
}

/// [`CARD`]'s `kind` lines with each answer given again by `change(lead words, last word,
/// the line's answer to a last word)`, each line folded again as `d3d9-capture` folds it (the
/// most common answer first, the others by their last word in the domain's order); every other
/// line as it was.
pub fn rewrite_answers(
    card: &str,
    kind: &str,
    change: impl Fn(&str, &str, &dyn Fn(&str) -> String) -> String,
) -> String {
    let mut domains: std::collections::HashMap<&str, Vec<&str>> = Default::default();
    let mut out = String::with_capacity(card.len());
    for line in card.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == "Domain" {
            domains.insert(f[1], f[2].split(',').collect());
        }
        if f[0] != kind {
            out += line;
            out.push('\n');
            continue;
        }
        let domain = &domains[f[4]];
        let exceptions: std::collections::HashMap<&str, &str> = f[3]
            .split(',')
            .filter(|e| !e.is_empty())
            .map(|e| e.split_once('=').expect("word=answer"))
            .collect();
        let answer = |w: &str| exceptions.get(w).unwrap_or(&f[2]).to_string();
        let answers: Vec<(&str, String)> = domain
            .iter()
            .map(|&w| (w, change(f[1], w, &answer)))
            .collect();
        let mut tally: Vec<(&str, usize)> = Vec::new();
        for (_, a) in &answers {
            match tally.iter_mut().find(|(t, _)| *t == a) {
                Some(t) => t.1 += 1,
                None => tally.push((a, 1)),
            }
        }
        tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let default = tally[0].0;
        let rest: Vec<String> = answers
            .iter()
            .filter(|(_, a)| a != default)
            .map(|(w, a)| format!("{w}={a}"))
            .collect();
        out += &format!(
            "{}\t{}\t{default}\t{}\t{}\n",
            f[0],
            f[1],
            rest.join(","),
            f[4]
        );
    }
    out
}

/// `card` with each `CheckDeviceMultiSampleType` answer for a count of 2 to 16 samples the card
/// does not draw with (`counts`) replaced by that line's answer for 16 samples (not available on
/// the captured card for any format).
pub fn honour_samples(card: &str, counts: &[u32]) -> String {
    rewrite_answers(card, "CheckDeviceMultiSampleType", |_, w, answer| {
        let n = u32::from_str_radix(w, 16).expect("a sample count");
        if (2..=16).contains(&n) && !counts.contains(&n) {
            answer("10")
        } else {
            answer(w)
        }
    })
}

/// `D3DUSAGE_QUERY_POSTPIXELSHADER_BLENDING`, and the 32-bit float formats (`D3DFMT_R32F`,
/// `G32R32F`, `A32B32G32R32F`) whose blending wgpu takes only with `FLOAT32_BLENDABLE`.
const BLENDING_QUERY: &str = "80000";
const FLOAT32_FORMATS: [&str; 3] = ["72", "73", "74"];

/// `card` with `CheckDeviceFormat`'s blending query for the 32-bit float formats answered
/// `D3DERR_NOTAVAILABLE` where the card cannot blend them (`blendable` false).
pub fn honour_blending(card: &str, blendable: bool) -> String {
    if blendable {
        return card.to_string();
    }
    rewrite_answers(card, "CheckDeviceFormat", |lead, w, answer| {
        let usage = lead.split(' ').nth(3);
        if usage == Some(BLENDING_QUERY) && FLOAT32_FORMATS.contains(&w) {
            "8876086a".to_string()
        } else {
            answer(w)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(size: (u32, u32), scale: f64, hz: u32, primary: bool) -> Monitor {
        Monitor {
            size,
            scale,
            refresh_millihertz: Some(hz),
            modes: Vec::new(),
            primary,
        }
    }

    fn card() -> Card {
        Card {
            name: "AMD Radeon RX 7800 XT (RADV NAVI32)".into(),
            vendor: 0x1002,
            device: 0x747e,
            driver: "radv".into(),
            driver_info: "Mesa 25.0.7".into(),
            sample_counts: vec![1, 2, 4, 8],
            bc: true,
            float32_blendable: true,
        }
    }

    /// The table these lines make, read back through the Windows table's own reader.
    fn engine_with(table: &str) -> crate::Engine {
        let mut e = crate::Engine::new();
        crate::d3d9::load_adapter_table(&mut e, table);
        e
    }

    #[test]
    fn rates_are_whole_hertz_rounded_down() {
        assert_eq!(hertz(59_940), 59);
        assert_eq!(hertz(59_950), 59);
        assert_eq!(hertz(59_999), 60);
        assert_eq!(hertz(60_000), 60);
        assert_eq!(hertz(143_856), 143);
        assert_eq!(hertz(0), 0);
    }

    /// A monitor that reports no rate: its own size's best mode, else 60; never 0 Hz.
    #[test]
    fn a_monitor_without_a_rate_never_answers_zero_hertz() {
        let mut m = monitor((1920, 1080), 1.0, 0, true);
        m.refresh_millihertz = None;
        assert_eq!(current_hertz(&m), 60);
        assert!(
            modes(&m).iter().all(|&(_, _, r)| r == 60),
            "{:?}",
            modes(&m)
        );
        m.modes = vec![
            Mode {
                width: 1920,
                height: 1080,
                bit_depth: 32,
                refresh_millihertz: 74_973,
            },
            Mode {
                width: 1920,
                height: 1080,
                bit_depth: 32,
                refresh_millihertz: 0,
            },
        ];
        assert_eq!(current_hertz(&m), 74);
        assert!(
            modes(&m).iter().all(|&(_, _, r)| r == 74),
            "{:?}",
            modes(&m)
        );
        let lines = display_lines(&[m], "x").unwrap();
        assert!(lines.contains(&format!(
            "GetAdapterDisplayMode\t0\t0\t{}\n",
            display_mode((1920, 1080, 74))
        )));
    }

    /// Two monitors of different shape and no primary (Wayland): the first is described, and it is
    /// the one the window layer is told to put the window on (`described`); with a primary, it.
    #[test]
    fn with_no_primary_the_first_monitor_is_described_and_shown_on() {
        assert_eq!(described([false, false]), Some(0));
        assert_eq!(described([false, true]), Some(1));
        assert_eq!(described([]), None);
        let monitors = [
            monitor((2560, 1080), 1.0, 60_000, false),
            monitor((1920, 1080), 1.0, 60_000, false),
        ];
        let mut e = engine_with(&display_lines(&monitors, "c").unwrap());
        assert_eq!(crate::d3d9::system_metric(&mut e, 0), Some(2560));
        assert_eq!(crate::d3d9::system_metric(&mut e, 1), Some(1080));
    }

    /// The card's refusal (no DXT textures) and the 32-bit float blending query.
    #[test]
    fn a_card_without_dxt_is_refused_and_float_blending_follows_the_card() {
        let mut c = card();
        assert_eq!(c.refusal(), None);
        c.bc = false;
        assert!(c.refusal().unwrap().contains("DXT"));
        assert_eq!(honour_blending(CARD, true), CARD.replace("\r\n", "\n"));
        let no = honour_blending(CARD, false);
        let row = |t: &str| {
            t.lines()
                .find(|l| l.starts_with("CheckDeviceFormat\t0 1 16 80000 3\t"))
                .unwrap()
                .to_string()
        };
        assert!(row(CARD).contains("72=0,73=0,74=0"));
        assert!(
            !row(&no).contains("72=") && !row(&no).contains("74="),
            "{}",
            row(&no)
        );
        assert!(row(&no).contains("71=0,c7=8876086c"));
        assert_eq!(no.lines().count(), CARD.lines().count());
        let _ = engine_with(&no);
    }

    /// The monitor's own modes of 24 or 32 bits, the scaled sizes that fit at each rate of its own
    /// size, its current mode always, by size and rate, no repeats.
    #[test]
    fn the_modes_are_the_monitors_and_the_scaled_sizes_that_fit() {
        let mut m = monitor((1920, 1080), 1.0, 143_856, true);
        let mode = |width, height, bit_depth, refresh_millihertz| Mode {
            width,
            height,
            bit_depth,
            refresh_millihertz,
        };
        m.modes = vec![
            mode(1920, 1080, 32, 60_000),
            mode(1920, 1080, 32, 143_856),
            mode(1920, 1080, 16, 30_000),
            mode(1280, 1024, 24, 75_025),
            mode(1920, 1080, 32, 60_000),
        ];
        let list = modes(&m);
        assert_eq!(list.first(), Some(&(640, 480, 60)));
        assert!(list.contains(&(640, 480, 143)));
        assert!(list.contains(&(1280, 1024, 75)) && list.contains(&(1280, 1024, 60)));
        assert!(!list.contains(&(1920, 1080, 30)));
        assert!(!list.iter().any(|&(w, h, _)| w > 1920 || h > 1080));
        assert_eq!(list.last(), Some(&(1920, 1080, 143)));
        let mut sorted = list.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, list);
        // 19 scaled sizes fit (the last is the monitor's own), each at 60 and 143; its 1280x1024
        // at 75 too.
        assert_eq!(list.len(), 19 * 2 + 1);
        // A monitor that reports no modes (Wayland may) still has its own size.
        let bare = monitor((800, 600), 1.0, 60_000, true);
        assert_eq!(
            modes(&bare),
            [(640, 480, 60), (720, 480, 60), (800, 600, 60)]
        );
    }

    /// The game reads the screen in scaled pixels, the display mode and modes in real pixels; one
    /// display device per monitor, the primary one flagged; a format other than X8R8G8B8 has none.
    #[test]
    fn the_display_lines_read_back_through_the_tables_reader() {
        let monitors = [
            monitor((1920, 1080), 1.0, 60_000, false),
            monitor((2560, 1440), 1.25, 165_000, true),
        ];
        let lines = display_lines(&monitors, "the card").unwrap();
        let mut e = engine_with(&lines);
        assert_eq!(crate::d3d9::system_metric(&mut e, 0), Some(2048));
        assert_eq!(crate::d3d9::system_metric(&mut e, 1), Some(1152));
        assert_eq!(crate::d3d9::system_metric(&mut e, 0x50), Some(2));
        let info = e.mem.alloc(0x1a8);
        e.mem.set_u32(info, 0x1a8);
        let devices: Vec<(u32, u8)> = (0..3)
            .map(|i| {
                let ok = crate::d3d9::enum_display_devices_a(
                    &mut e,
                    crate::Ptr::new(0),
                    i,
                    crate::Ptr::new(info),
                    0,
                );
                (ok, e.mem.u8(info + 0xa4))
            })
            .collect();
        assert_eq!(devices, [(1, 1), (1, 5), (0, 5)]);
        assert_eq!(e.mem.bytes(info + 4, 13), b"\\\\.\\DISPLAY2\0");
        assert_eq!(e.mem.bytes(info + 0x24, 9), b"the card\0");
        assert!(lines.contains(&format!(
            "GetAdapterDisplayMode\t0\t0\t{}\n",
            display_mode((2560, 1440, 165))
        )));
        let count = modes(&monitors[1]).len();
        assert!(lines.contains(&format!("GetAdapterModeCount\t0 16\t{count:x}\n")));
        assert!(lines.contains("GetAdapterModeCount\t0 17\t0\n"));
        assert_eq!(
            lines.matches("GetAdapterModeCount").count(),
            MODE_FORMATS.len()
        );
        assert_eq!(lines.matches("EnumAdapterModes").count(), count);
        assert!(display_lines(&[], "x").is_err());
    }

    /// The identifier: the driver and the card's names, the vendor and device, a fixed device
    /// identifier for these facts, the WHQL level only with flag 2.
    #[test]
    fn the_identifier_names_the_card() {
        let c = card();
        let id = identifier(&c, 0);
        assert_eq!(id.len(), 1100);
        assert_eq!(&id[..5], b"radv\0");
        assert_eq!(&id[512..512 + 36], b"AMD Radeon RX 7800 XT (RADV NAVI32)\0");
        assert_eq!(&id[1024..1037], b"\\\\.\\DISPLAY1\0");
        let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        assert_eq!((word(&id, 1064), word(&id, 1068)), (0x1002, 0x747e));
        assert_eq!(word(&id, 1096), 0);
        assert_eq!(word(&identifier(&c, 2), 1096), 1);
        assert_eq!(id[1080..1096], identifier(&c, 2)[1080..1096]);
        let mut other = c.clone();
        other.driver_info = "Mesa 25.1.0".into();
        assert_ne!(id[1080..1096], identifier(&other, 0)[1080..1096]);
        let lines = card_lines(&c);
        assert_eq!(lines.lines().count(), 2);
        assert!(lines.starts_with("GetAdapterIdentifier\t0 0\t0\t"));
    }

    /// The captured card's answers are what the game needs: a hardware device of shader model 3,
    /// the queries the renderer makes.
    #[test]
    fn the_cards_answers_are_shader_model_3() {
        let lines: Vec<&str> = CARD.lines().collect();
        let caps = lines
            .iter()
            .find(|l| l.starts_with("GetDeviceCaps\t0 1\t0\t"))
            .unwrap();
        let bytes: Vec<u8> = {
            let h = caps.split('\t').nth(3).unwrap();
            (0..h.len() / 2)
                .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
                .collect()
        };
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        // D3DCAPS9: VertexShaderVersion at 0xc4, PixelShaderVersion at 0xcc.
        assert_eq!(word(0xc4), 0xfffe_0300);
        assert_eq!(word(0xcc), 0xffff_0300);
        for kind in [
            "CheckDeviceType",
            "CheckDeviceFormat",
            "CheckDeviceMultiSampleType",
            "CheckDepthStencilMatch",
            "CheckDeviceFormatConversion",
            "CreateQuery",
        ] {
            assert!(lines.iter().any(|l| l.starts_with(kind)), "{kind}");
        }
        for line in &lines {
            let kind = line.split('\t').next().unwrap();
            assert!(
                kind == "GetDeviceCaps"
                    || kind == "Domain"
                    || kind.starts_with("Check")
                    || kind == "CreateQuery",
                "{line}"
            );
        }
    }

    /// A count the card cannot draw with is not available; the rest stays as captured; the folded
    /// lines read back through the table's reader.
    #[test]
    fn multisample_counts_follow_the_card() {
        // The answer of the HAL device, X8R8G8B8, full screen, to `samples`, read as the layer
        // reads a folded line (`d3d9::Adapter::check`).
        let ms = |table: &str, samples: u32| -> String {
            let mut domain = Vec::new();
            for line in table.lines() {
                let f: Vec<&str> = line.split('\t').collect();
                if f[0] == "Domain" && f[1] == "d2" {
                    domain = f[2].split(',').map(str::to_string).collect();
                }
                if f[0] == "CheckDeviceMultiSampleType" && f[1] == "0 1 16 0" {
                    let word = format!("{samples:x}");
                    assert!(domain.contains(&word) && f[4] == "d2");
                    return f[3]
                        .split(',')
                        .find_map(|e| e.strip_prefix(&format!("{word}=")))
                        .unwrap_or(f[2])
                        .to_string();
                }
            }
            panic!("no line")
        };
        // On the captured card: 2, 4 and 8 samples available.
        for n in [2, 4, 8] {
            assert!(ms(CARD, n).starts_with("0/"), "{n}");
        }
        assert!(ms(CARD, 16).starts_with("8876086a/"));
        let fewer = honour_samples(CARD, &[1, 4]);
        assert!(ms(&fewer, 4).starts_with("0/"));
        for n in [2, 8, 16] {
            assert_eq!(ms(&fewer, n), ms(CARD, 16), "{n}");
        }
        // No sample count other than these changes; every other line is as it was.
        assert_eq!(ms(&fewer, 1), ms(CARD, 1));
        assert_eq!(ms(&fewer, 0), ms(CARD, 0));
        assert_eq!(fewer.lines().count(), CARD.lines().count());
        // Both read through the layer's own reader.
        let _ = engine_with(CARD);
        let _ = engine_with(&fewer);
        assert_eq!(
            honour_samples(CARD, &(1..=16).collect::<Vec<_>>()),
            CARD.replace("\r\n", "\n")
        );
    }
}
