//! Kept draw records ([`Kept`]): a draw's own commands kept by number and carried out again for a
//! [`Cmd::DrawItem`], the renderer design's items (stage 1).
//!
//! The game side marks a draw it will draw again with [`Cmd::RecordNext`]: the commands that follow,
//! up to and including the next draw, are carried out as they come and kept as that record. Later,
//! while the bytes the draw's set-up read stay the same, it sends one [`Cmd::DrawItem`] in place of
//! those commands: the kept ones are carried out again in their order, each `SetConstants` with its
//! registers from the item (they follow the camera, the sun and the lamps; everything else is the
//! record's). So the backend gets exactly the commands the draw would have sent, and the device
//! state after an item is the one after the draw.
//!
//! A check run (`NV_DRAW_ITEMS=check`) sends both forms of each draw it could have drawn from its
//! record: the item ([`Cmd::DrawItem`] with `check` 1) and then its old form (the draw's own commands
//! after a [`Cmd::RecordNext`] with `check` 1, which also renew the record as the game side renews
//! its own). [`Form::Old`] carries out the old form and leaves the item; [`Form::Items`] draws the
//! item and keeps the old form without carrying it out: one recording replayed both ways gives the
//! two pictures and the two digest lists to compare.
//!
//! **Patched items** (the design's item form, A.4): a record drawn with no command of its set-up at
//! all, so the game side neither calls the state cache's setters nor the device for it. The record
//! is then every command of the draw from its set-up's start (the `RecordNext` before the set-up),
//! and, right after its draw, the states its setter calls leave ([`Cmd::KeepStates`] and that many
//! state sets: each state the setters set, to its last value, whether the cache let the set through
//! or not). A [`Cmd::DrawPatched`] sets those states, carries out the record's commands with their
//! own registers, sets its patches (the registers that follow the camera, the sun and the lamps)
//! and draws. The states the setters set end as the set-up leaves them (the record's own sets come
//! after the kept states, and a set the cache let through is the setter's last value); every other
//! state is the one the device holds, as for the draw itself: nothing is taken from the time the
//! record was made.

use crate::cmd::{Backend, Cmd, Data, Reply};
use std::collections::HashMap;

/// Which form of a check run's draws a [`Kept`] carries out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Form {
    /// The draws' own commands; a check run's items are left (play, and a check run replayed
    /// "old").
    #[default]
    Old,
    /// A check run's items drawn from their records; the old forms beside them only kept.
    Items,
}

/// What a [`Kept`] did, counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Records kept.
    pub records: u64,
    /// Items carried out.
    pub items: u64,
    /// Check items left ([`Form::Old`]) or old forms only kept ([`Form::Items`]).
    pub other_form: u64,
    /// Items of a record not kept (never made, dropped, or cut short): nothing drawn.
    pub missing: u64,
    /// Items whose registers are not the size of their record's: nothing drawn.
    pub mismatched: u64,
    /// Records given up before their draw (a `Present`, a reset, too many commands), and kept states
    /// not kept ([`Cmd::KeepStates`] for a record other than the one made last, more of them than a
    /// record keeps, or cut short by a command that is no state).
    pub cut: u64,
}

/// Commands kept as they came (a record, or the platform's held draw commands), each payload a
/// range of `bytes`, with the hash of their shape: every command whole but each `SetConstants`'
/// registers (an item brings those).
pub struct Commands {
    cmds: Vec<Cmd<(u32, u32)>>,
    bytes: Vec<u8>,
    /// The bytes their `SetConstants` take, together: what an item brings.
    constants: u64,
    shape: u64,
    scratch: Vec<u8>,
}

impl Default for Commands {
    fn default() -> Commands {
        Commands {
            cmds: Vec::new(),
            bytes: Vec::new(),
            constants: 0,
            shape: crate::wire::HASH_START,
            scratch: Vec::new(),
        }
    }
}

impl Commands {
    pub fn keep(&mut self, cmd: Cmd<Data<'_>>) {
        let bytes = &mut self.bytes;
        let kept = cmd.map(|d| {
            // 4-aligned, as the stream keeps words; read straight into place (no copy of their own made).
            let at = bytes.len().next_multiple_of(4);
            let len = d.len() as usize;
            bytes.resize(at + len, 0);
            d.copy_to(0, &mut bytes[at..]);
            (at as u32, len as u32)
        });
        self.scratch.clear();
        if let Cmd::SetConstants { data: (_, len), .. } = kept {
            self.constants += u64::from(len);
            crate::wire::encode(&cmd.map(|_| Data::Bytes(&[])), &mut self.scratch);
        } else {
            crate::wire::encode(&cmd, &mut self.scratch);
        }
        self.shape = crate::wire::hash(self.shape, &self.scratch);
        self.cmds.push(kept);
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    /// The hash of the commands but their registers: two lists of the same shape differ at most
    /// in their registers.
    pub fn shape(&self) -> u64 {
        self.shape
    }

    /// The bytes of their registers, together.
    pub fn constant_bytes(&self) -> u64 {
        self.constants
    }

    /// Empty again, keeping the room it had (the platform's held draw commands are cleared at every draw).
    pub fn clear(&mut self) {
        self.cmds.clear();
        self.bytes.clear();
        self.constants = 0;
        self.shape = crate::wire::HASH_START;
    }

    /// The commands as they came.
    pub fn iter(&self) -> impl Iterator<Item = Cmd<Data<'_>>> {
        self.cmds.iter().map(|c| {
            c.map(|(from, len)| {
                Data::Bytes(&self.bytes[from as usize..from as usize + len as usize])
            })
        })
    }

    /// The commands with each `SetConstants`' registers taken in turn from `given` (as long as
    /// [`Commands::constant_bytes`]).
    pub fn with_registers<'a>(&'a self, given: &'a [u8]) -> impl Iterator<Item = Cmd<Data<'a>>> {
        let mut at = 0usize;
        self.cmds.iter().map(move |c| match *c {
            Cmd::SetConstants {
                device,
                kind,
                start,
                count,
                data: (_, len),
            } => {
                let len = len as usize;
                let data = Data::Bytes(&given[at..at + len]);
                at += len;
                Cmd::SetConstants {
                    device,
                    kind,
                    start,
                    count,
                    data,
                }
            }
            other => other.map(|(from, len)| {
                Data::Bytes(&self.bytes[from as usize..from as usize + len as usize])
            }),
        })
    }
}

/// A kept record: the draw's commands, and the states it sets first as a patched item.
#[derive(Default)]
struct Record {
    cmds: Commands,
    /// [`Cmd::KeepStates`]' commands; `None` until they came (a [`Cmd::DrawPatched`] of it draws nothing).
    states: Option<Commands>,
}

/// A record being made: its number, its form, its commands so far.
struct Pending {
    id: u32,
    check: bool,
    record: Commands,
}

/// [`Cmd::KeepStates`]' commands being taken: the record they are for (`None`: dropped), how many are
/// still to come.
struct Taking {
    id: Option<u32>,
    left: u32,
    states: Commands,
}

/// Whether `cmd` is a state set that [`Cmd::KeepStates`] may keep: what a set-up's setter calls send
/// (render, stage and sampler states, textures, the declaration or format, shaders, streams, indices,
/// registers, clip planes, the fixed-function material and lights). Never a draw, a target, a
/// transform (the port's own frame values travel as transforms), a resource or a sync call.
pub fn is_state<P>(cmd: &Cmd<P>) -> bool {
    matches!(
        cmd,
        Cmd::SetRenderState { .. }
            | Cmd::SetTextureStageState { .. }
            | Cmd::SetSamplerState { .. }
            | Cmd::SetTexture { .. }
            | Cmd::SetVertexDeclaration { .. }
            | Cmd::SetFvf { .. }
            | Cmd::SetShader { .. }
            | Cmd::SetStreamSource { .. }
            | Cmd::SetIndices { .. }
            | Cmd::SetConstants { .. }
            | Cmd::SetClipPlane { .. }
            | Cmd::SetMaterial { .. }
            | Cmd::SetLight { .. }
            | Cmd::LightEnable { .. }
    )
}

/// One patch of a [`Cmd::DrawPatched`]: a `SetConstants`' kind, first register, register count and
/// registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patch<'a> {
    pub kind: u8,
    pub start: u32,
    pub count: u32,
    pub registers: &'a [u8],
}

/// The bytes one register of `SetConstants` kind `kind` takes (a boolean's word, else four words).
fn register_bytes(kind: u32) -> u64 {
    if kind % 3 == 2 {
        4
    } else {
        16
    }
}

/// A [`Cmd::DrawPatched`]'s patches, in order; an error names the first that is not whole.
pub fn patches(bytes: &[u8]) -> impl Iterator<Item = Result<Patch<'_>, String>> {
    let mut at = 0usize;
    std::iter::from_fn(move || {
        let here = at;
        if here == bytes.len() {
            return None;
        }
        let word = |i: usize| {
            bytes
                .get(at + 4 * i..at + 4 * i + 4)
                .map(|w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
        };
        let (Some(kind), Some(start), Some(count)) = (word(0), word(1), word(2)) else {
            at = bytes.len();
            return Some(Err(format!("a patch's head at byte {here} is cut short")));
        };
        if kind > 5 {
            at = bytes.len();
            return Some(Err(format!("a patch of kind {kind}")));
        }
        let len = register_bytes(kind) * u64::from(count);
        let from = at + 12;
        let Some(to) = usize::try_from(len)
            .ok()
            .and_then(|len| from.checked_add(len))
            .filter(|&to| to <= bytes.len())
        else {
            at = bytes.len();
            return Some(Err(format!(
                "a patch of {count} registers of kind {kind} is cut short"
            )));
        };
        at = to;
        Some(Ok(Patch {
            kind: kind as u8,
            start,
            count,
            registers: &bytes[from..to],
        }))
    })
}

/// Whether a [`Cmd::DrawPatched`]'s patches are whole ([`Cmd::check`]).
pub fn patches_check(bytes: &[u8]) -> Result<(), String> {
    patches(bytes).try_for_each(|p| p.map(|_| ()))
}

/// The most commands one record keeps: a draw's own commands are a handful (streams, indices,
/// constants, the draw); more means the draw never came, and the record is given up. The game's
/// side holds no longer run (`engine_platform`'s `d3d9items` sends a longer one as it came).
pub const MOST_COMMANDS: usize = 256;

/// A reporting [`Kept`] ([`Kept::reporting`]) prints its counts every this many frames.
const REPORT_PRESENTS: u64 = 1000;

/// ... and, when an item found no record or the wrong registers, or a record was cut, at most
/// this often (frames).
const BAD_REPORT_GAP: u64 = 60;

/// A backend that carries out [`Cmd::RecordNext`], [`Cmd::DrawItem`], [`Cmd::ForgetRecords`],
/// [`Cmd::KeepStates`] and [`Cmd::DrawPatched`], and hands every other command to `inner` ([`Form`]
/// says which form of a check run it carries out). Without them it adds one test a command.
pub struct Kept<B> {
    inner: B,
    form: Form,
    records: HashMap<u32, Record>,
    pending: Option<Pending>,
    /// The record whose draw came last, while no other record began: what [`Cmd::KeepStates`] may name.
    last_made: Option<u32>,
    taking: Option<Taking>,
    counts: Counts,
    warned: bool,
    /// The game's own run ([`Kept::reporting`]): its counts printed at `Present`s; the frames
    /// seen, the bad counts last printed and the frame before which no bad count is printed.
    report: bool,
    presents: u64,
    reported_bad: u64,
    quiet_until: u64,
}

impl<B: Backend> Kept<B> {
    /// `inner` with the records, carrying out the old form of a check run's draws.
    pub fn new(inner: B) -> Kept<B> {
        Kept::with_form(inner, Form::Old)
    }

    pub fn with_form(inner: B, form: Form) -> Kept<B> {
        Kept {
            inner,
            form,
            records: HashMap::new(),
            pending: None,
            last_made: None,
            taking: None,
            counts: Counts::default(),
            warned: false,
            report: false,
            presents: 0,
            reported_bad: 0,
            quiet_until: 0,
        }
    }

    /// The game's own run: the counts go to the log ([`Kept::summary`]) every [`REPORT_PRESENTS`]
    /// frames, and soon after an item found no record or the wrong registers or a record was cut
    /// (the game's side cannot learn of those: such an item draws nothing).
    pub fn reporting(mut self) -> Kept<B> {
        self.report = true;
        self
    }

    /// The counts as the log line gives them.
    pub fn summary(&self) -> String {
        let c = self.counts;
        format!(
            "NV_DRAW_ITEMS backend: {} records kept ({} now), {} items drawn, {} of the other form; {} items of no record, {} with the wrong registers, {} records cut",
            c.records,
            self.records.len(),
            c.items,
            c.other_form,
            c.missing,
            c.mismatched,
            c.cut
        )
    }

    /// A frame presented: the line to print, if one is due ([`Kept::reporting`]).
    pub(crate) fn presented(&mut self) -> Option<String> {
        if !self.report {
            return None;
        }
        self.presents += 1;
        let bad = self.counts.missing + self.counts.mismatched + self.counts.cut;
        let due = self.presents % REPORT_PRESENTS == 0
            || (bad != self.reported_bad && self.presents >= self.quiet_until);
        if !due {
            return None;
        }
        if bad != self.reported_bad {
            self.reported_bad = bad;
            self.quiet_until = self.presents + BAD_REPORT_GAP;
        }
        Some(self.summary())
    }

    pub fn inner(&self) -> &B {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    pub fn into_inner(self) -> B {
        self.inner
    }

    pub fn counts(&self) -> Counts {
        self.counts
    }

    /// How many records are kept.
    pub fn records(&self) -> usize {
        self.records.len()
    }

    /// A record's end that is not its draw: it is given up.
    fn cut(&mut self) {
        if self.pending.take().is_some() {
            self.counts.cut += 1;
        }
    }

    fn warn(&mut self, what: std::fmt::Arguments<'_>) {
        if !self.warned {
            self.warned = true;
            eprintln!("kept draw records: {what} (counted from here on; first only)");
        }
    }

    fn draw_item(&mut self, id: u32, constants: Data<'_>) {
        let Some(r) = self.records.get(&id) else {
            self.counts.missing += 1;
            self.warn(format_args!("an item of record {id}, which is not kept"));
            return;
        };
        if u64::from(constants.len()) != r.cmds.constants {
            let want = r.cmds.constants;
            self.counts.mismatched += 1;
            self.warn(format_args!(
                "an item of record {id} with {} bytes of registers, not {want}",
                constants.len()
            ));
            return;
        }
        let given = constants.bytes();
        let Kept { inner, records, .. } = self;
        for cmd in records[&id].cmds.with_registers(&given) {
            inner.execute(cmd);
        }
        self.counts.items += 1;
    }

    /// A [`Cmd::DrawPatched`]: record `id`'s kept states, its commands up to its draw, the patches,
    /// its draw. Nothing for a record not kept or kept without its states, or for patches not whole.
    fn draw_patched(&mut self, device: u32, id: u32, given: Data<'_>) {
        let Some(r) = self.records.get(&id) else {
            self.counts.missing += 1;
            self.warn(format_args!(
                "a patched item of record {id}, which is not kept"
            ));
            return;
        };
        if r.states.is_none() {
            self.counts.missing += 1;
            self.warn(format_args!(
                "a patched item of record {id}, which was kept without its states"
            ));
            return;
        }
        let bytes = given.bytes();
        if let Err(why) = patches_check(&bytes) {
            self.counts.mismatched += 1;
            self.warn(format_args!("a patched item of record {id}: {why}"));
            return;
        }
        let Kept { inner, records, .. } = self;
        let r = &records[&id];
        for cmd in r.states.iter().flat_map(Commands::iter) {
            inner.execute(cmd);
        }
        // A record ends with its draw (it is kept only then).
        let last = r.cmds.len() - 1;
        let mut draw = None;
        for (i, cmd) in r.cmds.iter().enumerate() {
            if i == last {
                draw = Some(cmd);
            } else {
                inner.execute(cmd);
            }
        }
        for p in patches(&bytes).flatten() {
            inner.execute(Cmd::SetConstants {
                device,
                kind: p.kind,
                start: p.start,
                count: p.count,
                data: Data::Bytes(p.registers),
            });
        }
        inner.execute(draw.expect("a record's draw"));
        self.counts.items += 1;
    }

    /// A state of [`Cmd::KeepStates`] while they are taken; `false` (and they are given up) for any
    /// other command, which is then carried out as ever.
    fn take_state(&mut self, cmd: &Cmd<Data<'_>>) -> bool {
        let Some(t) = self.taking.as_mut() else {
            return false;
        };
        if !is_state(cmd) {
            self.taking = None;
            self.counts.cut += 1;
            self.warn(format_args!(
                "a record's kept states cut short by {}",
                cmd.name()
            ));
            return false;
        }
        if t.id.is_some() {
            t.states.keep(*cmd);
        }
        t.left -= 1;
        if t.left == 0 {
            let t = self.taking.take().expect("states being taken");
            if let Some(r) = t.id.and_then(|id| self.records.get_mut(&id)) {
                r.states = Some(t.states);
            }
        }
        true
    }
}

/// Whether `cmd` ends a record (a draw).
fn is_draw<P>(cmd: &Cmd<P>) -> bool {
    matches!(
        cmd,
        Cmd::DrawPrimitive { .. } | Cmd::DrawIndexedPrimitive { .. }
    )
}

impl<B: Backend> Backend for Kept<B> {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        if self.take_state(&cmd) {
            return Reply::None;
        }
        match cmd {
            Cmd::RecordNext {
                id, forget, check, ..
            } => {
                self.cut();
                if forget != 0 {
                    self.records.remove(&forget);
                }
                // A record made again: the old one goes now, so an item never draws it if the new
                // one is cut.
                self.records.remove(&id);
                self.last_made = None;
                self.pending = Some(Pending {
                    id,
                    check: check != 0,
                    record: Commands::default(),
                });
                Reply::None
            }
            Cmd::KeepStates { id, count, .. } => {
                self.cut();
                let id =
                    (self.last_made == Some(id) && self.records.contains_key(&id)).then_some(id);
                if id.is_none() || count as usize > MOST_COMMANDS {
                    self.counts.cut += 1;
                    self.warn(format_args!(
                        "{count} kept states of a record that is not the one made last, or too many"
                    ));
                }
                if count as usize > MOST_COMMANDS {
                    return Reply::None;
                }
                let t = Taking {
                    id,
                    left: count,
                    states: Commands::default(),
                };
                if count == 0 {
                    if let Some(r) = id.and_then(|id| self.records.get_mut(&id)) {
                        r.states = Some(t.states);
                    }
                } else {
                    self.taking = Some(t);
                }
                Reply::None
            }
            Cmd::DrawPatched {
                device,
                id,
                check,
                patches,
            } => {
                self.cut();
                if check != 0 && self.form == Form::Old {
                    self.counts.other_form += 1;
                } else {
                    self.draw_patched(device, id, patches);
                }
                Reply::None
            }
            Cmd::DrawItem {
                id,
                check,
                constants,
                ..
            } => {
                self.cut();
                if check != 0 && self.form == Form::Old {
                    self.counts.other_form += 1;
                } else {
                    self.draw_item(id, constants);
                }
                Reply::None
            }
            Cmd::ForgetRecords { ids, .. } => {
                self.cut();
                for id in ids.to_words().iter() {
                    self.records.remove(id);
                }
                Reply::None
            }
            Cmd::CreateDevice { .. } | Cmd::Reset { .. } => {
                // The records' objects are the old device's: all go.
                self.cut();
                self.records.clear();
                self.last_made = None;
                self.inner.execute(cmd)
            }
            _ => {
                if matches!(cmd, Cmd::Present { .. }) {
                    if let Some(line) = self.presented() {
                        eprintln!("{line}");
                    }
                }
                let Some(p) = self.pending.as_mut() else {
                    return self.inner.execute(cmd);
                };
                if matches!(cmd, Cmd::Present { .. }) || p.record.len() >= MOST_COMMANDS {
                    self.cut();
                    return self.inner.execute(cmd);
                }
                let only_kept = p.check && self.form == Form::Items && !cmd.is_sync();
                p.record.keep(cmd);
                let draw = is_draw(&cmd);
                if draw {
                    let p = self.pending.take().expect("a record being made");
                    if p.check {
                        self.counts.other_form += u64::from(only_kept);
                    }
                    self.counts.records += 1;
                    self.last_made = Some(p.id);
                    self.records.insert(
                        p.id,
                        Record {
                            cmds: p.record,
                            states: None,
                        },
                    );
                }
                if only_kept {
                    Reply::None
                } else {
                    self.inner.execute(cmd)
                }
            }
        }
    }

    fn idle(&mut self) -> bool {
        self.inner.idle()
    }
}
