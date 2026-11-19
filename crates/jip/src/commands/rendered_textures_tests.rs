//! The drawing's stand-ins, shared by `ProjectExtraCamera`'s and `ProjectUITile`'s tests, and the
//! texture lookup and rounding on their own. Expectations from JIP's `internal/netimmerse.cpp`
//! (`GenerateRenderedTexture`, `GenerateRenderedUITexture`, `GetGRTCullingProcess`) and
//! `nvse/GameObjects.cpp` (`GetTexturePtr`).

use super::super::nif_nodes::tests::*;
use super::*;
use crate::commands::test_support::CALLS;

/// The drawing's game calls.
pub(crate) const CREATE_RENDERED_TEXTURE: u32 = 0x00b6_d170;
pub(crate) const BEGIN_FRAME: u32 = 0x00b6_b8d0;
pub(crate) const CULL_SCENE: u32 = 0x00b6_bee0;
pub(crate) const DRAW_SCENE: u32 = 0x00b6_c0d0;
pub(crate) const END_FRAME: u32 = 0x00b6_b790;
pub(crate) const NI_ALLOC: u32 = 0x00aa_13e0;
pub(crate) const CULLING_PROCESS_CTOR: u32 = 0x004a_0eb0;
pub(crate) const ACCUMULATOR_CTOR: u32 = 0x00b6_60d0;
pub(crate) const WATER_UPDATE: u32 = 0x004e_21b0;
/// The game's words the drawing reads.
pub(crate) const RENDERER_WORD: u32 = 0x011f_4748;
pub(crate) const EXTRA_CAMERA: u32 = 0x011f_91c5;
pub(crate) const HDR: u32 = 0x011f_941e;
pub(crate) const HIGH_ACTOR_LIST: u32 = 0x011e_0f00;
pub(crate) const TES_WORD: u32 = 0x011d_ea10;
pub(crate) const WATER_WORDS: u32 = 0x011c_7b70;
pub(crate) const IM_WORD: u32 = 0x011d_8a80;
/// The test's own objects: the texture manager, the renderer, the shadow scene node, a rendered
/// texture (its vtable's +4 the delete) and the texture it made, the old texture in a slot, the
/// TES and its water manager, what `Ni_Alloc` hands out, the interface manager, its scene and the
/// interface camera.
pub(crate) const MANAGER_AT: u32 = AREA + 0xd_0000;
pub(crate) const RENDERER_AT: u32 = AREA + 0xd_1000;
pub(crate) const SCENE_AT: u32 = AREA + 0xd_2000;
pub(crate) const RENDERED_AT: u32 = AREA + 0xd_3000;
pub(crate) const RENDERED_VTBL: u32 = AREA + 0xd_3800;
pub(crate) const MADE_AT: u32 = AREA + 0xd_4000;
pub(crate) const OLD_AT: u32 = AREA + 0xd_4800;
pub(crate) const TEXTURE_VTBL: u32 = AREA + 0xd_4c00;
pub(crate) const TES_AT: u32 = AREA + 0xd_5000;
pub(crate) const WATER_AT: u32 = AREA + 0xd_5800;
pub(crate) const ALLOC_AT: u32 = AREA + 0xd_6000;
pub(crate) const IM_AT: u32 = AREA + 0xd_7000;
pub(crate) const UI_SCENE_AT: u32 = AREA + 0xd_8000;
pub(crate) const UI_CAMERA_AT: u32 = AREA + 0xd_9000;
/// The deletes (stand-in addresses of the tests').
pub(crate) const RENDERED_DELETE: u32 = 0x00de_0004;
pub(crate) const TEXTURE_DELETE: u32 = 0x00de_0008;
/// The rendered texture's render target group.
pub(crate) const GROUP: u32 = 0x0777_0000;
/// The renderer's clear mask before a draw.
pub(crate) const MASK: u32 = 0xff00_ff00;

pub(crate) fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

/// The drawing's stand-ins: the manager makes the rendered texture at [`RENDERED_AT`] (its
/// texture [`MADE_AT`], counted 0), `Ni_Alloc` hands out [`ALLOC_AT`], the constructors answer
/// `this`; the passes record their words, the scene's byte +0x130, the renderer's mask, then the
/// interface camera's view port (four float words) and the node's flags byte when the second
/// word is a node of the test's interface; the rest record.
pub(crate) fn drawing(e: &mut Engine) {
    for page in [
        0x011f_4000,
        0x011f_9000,
        0x011e_0000,
        0x011d_e000,
        0x011c_7000,
        0x011d_8000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(TEXTURE_MANAGER, MANAGER_AT);
    e.mem.set_u32(RENDERER_WORD, RENDERER_AT);
    e.mem.set_u32(SHADOW_SCENE_NODE, SCENE_AT);
    e.mem.set_u32(RENDERER_AT + RENDERER_CLEAR_MASK, MASK);
    e.mem.set_u32(RENDERED_AT, RENDERED_VTBL);
    e.mem.set_u32(RENDERED_VTBL + 4, RENDERED_DELETE);
    e.mem.set_u32(RENDERED_AT + TEXTURE_TARGET_GROUP, GROUP);
    e.mem.set_u32(RENDERED_AT + TEXTURE_NI_TEXTURE, MADE_AT);
    for t in [MADE_AT, OLD_AT] {
        e.mem.set_u32(t, TEXTURE_VTBL);
    }
    e.mem.set_u32(TEXTURE_VTBL + 4, TEXTURE_DELETE);
    e.mem.set_u32(OLD_AT + 4, 1);
    e.mem.set_u32(IM_WORD, IM_AT);
    e.mem.set_u32(IM_AT + 4, UI_SCENE_AT);
    e.mem.set_u32(UI_SCENE_AT + 0xac, UI_CAMERA_AT);
    e.register(CREATE_RENDERED_TEXTURE, |_, w| {
        record(CREATE_RENDERED_TEXTURE, w);
        Ret {
            eax: RENDERED_AT,
            ..Ret::default()
        }
    });
    e.register(NI_ALLOC, |_, w| {
        record(NI_ALLOC, w);
        Ret {
            eax: ALLOC_AT,
            ..Ret::default()
        }
    });
    // A handler is a plain function (no captures): one closure per address.
    macro_rules! answers_this {
        ($f:expr) => {
            e.register($f, |_, w| {
                record($f, w);
                Ret {
                    eax: w[0],
                    ..Ret::default()
                }
            })
        };
    }
    macro_rules! records {
        ($f:expr) => {
            e.register($f, |_, w| {
                record($f, w);
                Ret::default()
            })
        };
    }
    macro_rules! pass {
        ($f:expr) => {
            e.register($f, |e, w| {
                record($f, &pass_words(e, $f, w));
                Ret::default()
            })
        };
    }
    answers_this!(CULLING_PROCESS_CTOR);
    answers_this!(ACCUMULATOR_CTOR);
    records!(BEGIN_FRAME);
    records!(END_FRAME);
    records!(RENDERED_DELETE);
    records!(TEXTURE_DELETE);
    pass!(CULL_SCENE);
    pass!(DRAW_SCENE);
    e.register(WATER_UPDATE, |e, w| {
        let mut words = w.to_vec();
        words.push(u32::from(e.mem.u8(EXTRA_CAMERA)));
        words.push(e.mem.u32(WATER_WORDS));
        words.push(e.mem.u32(WATER_WORDS + 0x50));
        record(WATER_UPDATE, &words);
        Ret::default()
    });
}

/// What a pass stand-in records: its words, the scene's byte +0x130, the renderer's mask; for the
/// interface camera also its view port (four float words) and, for the accumulate pass, the node's
/// flags byte.
fn pass_words(e: &Engine, f: u32, w: &[u32]) -> Vec<u32> {
    let mut words = w.to_vec();
    words.push(u32::from(e.mem.u8(SCENE_AT + SCENE_RENDERING_TEXTURE)));
    words.push(e.mem.u32(RENDERER_AT + RENDERER_CLEAR_MASK));
    if w[0] == UI_CAMERA_AT {
        words.extend((0..4).map(|i| e.mem.u32(UI_CAMERA_AT + CAMERA_VIEW_PORT + 4 * i)));
        if f == CULL_SCENE {
            words.push(u32::from(e.mem.u8(w[1] + NODE_FLAGS)));
        }
    }
    words
}

/// The tile menu array (`*011F350C`) holding, for the HUD (menu 0x3EC), a tile of class `vtable`
/// (a path `HUDMainMenu` names it: a path without a slash is the menu's own tile). The tile.
pub(crate) fn hud_tile(e: &mut Engine, vtable: u32) -> u32 {
    if !e.mem.is_mapped(0x011f_3000) {
        e.map(0x011f_3000, 0x1000);
    }
    let array = object(e, NI_NODE, None);
    let tile = object(e, vtable, None);
    e.mem.set_u32(0x011f_350c, array);
    e.mem.set_u32(array + 4 * (0x3ec - 0x3e9), tile);
    tile
}

/// `head`'s class a tri-based geometry (vtable +0x1C, as `GetTexturePtr` asks) with a shader of
/// type `kind` whose texture word (lighting: the slot pointer at +0xAC; tile: +0x60) is set up:
/// for a lighting shader the slot `slot` holding the old texture. The shader.
pub(crate) fn shaded(e: &mut Engine, head: u32, kind: u8, slot: u32) -> u32 {
    e.mem.set_u32(NI_TRI_SHAPE + 0x1c, super::RETURN_THIS2);
    let shader = object(e, NI_NODE, None);
    e.mem.set_u8(shader + SHADER_TYPE, kind);
    if kind == 8 {
        e.mem.set_u32(shader + 0xac, slot);
        e.mem.set_u32(slot, OLD_AT);
    }
    e.mem.set_u32(head + GEOMETRY_SHADER, shader);
    shader
}

/// The texture place: a lighting shader's slot pointer, a tile shader's texture word; 0 for an
/// empty name, no 3D, no block, a block that is not tri-based geometry, no shader or another
/// kind. The root is `GetNiNode`'s: the first person node for the player in first person.
#[test]
fn texture_ptr_follows_get_texture_ptr() {
    let mut e = scene_engine();
    let head = shape(&mut e, b"Head");
    let other = shape(&mut e, b"Other");
    let body = node(&mut e, b"Body", &[head, other]);
    let refr = reference(&mut e, 0x1234, false, body);
    let slot = object(&mut e, NI_NODE, None);
    let lit = shaded(&mut e, head, 8, slot);
    assert_eq!(texture_ptr(&e, refr, b"Head"), slot);
    e.mem.set_u8(lit + SHADER_TYPE, 0x15);
    assert_eq!(texture_ptr(&e, refr, b"Head"), lit + 0x60);
    e.mem.set_u8(lit + SHADER_TYPE, 9);
    assert_eq!(texture_ptr(&e, refr, b"Head"), 0);
    assert_eq!(texture_ptr(&e, refr, b"Other"), 0, "no shader");
    assert_eq!(texture_ptr(&e, refr, b"Body"), 0, "a node");
    assert_eq!(texture_ptr(&e, refr, b""), 0);
    assert_eq!(texture_ptr(&e, refr, b"Nowhere"), 0);
    let bare = reference(&mut e, 0x1235, false, 0);
    assert_eq!(texture_ptr(&e, bare, b"Head"), 0);
    // A geometry that is not tri-based (+0x18 answers, +0x1C does not): none.
    e.mem.set_u8(lit + SHADER_TYPE, 8);
    e.mem.set_u32(NI_TRI_SHAPE + 0x1c, 0);
    assert_eq!(texture_ptr(&e, refr, b"Head"), 0, "not tri-based");
    e.mem.set_u32(NI_TRI_SHAPE + 0x1c, super::RETURN_THIS2);
    assert_eq!(texture_ptr(&e, refr, b"Head"), slot);
    // The player in first person: the first person node's blocks.
    let first_head = shape(&mut e, b"Head");
    let first = node(&mut e, b"First", &[first_head]);
    let pc = player(&mut e, body, first, false);
    let first_slot = object(&mut e, NI_NODE, None);
    shaded(&mut e, first_head, 8, first_slot);
    e.mem.set_u8(lit + SHADER_TYPE, 8);
    assert_eq!(texture_ptr(&e, pc, b"Head"), first_slot);
    e.mem.set_u8(pc + IS_3RD_PERSON, 1);
    assert_eq!(texture_ptr(&e, pc, b"Head"), slot);
}

/// `cvtps2dq`: to the nearest, halves to even; out of range and NaN the "integer indefinite".
#[test]
fn sizes_round_as_cvtps2dq() {
    assert_eq!(cvtps2dq(300.4), 300);
    assert_eq!(cvtps2dq(200.5), 200);
    assert_eq!(cvtps2dq(201.5), 202);
    assert_eq!(cvtps2dq(-1.5), (-2i32) as u32);
    assert_eq!(cvtps2dq(f32::NAN), 0x8000_0000);
    assert_eq!(cvtps2dq(3.0e9), 0x8000_0000);
}

/// No texture made (the game's `CreateRenderedTexture` answers 0, as for an area that rounds to
/// 0): neither drawing does more than ask for it. No culling process, no frame, no passes, no
/// replace or release; the slot keeps its texture, counted as it was.
#[test]
fn no_texture_made_draws_nothing() {
    let mut e = scene_engine();
    drawing(&mut e);
    e.register(CREATE_RENDERED_TEXTURE, |_, w| {
        record(CREATE_RENDERED_TEXTURE, w);
        Ret::default()
    });
    let slot = object(&mut e, NI_NODE, None);
    e.mem.set_u32(slot, OLD_AT);
    let node = object(&mut e, NI_NODE, None);
    let params = TextureParams {
        width: 64,
        height: 64,
        format: A8R8G8B8,
        render_mode: 0,
        bg_color_mask: 0,
    };
    CALLS.with(|c| c.borrow_mut().clear());
    generate_rendered_texture(&mut e, UI_CAMERA_AT, params, slot);
    generate_rendered_ui_texture(&mut e, node, [0.0, 0.0, 0.4, 0.4], slot, 360.0);
    let made: Vec<u32> = CALLS.with(|c| c.borrow().iter().map(|(f, _)| *f).collect());
    assert_eq!(made, [CREATE_RENDERED_TEXTURE; 2]);
    assert_eq!(e.mem.u32(slot), OLD_AT);
    assert_eq!(e.mem.u32(OLD_AT + 4), 1);
    assert_eq!(e.ext_free::<UiCulling>().0.load(Ordering::Acquire), 0);
}
