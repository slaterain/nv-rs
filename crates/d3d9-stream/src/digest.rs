//! A backend for checks ([`StateDigest`]): each draw's device state as one hash, so two streams
//! whose commands differ only in states set again to the value they had give the same digests,
//! and any difference a draw could see gives another.

use crate::cmd::{Backend, Cmd, Data, Reply};
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

/// Each draw's device state hashed, in the order the draws come ([`StateDigest::new`]'s list):
/// every state set so far (render, stage and sampler states, textures, transforms, viewport,
/// scissor, clip planes, targets, streams, indices, the vertex layout, shaders and each constant
/// register, material and lights) by its last value, every other command so far (creations,
/// uploads, clears, copies, queries, presents) in order, and the draw itself. A backend for tests:
/// it answers the sync commands with nothing.
pub struct StateDigest {
    state: BTreeMap<Key, Vec<u8>>,
    /// The hash of every command so far that sets no state and draws nothing.
    events: u64,
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
                digests: Arc::clone(&digests),
                encoded: Vec::new(),
            },
            digests,
        )
    }

    fn set(&mut self, key: Key) {
        self.state.insert(key, self.encoded.clone());
    }

    fn draw(&mut self) {
        let mut h = self.events;
        for (key, value) in &self.state {
            h = wire::hash(h, format!("{key:?}").as_bytes());
            h = wire::hash(h, value);
        }
        h = wire::hash(h, &self.encoded);
        self.digests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(h);
    }
}

impl Backend for StateDigest {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        self.encoded.clear();
        wire::encode(&cmd, &mut self.encoded);
        let name = cmd.name();
        let at = |a: u32, b: u32| Key::At(name, a, b);
        match cmd {
            Cmd::SetRenderTarget { index, .. } => self.set(at(index, 0)),
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
            Cmd::SetDepthStencilSurface { .. }
            | Cmd::SetViewport { .. }
            | Cmd::SetScissorRect { .. }
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
            _ => self.events = wire::hash(self.events, &self.encoded),
        }
        match cmd {
            Cmd::CreateDevice { .. } => Reply::Device {
                created: true,
                native: None,
            },
            Cmd::ReadSurface { .. } => Reply::Bytes(None),
            Cmd::GetRenderTargetData { .. } => Reply::Rows(None),
            Cmd::QueryGetData { .. } => Reply::Query(None),
            _ => Reply::None,
        }
    }
}
