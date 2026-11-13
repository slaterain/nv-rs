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
    // The scheme the host will run (any spelling of real threads counts as real).
    let stepped =
        engine::thread_host::Scheme::from_name(Some(&choice.threads)).map_or(true, |s| !s.real);
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
        eprintln!("NV_DRAW_THREAD: refused, off: {why}");
        return;
    }
    engine::draw_thread::turn_on(e, hooks());
    eprintln!(
        "NV_DRAW_THREAD: on (experimental): the seam is on, but every frame still draws on main until the \
         draw family's packet hand-off is in"
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
        device_cut,
        device_close,
        device_finish,
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
    families_reason(|f| e.typed_family_direct(f))
        .or_else(|| hooks_reason(|a| !e.wraps_at(a).is_empty()))
        .or_else(|| poison_reason(e.real.is_some(), engine_typed::draw::capture::mode()))
}

/// A family the draw runs in that is not direct (`direct` answers for each).
pub fn families_reason(direct: impl Fn(FamilyId) -> bool) -> Option<&'static str> {
    draw_families()
        .iter()
        .any(|&(_, f)| !direct(f))
        .then_some("a family the world draw runs in is not direct (switched off, or a hook or a fix row on it)")
}

/// A wrap on the hooked drawing set (`wrapped` answers for each address).
pub fn hooks_reason(wrapped: impl Fn(u32) -> bool) -> Option<&'static str> {
    HOOKED_DRAW_SET
        .iter()
        .any(|&a| wrapped(a))
        .then_some("an extender hooks a function the world draw calls")
}

/// POISON with real threads: refused (the scribble would be read by the other threads).
pub fn poison_reason(real: bool, mode: engine_typed::draw::capture::Mode) -> Option<&'static str> {
    (real && mode == engine_typed::draw::capture::Mode::Poison)
        .then_some("NV_DRAW_THREAD=poison is a check for stepped threads only")
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
        // The eight families applied, geometry turned on (it is off by default: until it is, the draw stays on
        // main).
        let mut e = Engine::new_untyped_for_test();
        let families: Vec<&engine::typed::Family> = vec![
            &engine_typed::draw::DRAW,
            &engine_typed::draw_f::DRAW_F,
            &engine_typed::shader::SHADER,
            &engine_typed::render::RENDER,
            &engine_typed::dispatch::DISPATCH,
            &engine_typed::crt::CRT,
            &engine_typed::pathing::PATHING,
            &engine_typed::geometry::GEOMETRY,
        ];
        e.typed_apply_for_test(&families, "all,geometry");
        assert_eq!(serial_families_and_hooks(&e), None);
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

#[cfg(test)]
mod serial_tests {
    use super::*;
    use engine_typed::draw::capture::Mode;

    /// With the default switch the geometry family is off, so the draw stays on main until it is on.
    #[test]
    fn the_geometry_family_off_keeps_the_draw_on_main() {
        let mut e = Engine::new_untyped_for_test();
        let families: Vec<&engine::typed::Family> = vec![
            &engine_typed::draw::DRAW,
            &engine_typed::draw_f::DRAW_F,
            &engine_typed::shader::SHADER,
            &engine_typed::render::RENDER,
            &engine_typed::dispatch::DISPATCH,
            &engine_typed::crt::CRT,
            &engine_typed::pathing::PATHING,
            &engine_typed::geometry::GEOMETRY,
        ];
        e.typed_apply_for_test(&families, "all");
        assert!(!e.typed_family_direct(engine_typed::geometry::FAMILY));
        assert!(serial_families_and_hooks(&e).is_some());
    }

    /// A wrap on the hooked drawing set, and only there, keeps the draw on main.
    #[test]
    fn a_wrap_on_the_hooked_drawing_set_keeps_the_draw_on_main() {
        assert_eq!(hooks_reason(|_| false), None);
        assert!(hooks_reason(|a| a == 0x00b5_4090).is_some());
        assert_eq!(hooks_reason(|a| a == 0x0040_1000), None);
    }

    /// POISON is refused with real threads only.
    #[test]
    fn poison_is_refused_with_real_threads() {
        assert!(poison_reason(true, Mode::Poison).is_some());
        assert_eq!(poison_reason(false, Mode::Poison), None);
        assert_eq!(poison_reason(true, Mode::On), None);
        assert_eq!(poison_reason(true, Mode::Off), None);
    }

    /// Any spelling of real threads without the render thread is refused.
    #[test]
    fn real_threads_named_any_way_need_the_render_thread() {
        let choice = crate::fixes::Choice::from_parts(None, |n: &str| match n {
            "NV_DRAW_THREAD" => Some("1".into()),
            "NV_THREADS" => Some("real,notimeout".into()),
            "NV_RENDER_THREAD" => Some("0".into()),
            _ => None,
        })
        .unwrap();
        assert!(refusal(&choice).is_some());
    }

    /// The host's choice must decide the draw family's capture mode (export E5, PR B): today the draw family reads
    /// NV_DRAW_THREAD itself, so NV_DRAW_THREAD=poison with real threads still scribbles on main, a refused row still
    /// captures, and the Fallout.ini line never turns capture on. PR B turns this test on.
    #[test]
    #[ignore = "needs the draw family's capture::set_mode (E5, PR B)"]
    fn the_hosts_choice_decides_the_capture_mode() {
        let poison_real = crate::fixes::Choice::from_parts(None, |n: &str| match n {
            "NV_DRAW_THREAD" => Some("poison".into()),
            "NV_THREADS" => Some("real".into()),
            "NV_RENDER_THREAD" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let mut e = Engine::new();
        install(&mut e, &poison_real);
        assert_ne!(engine_typed::draw::capture::mode(), Mode::Poison);
    }
}
