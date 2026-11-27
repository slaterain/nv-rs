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
    /// Records given up before their draw (a `Present`, a reset, too many commands).
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
            // 4-aligned, as the stream keeps words.
            bytes.resize(bytes.len().next_multiple_of(4), 0);
            let at = bytes.len() as u32;
            let b = d.bytes();
            bytes.extend_from_slice(&b);
            (at, b.len() as u32)
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

    pub fn clear(&mut self) {
        *self = Commands {
            scratch: std::mem::take(&mut self.scratch),
            ..Commands::default()
        };
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

type Record = Commands;

/// A record being made: its number, its form, its commands so far.
struct Pending {
    id: u32,
    check: bool,
    record: Record,
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

/// A backend that carries out [`Cmd::RecordNext`], [`Cmd::DrawItem`] and [`Cmd::ForgetRecords`]
/// and hands every other command to `inner` ([`Form`] says which form of a check run it carries
/// out). Without them it adds one test a command.
pub struct Kept<B> {
    inner: B,
    form: Form,
    records: HashMap<u32, Record>,
    pending: Option<Pending>,
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
        if u64::from(constants.len()) != r.constants {
            let want = r.constants;
            self.counts.mismatched += 1;
            self.warn(format_args!(
                "an item of record {id} with {} bytes of registers, not {want}",
                constants.len()
            ));
            return;
        }
        let given = constants.bytes();
        let Kept { inner, records, .. } = self;
        for cmd in records[&id].with_registers(&given) {
            inner.execute(cmd);
        }
        self.counts.items += 1;
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
        match cmd {
            Cmd::RecordNext {
                id, forget, check, ..
            } => {
                self.cut();
                if forget != 0 {
                    self.records.remove(&forget);
                }
                self.pending = Some(Pending {
                    id,
                    check: check != 0,
                    record: Record::default(),
                });
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
                    self.records.insert(p.id, p.record);
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
