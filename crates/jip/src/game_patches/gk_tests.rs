//! Group GK's game patches: G95 (projectile lights, with `SetProjectileLight`), G96 (the muzzle flash light's
//! fade) and G97 (the explosion light's fade), from JIP's assembly case by case. The scenes are
//! `nif_nodes_tests.rs`'s trees; the game functions the handlers call are stand-ins that record their words.

use super::*;
use crate::commands::nif_nodes::tests::{node, object, reference, scene_engine, shape, NI_NODE};
use crate::commands::test_support::{calls, form, r, set_ref, CALLS, FORMS};
use crate::game_patches::gl::lights::{active_point_lights, POINT_LIGHT_VTABLE};
use engine_core::extend::jip_patches;

const ADD_OBJECT: u32 = 0x00a5_ed10;
const REMOVE_OBJECT: u32 = 0x00a5_d8f0;
const ADD_STRING: u32 = 0x00a5_b690;
const CREATE: u32 = 0x00a7_d6e0;
/// The forms: a projectile (type 0x33, its light +0x70, muzzle flash light +0x74), two lights (0x1E, their fade
/// +0xB4), an explosion (0x51, its light +0x80); a muzzle flash and an explosion reference.
const PROJ: u32 = FORMS + 0x1000;
const LIGHT: u32 = FORMS + 0x1200;
const OTHER_LIGHT: u32 = FORMS + 0x1400;
const EXPL: u32 = FORMS + 0x1600;
const FLASH: u32 = FORMS + 0x1800;
const EXPLOSION: u32 = FORMS + 0x1a00;
const FLASH_LIGHT: u32 = FORMS + 0x1c00;
const HOLDER: u32 = FORMS + 0x1e00;
/// The bound objects' vtable (`IsBoundObject`, +0xE8, answering true) for `SetProjectileLight`'s reader.
const VB: u32 = FORMS + 0x7800;
const IS_BOUND: u32 = 0x0050_3000;
/// A point light's form (+0xE8), `resetTraits` (+0x9E), its high flags byte (+0x33) and dimmer (+0xC4) and the
/// change number the game's setter counts (+0xA8).
const LIGHT_FORM: u32 = 0xe8;
const RESET: u32 = 0x9e;
const FLAGS_HIGH: u32 = 0x33;
const CHANGES: u32 = 0xa8;

fn rec(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn clear() {
    CALLS.with(|x| x.borrow_mut().clear());
}

/// JIP installed with the row on or off (the settings file: none, JIP's defaults), the node and light stand-ins,
/// the forms, and the game's `AttachLight` string (its holder at `011C620C`).
fn world(row: bool) -> Engine {
    let mut e = scene_engine();
    if !e.mem.is_mapped(0x011c_6000) {
        e.map(0x011c_6000, 0x1000);
    }
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    if !e.is_registered(0x0061_a310) {
        e.register(0x0061_a310, |_, _| Ret::default());
    }
    crate::game_patches::set_row(&mut e, row);
    e.mem.set_u32(NI_NODE + 0xdc, ADD_OBJECT);
    e.mem.set_u32(NI_NODE + 0xe8, REMOVE_OBJECT);
    e.register(ADD_OBJECT, |e, w| {
        rec(ADD_OBJECT, w);
        // The light now has its parent, as `NiNode::AttachChild` leaves it.
        e.mem.set_u32(w[1] + 0x18, w[0]);
        Ret::default()
    });
    e.register(REMOVE_OBJECT, |_, w| {
        rec(REMOVE_OBJECT, w);
        Ret::default()
    });
    e.register(CREATE, |e, w| {
        rec(CREATE, w);
        object(e, POINT_LIGHT_VTABLE, None).into_ret()
    });
    e.register(ADD_STRING, |e, w| {
        let text = nvse::table::guest_text(e, w[0]);
        crate::commands::nif_nodes::tests::fixed(e, &text).into_ret()
    });
    e.register(IS_BOUND, |_, _| 1u32.into_ret());
    e.mem.set_u32(VB + 0xe8, IS_BOUND);
    form(&mut e, PROJ, 0x33, 0x0000_0b01);
    form(&mut e, LIGHT, 0x1e, 0x0000_011e);
    form(&mut e, OTHER_LIGHT, 0x1e, 0x0000_021e);
    form(&mut e, EXPL, 0x51, 0x0000_0e51);
    for f in [PROJ, LIGHT, OTHER_LIGHT, EXPL] {
        e.mem.set_u32(f, VB);
    }
    set_ref(&mut e, 1, PROJ);
    set_ref(&mut e, 2, LIGHT);
    set_ref(&mut e, 3, OTHER_LIGHT);
    e.mem.set_f32(LIGHT + 0xb4, 0.75);
    e.mem.set_f32(OTHER_LIGHT + 0xb4, 1.5);
    let name = crate::commands::nif_nodes::tests::fixed(&mut e, b"AttachLight");
    e.mem.set_u32(HOLDER, name);
    e.mem.set_u32(0x011c_620c, HOLDER);
    clear();
    e
}

/// A fired projectile of [`PROJ`] whose 3D's root holds `children`.
fn projectile(e: &mut Engine, children: &[u32]) -> (u32, u32) {
    let root = node(e, b"Root", children);
    let refr = reference(e, 0xff00_0101, false, root);
    e.mem.set_u32(refr + 0x20, PROJ);
    (refr, root)
}

/// The `AttachLight` fixed string the game made.
fn attach_name(e: &Engine) -> u32 {
    e.mem.u32(HOLDER)
}

/// With the row off nothing of the group is in; with it on, its four seams.
#[test]
fn gk_goes_in_with_the_row() {
    let e = world(false);
    for slot in [
        ADD_PROJECTILE_LIGHT,
        REMOVE_PROJECTILE_LIGHT,
        MUZZLE_FLASH_FADE,
        EXPLOSION_LIGHT_FADE,
    ] {
        assert!(e.site_owners(slot.addr()).is_empty(), "{:x}", slot.addr());
    }
    assert_eq!(e.jip_patches() & jip_patches::FIXES, 0);
    let e = world(true);
    for slot in [
        ADD_PROJECTILE_LIGHT,
        REMOVE_PROJECTILE_LIGHT,
        MUZZLE_FLASH_FADE,
        EXPLOSION_LIGHT_FADE,
    ] {
        assert_eq!(
            e.site_owners(slot.addr()),
            vec![Owner::Jip],
            "{:x}",
            slot.addr()
        );
    }
}

/// G95's add: the form's light, under the root's `AttachLight` node, one of JIP's point lights (made, named,
/// added under the node, its form set, marked to be set from it, listed), kept at +0x114. Answers the call made.
#[test]
fn g95_a_projectile_with_a_light_gets_jips_point_light_at_its_attach_node() {
    let mut e = world(true);
    let attach = node(&mut e, b"AttachLight", &[]);
    let inner = node(&mut e, b"Barrel", &[attach]);
    let (refr, _) = projectile(&mut e, &[inner]);
    e.mem.set_u32(attach + 8, attach_name(&e));
    e.mem.set_u32(PROJ + 0x70, LIGHT);
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    let light = e.mem.u32(refr + 0x114);
    assert_ne!(light, 0);
    assert_eq!(e.mem.u32(light), POINT_LIGHT_VTABLE);
    let added: Vec<_> = calls().into_iter().filter(|c| c.0 == ADD_OBJECT).collect();
    assert_eq!(added, vec![(ADD_OBJECT, vec![attach, light, 1])]);
    assert_eq!(e.mem.u32(light + LIGHT_FORM), LIGHT);
    assert_eq!(e.mem.u8(light + RESET), 1);
    assert_eq!(e.mem.u8(light + FLAGS_HIGH) & 0x60, 0x60);
    assert_eq!(active_point_lights(&mut e), vec![light]);
}

/// G95's add without an `AttachLight` node, or with an `AttachLight` block that is not a node: under the root.
#[test]
fn g95_without_an_attach_node_the_light_goes_under_the_root() {
    let mut e = world(true);
    let (refr, root) = projectile(&mut e, &[]);
    e.mem.set_u32(PROJ + 0x70, LIGHT);
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    let light = e.mem.u32(refr + 0x114);
    assert_eq!(calls()[1], (ADD_OBJECT, vec![root, light, 1]));

    let mut e = world(true);
    let geometry = shape(&mut e, b"AttachLight");
    let (refr, root) = projectile(&mut e, &[geometry]);
    e.mem.set_u32(geometry + 8, attach_name(&e));
    e.mem.set_u32(PROJ + 0x70, LIGHT);
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    let light = e.mem.u32(refr + 0x114);
    assert_eq!(calls()[1], (ADD_OBJECT, vec![root, light, 1]));
}

/// G95's add makes nothing (and still answers the call made, so the game's maker does not run) for a projectile
/// flagged 0x20 or 0x800, a form without a light, no loaded 3D, or a 3D without a root.
#[test]
fn g95_no_light_without_a_form_light_or_a_3d() {
    for case in 0..5 {
        let mut e = world(true);
        let (refr, _) = projectile(&mut e, &[]);
        e.mem.set_u32(PROJ + 0x70, LIGHT);
        match case {
            0 => e.mem.set_u32(refr + 8, 0x20),
            1 => e.mem.set_u32(refr + 8, 0x800),
            2 => e.mem.set_u32(PROJ + 0x70, 0),
            3 => e.mem.set_u32(refr + 0x64, 0),
            _ => {
                let loaded = e.mem.u32(refr + 0x64);
                e.mem.set_u32(loaded + 0x14, 0);
            }
        }
        assert_eq!(
            e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]),
            Some(0),
            "{case}"
        );
        assert_eq!(e.mem.u32(refr + 0x114), 0, "{case}");
        assert!(calls().is_empty(), "{case}");
        assert!(active_point_lights(&mut e).is_empty(), "{case}");
    }
    // Other flags do not stop it.
    let mut e = world(true);
    let (refr, _) = projectile(&mut e, &[]);
    e.mem.set_u32(PROJ + 0x70, LIGHT);
    e.mem.set_u32(refr + 8, !0x820);
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    assert_ne!(e.mem.u32(refr + 0x114), 0);
}

/// G95's remove: the word cleared and the light taken off its parent; without a parent only the word; without a
/// light nothing. Always the call made (the game's remover does not run).
#[test]
fn g95_the_light_goes_with_the_projectile() {
    let mut e = world(true);
    let (refr, root) = projectile(&mut e, &[]);
    e.mem.set_u32(PROJ + 0x70, LIGHT);
    e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]);
    let light = e.mem.u32(refr + 0x114);
    clear();
    assert_eq!(e.site(REMOVE_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    assert_eq!(e.mem.u32(refr + 0x114), 0);
    assert_eq!(calls(), vec![(REMOVE_OBJECT, vec![root, light])]);

    clear();
    assert_eq!(e.site(REMOVE_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    assert!(calls().is_empty(), "no light: nothing");

    e.mem.set_u32(refr + 0x114, light);
    e.mem.set_u32(light + 0x18, 0);
    assert_eq!(e.site(REMOVE_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    assert_eq!(e.mem.u32(refr + 0x114), 0);
    assert!(calls().is_empty(), "no parent: the word only");
}

/// `SetProjectileLight` end to end: the light a script gives a projectile form is the one its fired projectiles
/// get; taken away (no light given), they get none.
#[test]
fn g95_set_projectile_light_lights_the_fired_projectiles() {
    let mut e = world(true);
    crate::test_support::call(&mut e, 0x2933, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(PROJ + 0x70), OTHER_LIGHT);
    let (refr, _) = projectile(&mut e, &[]);
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    let light = e.mem.u32(refr + 0x114);
    assert_ne!(light, 0);
    assert_eq!(e.mem.u32(light + LIGHT_FORM), OTHER_LIGHT);

    crate::test_support::call(&mut e, 0x2933, &[r(1)]);
    assert_eq!(e.mem.u32(PROJ + 0x70), 0);
    let (refr, _) = projectile(&mut e, &[]);
    clear();
    assert_eq!(e.site(ADD_PROJECTILE_LIGHT.addr(), &[refr]), Some(0));
    assert_eq!(e.mem.u32(refr + 0x114), 0);
    assert!(calls().is_empty());
}

/// A muzzle flash of [`PROJ`] (on or off) and its light (dimmer 0.25, change number 7).
fn flash(e: &mut Engine, on: bool) -> u32 {
    e.mem.set_u8(FLASH, u8::from(on));
    e.mem.set_u32(FLASH + 0x14, PROJ);
    e.mem.set_u32(FLASH + 0x10, FLASH_LIGHT);
    e.mem.set_f32(FLASH_LIGHT + 0xc4, 0.25);
    e.mem.set_u32(FLASH_LIGHT + CHANGES, 7);
    FLASH_LIGHT
}

/// G96: on, the light's dimmer is its form's fade; off, 0; the change number is not counted.
#[test]
fn g96_the_flash_light_shines_at_its_forms_fade() {
    let mut e = world(true);
    e.mem.set_u32(PROJ + 0x74, OTHER_LIGHT);
    let light = flash(&mut e, true);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), Some(0));
    assert_eq!(e.mem.f32(light + 0xc4), 1.5);
    assert_eq!(e.mem.u32(light + CHANGES), 7);
    let light = flash(&mut e, false);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), Some(0));
    assert_eq!(e.mem.u32(light + 0xc4), 0);
}

/// Page 0 mapped, its words at +0x74 (a muzzle flash light form's place in a null projectile form) and +0xB4 (a
/// fade's place in a null light form) set, so a read through null shows what it read.
fn null_page(e: &mut Engine) {
    if !e.mem.is_mapped(0) {
        e.map(0, 0x1000);
    }
    e.mem.set_u32(0x74, OTHER_LIGHT);
    e.mem.set_f32(0xb4, 0.8);
}

/// G96 without the projectile form or its muzzle flash light: with the missing-data guard row the game's code
/// (`None`, nv-rs's guard); without it read through null, as JIP.
#[test]
fn g96_a_flash_without_its_light_form_follows_the_guard_row() {
    use nvse::commands::array::{set_fix, NULL_GUARD};
    let mut e = world(true);
    null_page(&mut e);
    set_fix(&mut e, NULL_GUARD, true);
    let light = flash(&mut e, true);
    e.mem.set_u32(PROJ + 0x74, 0);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), None);
    assert_eq!(e.mem.f32(light + 0xc4), 0.25);
    e.mem.set_u32(PROJ + 0x74, OTHER_LIGHT);
    e.mem.set_u32(FLASH + 0x14, 0);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), None);
    assert_eq!(e.mem.f32(light + 0xc4), 0.25);

    set_fix(&mut e, NULL_GUARD, false);
    let light = flash(&mut e, true);
    e.mem.set_u32(FLASH + 0x14, 0);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), Some(0));
    assert_eq!(e.mem.f32(light + 0xc4), 1.5, "the form read at 0x74");
    let light = flash(&mut e, true);
    e.mem.set_u32(PROJ + 0x74, 0);
    assert_eq!(e.site(MUZZLE_FLASH_FADE.addr(), &[FLASH, light]), Some(0));
    assert_eq!(e.mem.f32(light + 0xc4), 0.8, "the fade read at 0xB4");
}

/// An explosion of [`EXPL`] at `age` of `life`, its light (dimmer 0.25, change number 7).
fn explosion(e: &mut Engine, age: f32, life: f32) -> u32 {
    e.mem.set_u32(EXPLOSION + 0x20, EXPL);
    e.mem.set_f32(EXPLOSION + 0x88, age);
    e.mem.set_f32(EXPLOSION + 0x8c, life);
    e.mem.set_u32(EXPLOSION + 0xc4, FLASH_LIGHT);
    e.mem.set_f32(FLASH_LIGHT + 0xc4, 0.25);
    e.mem.set_u32(FLASH_LIGHT + CHANGES, 7);
    FLASH_LIGHT
}

/// G97: the dimmer `1 - 2 age / life` held between 0 and 1, times the form light's fade (0.75); unordered held
/// as `minss` and `maxss` hold it (1, then kept); the change number not counted. No light: nothing.
#[test]
fn g97_the_explosion_light_fades_from_its_forms_fade() {
    let mut e = world(true);
    e.mem.set_u32(EXPL + 0x80, LIGHT);
    for (age, life, want) in [
        (0.0f32, 2.0f32, 0.75f32),
        (0.5, 2.0, 0.375),
        (1.0, 2.0, 0.0),
        (1.5, 2.0, 0.0),
        (-1.0, 2.0, 0.75),
        (0.0, 0.0, 0.75),
        (1.0, 0.0, 0.0),
    ] {
        let light = explosion(&mut e, age, life);
        assert_eq!(e.site(EXPLOSION_LIGHT_FADE.addr(), &[EXPLOSION]), Some(0));
        assert_eq!(e.mem.f32(light + 0xc4), want, "{age} of {life}");
        assert_eq!(e.mem.u32(light + CHANGES), 7);
    }
    let light = explosion(&mut e, 0.5, 4.0);
    let left = 1.0f32 - 1.0 / 4.0;
    assert_eq!(e.site(EXPLOSION_LIGHT_FADE.addr(), &[EXPLOSION]), Some(0));
    assert_eq!(e.mem.f32(light + 0xc4), left * 0.75);

    explosion(&mut e, 0.5, 2.0);
    e.mem.set_u32(EXPLOSION + 0xc4, 0);
    assert_eq!(e.site(EXPLOSION_LIGHT_FADE.addr(), &[EXPLOSION]), Some(0));
    assert_eq!(e.mem.f32(FLASH_LIGHT + 0xc4), 0.25, "no light: nothing");
}

/// G97 with a form without a light: with the missing-data guard row the game's code (`None`, nv-rs's guard);
/// without it read through null, as JIP.
#[test]
fn g97_an_explosion_without_its_light_form_follows_the_guard_row() {
    use nvse::commands::array::{set_fix, NULL_GUARD};
    let mut e = world(true);
    null_page(&mut e);
    e.mem.set_u32(EXPL + 0x80, 0);
    set_fix(&mut e, NULL_GUARD, true);
    let light = explosion(&mut e, 0.5, 2.0);
    assert_eq!(e.site(EXPLOSION_LIGHT_FADE.addr(), &[EXPLOSION]), None);
    assert_eq!(e.mem.f32(light + 0xc4), 0.25);
    set_fix(&mut e, NULL_GUARD, false);
    let light = explosion(&mut e, 0.5, 2.0);
    assert_eq!(e.site(EXPLOSION_LIGHT_FADE.addr(), &[EXPLOSION]), Some(0));
    assert_eq!(
        e.mem.f32(light + 0xc4),
        0.4,
        "0.5 times the fade read at 0xB4"
    );
}
