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

/// Each draw's device state hashed, in the order the draws come ([`StateDigest::new`]'s list):
/// every state set so far (render, stage and sampler states, textures, transforms, viewport,
/// scissor, clip planes, targets, streams, indices, the vertex layout, shaders and each constant
/// register, material and lights) by its last value, every other command so far (creations,
/// uploads, clears, copies, queries issued, presents) in order, and the draw itself. A new render
/// target 0 resets the viewport and the scissor rectangle, and `Reset` every state, as the device
/// does. Left out, as [`crate::Stream::record_repeatable`] leaves them out: the real windows'
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
            },
            digests,
        )
    }

    fn set(&mut self, key: Key) {
        self.state.insert(key, self.encoded.clone());
    }

    fn event(&mut self) {
        self.events = wire::hash(self.events, &self.encoded);
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
            Cmd::SetTexture { sampler, .. } => self.set(at(sampler, 0)),
            Cmd::SetShader { pixel, .. } => self.set(at(u32::from(pixel), 0)),
            Cmd::SetStreamSource { stream, .. } => self.set(at(stream, 0)),
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
            | Cmd::SetIndices { .. }
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
            | Cmd::DrawPrimitiveUp { .. } => self.draw(),
            _ => self.event(),
        }
        reply
    }
}

impl Drop for StateDigest {
    fn drop(&mut self) {
        self.push(self.destroyed);
    }
}
