//! The terminal's sounds, played as the game plays them: through the same DirectSound mixer the
//! game's sound goes through (`ds_mix`: its first sample-rate converter, its volume, the order it
//! adds voices in), on the player's default output device (`ds_mix::sound_out`). What each sound
//! is and when it plays: `crate::sound_files` and the module of `crate::pacing`.
//!
//! - A sound the terminal keeps one handle for (the hum, the line's scroll, the "enter" sound)
//!   plays only when it is not already playing: `BSAudioManager::Play` 00adb3e0 asks `IsPlaying`
//!   first, so a sound asked for every frame plays through and starts again once it has ended.
//!   "Ended" is when DirectSound's write cursor reaches the buffer's end (the mixer's converter has
//!   been handed its last frame), when a new `Play` starts it over from its start
//!   (`engine-platform`'s dsound layer, measured `replays` 2026-10-30).
//! - The others are a new sound each time (`GetSoundHandleByName` for each typed character and for
//!   each tile sound, flags 0x121: it frees itself when it ends), so they overlap.
//! - The hum loops from a random place (`loop_start`).
//! - A character's sound asked `delay` milliseconds later (`PlayAfter` 00adb4e0: the manager's
//!   message at `GetTickCount() + delay`, at most 10000) starts with the first device period that
//!   begins at or after its time.
//! - The voices are mixed newest first, as DirectSound sums them (ds_mix::mixer).
//! - The device: the launcher mixes as the game's device does once it is locked to the endpoint's
//!   mix format (in the game the first 3D sound locks it; the launcher has none): the stream is
//!   opened in the mix format as it is, and each voice placed on its speakers by DirectSound's
//!   matrix (`render_voice_on`); on a two-channel endpoint that is the plain two-channel mix.

use crate::sound_files::{
    direct_sound_volume, loop_start, pick, wav, Clip, Cue, Rand, Volumes, LOOP,
};
use ds_mix::mixer::{render_voice_on, Place, Voice};
use ds_mix::sound_out::{self, Layout, Output, Source};
use std::sync::{Arc, Mutex};

/// One of the terminal's sounds, ready to play: its files (one, or a folder's) and its volume.
#[derive(Clone, Debug)]
pub struct Bank {
    pub clips: Vec<Clip>,
    pub volume: i32,
    pub looping: bool,
}

/// What plays.
struct Playing {
    cue: Cue,
    voice: Voice,
    frames: usize,
}

impl Playing {
    /// Whether DirectSound would say it stopped (its write cursor at the end).
    fn ended(&self) -> bool {
        match self.voice.position {
            None => true,
            Some(p) => !self.voice.looping && p >= self.frames,
        }
    }
}

/// Whether `cue` is a sound the terminal keeps one handle for (played only when not playing).
pub fn kept(cue: Cue) -> bool {
    matches!(cue, Cue::Hum | Cue::Scroll | Cue::Enter)
}

/// The mix: what plays, what waits for its time, and the frames given so far (its clock).
pub struct Mix {
    rate: u32,
    place: Place,
    banks: Vec<(Cue, Bank)>,
    playing: Vec<Playing>,
    waiting: Vec<(u64, Playing)>,
    frames: u64,
    rand: Rand,
    last_pick: Vec<(Cue, usize)>,
    scratch: Vec<f32>,
}

impl Mix {
    /// A mix at the device's `rate` on `layout` of `banks`; `seed` the random numbers' start.
    pub fn new(rate: u32, layout: Layout, banks: Vec<(Cue, Bank)>, seed: u32) -> Option<Mix> {
        Some(Mix {
            rate,
            place: Place::new(layout)?,
            banks,
            playing: Vec::new(),
            waiting: Vec::new(),
            frames: 0,
            rand: Rand(seed),
            last_pick: Vec::new(),
            scratch: Vec::new(),
        })
    }

    /// A new voice of `cue` (a folder's file picked as the game picks it), or None without it.
    fn voice(&mut self, cue: Cue) -> Option<Playing> {
        let bank = self.banks.iter().find(|(c, _)| *c == cue)?.1.clone();
        let i = if bank.clips.len() > 1 {
            let last = self.last_pick.iter().find(|(c, _)| *c == cue).map(|p| p.1);
            let i = pick(&mut self.rand, bank.clips.len(), last)?;
            self.last_pick.retain(|(c, _)| *c != cue);
            self.last_pick.push((cue, i));
            i
        } else {
            0
        };
        let clip = bank.clips.get(i)?;
        let mut v = Voice::new(clip.data.clone(), clip.channels, clip.bits);
        (v.rate, v.device) = (clip.rate, self.rate);
        v.volume = bank.volume;
        v.looping = bank.looping;
        let at = if bank.looping {
            loop_start(&mut self.rand, clip)
        } else {
            0
        };
        v.start(at);
        Some(Playing {
            cue,
            voice: v,
            frames: clip.frames(),
        })
    }

    /// Plays `cue` now (at the next device period): a kept sound only when it is not playing.
    pub fn play(&mut self, cue: Cue) {
        if kept(cue) {
            if self.playing.iter().any(|p| p.cue == cue && !p.ended()) {
                return;
            }
            self.playing.retain(|p| p.cue != cue);
        }
        if let Some(p) = self.voice(cue) {
            self.playing.insert(0, p);
        }
    }

    /// Plays `cue` (a new sound) `delay_ms` milliseconds from now (at most 10000).
    pub fn play_after(&mut self, cue: Cue, delay_ms: u64) {
        let at = self.frames + delay_ms.min(10_000) * u64::from(self.rate) / 1000;
        if let Some(p) = self.voice(cue) {
            self.waiting.push((at, p));
        }
    }

    /// Everything stops at once (the hum's hard stop, `~ComputersMenu` 00757a50).
    pub fn stop(&mut self) {
        self.playing.clear();
        self.waiting.clear();
    }

    /// Whether anything plays or waits.
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        !self.playing.is_empty() || !self.waiting.is_empty()
    }

    /// The next frames, `out` holding `channels` a frame (more than the mix's on macOS, where the
    /// mix goes in the first channels).
    pub fn render(&mut self, channels: u16, out: &mut [f32]) {
        out.fill(0.0);
        let width = usize::from(self.place.layout.channels).max(1);
        let frames = out.len() / usize::from(channels).max(1);
        // What waits for this period's start starts now, newest first.
        let now = self.frames;
        let mut i = 0;
        while i < self.waiting.len() {
            if self.waiting[i].0 <= now {
                let (_, p) = self.waiting.remove(i);
                self.playing.insert(0, p);
            } else {
                i += 1;
            }
        }
        let mut mix = std::mem::take(&mut self.scratch);
        mix.clear();
        mix.resize(frames * width, 0.0);
        let mut covered = 0;
        for p in self.playing.iter_mut() {
            covered = covered.max(render_voice_on(
                &mut p.voice,
                &mut mix,
                &self.place,
                covered,
            ));
        }
        // A sound that ended and is kept by no handle goes (a kept one stays to be asked again).
        self.playing
            .retain(|p| p.voice.position.is_some() || kept(p.cue));
        if usize::from(channels) == width {
            let n = mix.len().min(out.len());
            out[..n].copy_from_slice(&mix[..n]);
        } else {
            sound_out::widen(&mix, width as u16, out, channels);
        }
        self.scratch = mix;
        self.frames += frames as u64;
    }
}

/// The output's source: the mix behind a lock the window's thread also takes.
struct Shared(Arc<Mutex<Mix>>);

impl Source for Shared {
    fn render(&mut self, _rate: u32, channels: u16, out: &mut [f32]) {
        match self.0.lock() {
            Ok(mut m) => m.render(channels, out),
            Err(_) => out.fill(0.0),
        }
    }
}

/// The steps of Play's start, in their order: the sounds stopped and the device let go BEFORE
/// Play's work (which starts the game's process) begins, so the game never opens a device the
/// launcher still holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayStep {
    StopSounds,
    ReleaseDevice,
    StartWork,
}

/// [`PlayStep`]s in the order `App::play` does them.
pub const PLAY_ORDER: [PlayStep; 3] = [
    PlayStep::StopSounds,
    PlayStep::ReleaseDevice,
    PlayStep::StartWork,
];

/// Whether the terminal is heard now: its sounds on (Settings), the window shown and focused (the
/// game stops running, and sounding, when its window loses focus: our reading of that for the
/// launcher, not a traced terminal rule), and no Play work running (the game may be starting).
pub fn heard(on: bool, shown: bool, focused: bool, working: bool) -> bool {
    on && shown && focused && !working
}

/// The terminal's sound: its banks (read once), the device while it is heard, and the mix.
pub struct Sounds {
    banks: Vec<(Cue, Bank)>,
    open: Option<(Output, Arc<Mutex<Mix>>)>,
    /// The device would not open: not tried again (said once).
    refused: bool,
}

impl Sounds {
    /// From `banks` (none: silent).
    pub fn new(banks: Vec<(Cue, Bank)>) -> Sounds {
        Sounds {
            banks,
            open: None,
            refused: false,
        }
    }

    /// Opens the device if it is not open (and the banks are there): the hum starts. A line for
    /// the log when it would not open.
    pub fn open(&mut self) -> Option<String> {
        if self.open.is_some() || self.refused || self.banks.is_empty() {
            return None;
        }
        let Some((rate, layout)) = sound_out::device_mix() else {
            self.refused = true;
            return Some("sounds: no sound device: the launcher is silent".into());
        };
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
        let Some(mix) = Mix::new(rate, layout, self.banks.clone(), seed) else {
            self.refused = true;
            return Some(format!(
                "sounds: {}: the launcher is silent",
                sound_out::not_provided(layout)
            ));
        };
        let mix = Arc::new(Mutex::new(mix));
        let source: Arc<Mutex<dyn Source>> = Arc::new(Mutex::new(Shared(mix.clone())));
        match sound_out::start(source, None) {
            Ok(out) => {
                if let Ok(mut m) = mix.lock() {
                    m.play(Cue::Hum);
                }
                self.open = Some((out, mix));
                None
            }
            Err(e) => {
                self.refused = true;
                Some(format!(
                    "sounds: the sound device would not open ({e:?}): the launcher is silent"
                ))
            }
        }
    }

    /// Stops everything and lets the device go (the output's thread ends).
    pub fn release(&mut self) {
        if let Some((out, mix)) = self.open.take() {
            if let Ok(mut m) = mix.lock() {
                m.stop();
            }
            drop(out);
        }
    }

    /// Everything stops at once; the device stays open.
    pub fn stop(&self) {
        self.with(Mix::stop);
    }

    /// Whether the device is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    fn with(&self, f: impl FnOnce(&mut Mix)) {
        if let Some((_, mix)) = &self.open {
            if let Ok(mut m) = mix.lock() {
                f(&mut m);
            }
        }
    }

    pub fn play(&self, cue: Cue) {
        self.with(|m| m.play(cue));
    }

    pub fn play_after(&self, cue: Cue, delay_ms: u64) {
        self.with(|m| m.play_after(cue, delay_ms));
    }
}

/// The archive that holds the game's sounds.
pub const SOUND_ARCHIVE: &str = "Fallout - Sound.bsa";

/// The terminal's sounds read from the player's game at `folder` with the volumes `v`, and the
/// lines for the log (what was not found). None of them: silent.
pub fn load(folder: &std::path::Path, v: Volumes) -> (Vec<(Cue, Bank)>, Vec<String>) {
    let mut said = Vec::new();
    let Some(game) = crate::game::Game::at(folder) else {
        said.push(format!(
            "sounds: no game at {}: the launcher is silent",
            folder.display()
        ));
        return (Vec::new(), said);
    };
    let esm = crate::game::child(&game.data, "FalloutNV.esm");
    let records = esm
        .and_then(|p| std::fs::File::open(p).ok())
        .map(std::io::BufReader::new)
        .and_then(|mut f| {
            let ids: Vec<&str> = Cue::ALL.iter().map(|c| c.editor_id()).collect();
            crate::sound_files::soun_records(&mut f, &ids).ok()
        })
        .unwrap_or_default();
    let archive =
        crate::game::child(&game.data, SOUND_ARCHIVE).and_then(|p| bsa::Archive::open(p).ok());
    let mut banks = Vec::new();
    for cue in Cue::ALL {
        let Some((_, r)) = records
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(cue.editor_id()))
        else {
            said.push(format!(
                "sounds: {} is not in FalloutNV.esm: not played",
                cue.editor_id()
            ));
            continue;
        };
        let path = format!("sound\\{}", r.file.trim_start_matches(['\\', '/']));
        let files = if path.ends_with(['\\', '/']) {
            game.list(&path, ".wav", archive.as_ref())
        } else {
            vec![path]
        };
        let clips: Vec<Clip> = files
            .iter()
            .filter_map(|f| game.read_in(f, archive.as_ref()).and_then(|b| wav(&b)))
            .collect();
        if clips.is_empty() {
            said.push(format!(
                "sounds: {} ({}) was not found or is not a PCM .wav: not played",
                cue.editor_id(),
                r.file
            ));
            continue;
        }
        banks.push((
            cue,
            Bank {
                clips,
                volume: direct_sound_volume(v, r.attenuation),
                // The hum is asked for with the loop flag (0x31); the record's own flag too.
                looping: cue == Cue::Hum || r.flags & LOOP != 0,
            },
        ));
    }
    (banks, said)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up clip: `frames` frames of a 16-bit mono ramp at `rate`.
    fn clip(frames: usize, rate: u32) -> Clip {
        Clip {
            data: (0..frames)
                .flat_map(|i| ((i % 200) as i16 * 100 + 50).to_le_bytes())
                .collect(),
            channels: 1,
            bits: 16,
            rate,
        }
    }

    fn bank(frames: usize, volume: i32, looping: bool) -> Bank {
        Bank {
            clips: vec![clip(frames, 48000)],
            volume,
            looping,
        }
    }

    /// A mix at 48000 Hz on two channels: the scroll a short clip, a single character's sound,
    /// the hum looping.
    fn mix() -> Option<Mix> {
        ds_mix::resampler::tables()?;
        Mix::new(
            48000,
            Layout::STEREO,
            vec![
                (Cue::Scroll, bank(1000, -600, false)),
                (Cue::Single, bank(1000, 0, false)),
                (Cue::Hum, bank(4800, 0, true)),
            ],
            1,
        )
    }

    fn period(m: &mut Mix) -> Vec<f32> {
        let mut out = vec![0f32; 2 * 480];
        m.render(2, &mut out);
        out
    }

    /// The mixer's volume is the bank's: a 48000 Hz clip on a 48000 Hz device is passed through
    /// DirectSound's converter, each sample times the gain of its volume.
    #[test]
    fn a_sound_plays_at_its_volume_through_the_games_mixer() {
        let Some(mut m) = mix() else { return };
        m.play(Cue::Scroll);
        let out = period(&mut m);
        let g = ds_mix::mixer::gain(-600);
        let first = f32::from(50i16) / 32768.0;
        assert_eq!(out[0].to_bits(), (first * g).to_bits());
        assert_eq!(out[0], out[1]);
        // Nothing asked: silence.
        let mut quiet = mix().unwrap();
        assert!(period(&mut quiet).iter().all(|&s| s == 0.0));
    }

    /// A sound the terminal keeps one handle for plays only when it is not playing; once it has
    /// ended it plays again from its start. A character's sound is a new one each time.
    #[test]
    fn a_kept_sound_is_not_restarted_while_it_plays() {
        let Some(mut m) = mix() else { return };
        m.play(Cue::Scroll);
        period(&mut m);
        m.play(Cue::Scroll);
        assert_eq!(m.playing.len(), 1);
        // 1000 frames: the converter has been handed them all after the third period.
        period(&mut m);
        period(&mut m);
        assert!(m.playing[0].ended());
        m.play(Cue::Scroll);
        assert_eq!(m.playing.len(), 1);
        assert_eq!(m.playing[0].voice.position, Some(0));
        m.play(Cue::Single);
        m.play(Cue::Single);
        assert_eq!(m.playing.len(), 3);
        // Newest first, as DirectSound adds them.
        assert_eq!(m.playing[0].cue, Cue::Single);
    }

    /// A kept sound counts as ended once DirectSound's write cursor is at its end (the converter
    /// handed its last frame), before its last converted frames are heard: asked again then, it
    /// starts over at once, as the game's buffer does.
    #[test]
    fn a_kept_sound_ends_when_its_write_cursor_reaches_the_end() {
        let Some(mut m) = mix() else { return };
        m.play(Cue::Scroll);
        // 1340 frames: the third period has been converted (all 1000 frames handed), 100 of its
        // frames are still to be heard.
        let mut out = vec![0f32; 2 * 1340];
        m.render(2, &mut out);
        assert_eq!(m.playing[0].voice.position, Some(1000));
        assert!(m.playing[0].ended());
        m.play(Cue::Scroll);
        assert_eq!(m.playing.len(), 1);
        assert_eq!(m.playing[0].voice.position, Some(0));
    }

    /// A character's sound asked 15 ms later starts with the first period that begins at or
    /// after its time; everything stops at once on `stop`.
    #[test]
    fn a_later_sound_waits_for_its_period_and_stop_is_at_once() {
        let Some(mut m) = mix() else { return };
        m.play_after(Cue::Single, 15);
        assert!(period(&mut m).iter().all(|&s| s == 0.0));
        assert!(period(&mut m).iter().all(|&s| s == 0.0));
        // 15 ms = 720 frames: the period from 960 hears it.
        assert!(period(&mut m).iter().any(|&s| s != 0.0));
        m.play(Cue::Hum);
        assert!(m.busy());
        m.stop();
        assert!(!m.busy());
        assert!(period(&mut m).iter().all(|&s| s == 0.0));
        // A wait is held at 10 s, as the game's PlayAfter holds it.
        m.play_after(Cue::Single, 60_000);
        assert_eq!(m.waiting[0].0, m.frames + 480_000);
    }

    /// The hum loops from a place inside it; a stream wider than the mix gets it in its first
    /// channels.
    #[test]
    fn the_hum_loops_and_a_wider_stream_gets_the_mix_first() {
        let Some(mut m) = mix() else { return };
        m.play(Cue::Hum);
        let at = m.playing[0].voice.position.unwrap();
        assert!(at < 4800);
        for _ in 0..30 {
            period(&mut m);
        }
        assert!(!m.playing[0].ended());
        let mut wide = vec![1f32; 4 * 480];
        m.render(4, &mut wide);
        assert!(wide.chunks(4).all(|f| f[2] == 0.0 && f[3] == 0.0));
        assert!(wide.chunks(4).any(|f| f[0] != 0.0));
    }

    /// The device is let go before Play's work (which starts the game) begins; the terminal is
    /// heard only with its sounds on, its window shown and focused, and no Play work running.
    #[test]
    fn the_device_goes_before_the_game_starts_and_is_heard_only_when_shown() {
        let at = |s| PLAY_ORDER.iter().position(|&p| p == s).unwrap();
        assert!(at(PlayStep::StopSounds) < at(PlayStep::ReleaseDevice));
        assert!(at(PlayStep::ReleaseDevice) < at(PlayStep::StartWork));
        assert!(heard(true, true, true, false));
        assert!(!heard(false, true, true, false));
        assert!(!heard(true, false, true, false));
        assert!(!heard(true, true, false, false));
        assert!(!heard(true, true, true, true));
    }

    /// No banks (no game, no files): nothing opens, nothing plays, no panic.
    #[test]
    fn without_its_files_the_terminal_is_silent() {
        let mut s = Sounds::new(Vec::new());
        assert_eq!(s.open(), None);
        assert!(!s.is_open());
        s.play(Cue::Hum);
        s.release();
    }

    /// Not a check of the code alone: every sound read from the player's own game (the folder in
    /// `NV_RS_GAME`; nothing is written), named with its files, rate and volume
    /// (`nv-rs --ignored the_players_game --nocapture`).
    #[test]
    #[ignore]
    fn the_players_game_gives_every_sound() {
        let Some(game) = std::env::var_os("NV_RS_GAME") else {
            return;
        };
        let (banks, said) = load(std::path::Path::new(&game), Volumes::default());
        for (cue, b) in &banks {
            let rates: Vec<u32> = b.clips.iter().map(|c| c.rate).collect();
            println!(
                "{:?}: {} files, rates {rates:?}, volume {}, looping {}",
                cue,
                b.clips.len(),
                b.volume,
                b.looping
            );
        }
        assert!(said.is_empty(), "{said:?}");
        assert_eq!(banks.len(), Cue::ALL.len());
    }

    /// Not a check: the terminal's sounds on the real default device, MUTED (the device is fed
    /// silence while the mix is made, `sound_out::mute_device`: nothing is heard), the hum looping
    /// for `NV_RS_HOLD_S` seconds (65), so the process's processor time can be read from outside
    /// (`nv-rs --ignored idle_cost --nocapture`, with `NV_RS_GAME`).
    #[test]
    #[ignore]
    fn idle_cost_of_the_hum_on_a_muted_device() {
        let Some(game) = std::env::var_os("NV_RS_GAME") else {
            return;
        };
        let hold = std::env::var("NV_RS_HOLD_S")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(65);
        sound_out::mute_device();
        let (banks, _) = load(std::path::Path::new(&game), Volumes::default());
        let mut s = Sounds::new(banks);
        assert_eq!(s.open(), None);
        println!("hum playing, muted, for {hold} s");
        std::thread::sleep(std::time::Duration::from_secs(hold));
        s.release();
    }
}
