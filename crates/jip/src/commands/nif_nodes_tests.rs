//! Tests of the scene-graph lookup on small trees built in guest memory with the classes' real
//! vtables (the slot words JIP's `IS_NODE` / `IS_GEOMETRY` read written at the exe's addresses),
//! expectations from JIP's asm (`netimmerse.cpp`, `GameObjects.cpp`, `GetNifBlock`). The builder
//! is shared with `jip_fn_misc_ref_nodes_tests.rs`.

// The code's functions only: every layout number the scenes are built with is written below as
// the exe has it (a literal, with the function it was read from), so a wrong constant in the code
// is caught instead of being built into the scene too.
use super::{
    bounding_box, extra_data_of_class, floats3, get_block, get_node, matrix, name_text, ni_block,
    ni_node, nif_block, ref_ni_node, to_euler_pry, to_euler_pry_inv, FLT_180_D_PI,
};
use crate::commands::test_support::engine;
use engine_core::prelude::*;

/// `GetAsNiNode` (vtable +0xC) of `NiNode` `0109B5AC` and `BSFadeNode` `010A8F90`: `006815C0`
/// (returns `this`).
pub(crate) const RETURN_THIS: u32 = 0x0068_15c0;
/// Vtable slot +0x18 of `NiTriShape` `0109D454` (and the other geometry classes): `00E68810`
/// (returns `this`).
pub(crate) const RETURN_THIS2: u32 = 0x00e6_8810;
/// Vtable +0xFC of `Actor` `01084254` (`Character`, `Creature`, `PlayerCharacter` too): `008D0360`
/// (returns true; `TESObjectREFR` `0102F55C` has `0047C850`).
pub(crate) const RETURN_TRUE: u32 = 0x008d_0360;
/// `m_blockName` +8: `NiAVObject::GetObjectByName` `00A59E20` compares `[this+8]`.
pub(crate) const NAME: u32 = 0x08;
/// Extra data array +0x10, its count (u16) +0x14: `NiObjectNET::GetExtraData` `00A5BDD0`.
pub(crate) const EXTRA_LIST: u32 = 0x10;
pub(crate) const EXTRA_COUNT: u32 = 0x14;
/// Parent +0x18: `00A68BF0` takes the parent's world transform from `[this+0x18]`.
pub(crate) const PARENT: u32 = 0x18;
/// Collision object +0x1C: `NiAVObject::UpdateWorldData` `00A68C60` calls `[this+0x1C]`.
pub(crate) const COLLISION_OBJECT: u32 = 0x1c;
/// Flags +0x30: `00A68BF0` tests bit 9 of `[this+0x30]`.
pub(crate) const FLAGS: u32 = 0x30;
/// Local transform +0x34 (rotation, translation +0x58, scale +0x64) copied to the world transform
/// +0x68 (translation +0x8C) by `00A68BF0` (13 words); `NiTransform::operator*` `0062C250`:
/// rotation +0, translation +0x24, scale +0x30.
pub(crate) const LOCAL_ROTATE: u32 = 0x34;
pub(crate) const LOCAL_TRANSLATE: u32 = 0x58;
pub(crate) const LOCAL_SCALE: u32 = 0x64;
pub(crate) const WORLD_ROTATE: u32 = 0x68;
pub(crate) const WORLD_TRANSLATE: u32 = 0x8c;
/// Children data +0xA0, size (u16) +0xA6: `NiNode::GetObjectByName` `00A5E560`.
pub(crate) const CHILDREN: u32 = 0xa0;
pub(crate) const CHILDREN_SIZE: u32 = 0xa6;
/// A form's `refID` +0xC (JIP's `IsPlayer`: 0x14).
pub(crate) const REF_ID: u32 = 0x0c;
pub(crate) const PLAYER_ID: u32 = 0x14;
/// Loaded data +0x64, its root node +0x14: `TESObjectREFR::Get3D` `0043FCD0`.
pub(crate) const LOADED_DATA: u32 = 0x64;
pub(crate) const ROOT_NODE: u32 = 0x14;
/// First person node +0x694, third person flag (byte) +0x64A: `PlayerCharacter::Get3D`
/// `00950B60`.
pub(crate) const NODE_1ST_PERSON: u32 = 0x694;
pub(crate) const IS_3RD_PERSON: u32 = 0x64a;
/// An actor's process +0x68, its level (byte) +0x28, `MiddleHighProcess`' bound +0x224
/// (`GetBSBound` `009EE020`).
pub(crate) const PROCESS: u32 = 0x68;
pub(crate) const PROCESS_LEVEL: u32 = 0x28;
pub(crate) const PROCESS_BOUND: u32 = 0x224;
/// `BSBound`'s vtable (`010C2B64`, +0x4C `BSBound::LoadBinary` `00C54E30`: centre +0xC, half
/// extents +0x18).
pub(crate) const BS_BOUND_VTBL: u32 = 0x010c_2b64;
pub(crate) const BOUND_CENTRE: u32 = 0x0c;
pub(crate) const BOUND_DIMENSIONS: u32 = 0x18;
/// `TESObjectREFR::GetScale` `00567400`.
pub(crate) const GET_SCALE: u32 = 0x0056_7400;

/// The test's scene memory.
pub(crate) const AREA: u32 = 0x0500_0000;
const AREA_SIZE: u32 = 0x10_0000;
/// Real vtables: `NiNode`, `BSFadeNode`, `NiTriShape` (geometry), `NiCamera` (neither).
pub(crate) const NI_NODE: u32 = 0x0109_b5ac;
pub(crate) const BS_FADE_NODE: u32 = 0x010a_8f90;
pub(crate) const NI_TRI_SHAPE: u32 = 0x0109_d454;
pub(crate) const NI_CAMERA: u32 = 0x0109_cb9c;
/// `ADDR_ReturnNull`-like slot of every other class (`00ACBB70`).
const RETURN_NULL: u32 = 0x00ac_bb70;
/// Where a test reference's vtables are (a plain reference's and an actor's).
pub(crate) const REF_VTBL: u32 = AREA + 0xf_0000;
pub(crate) const ACTOR_VTBL: u32 = AREA + 0xf_0400;
/// Unmapped: a damaged pointer.
pub(crate) const BAD: u32 = 0x7ff0_0000;

/// An engine with the scene memory and the vtables' slots (as the exe has them) mapped.
pub(crate) fn scene_engine() -> Engine {
    let mut e = engine();
    e.map(AREA, AREA_SIZE);
    e.map(0x0109_b000, 0x4000);
    e.map(0x010a_8000, 0x1000);
    e.map(0x010c_2000, 0x1000);
    for vt in [NI_NODE, BS_FADE_NODE] {
        e.mem.set_u32(vt + 0xc, RETURN_THIS);
        e.mem.set_u32(vt + 0x18, RETURN_NULL);
    }
    e.mem.set_u32(NI_TRI_SHAPE + 0xc, RETURN_NULL);
    e.mem.set_u32(NI_TRI_SHAPE + 0x18, RETURN_THIS2);
    e.mem.set_u32(NI_CAMERA + 0xc, RETURN_NULL);
    e.mem.set_u32(NI_CAMERA + 0x18, RETURN_NULL);
    e.mem.set_u32(ACTOR_VTBL + 0xfc, RETURN_TRUE);
    e
}

/// The next free byte of [`AREA`] (kept at its last word).
fn alloc(e: &mut Engine, size: u32) -> u32 {
    let top = AREA + AREA_SIZE - 4;
    let mut at = e.mem.u32(top);
    if at == 0 {
        at = AREA + 0x1000;
    }
    e.mem.set_u32(top, at + size.next_multiple_of(16));
    at
}

/// A fixed string holding `text` (its 0 after it).
pub(crate) fn fixed(e: &mut Engine, text: &[u8]) -> u32 {
    let at = alloc(e, text.len() as u32 + 1);
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    at
}

/// An object of class `vtbl` named `name` (none: a null name), 0x100 bytes, zeroed.
pub(crate) fn object(e: &mut Engine, vtbl: u32, name: Option<&[u8]>) -> u32 {
    let at = alloc(e, 0x100);
    for i in 0..0x40 {
        e.mem.set_u32(at + 4 * i, 0);
    }
    e.mem.set_u32(at, vtbl);
    let n = match name {
        Some(t) => fixed(e, t),
        None => 0,
    };
    e.mem.set_u32(at + NAME, n);
    at
}

/// `node`'s children set to `slots` (0 an empty slot), the array's size their count.
pub(crate) fn set_children(e: &mut Engine, node: u32, slots: &[u32]) {
    let data = alloc(e, 4 * slots.len() as u32 + 4);
    for (i, &c) in slots.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, c);
        if c != 0 && c != BAD {
            e.mem.set_u32(c + PARENT, node);
        }
    }
    e.mem.set_u32(node + CHILDREN, data);
    e.mem.set_u16(node + CHILDREN_SIZE, slots.len() as u16);
}

/// A node of class `NiNode` named `name` with `slots` as its children.
pub(crate) fn node(e: &mut Engine, name: &[u8], slots: &[u32]) -> u32 {
    let n = object(e, NI_NODE, Some(name));
    set_children(e, n, slots);
    n
}

/// A geometry block (`NiTriShape`) named `name`.
pub(crate) fn shape(e: &mut Engine, name: &[u8]) -> u32 {
    object(e, NI_TRI_SHAPE, Some(name))
}

/// A reference (vtable [`REF_VTBL`], or [`ACTOR_VTBL`]) with id `id`, its loaded root `root` (0:
/// no loaded data).
pub(crate) fn reference(e: &mut Engine, id: u32, actor: bool, root: u32) -> u32 {
    let at = alloc(e, 0x700);
    for i in 0..0x1c0 {
        e.mem.set_u32(at + 4 * i, 0);
    }
    e.mem.set_u32(at, if actor { ACTOR_VTBL } else { REF_VTBL });
    e.mem.set_u8(at + 4, 0x3a);
    e.mem.set_u32(at + REF_ID, id);
    set_root(e, at, root);
    at
}

/// The reference's loaded data with root `root` (0: none).
pub(crate) fn set_root(e: &mut Engine, refr: u32, root: u32) {
    if root == 0 {
        e.mem.set_u32(refr + LOADED_DATA, 0);
    } else {
        let loaded = alloc(e, 0x20);
        e.mem.set_u32(loaded + ROOT_NODE, root);
        e.mem.set_u32(refr + LOADED_DATA, loaded);
    }
}

/// The player (id 0x14) with root `third`, first person node `first`, the camera flag.
pub(crate) fn player(e: &mut Engine, third: u32, first: u32, third_person: bool) -> u32 {
    let p = reference(e, PLAYER_ID, true, third);
    e.mem.set_u32(p + NODE_1ST_PERSON, first);
    e.mem.set_u8(p + IS_3RD_PERSON, u8::from(third_person));
    p
}

/// A `BSBound` (or, `vtbl` other, extra data of another class) with centre and half extents.
pub(crate) fn bound(e: &mut Engine, vtbl: u32, centre: [f32; 3], dims: [f32; 3]) -> u32 {
    let b = alloc(e, 0x24);
    e.mem.set_u32(b, vtbl);
    for i in 0..3 {
        e.mem.set_f32(b + BOUND_CENTRE + 4 * i, centre[i as usize]);
        e.mem
            .set_f32(b + BOUND_DIMENSIONS + 4 * i, dims[i as usize]);
    }
    b
}

/// The object's extra data array set to `slots`.
pub(crate) fn set_extra(e: &mut Engine, object: u32, slots: &[u32]) {
    let data = alloc(e, 4 * slots.len() as u32 + 4);
    for (i, &x) in slots.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, x);
    }
    e.mem.set_u32(object + EXTRA_LIST, data);
    e.mem.set_u16(object + EXTRA_COUNT, slots.len() as u16);
}

// ---- GetBlock ----

/// Depth first, each child compared before its own children, children in order: a deeper block
/// of the name comes before a later sibling of it.
#[test]
fn block_walk_order() {
    let mut e = scene_engine();
    let deep = shape(&mut e, b"T");
    let a = node(&mut e, b"A", &[deep]);
    let sibling = shape(&mut e, b"T");
    let root = node(&mut e, b"Root", &[a, sibling]);
    assert_eq!(get_block(&e, root, b"T"), deep);
    assert_eq!(get_block(&e, root, b"A"), a, "a node itself is a block");
    // A child's own name is compared before what is below it.
    let inner = shape(&mut e, b"B");
    let b = node(&mut e, b"B", &[inner]);
    let root2 = node(&mut e, b"R2", &[b]);
    assert_eq!(get_block(&e, root2, b"B"), b);
}

/// The root's own name finds the root; an empty name finds nothing; names compare exactly (case,
/// length).
#[test]
fn block_root_empty_exact() {
    let mut e = scene_engine();
    let s = shape(&mut e, b"Bip01 Head");
    let root = node(&mut e, b"Scene Root", &[s]);
    assert_eq!(get_block(&e, root, b"Scene Root"), root);
    assert_eq!(get_block(&e, root, b""), 0);
    assert_eq!(get_block(&e, root, b"Bip01 Head"), s);
    assert_eq!(get_block(&e, root, b"bip01 head"), 0, "case-sensitive");
    assert_eq!(get_block(&e, root, b"Bip01 Hea"), 0, "the whole name");
    assert_eq!(get_block(&e, root, b"Bip01 Head2"), 0);
}

/// Empty slots are skipped, slots past the array's size are not looked at, a block with no name
/// never matches, a geometry block's (or other non-node's) children fields are not walked.
#[test]
fn block_slots_and_classes() {
    let mut e = scene_engine();
    let hidden = shape(&mut e, b"Hidden");
    let unnamed = object(&mut e, NI_TRI_SHAPE, None);
    let s = shape(&mut e, b"S");
    let root = node(&mut e, b"Root", &[0, unnamed, s, hidden]);
    e.mem.set_u16(root + CHILDREN_SIZE, 3);
    assert_eq!(get_block(&e, root, b"S"), s);
    assert_eq!(get_block(&e, root, b"Hidden"), 0, "past the size");
    // A geometry block with a "children" array is not walked; a camera neither.
    let under = shape(&mut e, b"Under");
    let g = shape(&mut e, b"G");
    set_children(&mut e, g, &[under]);
    let cam = object(&mut e, NI_CAMERA, Some(b"Cam"));
    set_children(&mut e, cam, &[under]);
    let root2 = node(&mut e, b"R", &[g, cam]);
    assert_eq!(get_block(&e, root2, b"Under"), 0);
    // A BSFadeNode is a node.
    let fade = object(&mut e, BS_FADE_NODE, Some(b"Fade"));
    set_children(&mut e, fade, &[under]);
    let root3 = node(&mut e, b"R3", &[fade]);
    assert_eq!(get_block(&e, root3, b"Under"), under);
}

/// A damaged model: a child pointer into unmapped memory, a children array that is unmapped, a
/// name with no end, a cycle back to the root: nothing found, nothing stopped.
#[test]
fn block_damaged_models() {
    let mut e = scene_engine();
    let s = shape(&mut e, b"S");
    let root = node(&mut e, b"Root", &[BAD, s]);
    assert_eq!(get_block(&e, root, b"S"), s, "the bad slot is passed over");
    let broken = node(&mut e, b"Broken", &[]);
    e.mem.set_u32(broken + CHILDREN, BAD);
    e.mem.set_u16(broken + CHILDREN_SIZE, 5);
    let root2 = node(&mut e, b"R2", &[broken, s]);
    assert_eq!(get_block(&e, root2, b"S"), s);
    // A name running to the end of readable memory.
    let endless = object(&mut e, NI_TRI_SHAPE, None);
    let at = AREA + AREA_SIZE - 0x10;
    for i in 0..0x0c {
        e.mem.set_u8(at + i, b'x');
    }
    e.mem.set_u32(endless + NAME, at);
    let root3 = node(&mut e, b"R3", &[endless, s]);
    assert_eq!(get_block(&e, root3, b"xxxxxxxxxxxx"), 0);
    assert_eq!(get_block(&e, root3, b"S"), s);
    // A cycle: a child node holding the root.
    let loop_node = node(&mut e, b"Loop", &[]);
    let root4 = node(&mut e, b"R4", &[loop_node]);
    set_children(&mut e, loop_node, &[root4]);
    assert_eq!(get_block(&e, root4, b"Missing"), 0, "bounded");
    // A deep chain past the depth bound.
    let mut below = shape(&mut e, b"Bottom");
    for _ in 0..514 {
        below = node(&mut e, b"Link", &[below]);
    }
    assert_eq!(get_block(&e, below, b"Bottom"), 0, "below the depth bound");
}

/// A chain of `levels` nodes above a geometry block named `T`: the top node.
fn chain(e: &mut Engine, levels: u32) -> u32 {
    let mut below = shape(e, b"T");
    for _ in 0..levels {
        below = node(e, b"Link", &[below]);
    }
    below
}

/// The depth bound at its edge: a block 511 levels below the start (its children are level 1) is
/// found, one 512 levels below is not.
#[test]
fn block_depth_edge() {
    let mut e = scene_engine();
    let top = chain(&mut e, 511);
    assert_ne!(get_block(&e, top, b"T"), 0, "511 levels below");
    let mut e = scene_engine();
    let top = chain(&mut e, 512);
    assert_eq!(get_block(&e, top, b"T"), 0, "512 levels below");
}

/// `Root [A, Target]` with node A holding `fillers` geometry blocks: the walk reads A's slot,
/// A's `fillers` slots, then Target's. The root and Target.
fn wide(e: &mut Engine, fillers: usize) -> (u32, u32) {
    let filler = shape(e, b"F");
    let target = shape(e, b"Target");
    let a = node(e, b"A", &vec![filler; fillers]);
    (node(e, b"Root", &[a, target]), target)
}

/// The visit bound at its edge: Target as the 65,536th slot read is found, as the 65,537th not.
#[test]
fn block_visit_bound() {
    let mut e = scene_engine();
    let (root, target) = wide(&mut e, 65_534);
    assert_eq!(
        get_block(&e, root, b"Target"),
        target,
        "read 1 + 65,534 + 1"
    );
    let mut e = scene_engine();
    let (root, _) = wide(&mut e, 65_535);
    assert_eq!(get_block(&e, root, b"Target"), 0, "read 1 + 65,535 + 1");
}

/// Every slot read is charged to the walk, empty ones too: a damaged node whose count word is
/// 0xFFFFFFFF (size 0xFFFF) of empty slots spends the walk, so its later sibling is not found;
/// with one slot fewer it is. A slot near its end holding itself (a loop of 65,534 empty slots a
/// level) still ends.
#[test]
fn block_empty_slots_charged() {
    for (count, found) in [(0xffff_ffff_u32, false), (0xfffe_fffe, true)] {
        let mut e = scene_engine();
        let target = shape(&mut e, b"Target");
        let damaged = node(&mut e, b"Damaged", &[]);
        let data = alloc(&mut e, 4 * 0xffff);
        e.mem.write(data, &vec![0u8; 4 * 0xffff]);
        e.mem.set_u32(damaged + CHILDREN, data);
        // The array's capacity +0xA4 and size +0xA6 (the size is what the walks read).
        e.mem.set_u32(damaged + 0xa4, count);
        let root = node(&mut e, b"Root", &[damaged, target]);
        let want = if found { target } else { 0 };
        assert_eq!(get_block(&e, root, b"Target"), want, "count {count:#x}");
        e.mem.set_u32(data + 4 * 0xfffd, damaged);
        assert_eq!(get_block(&e, root, b"Missing"), 0);
        assert_eq!(get_block(&e, root, b"Target"), 0, "spent below the loop");
    }
}

/// `GetName` as JIP's array elements take it: the text `NULL` for a block without a name (JIP's
/// `NiFixedString::Get`); an empty text only for a name that cannot be read.
#[test]
fn name_texts() {
    let mut e = scene_engine();
    let named = shape(&mut e, b"Head");
    assert_eq!(name_text(&e, named), b"Head");
    let unnamed = object(&mut e, NI_TRI_SHAPE, None);
    assert_eq!(name_text(&e, unnamed), b"NULL");
    let bad = object(&mut e, NI_TRI_SHAPE, None);
    e.mem.set_u32(bad + NAME, BAD);
    assert_eq!(name_text(&e, bad), b"", "the text unreadable");
    assert_eq!(name_text(&e, BAD), b"", "the name pointer unreadable");
}

/// A script asking for `NULL`: JIP compares the text's fixed string (never 0 for a text) with each
/// block's name pointer, so a block without a name never matches; one really named `NULL` does.
#[test]
fn block_named_null() {
    let mut e = scene_engine();
    let unnamed = object(&mut e, NI_TRI_SHAPE, None);
    let root = node(&mut e, b"Root", &[unnamed]);
    assert_eq!(get_block(&e, root, b"NULL"), 0);
    let real = shape(&mut e, b"NULL");
    set_children(&mut e, root, &[unnamed, real]);
    assert_eq!(get_block(&e, root, b"NULL"), real);
    let start = object(&mut e, NI_NODE, None);
    set_children(&mut e, start, &[]);
    assert_eq!(get_block(&e, start, b"NULL"), 0, "the start without a name");
}

/// Reads near the top of the address space wrap instead of overflowing (nothing there is mapped).
#[test]
fn reads_near_the_top_wrap() {
    let e = scene_engine();
    assert_eq!(floats3(&e, 0xffff_fffc), None);
    assert_eq!(matrix(&e, 0xffff_fff0), None);
    assert_eq!(get_block(&e, 0xffff_fff0, b"S"), 0);
    assert_eq!(name_text(&e, 0xffff_fffc), b"");
    assert_eq!(ref_ni_node(&e, 0xffff_fff0), 0);
    assert_eq!(bounding_box(&e, 0xffff_fff0), 0);
}

/// `GetNode`: the block only when it is a node.
#[test]
fn node_only() {
    let mut e = scene_engine();
    let s = shape(&mut e, b"S");
    let n = node(&mut e, b"N", &[]);
    let root = node(&mut e, b"Root", &[s, n]);
    assert_eq!(get_node(&e, root, b"N"), n);
    assert_eq!(get_node(&e, root, b"S"), 0);
    assert_eq!(get_node(&e, root, b"Root"), root);
}

// ---- roots ----

/// `GetRefNiNode`, `GetNiNode`, `GetNiBlock`: the loaded root; the player's first person node
/// while the flag is clear, but nothing at all without loaded data.
#[test]
fn reference_roots() {
    let mut e = scene_engine();
    let third = node(&mut e, b"3rd", &[]);
    let first = node(&mut e, b"1st", &[]);
    let r = reference(&mut e, 0x1234, false, third);
    assert_eq!(ref_ni_node(&e, r), third);
    assert_eq!(ni_node(&e, r), third);
    let p = player(&mut e, third, first, false);
    assert_eq!(ni_node(&e, p), first);
    assert_eq!(ref_ni_node(&e, p), third);
    e.mem.set_u8(p + IS_3RD_PERSON, 2);
    assert_eq!(ni_node(&e, p), third, "any non-zero flag");
    e.mem.set_u8(p + IS_3RD_PERSON, 0);
    set_root(&mut e, p, 0);
    assert_eq!(ni_node(&e, p), 0, "no loaded data: none, first person too");
    assert_eq!(ref_ni_node(&e, p), 0);
    e.mem.set_u32(r + LOADED_DATA, BAD);
    assert_eq!(ni_node(&e, r), 0);
    assert_eq!(ref_ni_node(&e, r), 0);
    // GetNiBlock: the root for an empty name.
    let s = shape(&mut e, b"S");
    let root = node(&mut e, b"Root", &[s]);
    set_root(&mut e, r, root);
    assert_eq!(ni_block(&e, r, b""), root);
    assert_eq!(ni_block(&e, r, b"S"), s);
    set_root(&mut e, r, 0);
    assert_eq!(ni_block(&e, r, b""), 0);
}

/// JIP's `GetNifBlock`: the player's root by the argument's low byte (odd: third person, even
/// non-zero: first person, regardless of the camera); 0 or another reference: `GetNiNode`.
#[test]
fn nif_block_switch() {
    let mut e = scene_engine();
    let s3 = shape(&mut e, b"S");
    let s1 = shape(&mut e, b"S");
    let third = node(&mut e, b"3rd", &[s3]);
    let first = node(&mut e, b"1st", &[s1]);
    let p = player(&mut e, third, first, true);
    assert_eq!(nif_block(&e, p, 0, b"S"), s3, "0: the camera's (third)");
    assert_eq!(nif_block(&e, p, 2, b"S"), s1, "even: first person");
    e.mem.set_u8(p + IS_3RD_PERSON, 0);
    assert_eq!(nif_block(&e, p, 0, b"S"), s1, "0: the camera's (first)");
    assert_eq!(nif_block(&e, p, 1, b"S"), s3, "odd: third person");
    assert_eq!(nif_block(&e, p, 3, b""), third, "empty name: the root");
    assert_eq!(nif_block(&e, p, 0x101, b"S"), s3, "the low byte");
    assert_eq!(
        nif_block(&e, p, 0x100, b"S"),
        s1,
        "low byte 0: the camera's"
    );
    e.mem.set_u8(p + IS_3RD_PERSON, 1);
    assert_eq!(
        nif_block(&e, p, 0x100, b"S"),
        s3,
        "low byte 0: the camera's (third), not the even first person"
    );
    e.mem.set_u8(p + IS_3RD_PERSON, 0);
    set_root(&mut e, p, 0);
    assert_eq!(
        nif_block(&e, p, 2, b"S"),
        s1,
        "first person without loaded data"
    );
    assert_eq!(nif_block(&e, p, 1, b"S"), 0);
    let r = reference(&mut e, 0x1234, false, third);
    assert_eq!(nif_block(&e, r, 2, b"S"), s3, "not the player: its root");
}

// ---- extra data, bounds ----

/// From the end of the array, empty slots passed over, the first of the class found.
#[test]
fn extra_data_by_class() {
    let mut e = scene_engine();
    let n = node(&mut e, b"N", &[]);
    let b1 = bound(&mut e, BS_BOUND_VTBL, [1.0; 3], [1.0; 3]);
    let b2 = bound(&mut e, BS_BOUND_VTBL, [2.0; 3], [2.0; 3]);
    let other = bound(&mut e, 0x0109_da1c, [3.0; 3], [3.0; 3]);
    set_extra(&mut e, n, &[b1, b2, other, 0]);
    assert_eq!(extra_data_of_class(&e, n, BS_BOUND_VTBL), b2);
    set_extra(&mut e, n, &[other]);
    assert_eq!(extra_data_of_class(&e, n, BS_BOUND_VTBL), 0);
    set_extra(&mut e, n, &[]);
    assert_eq!(extra_data_of_class(&e, n, BS_BOUND_VTBL), 0);
    e.mem.set_u32(n + EXTRA_LIST, BAD);
    e.mem.set_u16(n + EXTRA_COUNT, 3);
    assert_eq!(extra_data_of_class(&e, n, BS_BOUND_VTBL), 0);
}

/// `GetBoundingBox`: an actor's high or middle-high process' bound (even none); a low process,
/// none, or a non-actor: the loaded root's `BSBound`.
#[test]
fn bounding_boxes() {
    let mut e = scene_engine();
    let root = node(&mut e, b"Root", &[]);
    let node_bound = bound(&mut e, BS_BOUND_VTBL, [0.0; 3], [1.0; 3]);
    set_extra(&mut e, root, &[node_bound]);
    let r = reference(&mut e, 0x1234, false, root);
    assert_eq!(bounding_box(&e, r), node_bound);
    let a = reference(&mut e, 0x1235, true, root);
    assert_eq!(bounding_box(&e, a), node_bound, "no process");
    let process = alloc(&mut e, 0x300);
    let process_bound = bound(&mut e, BS_BOUND_VTBL, [5.0; 3], [5.0; 3]);
    e.mem.set_u32(process + PROCESS_BOUND, process_bound);
    e.mem.set_u32(a + PROCESS, process);
    for (level, want) in [(0, process_bound), (1, process_bound), (2, node_bound)] {
        e.mem.set_u32(process + PROCESS_LEVEL, level);
        assert_eq!(bounding_box(&e, a), want, "level {level}");
    }
    e.mem.set_u32(process + PROCESS_LEVEL, 0x100);
    assert_eq!(bounding_box(&e, a), process_bound, "the level's byte");
    e.mem.set_u32(process + PROCESS_LEVEL, 1);
    e.mem.set_u32(process + PROCESS_BOUND, 0);
    assert_eq!(bounding_box(&e, a), 0, "the process has none: none");
    e.mem.set_u32(r, ACTOR_VTBL - 0x400 + 0x200);
    assert_eq!(
        bounding_box(&e, r),
        node_bound,
        "a vtable answering nothing"
    );
    set_root(&mut e, r, 0);
    assert_eq!(bounding_box(&e, r), 0);
}

// ---- angles ----

/// `ToEulerPRY` / `ToEulerPRYInv` read the matrix's own entries: identity is 0; a yaw of a
/// quarter turn by rows (`m1 = 1`) is pi/2 in the third lane of `ToEulerPRY`, and by columns
/// (`m3 = 1`) in that of `ToEulerPRYInv`.
#[test]
fn euler_angles() {
    let id = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(to_euler_pry(&id), [0.0, -0.0, 0.0]);
    assert_eq!(to_euler_pry_inv(&id), [0.0, -0.0, 0.0]);
    let rows = [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let h = std::f32::consts::FRAC_PI_2;
    assert_eq!(
        to_euler_pry(&rows)[2],
        super::super::ln_fn_math::atan2(1.0, 0.0)
    );
    assert!((to_euler_pry(&rows)[2] - h).abs() < 1e-6);
    assert!((to_euler_pry_inv(&rows)[2] + h).abs() < 1e-6);
    // Each lane's entries: m5/m8 and m7/m8; -m2 and -m6.
    let m = [0.5, 0.0, 0.25, 0.0, 1.0, 0.75, -0.5, 0.125, 2.0];
    use super::super::ln_fn_math::{asin, atan2};
    assert_eq!(
        to_euler_pry(&m),
        [atan2(0.75, 2.0), asin(-0.25), atan2(0.0, 0.5)]
    );
    assert_eq!(
        to_euler_pry_inv(&m),
        [atan2(0.125, 2.0), asin(0.5), atan2(0.0, 0.5)]
    );
    assert_eq!(FLT_180_D_PI.to_bits(), 57.295_78_f32.to_bits());
}
