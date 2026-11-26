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

/// A record: its commands, each payload a range of `bytes`.
#[derive(Default)]
struct Record {
    cmds: Vec<Cmd<(u32, u32)>>,
    bytes: Vec<u8>,
    /// The bytes its `SetConstants` take, together: what an item brings.
    constants: u64,
}

impl Record {
    fn keep(&mut self, cmd: Cmd<Data<'_>>) {
        let bytes = &mut self.bytes;
        let kept = cmd.map(|d| {
            // 4-aligned, as the stream keeps words.
            bytes.resize(bytes.len().next_multiple_of(4), 0);
            let at = bytes.len() as u32;
            let b = d.bytes();
            bytes.extend_from_slice(&b);
            (at, b.len() as u32)
        });
        if let Cmd::SetConstants { data: (_, len), .. } = kept {
            self.constants += u64::from(len);
        }
        self.cmds.push(kept);
    }
}

/// A record being made: its number, its form, its commands so far.
struct Pending {
    id: u32,
    check: bool,
    record: Record,
}

/// The most commands one record keeps: a draw's own commands are a handful (streams, indices,
/// constants, the draw); more means the draw never came, and the record is given up.
const MOST_COMMANDS: usize = 256;

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
        }
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
        let mut at = 0usize;
        let Kept { inner, records, .. } = self;
        let r = &records[&id];
        for c in &r.cmds {
            let cmd = match *c {
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
                    Data::Bytes(&r.bytes[from as usize..from as usize + len as usize])
                }),
            };
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
                let Some(p) = self.pending.as_mut() else {
                    return self.inner.execute(cmd);
                };
                if matches!(cmd, Cmd::Present { .. }) || p.record.cmds.len() >= MOST_COMMANDS {
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
