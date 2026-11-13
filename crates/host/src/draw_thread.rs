//! The fixes row that turns the world's draw on a thread of its own on (step 2, piece 8; experimental, off in every
//! preset): the wiring of `engine::draw_thread`'s seam to the device model's two producers
//! (`engine_platform::d3d9draw`) and to the draw family's packet hand-off, and the serial conditions only the host
//! can see (docs/FIXES.md, "World drawn on a second thread").

use crate::fixes::Choice;
use engine::draw_thread::{Hooks, Packet};
use engine::engine_typed;
use engine::prelude::*;
use engine::typed::FamilyId;

/// The typed families the world draw runs in (`measure/scope-1111/PLAN.md` (b)): every one must be direct (a fix
/// row or a wrap on any member sends its family by address, which reads guest memory, not the packet).
pub fn draw_families() -> [(&'static str, FamilyId); 8] {
    [
        ("draw", engine_typed::draw::FAMILY),
        ("draw_f", engine_typed::draw_f::FAMILY),
        ("shader", engine_typed::shader::FAMILY),
        ("render", engine_typed::render::FAMILY),
        ("dispatch", engine_typed::dispatch::FAMILY),
        ("crt", engine_typed::crt::FAMILY),
        ("pathing", engine_typed::pathing::FAMILY),
        ("geometry", engine_typed::geometry::FAMILY),
    ]
}

/// Translated functions the world draw calls by address where an extender hooks today
/// (`measure/typed-draw/DRAW-THREAD.md`, "Extender hooks inside the draw set"): `00b54090` (Depth Resolve's
/// texture lookup). A wrap or an active fix row here draws the world on main.
pub const HOOKED_DRAW_SET: &[u32] = &[0x00b5_4090];

/// Whether the run may turn the thread on, and why not: it needs real threads with the render thread for play;
/// stepped threads are the check form (the late gate), accepted.
pub fn refusal(choice: &Choice) -> Option<&'static str> {
    let stepped = choice.threads.trim() != crate::fixes::PLAY_THREADS;
    if !stepped && !choice.render_thread {
        return Some("it needs the render thread with real threads (bRenderThread=1)");
    }
    if crate::fixes::FIXES
        .iter()
        .filter(|f| choice.is_on(f))
        .any(|f| f.addresses.iter().any(|a| HOOKED_DRAW_SET.contains(a)))
    {
        return Some("a fix row replaces a function the world draw calls");
    }
    None
}

/// Binds the row for `e`'s run (`crate::fixes::install`, the row on): the seam's hooks, or a log line saying why
/// not.
pub fn install(e: &mut Engine, choice: &Choice) {
    if let Some(why) = refusal(choice) {
        eprintln!("NV_DRAW_THREAD: off: {why}");
        return;
    }
    engine::draw_thread::turn_on(e, hooks());
    eprintln!(
        "NV_DRAW_THREAD: on (experimental): the world drawn on a thread of its own where it can be"
    );
}

/// The hooks of a run with the row on.
pub fn hooks() -> Hooks {
    Hooks {
        serial,
        cut: hand_off_cut,
        adopt: hand_off_adopt,
        hand_back: hand_off_back,
        join: hand_off_join,
        device_cut: device_cut,
        device_close: device_close,
        device_finish: device_finish,
    }
}

/// The serial conditions the host sees. The first one stands until the draw family's hand-off is in (PR B: its
/// `cut::cut_for_thread`, `adopt`, `hand_back`, `join_handed`): without it the job would cut on the helper, after the
/// frame's tail, so every frame draws on main.
pub fn serial(e: &Engine) -> Option<&'static str> {
    if !HAND_OFF_BUILT {
        return Some("the draw family's packet hand-off is not built yet");
    }
    serial_families_and_hooks(e)
}

/// Whether the hand-off is wired (PR B flips it with the draw family's exports).
const HAND_OFF_BUILT: bool = false;

/// The families and hooks part of [`serial`] (the tests reach it directly).
pub fn serial_families_and_hooks(e: &Engine) -> Option<&'static str> {
    if draw_families()
        .iter()
        .any(|&(_, f)| !e.typed_family_direct(f))
    {
        return Some("a family the world draw runs in is not direct (switched off, or a hook or a fix row on it)");
    }
    if HOOKED_DRAW_SET.iter().any(|&a| !e.wraps_at(a).is_empty()) {
        return Some("an extender hooks a function the world draw calls");
    }
    if e.real.is_some()
        && engine_typed::draw::capture::mode() == engine_typed::draw::capture::Mode::Poison
    {
        return Some("NV_DRAW_THREAD=poison is a check for stepped threads only");
    }
    None
}

fn hand_off_cut(_: &mut Engine, _camera: u32, _accumulator: u32, _stepped: bool) -> Option<Packet> {
    None
}

fn hand_off_adopt(_: &mut Engine, packet: Packet) -> Result<(), Packet> {
    Err(packet)
}

fn hand_off_back(_: &mut Engine) -> Option<Packet> {
    None
}

fn hand_off_join(_: &mut Engine, _: Packet) {}

fn device_cut(e: &mut Engine, device: u32) -> bool {
    engine_platform::d3d9draw::draw_cut(e, device)
}

fn device_close(e: &mut Engine) {
    engine_platform::d3d9draw::draw_close(e);
}

fn device_finish(e: &mut Engine) {
    engine_platform::d3d9draw::draw_finish(e);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Until the hand-off is in, every frame draws on main whatever else holds.
    #[test]
    fn every_frame_draws_on_main_until_the_hand_off_is_in() {
        let e = Engine::new();
        assert_eq!(
            serial(&e),
            Some("the draw family's packet hand-off is not built yet")
        );
    }

    /// A registration over a member of a family the draw runs in (a fix row, a wrap) sends that family by address,
    /// and the draw stays on main; so does a family switched off.
    #[test]
    fn a_registration_over_a_draw_member_keeps_the_draw_on_main() {
        fn stand_in(_: &mut Engine, _: &[u32]) -> Ret {
            Ret::default()
        }
        let mut e = Engine::new();
        let all_direct = draw_families()
            .iter()
            .all(|&(_, f)| e.typed_family_direct(f));
        if all_direct {
            assert_eq!(serial_families_and_hooks(&e), None);
        }
        e.register(engine::draw_thread::WORLD_DRAW_FIRST, stand_in);
        assert!(!e.typed_family_direct(engine_typed::draw::FAMILY));
        assert_eq!(
            serial_families_and_hooks(&e),
            Some("a family the world draw runs in is not direct (switched off, or a hook or a fix row on it)")
        );
    }

    /// No row today replaces a function of the hooked drawing set, so the fixes preset does not refuse it.
    #[test]
    fn no_fix_row_sits_on_the_hooked_drawing_set() {
        for fix in crate::fixes::FIXES {
            for a in fix.addresses {
                assert!(!HOOKED_DRAW_SET.contains(a), "{}: {a:08x}", fix.name);
            }
        }
    }
}
