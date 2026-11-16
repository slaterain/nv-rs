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
    // The host's choice decides the draw family's packet (its export E5), not the variable.
    engine_typed::draw::capture::set_mode(Some(capture_mode(choice)));
    if let Some(why) = refusal(choice) {
        eprintln!("NV_DRAW_THREAD: refused, off: {why}");
        return;
    }
    if choice.draw_poison && real_threads(choice) {
        eprintln!("NV_DRAW_THREAD: poison is for stepped threads only: on without the scribble");
    }
    engine::draw_thread::turn_on(e, hooks());
    eprintln!("NV_DRAW_THREAD: on (experimental)");
}

/// Whether the run's threads are real (any spelling of real threads counts).
fn real_threads(choice: &Choice) -> bool {
    engine::thread_host::Scheme::from_name(Some(&choice.threads)).is_ok_and(|s| s.real)
}

/// The draw family's run-wide capture mode for a run with the row on (its export E5), from the host's choice: POISON
/// for the variable's `poison` on stepped threads (the check form, whose serial frames cut and scribble too);
/// otherwise off, a refused row included, so a frame drawn on main makes no cut and costs nothing: the hand-off takes
/// the packet itself, only for a frame it posts ([`hand_off_cut`]).
pub fn capture_mode(choice: &Choice) -> engine_typed::draw::capture::Mode {
    use engine_typed::draw::capture::Mode;
    if refusal(choice).is_none() && choice.draw_poison && !real_threads(choice) {
        Mode::Poison
    } else {
        Mode::Off
    }
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

/// The serial conditions the host sees ([`serial_families_and_hooks`]); the draw family's hand-off (its
/// `cut::cut_for_thread`, `adopt`, `hand_back`, `join_handed`) is wired below.
pub fn serial(e: &Engine) -> Option<&'static str> {
    serial_families_and_hooks(e)
}

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

use engine_typed::draw::{capture, cut};

/// Export E1: main's cut, the packet boxed for the seam. With the run-wide mode off ([`capture_mode`]: the row on
/// without the POISON check) this post's cut runs with the packet on, as this thread's mode for the call only, so
/// only a frame the seam posts is cut.
pub fn hand_off_cut(
    e: &mut Engine,
    camera: u32,
    accumulator: u32,
    stepped: bool,
) -> Option<Packet> {
    let before = capture::thread_mode();
    capture::set_thread_mode(Some(post_mode(capture::mode())));
    let packet = cut::cut_for_thread(e, camera, Ptr::new(accumulator), stepped);
    capture::set_thread_mode(before);
    packet.map(|p| Box::new(p) as Packet)
}

/// The mode a posted frame's cut runs in: the packet on where the run's mode is off; otherwise the run's (POISON's
/// check, or a mode a check set).
pub fn post_mode(run: capture::Mode) -> capture::Mode {
    match run {
        capture::Mode::Off => capture::Mode::On,
        other => other,
    }
}

/// Export E2: the drawing side adopts the packet; one the seam handed that is not the draw family's is a stop.
fn hand_off_adopt(e: &mut Engine, packet: Packet) -> Result<(), Packet> {
    let packet = packet.downcast::<capture::Detached>().unwrap_or_else(|_| {
        panic!("NV_DRAW_THREAD: the seam handed the drawing side a packet of another kind")
    });
    cut::adopt(e, *packet).map_err(|p| Box::new(p) as Packet)
}

/// Export E4, the drawing side: the packet handed back.
fn hand_off_back(e: &mut Engine) -> Option<Packet> {
    cut::hand_back(e).map(|p| Box::new(p) as Packet)
}

/// Export E4, main's side: the join.
fn hand_off_join(e: &mut Engine, packet: Packet) {
    let packet = packet.downcast::<capture::Detached>().unwrap_or_else(|_| {
        panic!("NV_DRAW_THREAD: the seam handed main a packet of another kind")
    });
    cut::join_handed(e, *packet);
}

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

    /// With the hand-off in, the default families decide: since the geometry family's gate (2026-11-16) every family
    /// the draw runs in is on by default, so the families no longer keep the draw on main (the row itself stays off in
    /// every preset).
    #[test]
    fn the_default_families_leave_the_draw_free_to_post() {
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
        assert!(e.typed_family_direct(engine_typed::geometry::FAMILY));
        assert_eq!(families_reason(|f| e.typed_family_direct(f)), None);
    }

    /// A registration over a member of a family the draw runs in (a fix row, a wrap) sends that family by address,
    /// and the draw stays on main; so does a family switched off.
    #[test]
    fn a_registration_over_a_draw_member_keeps_the_draw_on_main() {
        fn stand_in(_: &mut Engine, _: &[u32]) -> Ret {
            Ret::default()
        }
        // The eight families applied, each on.
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

    /// `NV_TYPED=geometry:off` turns the geometry family off, and the draw stays on main.
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
        e.typed_apply_for_test(&families, "all,geometry:off");
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

    /// A choice with the drawing thread's variable `draw` (`1` or `poison`), the threads `threads` and the render
    /// thread `render`.
    fn draw_choice(draw: &str, threads: &str, render: &str) -> crate::fixes::Choice {
        let (draw, threads, render) = (draw.to_string(), threads.to_string(), render.to_string());
        crate::fixes::Choice::from_parts(None, move |n: &str| match n {
            "NV_DRAW_THREAD" => Some(draw.clone()),
            "NV_THREADS" => Some(threads.clone()),
            "NV_RENDER_THREAD" => Some(render.clone()),
            _ => None,
        })
        .unwrap()
    }

    /// The host's choice decides the draw family's capture mode (export E5), whatever the process environment says:
    /// POISON only for `poison` on stepped threads; off for `poison` with real threads (no scribble beside the other
    /// threads), for a refused row and for the row on without the check (`1`, as the `Fallout.ini` line), so a frame
    /// drawn on main makes no cut. `install` sets exactly that, over whatever was set before.
    #[test]
    fn the_hosts_choice_decides_the_capture_mode() {
        use engine_typed::draw::capture;
        let cases = [
            (draw_choice("poison", "stepped", "0"), Mode::Poison),
            (draw_choice("poison", "real", "1"), Mode::Off),
            (draw_choice("poison", "real", "0"), Mode::Off),
            (draw_choice("1", "real", "1"), Mode::Off),
            (draw_choice("1", "real", "0"), Mode::Off),
            (draw_choice("1", "stepped", "0"), Mode::Off),
        ];
        for (choice, want) in &cases {
            assert_eq!(capture_mode(choice), *want, "{choice:?}");
            // What was set before (the stand-in for the variable) does not survive the install.
            let before = if *want == Mode::Poison {
                Mode::Off
            } else {
                Mode::Poison
            };
            capture::set_mode(Some(before));
            install(&mut Engine::new(), choice);
            let got = capture::mode();
            capture::set_mode(None);
            assert_eq!(got, *want, "{choice:?}");
        }
    }

    /// A default play choice leaves the row off (no install, no packet); with the row on and no check, the hand-off's
    /// own cut is the only cut, and it leaves this thread's mode as it found it (here: a frame with no camera, which
    /// it does not cut at all).
    #[test]
    fn a_default_run_makes_no_cut() {
        use engine_typed::draw::capture;
        let default = crate::fixes::Choice::from_parts(None, |_: &str| None).unwrap();
        assert!(!default.is_on(&crate::fixes::DRAW_THREAD));
        assert_eq!(capture_mode(&draw_choice("1", "real", "1")), Mode::Off);
        let mut e = Engine::new();
        assert!(hand_off_cut(&mut e, 0, 0, false).is_none());
        assert_eq!(capture::thread_mode(), None);
        assert!(!capture::is_open(&e));
        // A posted frame's cut: the packet on where the run's mode is off, POISON's check kept.
        assert_eq!(post_mode(Mode::Off), Mode::On);
        assert_eq!(post_mode(Mode::Poison), Mode::Poison);
        assert_eq!(post_mode(Mode::On), Mode::On);
    }
}
