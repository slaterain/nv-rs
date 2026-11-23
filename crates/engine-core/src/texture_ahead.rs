//! A seam between the game's texture making and the platform layer's D3DX translation: the loading thread that is
//! about to give a texture to the renderer (with the fixes preset's "texture files read before the renderer's lock"
//! row, its file is already read then) hands the file's bytes over here before it takes the renderer's source-data
//! lock, so the platform layer can work out the D3DX translation then (milliseconds for a large texture) and the
//! `D3DXCreateTextureFromFileInMemory` the making calls under the lock only finds the answer. Main, drawing, waits
//! on that lock while a loading thread holds it (the fight's start: about 50 waits a frame for 58 new textures).
//!
//! The platform layer installs its function ([`set`]); without one (unit tests of the game code) nothing happens.

use crate::Engine;
use std::sync::OnceLock;

/// The platform layer's preparing function: the file's bytes at `buffer` (`size` of them; 0 drops what the thread
/// kept) for the device `device`.
pub type Prepare = fn(&mut Engine, u32, u32, u32);

static PREPARE: OnceLock<Prepare> = OnceLock::new();

/// The platform layer's preparing function (the first one set stays).
pub fn set(f: Prepare) {
    let _ = PREPARE.set(f);
}

/// The texture file of `size` bytes at `buffer`, for `device`, is about to be made into a texture on this thread.
pub fn prepare(e: &mut Engine, buffer: u32, size: u32, device: u32) {
    if let Some(f) = PREPARE.get() {
        f(e, buffer, size, device);
    }
}

/// This thread's making is over: whatever it prepared and the making did not take is dropped.
pub fn done(e: &mut Engine) {
    prepare(e, 0, 0, 0);
}
