//! What DirectSound does with the game's sound buffers, as measured on the player's machine
//! (docs/SOUND.md): the mixer ([`mixer`]), DirectSound's two sample-rate converters
//! ([`resampler`]), where each kind of buffer goes on the device's speakers ([`speakers`]), the 3D
//! stage ([`ds3d`]), the distortion effect ([`dsfx`]) and the output to the player's default
//! device ([`sound_out`]: WASAPI on Windows, cpal on Linux and macOS).
//!
//! These were modules of `engine-platform` (its DirectSound layer, `dsound.rs`, and DirectShow's
//! music, `dshow.rs`, drive them); they live here so that a program without the engine, the
//! launcher, plays its sound through the same code. `engine-platform` re-exports each module under
//! its old name. Nothing here knows the engine.

// The lints engine-platform allows for its whole crate, kept for these modules as they were
// there (their code moved unchanged).
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::neg_multiply)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::manual_saturating_arithmetic)]
#![allow(clippy::eq_op)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::assign_op_pattern)]
#![cfg_attr(test, allow(clippy::type_complexity))]
#![cfg_attr(test, allow(clippy::useless_vec))]

#[cfg(test)]
#[path = "../../engine-platform/src/alloc_count.rs"]
mod alloc_count;
pub mod ds3d;
pub mod dsfx;
pub mod mixer;
pub mod resampler;
pub mod sound_out;
pub mod speakers;
