//! What the launcher's terminal sounds are made of, read from the player's own game at the start
//! (nothing of the game ships): the six SOUN records by editor id from `FalloutNV.esm` (only its
//! SOUN group is walked), their sound files (a loose file under `Data\Sound` first, else
//! `Fallout - Sound.bsa`, as the launcher's look reads its files), and the volume DirectSound is
//! given for each, worked out as the game's `BSWin32GameSound` does it.

use std::io::{Read, Seek, SeekFrom};

/// The terminal's sounds, by the names the game asks for them (`GetSoundHandleByName`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    /// `UIHackingFanHumLP`: the terminal's hum, looping while it is open (ComputersMenu's
    /// constructor 007578e3, played in `Create` 00757d87).
    Hum,
    /// `UIHackingCharScroll`: a line of text types (`AnimatingTextManager::Update` 00759b19).
    Scroll,
    /// `UIHackingCharEnter`: an intro line typed as if by a player starts (0075997e), or an entry
    /// is chosen (`DoClick` 007580c8).
    Enter,
    /// `UIHackingCharSingle`: one character of an intro line typed as if by a player (00759ae4).
    Single,
    /// `UIMenuOK`: an entry's tile is clicked (its `clicksound`, list_box_template.xml).
    Ok,
    /// `UIMenuFocus`: the focus moves to another entry's tile (its `mouseoversound`).
    Focus,
}

impl Cue {
    pub const ALL: [Cue; 6] = [
        Cue::Hum,
        Cue::Scroll,
        Cue::Enter,
        Cue::Single,
        Cue::Ok,
        Cue::Focus,
    ];

    /// The SOUN record's editor id.
    pub fn editor_id(self) -> &'static str {
        match self {
            Cue::Hum => "UIHackingFanHumLP",
            Cue::Scroll => "UIHackingCharScroll",
            Cue::Enter => "UIHackingCharEnter",
            Cue::Single => "UIHackingCharSingle",
            Cue::Ok => "UIMenuOK",
            Cue::Focus => "UIMenuFocus",
        }
    }
}

/// What a SOUN record says of its sound: the file or folder (`FNAM`, under `Data\Sound`), its
/// flags and its static attenuation (hundredths of a decibel; `SNDD`, or `SNDX` in older
/// records: both hold the flags at 4 and the static attenuation at 8).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub file: String,
    pub flags: u32,
    pub attenuation: u16,
}

/// The largest SOUN record read (FalloutNV.esm's are a few hundred bytes).
pub const MOST_RECORD: u32 = 64 * 1024;

/// The record's sound loops (its flag 0x10, `005e39b0`).
pub const LOOP: u32 = 0x10;

/// The SOUN records named `wanted` (editor ids, any case) in the plugin `file`, walked group by
/// group: every other top group is skipped by its size. A compressed record is passed over (no
/// SOUN record of FalloutNV.esm is).
pub fn soun_records<R: Read + Seek>(
    file: &mut R,
    wanted: &[&str],
) -> std::io::Result<Vec<(String, Record)>> {
    let mut found = Vec::new();
    let head = header(file)?;
    // The file's own header record (TES4).
    file.seek(SeekFrom::Current(i64::from(head.1)))?;
    while let Ok((kind, size, label, _)) = header(file) {
        if &kind != b"GRUP" {
            break;
        }
        let body = u64::from(size).saturating_sub(24);
        if &label != b"SOUN" {
            file.seek(SeekFrom::Current(body as i64))?;
            continue;
        }
        let mut left = body;
        while left >= 24 {
            let (kind, size, _, flags) = header(file)?;
            left -= 24;
            // A record larger than what is left of the group, or than any SOUN record could be,
            // is a damaged file: nothing more is read from it (no allocation of its size).
            if u64::from(size) > left || size > MOST_RECORD {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("a record of {size} bytes in a SOUN group of {left} left"),
                ));
            }
            let mut data = vec![0u8; size as usize];
            file.read_exact(&mut data)?;
            left = left.saturating_sub(u64::from(size));
            if &kind != b"SOUN" || flags & 0x0004_0000 != 0 {
                continue;
            }
            if let Some((id, r)) = record(&data) {
                if wanted.iter().any(|w| w.eq_ignore_ascii_case(&id)) {
                    found.push((id, r));
                }
            }
        }
        break;
    }
    Ok(found)
}

/// A record or group header: its type, size, the group's label (or the record's flags' bytes)
/// and the record's flags.
fn header<R: Read>(file: &mut R) -> std::io::Result<([u8; 4], u32, [u8; 4], u32)> {
    let mut h = [0u8; 24];
    file.read_exact(&mut h)?;
    let u = |o: usize| u32::from_le_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]);
    Ok((
        [h[0], h[1], h[2], h[3]],
        u(4),
        [h[8], h[9], h[10], h[11]],
        u(8),
    ))
}

/// A SOUN record's editor id and sound, from its subrecords.
fn record(data: &[u8]) -> Option<(String, Record)> {
    let (mut id, mut file, mut sound) = (None, None, None);
    let mut at = 0;
    while at + 6 <= data.len() {
        let kind = &data[at..at + 4];
        let n = usize::from(u16::from_le_bytes([data[at + 4], data[at + 5]]));
        let body = data.get(at + 6..at + 6 + n)?;
        let text = || {
            body.iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect::<String>()
        };
        match kind {
            b"EDID" => id = Some(text()),
            b"FNAM" => file = Some(text()),
            b"SNDD" | b"SNDX" if n >= 10 => {
                sound = Some((
                    u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
                    u16::from_le_bytes([body[8], body[9]]),
                ))
            }
            _ => {}
        }
        at += 6 + n;
    }
    let (flags, attenuation) = sound.unwrap_or((0, 0));
    Some((
        id?,
        Record {
            file: file?,
            flags,
            attenuation,
        },
    ))
}

/// A sound file's samples as DirectSound is given them: PCM, 8 or 16 bits, one or two channels.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub data: Vec<u8>,
    pub channels: u16,
    pub bits: u16,
    pub rate: u32,
}

impl Clip {
    /// Its frames.
    pub fn frames(&self) -> usize {
        self.data.len() / (usize::from(self.channels) * usize::from(self.bits) / 8).max(1)
    }
}

/// A `.wav` file's PCM samples; None for another format (the game's terminal sounds are all
/// 16-bit mono PCM).
pub fn wav(bytes: &[u8]) -> Option<Clip> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut format, mut data) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let n = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = &bytes[at + 8..(at + 8).saturating_add(n).min(bytes.len())];
        match id {
            b"fmt " if body.len() >= 16 => format = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        // Chunks are padded to an even size.
        at = at.saturating_add(8 + n + (n & 1));
    }
    let f = format?;
    let u16_at = |o: usize| u16::from_le_bytes([f[o], f[o + 1]]);
    let (tag, channels, bits) = (u16_at(0), u16_at(2), u16_at(14));
    let rate = u32::from_le_bytes([f[4], f[5], f[6], f[7]]);
    let ok = tag == 1 && matches!(channels, 1 | 2) && matches!(bits, 8 | 16) && rate > 0;
    let block = usize::from(channels) * usize::from(bits) / 8;
    let data = data?;
    ok.then(|| Clip {
        data: data[..data.len() / block * block].to_vec(),
        channels,
        bits,
        rate,
    })
}

/// The game's volume settings that reach an interface sound: the master volume and the effects
/// volume (`FalloutPrefs.ini [Audio] fDefaultMasterVolume`, `fDefaultEffectsVolume`, copied by
/// `BSAudioManager::Initialize` 00ad9da0; default 1.0, 00fb7189 and 00fb7369) and the master
/// multiplier (`Fallout.ini [Audio] fMasterVolumeMult`, default 1.3, 01022004). The two
/// secondary multipliers the manager keeps beside them start at 1.0 and are not changed by a
/// terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub master_mult: f32,
    pub effects: f32,
}

impl Default for Volumes {
    fn default() -> Volumes {
        Volumes {
            master: 1.0,
            master_mult: f32::from_bits(0x3fa6_6666),
            effects: 1.0,
        }
    }
}

impl Volumes {
    /// From the play profile's `Fallout.ini` (`ini`) and `FalloutPrefs.ini` (`prefs`) text; each
    /// value its default when the file lacks it or it is not a number.
    pub fn from_files(ini: &str, prefs: &str) -> Volumes {
        let d = Volumes::default();
        let get = |text: &str, key: &str, or: f32| {
            game_profile::settings::ini_value(text, "Audio", key)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(or)
        };
        Volumes {
            master: get(prefs, "fDefaultMasterVolume", d.master),
            master_mult: get(ini, "fMasterVolumeMult", d.master_mult),
            effects: get(prefs, "fDefaultEffectsVolume", d.effects),
        }
    }
}

/// The volume DirectSound is given for an interface sound of static attenuation `attenuation`
/// (hundredths of a decibel), in hundredths of a decibel from -10000 to -1: `BSWin32GameSound`'s
/// SetVolume 00aed660 with the effects branch of 00adac30 (a terminal's sounds have none of the
/// flags 0x4, 0x8, 0x800, 0x100000, 0x800000). The sound's own volume (1.0, clamped to 0..1) times
/// the master volume, its secondary (1.0) and the multiplier, stored as `f32`; times the effects
/// volume and its secondary (1.0), stored as `f32`; a product below 1e-5 (0101dae8) becomes 1e-5
/// (010718c0); `log10` stored as `f32`, times 2000.0 (01073490), truncated (`_ftol2`); minus the
/// static attenuation and the system and fader attenuations (both 0 for these sounds); held to
/// -10000..-1. The game's x87 products are worked here in `f64`, each stored as the game stores
/// it.
pub fn direct_sound_volume(v: Volumes, attenuation: u16) -> i32 {
    let own = 1.0f32;
    let scaled = ((f64::from(v.master) * 1.0) * f64::from(v.master_mult) * f64::from(own)) as f32;
    let mut product = ((f64::from(v.effects) * 1.0) * f64::from(scaled)) as f32;
    if f64::from(product) < f64::from_bits(0x3ee4_f8b5_8000_0000) {
        product = f32::from_bits(0x3727_c5ac);
    }
    let log = f64::from(product).log10() as f32;
    let main = (f64::from(log) * 2000.0) as i64 - i64::from(attenuation);
    main.clamp(-10000, -1) as i32
}

/// The game's `rand()` (the C library's: `x = x * 214013 + 2531011`, its bits 16..30), for the
/// hum's start and the folder picks; seeded once from the clock, as the game's start seeds it.
#[derive(Clone, Copy, Debug)]
pub struct Rand(pub u32);

impl Rand {
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.0 >> 16) & 0x7fff
    }
}

/// The folder pick of `005e3630`: one of `count` files, `rand() % count`, never the one picked
/// last time for that folder (`last`) when there is more than one.
pub fn pick(rand: &mut Rand, count: usize, last: Option<usize>) -> Option<usize> {
    if count == 0 {
        return None;
    }
    loop {
        let i = rand.next() as usize % count;
        if count == 1 || Some(i) != last {
            return Some(i);
        }
    }
}

/// Where a looping sound starts (`BSWin32GameSound` Play 00aed080: `SetCurrentPosition(rand() %
/// (size - 1))`, bytes, which DirectSound rounds down to a whole frame): the frame.
pub fn loop_start(rand: &mut Rand, clip: &Clip) -> usize {
    let size = clip.data.len() as u32;
    let block = (u32::from(clip.channels) * u32::from(clip.bits) / 8).max(1);
    if size < 2 {
        return 0;
    }
    ((rand.next() % (size - 1)) / block) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record or group: its header (type, size, label or flags) and body.
    fn unit(kind: &[u8; 4], label: [u8; 4], body: &[u8], group: bool) -> Vec<u8> {
        let mut v = kind.to_vec();
        let size = if group { body.len() + 24 } else { body.len() };
        v.extend((size as u32).to_le_bytes());
        v.extend(label);
        v.extend([0u8; 12]);
        v.extend(body);
        v
    }

    fn sub(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = kind.to_vec();
        v.extend((body.len() as u16).to_le_bytes());
        v.extend(body);
        v
    }

    /// A SOUN record of made-up values: editor id, file, flags, static attenuation, in `SNDD`
    /// (36 bytes) or the older `SNDX` (12).
    fn soun(id: &str, file: &str, flags: u32, att: u16, old: bool) -> Vec<u8> {
        let mut d = vec![0u8; if old { 12 } else { 36 }];
        d[4..8].copy_from_slice(&flags.to_le_bytes());
        d[8..10].copy_from_slice(&att.to_le_bytes());
        let mut body = sub(b"EDID", format!("{id}\0").as_bytes());
        body.extend(sub(b"FNAM", format!("{file}\0").as_bytes()));
        body.extend(sub(if old { b"SNDX" } else { b"SNDD" }, &d));
        unit(b"SOUN", [0; 4], &body, false)
    }

    /// Only the SOUN group is read, every other group passed by its size; the records named are
    /// found by editor id (any case), from SNDD or SNDX; a compressed one is passed over.
    #[test]
    fn the_soun_records_are_read_by_editor_id() {
        let mut esm = unit(b"TES4", [0; 4], &sub(b"HEDR", &[0; 12]), false);
        // A group of something else, holding a SOUN-looking record that must not be read.
        let decoy = soun("UIMenuOK", "wrong.wav", 0, 1, false);
        esm.extend(unit(b"GRUP", *b"WEAP", &decoy, true));
        let mut group = soun("UIMenuOK", "fx\\made\\ok.wav", 0, 308, false);
        group.extend(soun("SomethingElse", "x.wav", 0, 5, false));
        group.extend(soun(
            "UIHackingCharEnter",
            "fx\\made\\enter\\",
            0x10,
            485,
            true,
        ));
        let mut packed = soun("UIMenuFocus", "f.wav", 0, 1, false);
        packed[8..12].copy_from_slice(&0x0004_0000u32.to_le_bytes());
        group.extend(packed);
        esm.extend(unit(b"GRUP", *b"SOUN", &group, true));
        esm.extend(unit(b"GRUP", *b"MGEF", &[1, 2, 3], true));
        let mut r = std::io::Cursor::new(esm);
        let found =
            soun_records(&mut r, &["uimenuok", "UIHackingCharEnter", "UIMenuFocus"]).unwrap();
        assert_eq!(
            found,
            [
                (
                    "UIMenuOK".to_string(),
                    Record {
                        file: "fx\\made\\ok.wav".into(),
                        flags: 0,
                        attenuation: 308,
                    }
                ),
                (
                    "UIHackingCharEnter".to_string(),
                    Record {
                        file: "fx\\made\\enter\\".into(),
                        flags: LOOP,
                        attenuation: 485,
                    }
                ),
            ]
        );
        // A file that is no plugin: nothing, no panic.
        assert!(soun_records(&mut std::io::Cursor::new(vec![1u8; 10]), &["x"]).is_err());
    }

    fn wav_bytes(tag: u16, channels: u16, bits: u16, rate: u32, data: &[u8]) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend(tag.to_le_bytes());
        fmt.extend(channels.to_le_bytes());
        fmt.extend(rate.to_le_bytes());
        fmt.extend((rate * u32::from(channels * bits / 8)).to_le_bytes());
        fmt.extend((channels * bits / 8).to_le_bytes());
        fmt.extend(bits.to_le_bytes());
        let mut body = b"WAVE".to_vec();
        body.extend(b"fmt ");
        body.extend((fmt.len() as u32).to_le_bytes());
        body.extend(fmt);
        // An odd-sized chunk before the data: padded to an even size.
        body.extend(b"LIST");
        body.extend(3u32.to_le_bytes());
        body.extend([1, 2, 3, 0]);
        body.extend(b"data");
        body.extend((data.len() as u32).to_le_bytes());
        body.extend(data);
        let mut v = b"RIFF".to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    /// PCM .wav files of one or two channels, 8 or 16 bits; anything else is refused.
    #[test]
    fn a_pcm_wav_is_read_and_others_refused() {
        let c = wav(&wav_bytes(1, 1, 16, 22050, &[1, 0, 2, 0, 3])).unwrap();
        assert_eq!((c.channels, c.bits, c.rate), (1, 16, 22050));
        // The odd byte of a half frame left out.
        assert_eq!(c.data, [1, 0, 2, 0]);
        assert_eq!(c.frames(), 2);
        assert!(wav(&wav_bytes(3, 1, 32, 22050, &[0; 8])).is_none());
        assert!(wav(&wav_bytes(1, 6, 16, 22050, &[0; 12])).is_none());
        assert!(wav(b"RIFF\0\0\0\0WAVEjunk").is_none());
        assert!(wav(b"OggS").is_none());
    }

    /// The game's volume for an interface sound: the defaults (1.0, 1.3, 1.0) give
    /// trunc(log10(1.3) * 2000) = 227 less the static attenuation; nothing louder than -1; an
    /// effects volume of 0 is the floor 1e-5, -10000; half the effects volume 0.65, -374.
    #[test]
    fn the_volume_is_the_games() {
        let d = Volumes::default();
        assert_eq!(d.master_mult.to_bits(), 1.3f32.to_bits());
        assert_eq!(direct_sound_volume(d, 308), 227 - 308);
        assert_eq!(direct_sound_volume(d, 1366), 227 - 1366);
        assert_eq!(direct_sound_volume(d, 0), -1);
        let off = Volumes { effects: 0.0, ..d };
        assert_eq!(direct_sound_volume(off, 0), -10000);
        let half = Volumes { effects: 0.5, ..d };
        assert_eq!(direct_sound_volume(half, 0), -374);
        assert_eq!(direct_sound_volume(half, 100), -474);
        // Read from the profile's files; what is missing or not a number keeps its default.
        let v = Volumes::from_files(
            "[Audio]\r\nfMasterVolumeMult=1.0000\r\n",
            "[Audio]\r\nfDefaultMasterVolume=0.8000\r\nfDefaultEffectsVolume=odd\r\n",
        );
        assert_eq!(
            v,
            Volumes {
                master: 0.8,
                master_mult: 1.0,
                effects: 1.0
            }
        );
        assert_eq!(Volumes::from_files("", ""), d);
    }

    /// The C library's rand() from seed 1 (41, 18467, 6334, ...); a folder's pick never repeats
    /// the last one when it has more than one file; a loop starts on a whole frame inside it.
    #[test]
    fn the_random_picks_are_the_games() {
        let mut r = Rand(1);
        assert_eq!([r.next(), r.next(), r.next()], [41, 18467, 6334]);
        let mut r = Rand(7);
        let mut last = None;
        for _ in 0..200 {
            let i = pick(&mut r, 3, last).unwrap();
            assert!(i < 3 && Some(i) != last);
            last = Some(i);
        }
        assert_eq!(pick(&mut r, 1, Some(0)), Some(0));
        assert_eq!(pick(&mut r, 0, None), None);
        let clip = Clip {
            data: vec![0; 2 * 1000],
            channels: 1,
            bits: 16,
            rate: 22050,
        };
        for _ in 0..100 {
            assert!(loop_start(&mut r, &clip) < 1000);
        }
    }

    /// A damaged plugin whose SOUN record claims more bytes than its group holds, or more than
    /// any SOUN record could be: refused before anything of that size is made, no panic.
    #[test]
    fn an_oversized_record_is_refused_before_it_is_read() {
        let head = unit(b"TES4", [0; 4], &sub(b"HEDR", &[0; 12]), false);
        let mut huge = unit(b"SOUN", [0; 4], &[0; 8], false);
        huge[4..8].copy_from_slice(&0x7fff_fff0u32.to_le_bytes());
        let mut esm = head.clone();
        esm.extend(unit(b"GRUP", *b"SOUN", &huge, true));
        assert!(soun_records(&mut std::io::Cursor::new(esm), &["x"]).is_err());
        // Inside its group, but larger than a SOUN record can be.
        let big = unit(b"SOUN", [0; 4], &vec![0; MOST_RECORD as usize + 1], false);
        let mut esm = head;
        esm.extend(unit(b"GRUP", *b"SOUN", &big, true));
        assert!(soun_records(&mut std::io::Cursor::new(esm), &["x"]).is_err());
    }
}
