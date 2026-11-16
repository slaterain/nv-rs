//! The world draw's own state (the drawing thread's step 2, piece 3: `measure/scope-1111/PLAN.md`): the state the
//! draw writes that main's frame tail (the first-person and reflection draws, the interface) writes too, so that on
//! a thread of its own the draw needs a copy of it.
//!
//! - the shader statics: the rendering pass and its type (`011f91e0`, `011f91e4`), the shader and held
//!   accumulators (`011f95ec`, `011f95f0`), and the block from `011fa000` to `011ffe40` (the shader constants the
//!   set-ups fill and upload: the light rows, the fog, the land toggles, the eye, the matrices, Group C's water,
//!   sky and flag constants; the constant entry lists and the pass type tables; the render states' lock counters
//!   and their state indices, the coverage counter, the current shader and pass type of the batch draw);
//! - the current shader pass (`0126f74c`), the group zero flag (`011a9608`), the last light's point flag
//!   (`011f91a6`), the terrain fade flag (`011f94aa`), the shader mode (`011f91b8`) and the two function words
//!   it selects (`011f9f38`, `011f9f3c`);
//! - the renderer's render-state cache (`NiDX9RenderState`, the renderer's +0x8b8: 0x1114 bytes);
//! - the accumulator's camera (+8).
//!
//! At the cut ([`take`]) these are taken into the value packet as own blocks ([`capture::own`]); the draw reads and
//! writes them through [`capture::own_at`] and its kin, and at the join they go back by the device model's rule
//! ([`capture::end`]). Every typed reader and writer inside the draw goes through those accessors (the hygiene
//! ratchet [`tests`] keeps it so), but a translation the draw still reaches by address reads guest memory: so the
//! cut takes the own blocks only when no pass in the lists has a shader class that may reach one ([`ownable`]).
//! Until a class is typed through and listed clean in [`CLASSES`], its frames keep their own state in guest memory
//! (and, from piece 8, draw serially); the run's count says how many and why ([`report`]).

use super::capture;
use crate::shader::layout::BSShaderAccumulator;
use engine_core::ptr::Ptr;
use engine_core::Engine;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

/// The rendering pass and its type.
pub(crate) const RENDERING_PASS: (u32, u32) = (0x011f_91e0, 0x011f_91e8);
/// The shader accumulator and the held accumulator.
pub(crate) const ACCUMULATORS: (u32, u32) = (0x011f_95ec, 0x011f_95f4);
/// The shader statics' block.
pub const SHADER_STATICS: (u32, u32) = (0x011f_a000, 0x011f_fe40);
/// The Group C shaders' statics (`01200458` to `01202168`: the decal depth bias, the tall grass's, SpeedTree's,
/// lighting 3.0's and the no-lighting shader's constants and their entry pointers, lighting 3.0's pass table; the last,
/// the no-lighting shader's fog constants, are eight words from `01202148`: a block ending before them cut their upload
/// in two, the late gate's first record difference of this piece).
pub(crate) const GROUP_C_STATICS: (u32, u32) = (0x0120_0450, 0x0120_2168);
/// The current shader pass.
pub(crate) const CURRENT_PASS: (u32, u32) = (0x0126_f74c, 0x0126_f750);
/// The group zero flag (the texture stages' maximum anisotropy word).
pub(crate) const GROUP_ZERO_FLAG: (u32, u32) = (0x011a_9608, 0x011a_960c);
/// The last light's point flag the per-light set-up leaves (`011f91a6`) and the terrain fade flag vertex entry 21
/// clears (`011f94aa`; game logic reads it after the draw).
pub(crate) const LIGHT_FLAG: (u32, u32) = (0x011f_91a6, 0x011f_91a7);
pub(crate) const FADE_FLAG: (u32, u32) = (0x011f_94aa, 0x011f_94ab);
/// The shader manager's mode (`011f91b8`) and the two function words it selects (`011f9f38`, `011f9f3c`): the draw
/// sets them for its render mode (`own::select_shader_mode`, `00b4f450`), main's tail for its own.
pub(crate) const SHADER_MODE: (u32, u32) = (0x011f_91b8, 0x011f_91bc);
pub(crate) const MODE_FUNCTIONS: (u32, u32) = (0x011f_9f38, 0x011f_9f40);
/// The renderer's render-state cache (`NiDX9RenderState`) and its size.
pub(crate) const RENDER_STATE_SIZE: u32 = 0x1114;
/// The accumulator's camera (+8).
pub(crate) const ACC_CAMERA: (u32, u32) = (8, 0xc);
/// The camera's eye as the renderer's camera set-up keeps it (`011f474c`, three floats: `00e6c780` writes it; the
/// matrices `00b71a40` makes are relative to it).
pub(crate) const CAMERA_EYE: (u32, u32) = (0x011f_474c, 0x011f_4758);
/// The renderers (`011f4748`, the scene's, and `011f9508`; the same `NiDX9Renderer` in a run) whose per-draw words
/// every draw writes before it reads them: the geometry's property state and effects (+0xc, +0x10: `00a80610` and
/// the batch draw set them, each accumulator's finish `00b66570` clears +0xc, the points draw `00e70140` reads
/// through +0xc), the camera's directions and their model-space turns and the drawn bound (+0x634 to +0x674:
/// `00e6c780`, `00e6fbb0`, the points draw), the near plane and depth range (+0x684, +0x688: `00e6c780`), and the
/// world, view, projection, view-projection and inverse view matrices (+0x940 to +0xa80: `00e6fbb0`, `00e6c780`).
/// Main's own draws after the post (the world pass's extra passes, `00b6c0d0`) write them too, while the drawing
/// side draws: each side keeps its own (the drawing thread's first-post stop, 2026-11-16: main's finish cleared +0xc
/// between the particles' `00a80610` and `00e70140` on the drawing side, a read of address 0).
pub(crate) const RENDERER_GLOBALS: [u32; 2] = [0x011f_4748, 0x011f_9508];
pub(crate) const RENDERER_BLOCKS: [(u32, u32); 4] =
    [(0xc, 0x14), (0x634, 0x674), (0x684, 0x68c), (0x940, 0xa80)];

/// The global blocks, in one list (what [`take`] owns besides the render-state cache and the camera).
pub const GLOBAL_BLOCKS: [(u32, u32); 11] = [
    RENDERING_PASS,
    ACCUMULATORS,
    SHADER_STATICS,
    GROUP_C_STATICS,
    CURRENT_PASS,
    GROUP_ZERO_FLAG,
    LIGHT_FLAG,
    FADE_FLAG,
    SHADER_MODE,
    MODE_FUNCTIONS,
    CAMERA_EYE,
];

/// Whether `addr` lies in one of the own blocks of globals (for the checks).
pub fn is_own_global(addr: u32) -> bool {
    GLOBAL_BLOCKS
        .iter()
        .any(|&(lo, hi)| (lo..hi).contains(&addr))
}

/// The shader classes the world draw may meet, by vtable, with the first translation each still reaches by address
/// that reads or writes own state (0: none, the class is clean). A class not listed is not clean either. A class
/// is listed clean only once every slot the draw calls on it is typed and none of them reaches such a translation.
pub(crate) const CLASSES: &[(u32, u32)] = &[
    // `ShadowLightShader` and its three subclasses sharing its pass constants (slot 0xfc, `00b7b450`): every slot the
    // batch renderer calls is typed (the subclasses' own 0x78, 0xf0 and 0xf8 too, `00bce500`, `00bcce40`, `00bcc590`,
    // `00bcfd20`, `00bce800`, and the stage setters they reach); what they still call by address reads no own state
    // (`00a5f860` and `00c663d0` the counted pointers, `00b55560` the shader table).
    (0x010a_f2f8, 0),
    (0x010b_b7a8, 0),
    (0x010b_b980, 0),
    (0x010b_bb50, 0),
    // Group C: the SpeedTree leaves (0x78 `00bb26d0`, 0xf0 `00bb1b20`, with their lights `00bb1960`, camera rows
    // `00bb1a90` and scene lights `00b71360`), the no-lighting shader (0x64 `00bc8f20`, 0x80 `00bc8ed0`, 0x8c
    // `00bc8d90`), the tall grass (0xf0 `00baa3f0`, 0xf8 `00bac8d0`), the sky (0x8c `00b8a1a0`, 0xf0 `00b884e0`),
    // the water (0x8c `00b86b80`, 0xf0 `00b81760`, 0xfc `00b81d60`), lighting 3.0 (0x64 `00bbd000`, 0x80 `00bbc5c0`,
    // 0x8c `00bbeef0`, 0xf0 `00bbe020`, its stage slots `00c0cd80`, `00c0cf10` and `00c0bc90`; its pass for a type
    // `00c0abd0`; the pass constants' setters it calls by address work on the pass handed to them).
    (0x010b_9190, 0),
    (0x010b_b230, 0),
    (0x010b_8980, 0),
    (0x010a_fe18, 0),
    (0x010a_f9e0, 0),
    (0x010b_a0f8, 0),
    // The SpeedTree branch shader: 0x78 `00bd4890`, 0xf0 `00bd4030` typed with 0x7c and its fog slot; 0xfc is a
    // thunk to the shadow light shader's `00b7b450`.
    (0x010b_c070, 0),
];

/// A geometry's shader (+0xc0).
const GEOMETRY_SHADER: u32 = 0xc0;

/// The first reason the passes' shader classes keep own state in guest memory: the blocking translation of a
/// listed class, or the vtable of a class not listed. `None` when every pass's class is clean.
pub fn ownable(e: &Engine, passes: &[u32]) -> Option<u32> {
    let mut seen = engine_core::AddrSet::default();
    for &pass in passes {
        let geometry = super::packet::pass_geometry(e, pass);
        if geometry == 0 {
            continue;
        }
        // The render-state cache's slots a skinned geometry's bone matrices (slot 0xf8, `00e88ad0`) and the
        // particles' quads (slot 0x2c, `00e88690`) reach are typed in the shader family, through the own accessors.
        let shader = e.global::<u32>(geometry.wrapping_add(GEOMETRY_SHADER));
        if shader == 0 {
            continue;
        }
        let vtable = e.global::<u32>(shader);
        if !seen.insert(vtable) {
            continue;
        }
        match CLASSES.iter().find(|&&(v, _)| v == vtable) {
            Some(&(_, 0)) => {}
            Some(&(_, blocking)) => return Some(blocking),
            None => return Some(vtable),
        }
    }
    None
}

thread_local! {
    static FORCED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Makes this thread's cuts take the own state whatever the passes' classes (for the checks, whose test worlds'
/// shader classes are stand-ins).
pub fn set_forced(on: bool) {
    FORCED.with(|f| f.set(on));
}

/// [`ownable`], or `None` while [`set_forced`] holds.
pub(crate) fn decide(e: &Engine, passes: &[u32]) -> Option<u32> {
    if FORCED.with(std::cell::Cell::get) {
        return None;
    }
    ownable(e, passes)
}

/// Takes the own state into the open packet as own blocks: the global blocks, the render-state cache, the
/// accumulator's camera.
pub fn take(e: &Engine, acc: Ptr<BSShaderAccumulator>) {
    for (lo, hi) in GLOBAL_BLOCKS {
        capture::own(e, lo, 0, hi - lo);
    }
    if e.global::<u32>(crate::shader::state::RENDERER) != 0 {
        let cache = crate::shader::state::render_state(e).addr();
        // From +4: its vtable (set once) is read in place by every call through it.
        capture::own(e, cache, 4, RENDER_STATE_SIZE);
    }
    take_renderers(e);
    capture::own(e, acc.addr(), ACC_CAMERA.0, ACC_CAMERA.1);
}

/// The renderers' per-draw words ([`RENDERER_BLOCKS`]) as own blocks (one renderer held twice is taken once).
pub fn take_renderers(e: &Engine) {
    for global in RENDERER_GLOBALS {
        let renderer = e.global::<u32>(global);
        for (from, to) in RENDERER_BLOCKS {
            capture::own(e, renderer, from, to);
        }
    }
}

/// The cuts that took the own state, those that kept it in guest memory, and the first reason seen.
static OWNED: AtomicU64 = AtomicU64::new(0);
static KEPT: AtomicU64 = AtomicU64::new(0);
static FIRST_REASON: AtomicU32 = AtomicU32::new(0);

/// Counts a cut: the own state taken (`None`) or kept in guest memory for `reason`.
pub(crate) fn count(reason: Option<u32>) {
    match reason {
        None => {
            OWNED.fetch_add(1, Relaxed);
        }
        Some(r) => {
            KEPT.fetch_add(1, Relaxed);
            let _ = FIRST_REASON.compare_exchange(0, r, Relaxed, Relaxed);
        }
    }
}

/// The run's count: cuts that took the own state, cuts that kept it in guest memory, and the first reason (a
/// translation's address or a shader class's vtable; 0 for none).
pub fn report() -> (u64, u64, u32) {
    (
        OWNED.load(Relaxed),
        KEPT.load(Relaxed),
        FIRST_REASON.load(Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drawing thread's first-post stop (2026-11-16): the drawing side's particles set the renderer's current
    /// property state (`00a80610`) and read it back in the points draw (`00e70140`), while main's own scene draw after
    /// the post finished (`00b66570` clears it). With the renderer's per-draw words its own state, the drawing side
    /// reads its value whatever main does meanwhile, and the join keeps main's clear (main wrote it after the cut).
    #[test]
    fn mains_finish_does_not_clear_the_drawing_sides_property_state() {
        let mut e = Engine::new();
        if !e.mem.is_mapped(0x011f_4000) {
            e.map(0x011f_4000, 0x6000);
        }
        let renderer = e.mem.alloc(0xa80);
        let geometry = e.mem.alloc(0x100);
        e.mem.set_u32(0x011f_4748, renderer);
        e.mem.set_u32(0x011f_9508, renderer);
        assert!(capture::begin(&e));
        take_renderers(&e);
        // The drawing side: the particles' render immediate.
        crate::dispatch::scene::geometry_render_immediate(&mut e, Ptr::new(geometry), Ptr::new(renderer));
        // Main, with the packet on the drawing side's thread: its extra pass's finish.
        let packet = capture::detach(&e).expect("the packet");
        super::super::packet::clear_scene_word(&mut e);
        assert_eq!(e.mem.u32(renderer + 0xc), 0, "main clears its own");
        assert!(capture::attach(packet).is_ok());
        // The drawing side's points draw reads its own.
        assert_eq!(
            super::super::packet::current_property_state(&e, renderer),
            geometry + 0x9c
        );
        // The join: main wrote it after the cut, so main's value stays.
        let report = capture::end(&mut e);
        assert_eq!(e.mem.u32(renderer + 0xc), 0);
        assert!(report.kept_main >= 4);
    }

    /// Every renderer block the cut takes is inside the renderer's layout and apart from the others, and the camera's
    /// eye is an own global.
    #[test]
    fn the_renderer_blocks_are_apart_and_the_eye_is_owned() {
        for (i, a) in RENDERER_BLOCKS.iter().enumerate() {
            assert!(a.0 < a.1 && a.1 <= 0xa80);
            for b in &RENDERER_BLOCKS[i + 1..] {
                assert!(a.1 <= b.0 || b.1 <= a.0, "{a:x?} {b:x?}");
            }
        }
        assert!(is_own_global(0x011f_474c) && is_own_global(0x011f_4754));
    }

    /// The global blocks do not overlap.
    #[test]
    fn the_global_blocks_are_apart() {
        for (i, a) in GLOBAL_BLOCKS.iter().enumerate() {
            assert!(a.0 < a.1);
            for b in &GLOBAL_BLOCKS[i + 1..] {
                assert!(a.1 <= b.0 || b.1 <= a.0, "{a:x?} {b:x?}");
            }
        }
    }

    /// A constant block the draw uploads in one call lies in one own block (the packet's image of it needs one): the
    /// no-lighting shader's fog constants (`01202148`, eight words) at the end of Group C's.
    #[test]
    fn uploaded_constant_blocks_lie_in_one_own_block() {
        for (at, len) in [(0x0120_2148u32, 0x20u32), (0x011f_a280, 0x20)] {
            assert!(
                GLOBAL_BLOCKS
                    .iter()
                    .any(|&(lo, hi)| lo <= at && at + len <= hi),
                "{at:08x}+{len:#x} is not inside one own block"
            );
        }
    }

    /// The draw-time sources: no typed code reads or writes an own-state address past the packet. In these files a
    /// constant whose value lies in an own block of globals (defined in any of them: imported and module-qualified
    /// names count) may be used only in a call of an own-state accessor (`own_at`, `set_own_at`, `own_word`,
    /// `set_word`, `own_image`, `set_constant_word` and the helpers listed that route inside; whole names), and the
    /// render-state cache's fields only through `own_get`/`own_set` (or an `own_at` of an address made from them).
    /// A line may say why it is an exception with `// own:`.
    #[test]
    fn own_state_is_read_and_written_only_through_the_packet() {
        let sources: &[(&str, &str)] = &[
            ("draw/own.rs", include_str!("own.rs")),
            ("draw/packet.rs", include_str!("packet.rs")),
            ("draw/setups.rs", include_str!("setups.rs")),
            ("draw/passes.rs", include_str!("passes.rs")),
            ("draw/skin.rs", include_str!("skin.rs")),
            ("draw/alpha.rs", include_str!("alpha.rs")),
            ("draw/drivers.rs", include_str!("drivers.rs")),
            ("draw/camera.rs", include_str!("camera.rs")),
            ("draw/packed.rs", include_str!("packed.rs")),
            ("draw/lists.rs", include_str!("lists.rs")),
            ("draw/constants.rs", include_str!("constants.rs")),
            ("draw/group_c.rs", include_str!("group_c.rs")),
            ("draw/sky.rs", include_str!("sky.rs")),
            ("draw/speedtree.rs", include_str!("speedtree.rs")),
            ("draw/tall_grass.rs", include_str!("tall_grass.rs")),
            ("draw/water.rs", include_str!("water.rs")),
            ("draw/no_lighting.rs", include_str!("no_lighting.rs")),
            ("draw/lighting30.rs", include_str!("lighting30.rs")),
            ("shader/state.rs", include_str!("../shader/state.rs")),
            ("shader/batch.rs", include_str!("../shader/batch.rs")),
            ("shader/geometry.rs", include_str!("../shader/geometry.rs")),
            ("shader/lighting.rs", include_str!("../shader/lighting.rs")),
            ("shader/textures.rs", include_str!("../shader/textures.rs")),
            (
                "shader/accumulator.rs",
                include_str!("../shader/accumulator.rs"),
            ),
            ("render/draw.rs", include_str!("../render/draw.rs")),
            ("render/pass.rs", include_str!("../render/pass.rs")),
            ("render/passes.rs", include_str!("../render/passes.rs")),
            ("render/stage.rs", include_str!("../render/stage.rs")),
            (
                "shader/constants.rs",
                include_str!("../shader/constants.rs"),
            ),
        ];
        let accessors = [
            "own_at",
            "set_own_at",
            "own_word",
            "set_word",
            "set_half",
            "set_byte",
            "own_image",
            "set_constant_word",
            "own_get",
            "own_set",
            // Helpers whose accesses to the own blocks go through the accessors. The constant entries, switch objects
            // and pass stages some of them reach through an owned slot (an entry's +8 and +0x20, a stage's +8) are
            // heap objects the cut does not own: in the drawing thread's plan every draw of the frame after the cut runs
            // on the drawing side, so main's tail runs none of the set-ups that write them (the census of main's tail is
            // to confirm it).
            "counted",
            "apply_mask",
            "enable_entries",
            "entry_enabled",
            "table_bit",
            "set_constant_byte",
            "set_shader_global",
            "constant_word",
            "constant_byte",
            "copy_constants",
            "entry_on",
            "release_state_locks",
            "unlocked",
        ];
        // Files routed through and through: every guest access of theirs goes through the accessors, or is a plain
        // guest access that says why (`// own:` a local, a constant of the exe, main's frame value left to value
        // packet B); an own accessor there names no global outside the own blocks (it would only look routed).
        let full = ["shader/geometry.rs", "shader/lighting.rs"];
        // Test modules are left out: tests set the state up as they please.
        let code_of = |text: &'static str| text.split("#[cfg(test)]").next().unwrap_or(text);
        // The own-address constants of every scanned file (one may be imported, or named through its module), less
        // the names some file (or the cut's) also defines another way: those are told apart per file below.
        let defining: Vec<&str> = sources
            .iter()
            .map(|(_, text)| code_of(text))
            .chain([
                code_of(include_str!("cut.rs")),
                code_of(include_str!("capture.rs")),
            ])
            .collect();
        let mut all_own: Vec<String> = defining
            .iter()
            .flat_map(|code| constants(code, true))
            .filter(|c| !defining.iter().any(|code| defines_otherwise(code, c)))
            .collect();
        all_own.sort();
        all_own.dedup();
        let mut findings = Vec::new();
        for (name, text) in sources {
            let code = code_of(text);
            // A file's own constant of the same name with another value shadows the others'.
            let others = constants(code, false);
            let mut own_names: Vec<&String> =
                all_own.iter().filter(|c| !others.contains(c)).collect();
            let local_own = constants(code, true);
            own_names.extend(local_own.iter());
            let names_own = |body: &str| {
                own_names.iter().any(|c| has_word(body, c)) || !own_literals(body).is_empty()
            };
            if full.contains(name) {
                let lines: Vec<&str> = code.lines().collect();
                // Per function: the names that hold an address built from an own-block constant, and the names of
                // stack locals (a `with_stack` frame and what is made from it).
                let mut own_vars: Vec<String> = Vec::new();
                let mut locals: Vec<String> = Vec::new();
                for (n, line) in lines.iter().copied().enumerate() {
                    let body = line.split("//").next().unwrap_or(line);
                    if body.trim_start().starts_with("use ") {
                        continue;
                    }
                    track_names(body, &names_own, &mut own_vars, &mut locals);
                    let marked = line.contains("// own:")
                        || lines
                            .get(n + 1)
                            .is_some_and(|next| next.trim_start().starts_with("// own:"));
                    let raw = body.contains("e.global")
                        || body.contains("e.set_global")
                        || body.contains("e.mem.");
                    let cache = body.contains("NiDX9RenderState::")
                        && (body.contains("e.get(") || body.contains("e.set("));
                    // Each own accessor's address (its second argument) must be built from an own-block constant or a
                    // stack local; the generic float helpers count as accessors (their callers are checked). What this
                    // catches and what it does not: a name counts as built from an own constant (or a local) when its
                    // `let`/`for` binding's right-hand side merely MENTIONS one (`let x = ROWS + y` and `let x = f(ROWS)`
                    // alike), names are followed per function by text, not by type, and an address handed in as a
                    // parameter is unknown (marked). It flags an own accessor on a plain heap address (`light + 0x208`);
                    // it cannot prove the address is in an own block. The real net is the whole-frame POISON run.
                    let definition = ["fn ", "pub fn ", "pub(crate) fn "]
                        .iter()
                        .any(|p| body.trim_start().starts_with(p));
                    let unrouted = ["own_at", "set_own_at", "copy_float", "float_at"]
                        .iter()
                        .filter(|a| calls(body, a))
                        .filter_map(|a| address_argument(&lines, n, a))
                        .any(|address| {
                            !names_own(&address)
                                && !own_vars.iter().any(|v| has_word(&address, v))
                                && !locals.iter().any(|v| has_word(&address, v))
                        });
                    if (raw && (!marked || names_own(body)))
                        || cache
                        || (unrouted && !marked && !definition)
                    {
                        findings.push(format!("{name}:{}: raw access in `{}`", n + 1, line.trim()));
                    }
                }
                continue;
            }
            let lines: Vec<&str> = code.lines().collect();
            for (n, line) in lines.iter().copied().enumerate() {
                let body = line.split("//").next().unwrap_or(line);
                // A marker on the line, or on the next (where rustfmt moves a comment that ends a `{` line).
                let marked = line.contains("// own:")
                    || lines
                        .get(n + 1)
                        .is_some_and(|next| next.trim_start().starts_with("// own:"));
                if marked
                    || body.trim_start().starts_with("use ")
                    || body.trim_start().starts_with("const ")
                    || body.trim_start().starts_with("pub(crate) const ")
                    || body.trim_start().starts_with("pub const ")
                {
                    continue;
                }
                let mut tokens: Vec<String> = own_names
                    .iter()
                    .filter(|c| has_word(body, c))
                    .map(|c| (*c).clone())
                    .collect();
                tokens.extend(own_literals(body));
                if body.contains("NiDX9RenderState::")
                    && (body.contains("e.get(")
                        || body.contains("e.set(")
                        || body.contains("e.global")
                        || body.contains("e.set_global")
                        || body.contains("e.mem."))
                {
                    tokens.push("NiDX9RenderState field".to_string());
                }
                if tokens.is_empty() {
                    continue;
                }
                let routed = accessors.iter().any(|a| calls(body, a));
                let raw = body.contains("e.global")
                    || body.contains("e.set_global")
                    || body.contains("e.mem.")
                    || body.contains("e.get(")
                    || body.contains("e.set(");
                if !routed || raw {
                    findings.push(format!(
                        "{name}:{}: {} in `{}`",
                        n + 1,
                        tokens.join(", "),
                        line.trim()
                    ));
                }
            }
        }
        // The engine's own typed helpers the draw reaches by `typed_call!` write guest memory: one that writes an own
        // block (an own literal in its body) may not be called from the draw-time files. It sees only calls written
        // `leaf::NAME(` (not an imported `NAME(` or a by-address call to the helper's address), and only engine-core's
        // leaf.rs among the helpers outside this crate.
        let leaf = include_str!("../../../engine-core/src/typed/leaf.rs");
        let mut writers = Vec::new();
        let mut current: Option<(String, bool)> = None;
        for line in leaf.lines() {
            if let Some(rest) = line.strip_prefix("pub fn ") {
                if let Some((name, true)) = current.take() {
                    writers.push(name);
                }
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                current = Some((name, false));
            } else if let Some((_, writes)) = current.as_mut() {
                *writes |= !own_literals(line).is_empty();
            }
        }
        if let Some((name, true)) = current {
            writers.push(name);
        }
        assert!(
            writers.iter().any(|w| w == "store_colour"),
            "the leaf scan no longer sees store_colour writing the shader statics"
        );
        for (name, text) in sources {
            for (n, line) in code_of(text).lines().enumerate() {
                let body = line.split("//").next().unwrap_or(line);
                for w in &writers {
                    if calls(body, &format!("leaf::{w}")) && !line.contains("// own:") {
                        findings.push(format!(
                            "{name}:{}: `leaf::{w}` writes own state past the packet in `{}`",
                            n + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
        assert!(
            findings.is_empty(),
            "own state read or written past the packet:\n{}",
            findings.join("\n")
        );
    }

    /// Whether `word` occurs in `line` as a whole identifier.
    fn has_word(line: &str, word: &str) -> bool {
        line.match_indices(word).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + word.len()..].chars().next();
            !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    }

    /// Whether `code` defines a constant `name` that is not an own address (another value, or not an address).
    fn defines_otherwise(code: &str, name: &str) -> bool {
        code.lines().any(|line| {
            let t = line.trim_start();
            let t = t
                .strip_prefix("pub(crate) ")
                .or_else(|| t.strip_prefix("pub "))
                .unwrap_or(t);
            t.strip_prefix("const ")
                .and_then(|rest| rest.split_once(':'))
                .is_some_and(|(n, value)| n.trim() == name && own_literals(value).is_empty())
        })
    }

    /// Follows a function's names through `body`: a new function starts both lists afresh; a `with_stack` closure's
    /// frame is a local; a `let` or `for` binding made from an own-block constant (or a name holding one) holds an
    /// own address, one made from a local is a local.
    fn track_names(
        body: &str,
        names_own: &dyn Fn(&str) -> bool,
        own_vars: &mut Vec<String>,
        locals: &mut Vec<String>,
    ) {
        let t = body.trim_start();
        if ["fn ", "pub fn ", "pub(crate) fn "]
            .iter()
            .any(|p| t.starts_with(p))
        {
            own_vars.clear();
            locals.clear();
        }
        if let Some(at) = body.find("with_stack(") {
            if let Some(bar) = body[at..].find("|e, ") {
                let name: String = body[at + bar + 4..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    locals.push(name);
                }
            }
        }
        let binding = t
            .strip_prefix("let ")
            .map(|rest| rest.strip_prefix("mut ").unwrap_or(rest))
            .and_then(|rest| rest.split_once('='))
            .or_else(|| {
                t.strip_prefix("for ")
                    .and_then(|rest| rest.split_once(" in "))
            });
        let Some((lhs, rhs)) = binding else {
            return;
        };
        let name = lhs.split(':').next().unwrap_or(lhs).trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return;
        }
        if names_own(rhs) || own_vars.iter().any(|v| has_word(rhs, v)) {
            own_vars.push(name.to_string());
        } else if locals.iter().any(|v| has_word(rhs, v)) {
            locals.push(name.to_string());
        }
    }

    /// The second argument of the call of `name` that starts on line `n` (the address of an own accessor, after
    /// `e`), read on across lines until its parenthesis closes.
    fn address_argument(lines: &[&str], n: usize, name: &str) -> Option<String> {
        let line = lines[n].split("//").next().unwrap_or(lines[n]);
        let at = line.match_indices(name).find_map(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = &line[at + name.len()..];
            (!before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                && (after.starts_with('(') || after.starts_with("::<")))
            .then_some(at + name.len())
        })?;
        let mut text = line[at..].to_string();
        for next in lines.iter().skip(n + 1).take(8) {
            text.push(' ');
            text.push_str(next.split("//").next().unwrap_or(next));
        }
        let open = text.find('(')?;
        let (mut depth, mut args, mut current) = (0i32, Vec::new(), String::new());
        for c in text[open + 1..].chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' if depth == 0 => {
                    args.push(std::mem::take(&mut current));
                    break;
                }
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(std::mem::take(&mut current));
                    continue;
                }
                _ => {}
            }
            current.push(c);
        }
        args.get(1).map(|a| a.trim().to_string())
    }

    /// Whether `line` calls `name` (a whole identifier followed by `(` or `::<`).
    fn calls(line: &str, name: &str) -> bool {
        line.match_indices(name).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = &line[at + name.len()..];
            !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                && (after.starts_with('(') || after.starts_with("::<"))
        })
    }

    /// The names of the constants (and constant arrays) in `code` holding a guest address (an eight-digit hex
    /// literal): those in an own block when `own`, the others when not.
    fn constants(code: &str, own: bool) -> Vec<String> {
        let mut names = Vec::new();
        for line in code.lines() {
            let t = line.trim_start();
            let t = t
                .strip_prefix("pub(crate) ")
                .or_else(|| t.strip_prefix("pub "))
                .unwrap_or(t);
            let Some(rest) = t.strip_prefix("const ") else {
                continue;
            };
            let Some((name, value)) = rest.split_once(':') else {
                continue;
            };
            let address = value.split("0x").nth(1).is_some_and(|digits| {
                digits
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
                    .filter(|c| *c != '_')
                    .count()
                    == 8
            });
            if address && own_literals(value).is_empty() != own {
                names.push(name.trim().to_string());
            }
        }
        names
    }

    /// The hex literals in `text` that are own addresses.
    fn own_literals(text: &str) -> Vec<String> {
        text.match_indices("0x")
            .filter_map(|(at, _)| {
                let digits: String = text[at + 2..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
                    .filter(|c| *c != '_')
                    .collect();
                let value = u32::from_str_radix(&digits, 16).ok()?;
                (digits.len() == 8 && is_own_global(value)).then(|| format!("{value:#010x}"))
            })
            .collect()
    }
}
