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
    /// The files of the sounds kept by one handle, picked once ([`Sounds::with_opener`]).
    fixed: Vec<(Cue, usize)>,
    scratch: Vec<f32>,
}

impl Mix {
    /// A mix at the stream's `rate` on its `layout` of `banks`; `seed` the random numbers'
    /// start; `fixed` the files picked once for the sounds kept by one handle.
    pub fn new(
        rate: u32,
        layout: Layout,
        banks: Vec<(Cue, Bank)>,
        seed: u32,
        fixed: Vec<(Cue, usize)>,
    ) -> Option<Mix> {
        Some(Mix {
            rate,
            place: Place::new(layout)?,
            banks,
            playing: Vec::new(),
            waiting: Vec::new(),
            frames: 0,
            rand: Rand(seed),
            last_pick: Vec::new(),
            fixed,
            scratch: Vec::new(),
        })
    }

    /// The file `cue` plays next: its handle's own for a sound kept by one handle (picked once),
    /// else a folder's picked as the game picks it (never the last one twice running).
    fn pick_for(&mut self, cue: Cue, count: usize) -> Option<usize> {
        if let Some(&(_, i)) = self.fixed.iter().find(|(c, _)| *c == cue) {
            return (i < count).then_some(i);
        }
        if count <= 1 {
            return (count == 1).then_some(0);
        }
        let last = self.last_pick.iter().find(|(c, _)| *c == cue).map(|p| p.1);
        let i = pick(&mut self.rand, count, last)?;
        self.last_pick.retain(|(c, _)| *c != cue);
        self.last_pick.push((cue, i));
        Some(i)
    }

    /// A new voice of `cue`, or None without it.
    fn voice(&mut self, cue: Cue) -> Option<Playing> {
        let b = self.banks.iter().position(|(c, _)| *c == cue)?;
        let i = self.pick_for(cue, self.banks[b].1.clips.len())?;
        let bank = &self.banks[b].1;
        let (volume, looping) = (bank.volume, bank.looping);
        let clip = bank.clips.get(i)?;
        let mut v = Voice::new(clip.data.clone(), clip.channels, clip.bits);
        (v.rate, v.device) = (clip.rate, self.rate);
        v.volume = volume;
        v.looping = looping;
        let frames = clip.frames();
        let at = if looping {
            loop_start(&mut self.rand, clip)
        } else {
            0
        };
        v.start(at);
        Some(Playing {
            cue,
            voice: v,
            frames,
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

/// The output's source: the mix behind a lock the window's thread also takes; silence until the
/// mix is made for the stream that was opened ([`Sounds::open`]).
struct Shared(Arc<Mutex<Option<Mix>>>);

impl Source for Shared {
    fn render(&mut self, _rate: u32, channels: u16, out: &mut [f32]) {
        match self.0.lock() {
            Ok(mut m) => match m.as_mut() {
                Some(m) => m.render(channels, out),
                None => out.fill(0.0),
            },
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

/// [`PlayStep`]s in the order [`play_in_order`] does them.
pub const PLAY_ORDER: [PlayStep; 3] = [
    PlayStep::StopSounds,
    PlayStep::ReleaseDevice,
    PlayStep::StartWork,
];

/// Play's start: `stop`, `release` and `start_work` called in [`PLAY_ORDER`] (`App::start_play`
/// passes the sounds' stop and release, which waits for the device to be let go, and the work).
pub fn play_in_order(
    mut stop: impl FnMut(),
    mut release: impl FnMut(),
    mut start_work: impl FnMut(),
) {
    for step in PLAY_ORDER {
        match step {
            PlayStep::StopSounds => stop(),
            PlayStep::ReleaseDevice => release(),
            PlayStep::StartWork => start_work(),
        }
    }
}

/// Whether the terminal is heard now: its sounds on (Settings), the window shown and focused (the
/// game stops running, and sounding, when its window loses focus: our reading of that for the
/// launcher, not a traced terminal rule), and no Play work running (the game may be starting).
pub fn heard(on: bool, shown: bool, focused: bool, working: bool) -> bool {
    on && shown && focused && !working
}

/// How a stream is opened: `sound_out::start` on the default device in its mix format as it is
/// at that moment (the tests' opener hands back an idle stream of a made-up format).
pub type Opener =
    Arc<dyn Fn(Arc<Mutex<dyn Source>>) -> Result<Output, sound_out::Refused> + Send + Sync>;

/// The device while it is open, and the opening under way.
#[derive(Default)]
struct State {
    open: Option<(Output, Arc<Mutex<Option<Mix>>>)>,
    opening: Option<std::thread::JoinHandle<()>>,
    /// Still wanted when the opening ends (false: a release came first, the stream is let go).
    wanted: bool,
    /// When the device last would not open: tried again at most once a second
    /// ([`sound_out::try_again`]), said once until one opens.
    failed: Option<std::time::Instant>,
    said_failed: bool,
    /// Lines for the launcher's log, taken by the window ([`Sounds::take_lines`]).
    said: Vec<String>,
}

/// The terminal's sound: its banks (read once), the files of the sounds the terminal keeps one
/// handle for (picked once, [`Sounds::with_opener`]), and the device while it is heard. Opening
/// and letting go of the device never hold the window's thread ([`Sounds::open`],
/// [`Sounds::release_later`]) except at Play's start, which waits for it ([`Sounds::release`]).
#[derive(Clone)]
pub struct Sounds {
    banks: Arc<Vec<(Cue, Bank)>>,
    fixed: Vec<(Cue, usize)>,
    seed: u32,
    opener: Opener,
    state: Arc<Mutex<State>>,
}

impl Sounds {
    /// From `banks` (none: silent), on the default device.
    pub fn new(banks: Vec<(Cue, Bank)>) -> Sounds {
        let opener: Opener = Arc::new(|source| sound_out::start(source, None));
        Sounds::with_opener(banks, opener, clock_seed())
    }

    /// From `banks` with `opener` and the random numbers' start `seed`. A sound the terminal
    /// keeps one handle for has its file picked once, here, for the launcher's run: the game
    /// picks a handle's file when the handle is made (`GetSoundHandleByName`, in
    /// `ComputersMenu`'s constructor) and plays that one each time; a new handle (each typed
    /// character's) picks again.
    pub fn with_opener(banks: Vec<(Cue, Bank)>, opener: Opener, seed: u32) -> Sounds {
        let mut rand = Rand(seed);
        let fixed = banks
            .iter()
            .filter(|(c, b)| kept(*c) && b.clips.len() > 1)
            .filter_map(|(c, b)| Some((*c, pick(&mut rand, b.clips.len(), None)?)))
            .collect();
        Sounds {
            banks: Arc::new(banks),
            fixed,
            seed: rand.next(),
            opener,
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Opens the device, if it is not open or opening (and the banks are there), on a thread of
    /// its own: the stream in the device's mix format as it is now, then the mix made for the
    /// stream that opened (its rate and layout, whatever an earlier device's were), then the hum.
    /// An open stream whose device was lost (`Output::lost`: unplugged, disabled, or on Windows
    /// no longer the default) is let go and one opened on the default device as it is now, the
    /// hum starting again at a random place of its loop, as at every open. A device that would
    /// not open is tried again at most once a second.
    pub fn open(&self) {
        let now = std::time::Instant::now();
        let mut s = self.lock();
        s.wanted = true;
        if s.open.as_ref().is_some_and(|(out, _)| out.lost()) {
            if let Some(open) = s.open.take() {
                std::thread::spawn(move || let_go(open));
            }
            s.said.push(
                "sounds: the sound device was lost: opening the default device again".to_string(),
            );
        }
        if s.open.is_some() || s.opening.is_some() || self.banks.is_empty() {
            return;
        }
        if s.failed.is_some() && !sound_out::try_again(s.failed, now) {
            return;
        }
        let (me, banks, fixed) = (self.clone(), self.banks.clone(), self.fixed.clone());
        let seed = self.seed ^ clock_seed();
        s.opening = Some(std::thread::spawn(move || {
            let slot: Arc<Mutex<Option<Mix>>> = Arc::new(Mutex::new(None));
            let source: Arc<Mutex<dyn Source>> = Arc::new(Mutex::new(Shared(slot.clone())));
            let opened = (me.opener)(source);
            let mut s = me.lock();
            s.opening = None;
            match opened {
                // No longer wanted (a release came while it opened): the stream is let go before
                // any mix is made, so not a moment of the hum is heard (REVIEW-834b).
                Ok(out) if !s.wanted => {
                    drop(s);
                    drop(out);
                }
                Ok(out) => match Mix::new(out.rate, out.layout, (*banks).clone(), seed, fixed) {
                    Some(mut m) => {
                        m.play(Cue::Hum);
                        *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(m);
                        let (rate, channels) = (out.rate, out.layout.channels);
                        s.open = Some((out, slot));
                        if s.failed.take().is_some() {
                            s.said.push(format!(
                                "sounds: the sound device opened ({rate} Hz, {channels} channels)"
                            ));
                        }
                        s.said_failed = false;
                    }
                    None => {
                        let why = sound_out::not_provided(out.layout);
                        failed(&mut s, why);
                    }
                },
                Err(e) => failed(&mut s, format!("the sound device would not open ({e:?})")),
            }
        }));
    }

    /// Whether the window should wake now and then to look at the device ([`check_wake`]): the
    /// stream is open (its device may be lost while nothing else happens) or a try is due.
    pub fn watching(&self) -> bool {
        let s = self.lock();
        s.wanted && (s.open.is_some() || s.failed.is_some())
    }

    /// Stops everything and lets the device go, waiting until it is (an opening under way ends
    /// first and lets its stream go itself): Play's start.
    pub fn release(&self) {
        let (opening, open) = {
            let mut s = self.lock();
            s.wanted = false;
            (s.opening.take(), s.open.take())
        };
        if let Some(t) = opening {
            let _ = t.join();
        }
        if let Some(open) = open {
            let_go(open);
        }
    }

    /// The device let go on a thread of its own (the window does not wait for the sound system):
    /// the window minimised or not in front, the sounds turned off.
    pub fn release_later(&self) {
        let open = {
            let mut s = self.lock();
            s.wanted = false;
            s.open.take()
        };
        if let Some(open) = open {
            std::thread::spawn(move || let_go(open));
        }
    }

    /// The lines for the log said since the last call.
    pub fn take_lines(&self) -> Vec<String> {
        std::mem::take(&mut self.lock().said)
    }

    /// Everything stops at once; the device stays open.
    pub fn stop(&self) {
        self.with(Mix::stop);
    }

    /// Whether the device is open (an opening under way is not).
    pub fn is_open(&self) -> bool {
        self.lock().open.is_some()
    }

    fn with(&self, f: impl FnOnce(&mut Mix)) {
        let mix = self.lock().open.as_ref().map(|(_, m)| m.clone());
        if let Some(mix) = mix {
            if let Some(m) = mix.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                f(m);
            }
        }
    }

    pub fn play(&self, cue: Cue) {
        self.with(|m| m.play(cue));
    }

    pub fn play_after(&self, cue: Cue, delay_ms: u64) {
        self.with(|m| m.play_after(cue, delay_ms));
    }

    /// The tests' wait for an opening under way.
    #[cfg(test)]
    fn opened(&self) {
        let t = self.lock().opening.take();
        if let Some(t) = t {
            let _ = t.join();
        }
    }

    /// The open stream marked lost, as its thread marks it when the device goes (the tests').
    #[cfg(test)]
    fn mark_lost(&self) {
        if let Some((out, _)) = self.lock().open.as_ref() {
            out.mark_lost();
        }
    }

    /// The open mix's rate and channels (the tests').
    #[cfg(test)]
    fn mix_format(&self) -> Option<(u32, u16)> {
        let mut r = None;
        self.with(|m| r = Some((m.rate, m.place.layout.channels)));
        r
    }
}

/// A try that opened nothing: tried again in a second at the earliest, said the first time only.
fn failed(s: &mut State, why: String) {
    s.failed = Some(std::time::Instant::now());
    if !s.said_failed {
        s.said_failed = true;
        s.said.push(format!(
            "sounds: {why}: the launcher is silent (tried again each second)"
        ));
    }
}

/// When the window wakes next: at `wake` (its own frame's time, none when it waits for an
/// event), and while the sounds are `watching` at most [`sound_out::DEFAULT_CHECK`] after `now`,
/// so a device lost while nothing else happens is followed within a second.
pub fn check_wake(
    wake: Option<std::time::Instant>,
    watching: bool,
    now: std::time::Instant,
) -> Option<std::time::Instant> {
    if !watching {
        return wake;
    }
    let check = now + sound_out::DEFAULT_CHECK;
    Some(wake.map_or(check, |w| w.min(check)))
}

/// An open stream let go: everything stopped, then the output (its thread ends).
fn let_go((out, mix): (Output, Arc<Mutex<Option<Mix>>>)) {
    if let Some(m) = mix.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        m.stop();
    }
    drop(out);
}

/// A seed from the clock.
fn clock_seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32)
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
            Vec::new(),
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

    /// Sounds whose opener hands back an idle stream at `rate` on `layout` (no device).
    fn sounds_on(rate: u32, layout: Layout, banks: Vec<(Cue, Bank)>) -> Sounds {
        let opener: Opener = Arc::new(move |_source| Ok(Output::idle(rate, layout)));
        Sounds::with_opener(banks, opener, 7)
    }

    /// The mix is made for the stream that opened, at its rate and on its layout, whatever the
    /// device was before: opened at 44100 Hz on four channels, then again at 48000 on two (a
    /// headset plugged in while the launcher ran), each mix follows its own stream (REVIEW-834).
    #[test]
    fn the_mix_follows_the_stream_that_opened() {
        ds_mix::resampler::tables();
        let quad = Layout {
            channels: 4,
            mask: 0x33,
        };
        let banks = vec![(Cue::Hum, bank(4800, 0, true))];
        let s = sounds_on(44100, quad, banks.clone());
        s.open();
        s.opened();
        assert!(s.is_open());
        assert_eq!(s.mix_format(), Some((44100, 4)));
        s.release();
        assert!(!s.is_open());
        let s = sounds_on(48000, Layout::STEREO, banks);
        s.open();
        s.opened();
        assert_eq!(s.mix_format(), Some((48000, 2)));
    }

    /// Play's start stops the sounds and lets the device go before the work that starts the game
    /// begins: at that moment the device is no longer open.
    #[test]
    fn the_device_is_let_go_before_the_game_starts() {
        let s = sounds_on(48000, Layout::STEREO, vec![(Cue::Hum, bank(4800, 0, true))]);
        s.open();
        s.opened();
        assert!(s.is_open());
        let steps = std::cell::RefCell::new(Vec::new());
        play_in_order(
            || steps.borrow_mut().push("stop"),
            || {
                s.release();
                steps.borrow_mut().push("release");
            },
            || {
                assert!(!s.is_open(), "the device is still open as the game starts");
                steps.borrow_mut().push("start");
            },
        );
        assert_eq!(*steps.borrow(), ["stop", "release", "start"]);
        // A release while an opening is under way waits for it (REVIEW-834b): the opener takes
        // a moment and says it has returned just before it does; by the time release() returns,
        // and when the work starts, it has, and nothing stays open.
        let returned = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let r = returned.clone();
        let opener: Opener = Arc::new(move |_source| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            r.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(Output::idle(48000, Layout::STEREO))
        });
        let s = Sounds::with_opener(vec![(Cue::Hum, bank(4800, 0, true))], opener, 7);
        let done = || returned.load(std::sync::atomic::Ordering::SeqCst);
        s.open();
        play_in_order(
            || s.stop(),
            || {
                s.release();
                assert!(done(), "release() returned before the opening ended");
            },
            || {
                assert!(done(), "the work started before the opening ended");
                assert!(!s.is_open(), "the device is still open as the game starts");
            },
        );
        assert!(!s.is_open());
    }

    /// A stream whose device was lost (a headset plugged in, the device unplugged) is let go at
    /// the next look and one opened on the default device as it is now, the mix at its rate;
    /// the line for the log says so. Stand-in streams only: nothing is heard.
    #[test]
    fn a_lost_device_is_followed_to_the_new_default() {
        ds_mix::resampler::tables();
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = calls.clone();
        let opener: Opener = Arc::new(move |_source| {
            let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let rate = if n == 0 { 48000 } else { 44100 };
            Ok(Output::idle(rate, Layout::STEREO))
        });
        let s = Sounds::with_opener(vec![(Cue::Hum, bank(4800, 0, true))], opener, 7);
        s.open();
        s.opened();
        assert_eq!(s.mix_format(), Some((48000, 2)));
        assert!(s.watching());
        // Not lost: another look opens nothing.
        s.open();
        s.opened();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        s.take_lines();
        s.mark_lost();
        s.open();
        s.opened();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(s.mix_format(), Some((44100, 2)));
        assert!(s.take_lines().iter().any(|l| l.contains("lost")));
    }

    /// A device that would not open is tried again at most once a second, and said once; the
    /// window wakes within a second while it waits ([`check_wake`]).
    #[test]
    fn a_device_that_would_not_open_is_tried_again_each_second() {
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = calls.clone();
        let opener: Opener = Arc::new(move |_source| {
            c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(sound_out::Refused::NoDevice)
        });
        let s = Sounds::with_opener(vec![(Cue::Hum, bank(4800, 0, true))], opener, 7);
        s.open();
        s.opened();
        s.open();
        s.opened();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(!s.is_open());
        assert!(s.watching());
        assert_eq!(s.take_lines().len(), 1);
        // A second later it is tried again; said no more.
        s.lock().failed = Some(std::time::Instant::now() - sound_out::DEFAULT_CHECK);
        s.open();
        s.opened();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert!(s.take_lines().is_empty());
        let now = std::time::Instant::now();
        let soon = now + std::time::Duration::from_millis(100);
        assert_eq!(check_wake(None, false, now), None);
        assert_eq!(check_wake(Some(soon), true, now), Some(soon));
        assert_eq!(
            check_wake(None, true, now),
            Some(now + sound_out::DEFAULT_CHECK)
        );
    }

    /// An opening that ends after the sound was no longer wanted (the window lost focus while
    /// the device opened) lets its stream go without a moment of the hum: the stream, while it
    /// lived, was given only silence (REVIEW-834b).
    #[test]
    fn an_opening_no_longer_wanted_sounds_nothing() {
        ds_mix::resampler::tables();
        type Kept = Arc<Mutex<Option<Arc<Mutex<dyn Source>>>>>;
        let kept: Kept = Arc::new(Mutex::new(None));
        let gate = Arc::new(std::sync::Barrier::new(2));
        let (k, g) = (kept.clone(), gate.clone());
        let opener: Opener = Arc::new(move |source| {
            *k.lock().unwrap() = Some(source);
            g.wait();
            Ok(Output::idle(48000, Layout::STEREO))
        });
        let s = Sounds::with_opener(vec![(Cue::Hum, bank(4800, 0, true))], opener, 7);
        s.open();
        s.release_later();
        gate.wait();
        s.opened();
        assert!(!s.is_open());
        let source = kept.lock().unwrap().take().expect("the opener was called");
        let mut out = vec![1f32; 2 * 4800];
        source.lock().unwrap().render(48000, 2, &mut out);
        assert!(out.iter().all(|&x| x == 0.0), "the hum sounded");
        // Wanted again: it opens and the hum is given to the stream.
        s.open();
        gate.wait();
        s.opened();
        assert!(s.is_open());
        let source = kept.lock().unwrap().take().expect("the opener was called");
        source.lock().unwrap().render(48000, 2, &mut out);
        assert!(out.iter().any(|&x| x != 0.0), "the hum did not start");
    }

    /// The terminal is heard only with its sounds on, its window shown and focused, and no Play
    /// work running.
    #[test]
    fn the_terminal_is_heard_only_when_shown_and_in_front() {
        assert!(heard(true, true, true, false));
        assert!(!heard(false, true, true, false));
        assert!(!heard(true, false, true, false));
        assert!(!heard(true, true, false, false));
        assert!(!heard(true, true, true, true));
    }

    /// The "enter" sound is kept by one handle: its file is picked once and every play uses it;
    /// a typed character's sound picks again each time (REVIEW-834).
    #[test]
    fn the_enter_sound_keeps_its_file_and_a_character_picks_anew() {
        let three = |frames: [usize; 3]| Bank {
            clips: frames.iter().map(|&n| clip(n, 48000)).collect(),
            volume: 0,
            looping: false,
        };
        let banks = vec![
            (Cue::Enter, three([100, 200, 300])),
            (Cue::Single, three([110, 220, 330])),
        ];
        let s = sounds_on(48000, Layout::STEREO, banks.clone());
        assert_eq!(s.fixed.len(), 1);
        assert_eq!(s.fixed[0].0, Cue::Enter);
        let Some(mut m) = Mix::new(48000, Layout::STEREO, banks, 3, s.fixed.clone()) else {
            return;
        };
        let mut enter = Vec::new();
        let mut single = Vec::new();
        for _ in 0..6 {
            m.stop();
            m.play(Cue::Enter);
            enter.push(m.playing[0].frames);
            m.play(Cue::Single);
            single.push(m.playing[0].frames);
        }
        assert!(enter.iter().all(|&f| f == enter[0]), "{enter:?}");
        assert!(single.windows(2).all(|w| w[0] != w[1]), "{single:?}");
    }

    /// No banks (no game, no files): nothing opens, nothing plays, no panic.
    #[test]
    fn without_its_files_the_terminal_is_silent() {
        let s = Sounds::new(Vec::new());
        s.open();
        s.opened();
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
        let s = Sounds::new(banks);
        s.open();
        s.opened();
        assert!(s.is_open(), "{:?}", s.take_lines());
        println!("hum playing, muted, for {hold} s");
        std::thread::sleep(std::time::Duration::from_secs(hold));
        s.release();
    }
}
