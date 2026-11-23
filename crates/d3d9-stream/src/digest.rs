//! A backend for checks ([`StateDigest`]): each draw's device state as one hash, so two streams
//! whose commands differ only in states set again to the value they had give the same digests,
//! and any difference a draw could see gives another.

use crate::cmd::{Backend, Cmd, Data, Reply};
use crate::runner::repeatable_form;
use crate::wire;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Which state a command sets: its kind and the fields that say which one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// A state by the setting command's name and its index fields.
    At(&'static str, u32, u32),
    /// One shader constant register: its kind (float, int, bool for each stage) and number.
    Constant(u8, u32),
}

/// The viewport's and the scissor rectangle's keys: a new render target 0 sets both to its whole
/// size, so they hold that target's command until set again.
const VIEWPORT: Key = Key::At("SetViewport", 0, 0);
const SCISSOR: Key = Key::At("SetScissorRect", 0, 0);
/// The scissor test's render state (`D3DRS_SCISSORTESTENABLE`, 174): whether a `Clear` keeps to the rectangle.
const SCISSOR_TEST: Key = Key::At("SetRenderState", 174, 0);
/// Stream 0's key: `DrawPrimitiveUP` leaves it empty, as the device does.
const STREAM_0: Key = Key::At("SetStreamSource", 0, 0);

/// Whether a `Clear` reaches what `key` holds: the render targets, the depth surface, the viewport, the scissor
/// rectangle and its test. A `Clear` is hashed with them ([`StateDigest::clear`]), so a wrong target seen only by a
/// `Clear` is found there.
fn cleared_by(key: &Key) -> bool {
    matches!(
        key,
        Key::At("SetRenderTarget", _, _) | Key::At("SetDepthStencilSurface", _, _)
    ) || [VIEWPORT, SCISSOR, SCISSOR_TEST].contains(key)
}

/// Each draw's device state hashed, in the order the draws come ([`StateDigest::new`]'s list):
/// every state set so far (render, stage and sampler states, textures, transforms, viewport,
/// scissor, clip planes, targets, streams, indices, the vertex layout, shaders and each constant
/// register, material and lights) by its last value, every other command so far (creations,
/// uploads, clears, copies, queries issued, presents) in order, and the draw itself. A new render
/// target 0 resets the viewport and the scissor rectangle, `DrawPrimitiveUP` empties stream 0, and
/// `Reset` every state, as the device does; a state that holds an object set to none (no texture, shader, index or
/// vertex buffer) is the device's default and kept as never set ([`StateDigest::bind`]); a `Clear` is hashed with what it clears into (the
/// targets, the depth surface, the viewport, the scissor rectangle and its test). Left out, as [`crate::Stream::record_repeatable`] leaves them out: the real windows'
/// handles and the query reads (made as often as the GPU takes). The objects destroyed are hashed
/// as a set (the drawing side's ends come at the join, after their place in the serial order),
/// one more value at the end of the list once the backend is dropped. A backend for tests: it
/// answers the sync commands with nothing.
pub struct StateDigest {
    state: BTreeMap<Key, Vec<u8>>,
    /// The hash of every command so far that sets no state and draws nothing.
    events: u64,
    /// The objects destroyed: their commands' hashes added up, in any order.
    destroyed: u64,
    digests: Arc<Mutex<Vec<u64>>>,
    encoded: Vec<u8>,
    /// For [`StateDigest::stopping_at`]: the draw to describe, the draws and presents so far, the draws since the last
    /// present, the commands since the last draw that set no state (short forms), and the description once made.
    stop_at: Option<usize>,
    draws: usize,
    presents: u64,
    in_frame: u64,
    recent: std::collections::VecDeque<String>,
    snapshot: Option<DrawSnapshot>,
}

/// One draw's device state as [`StateDigest`] hashes it, in the open ([`crate::wire::draw_snapshot`]): which present it
/// comes after and its place among the draws since, every state by name with its bytes, the hash of the other commands
/// so far with the last of them, and the draw command.
#[derive(Clone, Debug, Default)]
pub struct DrawSnapshot {
    pub presents: u64,
    pub in_frame: u64,
    pub state: Vec<(String, Vec<u8>)>,
    pub events: u64,
    pub recent: Vec<String>,
    pub draw: String,
}

impl StateDigest {
    /// The backend, and the list its draws' digests go to.
    pub fn new() -> (StateDigest, Arc<Mutex<Vec<u64>>>) {
        let digests = Arc::new(Mutex::new(Vec::new()));
        (
            StateDigest {
                state: BTreeMap::new(),
                events: wire::HASH_START,
                destroyed: 0,
                digests: Arc::clone(&digests),
                encoded: Vec::new(),
                stop_at: None,
                draws: 0,
                presents: 0,
                in_frame: 0,
                recent: std::collections::VecDeque::new(),
                snapshot: None,
            },
            digests,
        )
    }

    /// As [`StateDigest::new`], and draw `n` (counted from 0) described in the open ([`StateDigest::snapshot`]).
    pub fn stopping_at(n: usize) -> (StateDigest, Arc<Mutex<Vec<u64>>>) {
        let (mut d, list) = StateDigest::new();
        d.stop_at = Some(n);
        (d, list)
    }

    /// The description of the draw [`StateDigest::stopping_at`] named, once it has come.
    pub fn snapshot(&self) -> Option<&DrawSnapshot> {
        self.snapshot.as_ref()
    }

    fn set(&mut self, key: Key) {
        self.state.insert(key, self.encoded.clone());
    }

    fn event(&mut self) {
        self.events = wire::hash(self.events, &self.encoded);
    }

    /// A `Clear`: hashed as an event with the state it clears into ([`cleared_by`]).
    fn clear(&mut self) {
        let mut h = wire::hash(self.events, &self.encoded);
        for (key, value) in self.state.iter().filter(|(key, _)| cleared_by(key)) {
            h = wire::hash(h, format!("{key:?}").as_bytes());
            h = wire::hash(h, value);
        }
        self.events = h;
    }

    /// A state that holds an object (a texture on a sampler, a shader, the index buffer, a stream's vertex buffer):
    /// `empty` (no object, and for a stream offset and stride 0) is what the device holds from its creation or a
    /// `Reset` until it is set, so it is kept as never set; any object is kept by its bytes. A stream that sends
    /// "none" where another sends nothing (a cache that skips a set equal to what it holds on one side only) leaves
    /// the device as it was, and every draw after sees the same; it cannot hide a difference a draw could see, since
    /// an object on one side and none on the other still differ (kept against absent), as do two objects.
    fn bind(&mut self, key: Key, empty: bool) {
        if empty {
            self.state.remove(&key);
        } else {
            self.set(key);
        }
    }

    /// Stream 0 emptied, as `DrawPrimitiveUP` leaves it (its `SetStreamSource(0, NULL, 0, 0)`): the device's default
    /// ([`StateDigest::bind`]).
    fn empty_stream_0(&mut self) {
        self.state.remove(&STREAM_0);
    }

    fn push(&self, digest: u64) {
        self.digests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(digest);
    }

    fn draw(&mut self) {
        let mut h = self.events;
        for (key, value) in &self.state {
            h = wire::hash(h, format!("{key:?}").as_bytes());
            h = wire::hash(h, value);
        }
        h = wire::hash(h, &self.encoded);
        self.push(h);
        if self.stop_at == Some(self.draws) {
            self.snapshot = Some(DrawSnapshot {
                presents: self.presents,
                in_frame: self.in_frame,
                state: self
                    .state
                    .iter()
                    .map(|(k, v)| (format!("{k:?}"), v.clone()))
                    .collect(),
                events: self.events,
                recent: self.recent.iter().cloned().collect(),
                draw: String::new(),
            });
        }
        self.draws += 1;
        self.in_frame += 1;
    }
}

impl Backend for StateDigest {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        let reply = match cmd {
            Cmd::CreateDevice { .. } => Reply::Device {
                created: true,
                native: None,
            },
            Cmd::ReadSurface { .. } => Reply::Bytes(None),
            Cmd::GetRenderTargetData { .. } => Reply::Rows(None),
            Cmd::QueryGetData { .. } => Reply::Query(None),
            _ => Reply::None,
        };
        let Some(written) = repeatable_form(cmd) else {
            return reply;
        };
        self.encoded.clear();
        wire::encode(&written, &mut self.encoded);
        let name = cmd.name();
        let at = |a: u32, b: u32| Key::At(name, a, b);
        match cmd {
            Cmd::SetRenderTarget { index, .. } => {
                self.set(at(index, 0));
                if index == 0 {
                    self.set(VIEWPORT);
                    self.set(SCISSOR);
                }
            }
            Cmd::Reset { .. } => {
                self.state.clear();
                self.event();
            }
            Cmd::Destroy { .. } => {
                self.destroyed = self
                    .destroyed
                    .wrapping_add(wire::hash(wire::HASH_START, &self.encoded));
            }
            Cmd::SetTransform { state, .. } => self.set(at(state, 0)),
            Cmd::SetClipPlane { index, .. } => self.set(at(index, 0)),
            Cmd::SetRenderState { state, .. } => self.set(at(state, 0)),
            Cmd::SetTextureStageState { stage, kind, .. } => self.set(at(stage, kind)),
            Cmd::SetSamplerState { sampler, kind, .. } => self.set(at(sampler, kind)),
            Cmd::SetTexture {
                sampler, texture, ..
            } => self.bind(at(sampler, 0), texture == 0),
            Cmd::SetShader { pixel, shader, .. } => self.bind(at(u32::from(pixel), 0), shader == 0),
            Cmd::SetStreamSource {
                stream,
                buffer,
                offset,
                stride,
                ..
            } => self.bind(at(stream, 0), buffer == 0 && offset == 0 && stride == 0),
            Cmd::SetIndices { buffer, .. } => self.bind(at(0, 0), buffer == 0),
            Cmd::SetLight { index, .. } | Cmd::LightEnable { index, .. } => self.set(at(index, 0)),
            Cmd::SetLod { texture, .. } => self.set(at(texture, 0)),
            // A declaration and an FVF replace each other: one vertex layout.
            Cmd::SetVertexDeclaration { .. } | Cmd::SetFvf { .. } => {
                self.set(Key::At("vertex layout", 0, 0))
            }
            Cmd::SetViewport { .. } => self.set(VIEWPORT),
            Cmd::SetScissorRect { .. } => self.set(SCISSOR),
            Cmd::SetDepthStencilSurface { .. }
            | Cmd::SetGammaRamp { .. }
            | Cmd::SetMaterial { .. }
            | Cmd::SetSoftwareVertexProcessing { .. } => self.set(at(0, 0)),
            Cmd::SetConstants {
                kind,
                start,
                count,
                data,
                ..
            } => {
                let bytes = data.bytes();
                let per = bytes.len() / count.max(1) as usize;
                for r in 0..count {
                    let at = r as usize * per;
                    self.state
                        .insert(Key::Constant(kind, start + r), bytes[at..at + per].to_vec());
                }
            }
            Cmd::DrawPrimitive { .. }
            | Cmd::DrawIndexedPrimitive { .. }
            | Cmd::DrawPrimitiveUp { .. } => {
                let at = self.draws;
                self.draw();
                if self.stop_at == Some(at) {
                    if let Some(s) = self.snapshot.as_mut() {
                        s.draw = short(&format!("{cmd:?}"));
                    }
                }
                self.recent.clear();
                if let Cmd::DrawPrimitiveUp { .. } = cmd {
                    self.empty_stream_0();
                }
            }
            Cmd::Clear { .. } => {
                self.clear();
                self.note_recent(name, &cmd);
            }
            _ => {
                self.event();
                self.note_recent(name, &cmd);
            }
        }
        reply
    }
}

impl StateDigest {
    /// For [`StateDigest::stopping_at`]: a command that sets no state and draws nothing noted among the recent ones,
    /// and a present counted.
    fn note_recent(&mut self, name: &str, cmd: &Cmd<Data<'_>>) {
        if self.stop_at.is_some() {
            if name == "Present" {
                self.presents += 1;
                self.in_frame = 0;
            }
            self.recent.push_back(short(&format!("{cmd:?}")));
            if self.recent.len() > 12 {
                self.recent.pop_front();
            }
        }
    }
}

/// A command's debug form cut to 160 characters (uploads carry their bytes).
fn short(s: &str) -> String {
    s.chars().take(160).collect()
}

impl Drop for StateDigest {
    fn drop(&mut self) {
        self.push(self.destroyed);
    }
}
