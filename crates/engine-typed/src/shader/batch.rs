//! The batch renderer (`BSBatchRenderer`, Xbox PDB): passes are filed by pass type into groups of five
//! lists, the groups are drawn in the order of the active pass type list, and each pass is drawn by
//! beginning the shader's pass when the type or shader changes, then the geometry (plain or skinned,
//! partition by partition). The alpha lists are sorted back to front by the camera's direction.
//!
//! **Floats.** The alpha sort's comparison ([`compare_by_depth`]) runs on the main thread only (the
//! thread census), so its depths are plain `f32` in the exe's order (the family's float rule:
//! [`super`]): the exe's own rounding at the main thread's 24-bit precision. The other float reads here
//! are comparisons.

use super::layout::{
    ActivePass, BSBatchRenderer, NiGeometry, NiGeometryData, NiProperty, NiSkinInstance,
    NiSkinPartition, PassGroup, PassHolder, PassList, PassNode, RenderPass,
};
use super::state::{self, DEVICE_SET_SAMPLER_STATE};
use super::{accumulator, elem_addr, geometry, low8, textures, FAMILY};
use crate::culling::layout::NiBound;
use crate::draw::capture;
use engine_core::abi::OneWord;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// A list word through the drawing thread's view (`draw::detach`, step 1): guest memory while it is closed.
#[inline(always)]
fn vget<S>(e: &Engine, p: Ptr<S>, f: engine_core::ptr::Field<S, u32>) -> u32 {
    crate::draw::detach::word(e, p.addr().wrapping_add(f.off))
}

/// Writes a list word through the view ([`vget`]).
#[inline(always)]
fn vset<S>(e: &mut Engine, p: Ptr<S>, f: engine_core::ptr::Field<S, u32>, v: u32) {
    crate::draw::detach::set_word(e, p.addr().wrapping_add(f.off), v);
}

/// A 16-bit list field through the view ([`vget`]).
#[inline(always)]
fn vget16<S>(e: &Engine, p: Ptr<S>, f: engine_core::ptr::Field<S, u16>) -> u16 {
    crate::draw::detach::half(e, p.addr().wrapping_add(f.off))
}

/// Writes a 16-bit list field through the view ([`vget`]).
#[inline(always)]
fn vset16<S>(e: &mut Engine, p: Ptr<S>, f: engine_core::ptr::Field<S, u16>, v: u16) {
    crate::draw::detach::set_half(e, p.addr().wrapping_add(f.off), v);
}

/// An 8-bit list field through the view ([`vget`]).
#[inline(always)]
fn vget8<S>(e: &Engine, p: Ptr<S>, f: engine_core::ptr::Field<S, u8>) -> u8 {
    crate::draw::detach::byte(e, p.addr().wrapping_add(f.off))
}

/// The pass and type being drawn (`011f91e0`, `011f91e4`).
pub(crate) const RENDERING_PASS: u32 = 0x011f_91e0;
pub(crate) const RENDERING_PASS_TYPE: u32 = 0x011f_91e4;
/// The pass type and shader whose pass is begun (`011ffe30`, `011ffe2c`), and the byte that says the
/// shader's per-geometry restore (slot 0x110) is still owed (`011ad8ec`).
const CURRENT_PASS_TYPE: u32 = 0x011f_fe30;
const CURRENT_SHADER: u32 = 0x011f_fe2c;
const RESTORE_OWED: u32 = 0x011a_d8ec;
/// The renderer as the batch renderer holds it (`011f4748`).
const RENDER_HOLDER: u32 = 0x011f_4748;
/// Whether the card takes alpha to coverage (`011f9421`).
const COVERAGE_AVAILABLE: u32 = 0x011f_9421;
/// The two lock counters the group drawing releases (`011ffa24`, `011ff9e8`).
const GROUP_COUNTERS: [u32; 2] = [0x011f_fa24, 0x011f_f9e8];
/// The marker word `00b99390` resets (`011ae63c`) and the sixteen words of stage textures (`0126f680`).
const PASS_MARKER: u32 = 0x011a_e63c;
const STAGE_TEXTURES: u32 = 0x0126_f680;
/// The alpha sort's settings (`011ffe35`: subtract the radius; `011ffe38`: the camera).
const SORT_SUBTRACT_RADIUS: u32 = 0x011f_fe35;
const SORT_CAMERA: u32 = 0x011f_fe38;
/// `NiAVObject`'s empty bound (`011f4288`).
const NULL_BOUND: u32 = 0x011f_4288;
/// `BSSkinInstance`'s type (`011f49d8`), what the skin's slot 8 answers for a dismember skin.
pub(crate) const SKIN_RTTI: u32 = 0x011f_49d8;

/// Callees by address: the map's `GetAt` (`thiscall(map, key, out)`) and `SetAt`, the active list's
/// clear, the memory manager and `operator new`, the batch renderer's constructor, the shadow-pass
/// test, the shadow light shader's per-geometry set-ups (the draw family's, compared by address), the
/// renderer's skinned test, the geometry holder's draw, a shader's pass tail of kind 0x11, the node
/// pool lock. The node pool's allocation is draw_f's (`0043a010`).
const MAP_GET_AT: u32 = 0x0085_3130;
const MAP_SET_AT: u32 = 0x0084_4700;
const ACTIVE_LIST_CLEAR: u32 = 0x00b6_4ec0;
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_FREE: u32 = 0x00aa_4060;
const OPERATOR_NEW: u32 = 0x00aa_13e0;
const RENDERER_CONSTRUCT: u32 = 0x00b9_a5b0;
const ACTIVE_INSERT_FIRST: u32 = 0x00b9_8e20;
const IS_SHADOW_PASS: u32 = 0x00b6_30f0;
const SETUP_TYPES_FC_FD: u32 = 0x00b7_dde0;
const SETUP_TYPE_CA: u32 = 0x00b7_ded0;
const SETUP_TYPE_D1: u32 = 0x00b7_dfe0;
const SETUP_TYPE_102: u32 = 0x00b7_e150;
const TAIL_KIND_11: u32 = 0x00bb_96d0;
const LOCK_ENTER: u32 = 0x0040_fbf0;
/// The global node pool: its lock (owner `011c5f80`, count `011c5f84`) and free list (`011c5f58`).
pub(crate) const POOL_LOCK: u32 = 0x011c_5f80;
const POOL_LOCK_COUNT: u32 = 0x011c_5f84;
const POOL_FREE: u32 = 0x011c_5f58;
/// The pass-type comparison the active list is kept in order with (`00b988a0`).
const PASS_TYPE_COMPARE: u32 = 0x00b9_88a0;
/// The alpha sort's comparison (this file's [`compare_by_depth`]).
const COMPARE_BY_DEPTH: u32 = 0x00b9_8b20;

/// The shader slots the drawing calls: begin a pass (0xf0 the pass choice, 0xfc its constants, 0xf8
/// the pair textures, 0x100 its render states, 0xf4 the begin's end), end it (0x104 the render states
/// put back, 0x114 the end's states put back), per geometry (0x78, 0x7c, 0x80 blending, 0x84 the alpha
/// function, 0x88 the alpha test, 0x6c, 0x8c, 0x110 the end's states, 0x64).
mod slot {
    pub const BEGIN: u32 = 0xf0;
    pub const BEGIN_CONSTANTS: u32 = 0xfc;
    pub const BEGIN_TEXTURES: u32 = 0xf8;
    pub const BEGIN_STATES: u32 = 0x100;
    pub const BEGIN_DONE: u32 = 0xf4;
    pub const END: u32 = 0x104;
    pub const END_RESTORE: u32 = 0x114;
    pub const GEOMETRY_A: u32 = 0x78;
    pub const GEOMETRY_B: u32 = 0x7c;
    pub const SECOND: u32 = 0x80;
    pub const FIRST: u32 = 0x84;
    pub const THIRD: u32 = 0x88;
    pub const DRAW: u32 = 0x6c;
    pub const GEOMETRY_DONE: u32 = 0x8c;
    pub const RESTORE: u32 = 0x110;
    pub const PARTITION: u32 = 0x64;
    /// The geometry's own: render (0xdc), draws itself (0x30, 0x34), its immediate draw with the
    /// renderer (0xe0, `OnlyRenderImmediate`).
    pub const OWN_RENDER: u32 = 0xdc;
    pub const OWN_A: u32 = 0x30;
    pub const OWN_B: u32 = 0x34;
    pub const RENDER_IMMEDIATE: u32 = 0xe0;
    /// A batch renderer's `AddPass` ([`super::add_to_group`]) and a map's hash.
    pub const ADD_PASS: u32 = 0x8c;
    pub const MAP_HASH: u32 = 4;
    /// A skin instance's type.
    pub const SKIN_TYPE: u32 = 8;
}

/// List `index` of group `group`.
#[inline(always)]
fn group_list(e: &Engine, this: Ptr<BSBatchRenderer>, group: u32, index: u32) -> Ptr<PassList> {
    let groups = e.get(this, BSBatchRenderer::pRenderPassA);
    Ptr::new(
        group
            .wrapping_mul(0x68)
            .wrapping_add(index.wrapping_mul(0x14))
            .wrapping_add(groups),
    )
}

/// The map's value for `key` (`GetAt` into a stack word).
#[inline(always)]
fn map_get(e: &mut Engine, this: Ptr<BSBatchRenderer>, key: u32) -> u32 {
    e.with_stack(4, |e, slot| {
        e.call(MAP_GET_AT, &args![this.addr().wrapping_add(0xc), key, slot]);
        e.global::<u32>(slot.addr())
    })
}

// Typed from 00b98ae0
/// `thiscall(renderer, group, out)`: the first of the group's five lists with a head; its index goes
/// to `out`. False when none has one.
pub fn first_list(e: &mut Engine, this: Ptr<BSBatchRenderer>, group: u32, out: u32) -> bool {
    let groups = e.get(this, BSBatchRenderer::pRenderPassA);
    for index in 0..5u32 {
        let list = group
            .wrapping_mul(0x68)
            .wrapping_add(index * 0x14)
            .wrapping_add(groups);
        if crate::draw::detach::word(e, list.wrapping_add(4)) != 0 {
            e.set_global(out, index);
            return true;
        }
    }
    false
}

// Typed from 00b99730
/// `thiscall(active list)`: drops the head entry: the second entry's type and link move into the head
/// (or the head's type clears when there is none) and the second is freed.
pub fn pop_active(e: &mut Engine, this: Ptr<ActivePass>) {
    let second = Ptr::<ActivePass>::new(vget(e, this, ActivePass::pNext));
    if second.is_null() {
        vset16(e, this, ActivePass::usItem, 0);
        return;
    }
    let next = vget(e, second, ActivePass::pNext);
    vset(e, this, ActivePass::pNext, next);
    let item = vget16(e, second, ActivePass::usItem);
    vset16(e, this, ActivePass::usItem, item);
    vset(e, second, ActivePass::pNext, 0);
    e.call(ACTIVE_LIST_CLEAR, &args![second]);
    e.call(MEMORY_FREE, &args![MEMORY_MANAGER, second]);
}

// Typed from 00b99900
/// `thiscall(renderer, type out, list out)`: takes the next active pass type at or above the first
/// one drawn (+0x20) into `pass_type`; false when there is none or it is above the last (+0x24).
/// Otherwise the entry is dropped and the answer is whether its group has a list with passes.
pub fn next_active(e: &mut Engine, this: Ptr<BSBatchRenderer>, pass_type: u32, out: u32) -> bool {
    let head = this.at(BSBatchRenderer::kActivePassIndexList);
    let mut node = head;
    if vget(e, node, ActivePass::pNext) == 0 && vget16(e, node, ActivePass::usItem) == 0 {
        return false;
    }
    let mut value = u32::from(vget16(e, node, ActivePass::usItem));
    e.set_global(pass_type, value);
    while value < e.get(this, BSBatchRenderer::uiFirstPass) {
        node = Ptr::new(vget(e, node, ActivePass::pNext));
        if node.is_null() {
            return false;
        }
        value = u32::from(vget16(e, node, ActivePass::usItem));
        e.set_global(pass_type, value);
    }
    if value > e.get(this, BSBatchRenderer::uiLastPass) {
        return false;
    }
    typed_call!(FAMILY, 0x00b9_9730, pop_active(e, head));
    let key = e.global::<u32>(pass_type);
    let group = map_get(e, this, key);
    typed_call!(FAMILY, 0x00b9_8ae0, first_list(e, this, group, out))
}

// Typed from 00b9a090
/// `thiscall(renderer, type, list out, flag out)`: makes sure a list is ready to draw: the current
/// type's group when it still has a list with passes, else the next active type's. The sort byte of
/// the list's first pass goes to `flag`. False when nothing is left.
pub fn next_group(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass_type: u32,
    element: u32,
    flag: u32,
) -> bool {
    e.with_stack(4, |e, slot| {
        let mut answer = true;
        let key = e.global::<u32>(pass_type);
        let mut ready = false;
        if key != 0 {
            e.call(MAP_GET_AT, &args![this.addr().wrapping_add(0xc), key, slot]);
            let group = e.global::<u32>(slot.addr());
            ready = typed_call!(FAMILY, 0x00b9_8ae0, first_list(e, this, group, element));
        }
        if !ready {
            answer = typed_call!(
                FAMILY,
                0x00b9_9900,
                next_active(e, this, pass_type, element)
            );
            if !answer {
                return false;
            }
        }
        let key = e.global::<u32>(pass_type);
        e.call(MAP_GET_AT, &args![this.addr().wrapping_add(0xc), key, slot]);
        let index = e.global::<u32>(element);
        let group = e.global::<u32>(slot.addr());
        let list = group_list(e, this, group, index);
        let node = Ptr::<PassNode>::new(vget(e, list, PassList::pHead));
        let first = Ptr::<RenderPass>::new(e.get(node, PassNode::pItem));
        let byte = e.get(first, RenderPass::ucSortFlag);
        e.set_global(flag, byte);
        answer
    })
}

// Typed from 00b9a120
/// `thiscall(renderer, type, list, flag out, second)`: draws the run of passes at the head of the
/// current list that share the first one's sort byte, removing each from the list (its node to the
/// list's spare chain). The list index picks the cull mode, the alpha test and alpha to coverage for
/// the run (lists 2 to 4 blend). A pass with another sort byte stops the run with its byte in `flag`
/// (true); else the next group is made ready ([`next_group`]).
pub fn render_group(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass_type: u32,
    element: u32,
    flag: u32,
    second: u32,
) -> bool {
    let value = map_get(e, this, e.global::<u32>(pass_type));
    let index = e.global::<u32>(element);
    let list = group_list(e, this, value, index);
    let head = Ptr::<PassNode>::new(vget(e, list, PassList::pHead));
    let first = Ptr::<RenderPass>::new(e.get(head, PassNode::pItem));
    let first_flag = e.get(first, RenderPass::ucSortFlag);
    let mut blend = 0u8;
    // Lists 0 and 1: no alpha test, no coverage.
    for (element_index, mode) in [(0u32, 1u32), (1, 3)] {
        if e.global::<u32>(element) == element_index {
            typed_call!(
                FAMILY,
                0x00b9_84d0,
                state::set_cull_mode_of_renderer(e, mode, element_index)
            );
            typed_call!(
                FAMILY,
                0x00b9_7ed0,
                state::set_alpha_test_enable(e, 0u8, 1u32)
            );
            typed_call!(
                FAMILY,
                0x00b9_8540,
                state::set_alpha_to_coverage(e, 0u8, 1u32)
            );
        }
    }
    for (element_index, mode, coverage) in [(2u32, 3u32, 1u8), (3, 1, 1), (4, 1, 0)] {
        if e.global::<u32>(element) == element_index {
            typed_call!(
                FAMILY,
                0x00b9_84d0,
                state::set_cull_mode_of_renderer(e, mode, 1u32)
            );
            typed_call!(
                FAMILY,
                0x00b9_7ed0,
                state::set_alpha_test_enable(e, 1u8, 1u32)
            );
            blend = 1;
            if e.global::<u8>(COVERAGE_AVAILABLE) != 0 {
                typed_call!(
                    FAMILY,
                    0x00b9_8540,
                    state::set_alpha_to_coverage(e, coverage, 1u32)
                );
            }
        }
    }
    // The exe keeps the blend byte in the low byte of the map value's stack word and passes the word.
    let blend_word = (value & 0xffff_ff00) | u32::from(blend);
    let group = || value.wrapping_mul(0x68);
    let mut stopped = false;
    let list = group_list(e, this, value, e.global::<u32>(element));
    let mut pass = Ptr::<RenderPass>::new(e.get(
        Ptr::<PassNode>::new(vget(e, list, PassList::pHead)),
        PassNode::pItem,
    ));
    if !pass.is_null() {
        loop {
            if e.get(pass, RenderPass::ucSortFlag) != first_flag {
                let byte = e.get(pass, RenderPass::ucSortFlag);
                e.set_global(flag, byte);
                stopped = true;
                break;
            }
            let key = e.global::<u32>(pass_type);
            typed_call!(
                FAMILY,
                0x00b9_94f0,
                render_pass(e, pass, key, blend_word, second, 0u32)
            );
            let index = e.global::<u32>(element);
            let group_at = e
                .get(this, BSBatchRenderer::pRenderPassA)
                .wrapping_add(group());
            let list = Ptr::<PassList>::new(group_at.wrapping_add(index.wrapping_mul(0x14)));
            let node = Ptr::<PassNode>::new(vget(e, list, PassList::pHead));
            if !node.is_null() {
                if node.addr() == vget(e, list, PassList::pTail) {
                    vset(e, list, PassList::pTail, 0);
                }
                let mut next = 0;
                if e.get(node, PassNode::pNext) != 0 {
                    next = e.get(node, PassNode::pNext);
                    e.set(Ptr::<PassNode>::new(next), PassNode::pPrev, 0);
                }
                let spare = e.get(list, PassList::pSpare);
                if spare != 0 {
                    let head = vget(e, list, PassList::pHead);
                    e.set(Ptr::<PassNode>::new(spare), PassNode::pPrev, head);
                }
                let head = Ptr::<PassNode>::new(vget(e, list, PassList::pHead));
                let spare = e.get(list, PassList::pSpare);
                e.set(head, PassNode::pNext, spare);
                let head = vget(e, list, PassList::pHead);
                e.set(list, PassList::pSpare, head);
                let count = vget(e, list, PassList::uiCount).wrapping_sub(1);
                vset(e, list, PassList::uiCount, count);
                vset(e, list, PassList::pHead, next);
            }
            let group_ptr = Ptr::<PassGroup>::new(group_at);
            let total = vget(e, group_ptr, PassGroup::uiCount).wrapping_sub(1);
            vset(e, group_ptr, PassGroup::uiCount, total);
            let list = group_list(e, this, value, e.global::<u32>(element));
            let head = Ptr::<PassNode>::new(vget(e, list, PassList::pHead));
            if head.is_null() {
                break;
            }
            pass = Ptr::new(e.get(head, PassNode::pItem));
            if pass.is_null() {
                break;
            }
        }
    }
    for counter in GROUP_COUNTERS {
        // own: read and written below through own_at/set_own_at
        let count = capture::own_at::<u32>(e, counter);
        if count != 0 {
            capture::set_own_at(e, counter, count - 1);
        }
    }
    if e.global::<u8>(COVERAGE_AVAILABLE) != 0 {
        let count = capture::own_at::<u32>(e, state::COVERAGE_COUNTER).wrapping_sub(1);
        capture::set_own_at(e, state::COVERAGE_COUNTER, count);
        typed_call!(
            FAMILY,
            0x00b9_8540,
            state::set_alpha_to_coverage(e, 0u8, 0u32)
        );
    }
    typed_call!(
        FAMILY,
        0x00b9_8540,
        state::set_alpha_to_coverage(e, 0u8, 0u32)
    );
    if stopped {
        return true;
    }
    typed_call!(
        FAMILY,
        0x00b9_a090,
        next_group(e, this, pass_type, element, flag)
    )
}

// Typed from 00b994f0
/// `BSBatchRenderer::RenderPassImmediately` (engine map), `cdecl(pass, type, first, second, third)`:
/// only for a pass of that type. Marks it the one being drawn, begins the shader's pass when the type
/// or shader changed (or for type 0x2f6), sets alpha to coverage from its alpha property where the card
/// has it, then draws it: skinned, by its own render (when it draws itself), or plain. A pass begun here
/// ends with the shader's tail (types 1 to 5 and 0x11).
pub fn render_pass(
    e: &mut Engine,
    pass: Ptr<RenderPass>,
    pass_type: u32,
    first: u32,
    second: u32,
    third: u32,
) {
    if u32::from(e.get(pass, RenderPass::usPassEnum)) != pass_type {
        return;
    }
    capture::set_own_at(e, RENDERING_PASS_TYPE, pass_type);
    capture::set_own_at(e, RENDERING_PASS, pass.addr());
    let geometry = e.get(pass, RenderPass::pGeometry);
    let shader = e.get(geometry, NiGeometry::m_spShader);
    let mut begun = false;
    if capture::own_at::<u32>(e, CURRENT_PASS_TYPE) != pass_type
        || pass_type == 0x2f6
        || shader != capture::own_at::<u32>(e, CURRENT_SHADER)
    {
        begun = true;
        typed_call!(FAMILY, 0x00b9_9390, begin_pass(e, pass_type, shader));
    }
    if e.global::<u8>(COVERAGE_AVAILABLE) != 0 {
        let geometry = e.get(pass, RenderPass::pGeometry);
        let alpha = Ptr::<NiProperty>::new(e.global::<u32>(elem_addr(
            geometry,
            NiGeometry::m_PropertyState,
            0,
            4,
        )));
        let on = !alpha.is_null() && (e.get(alpha, NiProperty::m_usFlags) >> 9) & 1 != 0;
        typed_call!(
            FAMILY,
            0x00b9_8540,
            state::set_alpha_to_coverage(e, u8::from(on), 0u32)
        );
    }
    let renderer = e.global::<u32>(state::RENDERER);
    let geometry = e.get(pass, RenderPass::pGeometry);
    let skinned: bool = typed_call!(
        crate::render::FAMILY,
        0x00e7_2c20,
        crate::render::draw::is_skinned(e, Ptr::new(renderer), Ptr::new(geometry.addr()), 0u32)
    );
    if skinned {
        typed_call!(
            FAMILY,
            0x00b9_9110,
            render_skinned(e, pass, first, second, third)
        );
    } else {
        let geometry = e.get(pass, RenderPass::pGeometry);
        let own = geometry_word(e, geometry.addr(), slot::OWN_A) != 0 || {
            let geometry = e.get(pass, RenderPass::pGeometry);
            geometry_word(e, geometry.addr(), slot::OWN_B) != 0
        };
        if own {
            let geometry = e.get(pass, RenderPass::pGeometry);
            let holder = e.global::<u32>(RENDER_HOLDER);
            e.vcall(geometry.addr(), slot::OWN_RENDER, &args![holder]);
        } else {
            typed_call!(
                FAMILY,
                0x00b9_8e80,
                render_standard(e, pass, first, second, third)
            );
        }
    }
    if begun {
        let shader = capture::own_at::<u32>(e, CURRENT_SHADER);
        if shader != 0 {
            #[allow(clippy::cast_possible_wrap)]
            let kind = e.global::<u32>(shader.wrapping_add(0x68)) as i32;
            if (1..=5).contains(&kind) {
                typed_call!(
                    crate::draw::FAMILY,
                    0x00b7_ac20,
                    crate::draw::setups::tail_kinds_1_to_5(e, Ptr::new(shader), pass_type)
                );
            } else if kind == 0x11 {
                e.call(TAIL_KIND_11, &args![shader, pass_type]);
            }
        }
    }
}

/// `00e88dc0` with the renderer's group manager and group, after the renderer's current property
/// state (+0xc) is set to the geometry's and its effects (+0x10) cleared. Answers the renderer.
fn add_to_geometry_group(
    e: &mut Engine,
    geometry: Ptr<NiGeometry>,
    data: Ptr<NiGeometryData>,
    skin: Ptr<NiSkinInstance>,
    partitions: u8,
) -> u32 {
    use super::layout::NiDX9Renderer as R;
    let holder = Ptr::<R>::new(e.global::<u32>(RENDER_HOLDER));
    let group = e.get(holder, R::pGeometryGroup);
    let manager = e.get(holder, R::pGeometryGroupManager);
    e.set(
        holder,
        R::m_pkCurrProp,
        geometry
            .addr()
            .wrapping_add(NiGeometry::m_PropertyState.off),
    );
    e.set(holder, R::m_pkCurrEffects, 0);
    typed_call!(
        FAMILY,
        0x00e8_8dc0,
        state::add_object_to_group(e, manager, group, data, skin, partitions, 0u32, 0u32)
    );
    holder.addr()
}

// Typed from 00b98e80
/// `cdecl(pass, first, second, third)`: draws a plain geometry: into the geometry group, then the
/// shader's per-geometry set-up (four types have their own set-up functions; the others the slots
/// 0x78 and 0x7c, the first and second slots unless the pass is a shadow pass, and the owed restore),
/// the draw (slot 0x6c), the geometry's immediate draw with the renderer (slot 0xe0, the draw
/// family's or the dispatch family's directly when the slot holds theirs), and the shader's geometry
/// end (slot 0x8c). The flags are the low bytes of their words.
pub fn render_standard(e: &mut Engine, pass: Ptr<RenderPass>, first: u32, second: u32, third: u32) {
    let (first, second, third) = (low8(first) != 0, low8(second) != 0, low8(third) != 0);
    let pass_type = u32::from(e.get(pass, RenderPass::usPassEnum));
    let geometry = e.get(pass, RenderPass::pGeometry);
    let data = e.get(geometry, NiGeometry::m_spModelData);
    let shader = e.get(geometry, NiGeometry::m_spShader);
    let state = geometry
        .addr()
        .wrapping_add(NiGeometry::m_PropertyState.off);
    let holder = add_to_geometry_group(e, geometry, data, Ptr::new(0), 0);
    let buffer = e.get(data, NiGeometryData::m_pkBuffData);
    let call_third = |e: &mut Engine| {
        if third {
            let flag = u32::from(e.get(pass, RenderPass::ucSortFlag));
            shader_alpha_test(e, shader, state, flag);
        }
    };
    let first_then_second = |e: &mut Engine| {
        if first {
            shader_alpha_function(e, shader, state);
        }
        if second {
            shader_blend(e, shader, state);
        }
    };
    let own = match pass_type {
        0xfc | 0xfd => Some((SETUP_TYPES_FC_FD, false)),
        0x102 => Some((SETUP_TYPE_102, false)),
        0xca => Some((SETUP_TYPE_CA, true)),
        0xd1 => Some((SETUP_TYPE_D1, true)),
        _ => None,
    };
    if let Some((setup, then_third)) = own {
        if setup == SETUP_TYPES_FC_FD {
            typed_call!(
                crate::render::FAMILY,
                0x00b7_dde0,
                crate::render::pass::geometry_pass(e, Ptr::new(shader), Ptr::new(state))
            );
        } else if setup == SETUP_TYPE_CA {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_ded0,
                crate::draw::setups::setup_type_ca(e, Ptr::new(shader), Ptr::new(state))
            );
        } else if setup == SETUP_TYPE_D1 {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_dfe0,
                crate::draw::setups::setup_type_d1(e, Ptr::new(shader), Ptr::new(state))
            );
        } else {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_e150,
                crate::draw::setups::setup_type_102(e, Ptr::new(shader), Ptr::new(state))
            );
        }
        first_then_second(e);
        if then_third {
            call_third(e);
        }
    } else {
        shader_geometry_setups(e, shader, state);
        if e.get(pass, RenderPass::ucSortFlag) != 0 {
            if second && e.call(IS_SHADOW_PASS, &args![pass_type]).u8() == 0 {
                shader_blend(e, shader, state);
            }
            if first
                && !(4..=5).contains(&pass_type)
                && !(0xe..=0xf).contains(&pass_type)
                && pass_type != 0x23a
            {
                shader_alpha_function(e, shader, state);
            }
        } else if e.global::<u8>(RESTORE_OWED) != 0
            && e.call(IS_SHADOW_PASS, &args![pass_type]).u8() == 0
        {
            shader_restore(e, shader, state);
            e.set_global(RESTORE_OWED, 0u8);
        }
        call_third(e);
    }
    shader_draw(e, shader, geometry, 0u32, buffer, state);
    render_immediate(e, geometry.addr(), holder);
    shader_geometry_done(e, shader, state);
}

// Typed from 00b99110
/// `cdecl(pass, first, second, third)`: draws a skinned geometry partition by partition (only the
/// partitions a dismember skin marks drawn): each starts with the shader's partition set-up (slot 0x64,
/// with the world bound), the first one also with the per-geometry set-up; each is drawn (slot 0x6c)
/// and handed to the renderer's partition draw (`00e6d310`, the draw family's), and a run of drawn
/// partitions that don't start a run is drawn with it. A draw that answers 0 inside a run ends the
/// whole call. The model data's dirty flags clear after, and the shader's geometry end runs.
pub fn render_skinned(e: &mut Engine, pass: Ptr<RenderPass>, first: u32, second: u32, third: u32) {
    let (first, second, third) = (low8(first) != 0, low8(second) != 0, low8(third) != 0);
    let pass_type = u32::from(e.get(pass, RenderPass::usPassEnum));
    let geometry = e.get(pass, RenderPass::pGeometry);
    let skin = e.get(geometry, NiGeometry::m_spSkinInstance);
    let data = e.get(geometry, NiGeometry::m_spModelData);
    let partition_data = e.get(skin, NiSkinInstance::m_spSkinPartition);
    let shader = e.get(geometry, NiGeometry::m_spShader);
    let state = geometry
        .addr()
        .wrapping_add(NiGeometry::m_PropertyState.off);
    let partitions = e.get(partition_data, NiSkinPartition::m_pkPartitions);
    let holder = add_to_geometry_group(e, geometry, data, skin, 1);
    let count = e.get(partition_data, NiSkinPartition::m_uiPartitions);
    let mut flags = 0u32;
    if e.vcall(skin.addr(), slot::SKIN_TYPE, &[]).u32() == SKIN_RTTI {
        flags = e.get(skin, NiSkinInstance::pPartitionFlags);
    }
    let mut first_partition = true;
    let mut index = 0u32;
    while index < count {
        if flags == 0 || e.global::<u8>(flags.wrapping_add(index * 4)) != 0 {
            let mut partition = partitions.wrapping_add(index.wrapping_mul(0x2c));
            let word = e.global::<u32>(partition.wrapping_add(0x28));
            let bound = typed_call!(leaf::FAMILY, 0x0043_d450, leaf::world_bound(e, geometry));
            partition_setup(
                e,
                shader,
                [
                    geometry.addr(),
                    skin.addr(),
                    partition,
                    word,
                    state,
                    0,
                    geometry.addr().wrapping_add(NiGeometry::m_kWorld.off),
                    bound,
                ],
            );
            if first_partition {
                shader_geometry_setups(e, shader, state);
                if e.get(pass, RenderPass::ucSortFlag) != 0
                    && e.call(IS_SHADOW_PASS, &args![pass_type]).u8() == 0
                {
                    if second {
                        shader_blend(e, shader, state);
                    }
                    if first && !(4..=5).contains(&pass_type) && !(0xe..=0xf).contains(&pass_type) {
                        shader_alpha_function(e, shader, state);
                    }
                } else if e.global::<u8>(RESTORE_OWED) != 0 {
                    shader_restore(e, shader, state);
                    e.set_global(RESTORE_OWED, 0u8);
                }
                if third {
                    let flag = u32::from(e.get(pass, RenderPass::ucSortFlag));
                    shader_alpha_test(e, shader, state, flag);
                }
                first_partition = false;
            }
            let drawn = shader_draw(e, shader, geometry, partition, word, state);
            partition_drawn(e, holder, drawn, partition, data.addr());
            if flags != 0 {
                let mut next = index + 1;
                while next < count {
                    if e.global::<u8>(flags.wrapping_add(index * 4 + 5)) != 0 {
                        break;
                    }
                    index += 1;
                    next += 1;
                    partition = partition.wrapping_add(0x2c);
                    if e.global::<u8>(flags.wrapping_add(index * 4)) != 0 {
                        let word = e.global::<u32>(partition.wrapping_add(0x28));
                        let drawn = shader_draw(e, shader, geometry, partition, word, state);
                        if drawn == 0 {
                            return;
                        }
                        partition_drawn(e, holder, drawn, partition, data.addr());
                    }
                }
            }
        }
        index += 1;
    }
    let data = e.get(geometry, NiGeometry::m_spModelData);
    let bits = e.get(data, NiGeometryData::m_usDirtyFlags) & 0xf000;
    e.set(data, NiGeometryData::m_usDirtyFlags, bits);
    shader_geometry_done(e, shader, state);
}

// Typed from 00b99390
/// `cdecl(type, shader)`: begins a shader's pass: the pass before ends, the marker resets, the shader
/// begins (slot 0xf0), the stage textures clear, then the shader's own begin for four type groups (types
/// 0xfc and 0xfd also switch the depth test on) or its states, constants and textures (slots 0xfc, 0xf8,
/// 0x100), and the begin's end (0xf4). The type and shader become the current ones; a restore is owed.
pub fn begin_pass(e: &mut Engine, pass_type: u32, shader: u32) {
    typed_call!(FAMILY, 0x00b9_88e0, end_pass(e));
    e.set_global(PASS_MARKER, 0xffffu16);
    if crate::slot_target(e, shader, slot::BEGIN) == 0x00b7_af80 {
        typed_call!(
            crate::draw::FAMILY,
            0x00b7_af80,
            crate::draw::setups::select_pass(e, Ptr::new(shader), pass_type)
        );
    } else {
        e.vcall(shader, slot::BEGIN, &args![pass_type]);
    }
    for i in 0..16 {
        e.set_global(STAGE_TEXTURES + i * 4, 0u32);
    }
    match pass_type {
        0xca | 0xd1 => {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_deb0,
                crate::draw::setups::begin_types_ca(e, Ptr::new(shader))
            );
        }
        0xfc | 0xfd => {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_dd50,
                crate::draw::setups::begin_types_fc(e, Ptr::new(shader))
            );
            typed_call!(FAMILY, 0x00b9_7de0, state::set_z_enable(e, 1u8, 0u32));
        }
        0x102 => {
            typed_call!(
                crate::draw::FAMILY,
                0x00b7_dd50,
                crate::draw::setups::begin_types_fc(e, Ptr::new(shader))
            );
        }
        _ => {
            if crate::slot_target(e, shader, slot::BEGIN_CONSTANTS) == 0x00b7_b450 {
                typed_call!(
                    crate::draw::FAMILY,
                    0x00b7_b450,
                    crate::draw::setups::setup_pass_constants(e, Ptr::new(shader), pass_type)
                );
            } else {
                e.vcall(shader, slot::BEGIN_CONSTANTS, &args![pass_type]);
            }
            if crate::slot_target(e, shader, slot::BEGIN_TEXTURES) == 0x00b7_cfb0 {
                typed_call!(
                    FAMILY,
                    0x00b7_cfb0,
                    accumulator::set_pair_textures(e, shader, pass_type)
                );
            } else {
                e.vcall(shader, slot::BEGIN_TEXTURES, &args![pass_type]);
            }
            if crate::slot_target(e, shader, slot::BEGIN_STATES) == 0x00be_0eb0 {
                typed_call!(
                    crate::draw::FAMILY,
                    0x00be_0eb0,
                    crate::draw::passes::set_pass_states(e, shader, pass_type)
                );
            } else {
                e.vcall(shader, slot::BEGIN_STATES, &args![pass_type]);
            }
        }
    }
    if crate::slot_target(e, shader, slot::BEGIN_DONE) == 0x00be_1f90 {
        typed_call!(
            crate::draw::FAMILY,
            0x00be_1f90,
            crate::draw::passes::begin_done(e, shader, pass_type)
        );
    } else {
        e.vcall(shader, slot::BEGIN_DONE, &args![pass_type]);
    }
    capture::set_own_at(e, CURRENT_PASS_TYPE, pass_type);
    capture::set_own_at(e, CURRENT_SHADER, shader);
    e.set_global(RESTORE_OWED, 1u8);
}

// Typed from 00b988e0
/// `BSBatchRenderer::EndPass` (engine map), no arguments: with a current shader, its pass ends (slot
/// 0x104 with the type) with the restore (slot 0x114) unless one is still owed; types 0xfe and 0x10b
/// set sampler 7's, and 0x230 sampler 2's, address mode back (state 8 to 0) on the device. The current
/// type and shader clear.
pub fn end_pass(e: &mut Engine) {
    let shader = capture::own_at::<u32>(e, CURRENT_SHADER);
    if shader != 0 {
        let pass_type = capture::own_at::<u32>(e, CURRENT_PASS_TYPE);
        if crate::slot_target(e, shader, slot::END) == 0x00be_0ee0 {
            typed_call!(
                crate::draw::FAMILY,
                0x00be_0ee0,
                crate::draw::passes::restore_pass_states(e, shader, pass_type)
            );
        } else {
            e.vcall(shader, slot::END, &args![pass_type]);
        }
        if e.global::<u8>(RESTORE_OWED) == 0 {
            let shader = capture::own_at::<u32>(e, CURRENT_SHADER);
            if crate::slot_target(e, shader, slot::END_RESTORE) == 0x00be_0e90 {
                typed_call!(
                    crate::draw::FAMILY,
                    0x00be_0e90,
                    crate::draw::passes::end_states_restored(e, shader)
                );
            } else {
                e.vcall(shader, slot::END_RESTORE, &[]);
            }
        }
        let sampler = match capture::own_at::<u32>(e, CURRENT_PASS_TYPE) {
            0xfe | 0x10b => Some(7u32),
            0x230 => Some(2),
            _ => None,
        };
        if let Some(sampler) = sampler {
            use super::layout::NiDX9Renderer as R;
            let holder = Ptr::<R>::new(e.global::<u32>(RENDER_HOLDER));
            let device = e.get(holder, R::m_pkD3DDevice9);
            e.vcall(
                device,
                DEVICE_SET_SAMPLER_STATE,
                &args![sampler, 8u32, 0u32],
            );
        }
    }
    capture::set_own_at(e, CURRENT_PASS_TYPE, 0u32);
    capture::set_own_at(e, CURRENT_SHADER, 0u32);
}

/// The distance of a pass's world bound along the camera's direction (as the exe sums it: x and y,
/// then z), stored to a float; minus the bound's radius (stored again) when asked. The exe orders the
/// operands of the second item's products the other way round, which changes nothing but the x87's
/// choice of NaN.
#[inline(always)]
fn depth(e: &Engine, item: u32, d: [f32; 3], subtract: bool) -> f32 {
    let pass = Ptr::<RenderPass>::new(e.global::<u32>(item));
    let geometry = e.get(pass, RenderPass::pGeometry);
    let mut bound = Ptr::<NiBound>::new(e.get(geometry, NiGeometry::m_kWorldBound));
    if bound.is_null() {
        bound = Ptr::new(NULL_BOUND);
    }
    // The bound's centre and radius as the value packet holds them (value packet B).
    let word = |index: u32| f32::from_bits(crate::draw::packet::bound_word(e, bound.addr(), index));
    let (x, y, z) = (word(0), word(1), word(2));
    let sum = (x * d[0] + y * d[1]) + z * d[2];
    if subtract {
        sum - word(3)
    } else {
        sum
    }
}

// Typed from 00b98b20
/// `cdecl(first, second)`, the alpha sort's comparison of two list items (pointers to passes): each
/// pass's depth along the camera's direction (+0x68, +0x74, +0x80), less its radius when asked
/// (`011ffe35`). 0 when equal, 1 when the second is deeper, -1 otherwise (a NaN included).
pub fn compare_by_depth(e: &mut Engine, first: u32, second: u32) -> i32 {
    let subtract = capture::own_at::<u8>(e, SORT_SUBTRACT_RADIUS) != 0;
    let camera = capture::own_at::<u32>(e, SORT_CAMERA);
    // The camera's direction as the value packet holds it (value packet B).
    let direction = |off: u32| f32::from_bits(capture::word(e, camera, off));
    let d = [direction(0x68), direction(0x74), direction(0x80)];
    let a = depth(e, first, d, subtract);
    let b = depth(e, second, d, subtract);
    if b == a {
        0
    } else if b > a {
        1
    } else {
        -1
    }
}

// Typed from 00b99a10
/// `thiscall(renderer, camera, subtract)`: sorts the two alpha holders' lists (+0x70, +0x74) back to
/// front by [`compare_by_depth`] for the camera.
pub fn sort_alphas(e: &mut Engine, this: Ptr<BSBatchRenderer>, camera: u32, subtract: u8) {
    capture::set_own_at(e, SORT_SUBTRACT_RADIUS, subtract);
    capture::set_own_at(e, SORT_CAMERA, camera);
    for index in 0..2 {
        let holder = e.global::<u32>(elem_addr(this, BSBatchRenderer::pAlphaGroups, index, 4));
        let list = Ptr::<PassHolder>::new(holder).at(PassHolder::kList);
        typed_call!(FAMILY, 0x00b9_9630, sort_list(e, list, COMPARE_BY_DEPTH));
    }
}

/// The shader's per-geometry textures and constants (slots 0x78 and 0x7c, `thiscall(shader, state)`):
/// the shadow light shader's (`00b7d200`, `00b7dab0`) directly when the slots hold them, as its classes
/// have; another shader's through the vtable. Each slot is read when its call is made.
#[inline(always)]
fn shader_geometry_setups(e: &mut Engine, shader: u32, state: u32) {
    if crate::slot_target(e, shader, slot::GEOMETRY_A) == 0x00b7_d200 {
        typed_call!(
            FAMILY,
            0x00b7_d200,
            textures::setup_geometry_textures(e, Ptr::new(shader), Ptr::new(state))
        );
    } else {
        e.vcall(shader, slot::GEOMETRY_A, &args![state]);
    }
    if crate::slot_target(e, shader, slot::GEOMETRY_B) == 0x00b7_dab0 {
        typed_call!(
            FAMILY,
            0x00b7_dab0,
            geometry::setup_geometry_constants(e, Ptr::new(shader), Ptr::new(state))
        );
    } else {
        e.vcall(shader, slot::GEOMETRY_B, &args![state]);
    }
}

/// The shader's geometry end (slot 0x8c, `thiscall(shader, state)`): the shadow light shader's
/// (`00b7c320`) directly when the slot holds it; another shader's through the vtable.
#[inline(always)]
fn shader_geometry_done(e: &mut Engine, shader: u32, state: u32) {
    if crate::slot_target(e, shader, slot::GEOMETRY_DONE) == 0x00b7_c320 {
        typed_call!(
            FAMILY,
            0x00b7_c320,
            accumulator::restore_states(e, shader, state)
        );
    } else {
        e.vcall(shader, slot::GEOMETRY_DONE, &args![state]);
    }
}

/// The shader's blending of the geometry (slot 0x80, `thiscall(shader, state)`): `00be1ff0` (this
/// family's) directly when the slot holds it, as the shadow light shader's classes have; another target
/// through the vtable.
#[inline(always)]
fn shader_blend(e: &mut Engine, shader: u32, state: u32) {
    if crate::slot_target(e, shader, slot::SECOND) == 0x00be_1ff0 {
        typed_call!(
            FAMILY,
            0x00be_1ff0,
            accumulator::set_blend_of(e, shader, state)
        );
    } else {
        e.vcall(shader, slot::SECOND, &args![state]);
    }
}

/// The shader's alpha function of the geometry (slot 0x84, `thiscall(shader, state)`): `00be20b0` (the
/// draw family's) directly when the slot holds it; another target through the vtable.
#[inline(always)]
fn shader_alpha_function(e: &mut Engine, shader: u32, state: u32) {
    if crate::slot_target(e, shader, slot::FIRST) == 0x00be_20b0 {
        typed_call!(
            crate::draw::FAMILY,
            0x00be_20b0,
            crate::draw::passes::alpha_function_of(e, shader, state)
        );
    } else {
        e.vcall(shader, slot::FIRST, &args![state]);
    }
}

/// The shader's alpha test and cull mode of the geometry (slot 0x88, `thiscall(shader, state, flag)`):
/// `00be20e0` (the draw family's) directly when the slot holds it; another target through the vtable.
#[inline(always)]
fn shader_alpha_test(e: &mut Engine, shader: u32, state: u32, flag: u32) {
    if crate::slot_target(e, shader, slot::THIRD) == 0x00be_20e0 {
        typed_call!(
            crate::draw::FAMILY,
            0x00be_20e0,
            crate::draw::passes::alpha_test_of(e, shader, state, low8(flag) != 0)
        );
    } else {
        e.vcall(shader, slot::THIRD, &args![state, flag]);
    }
}

/// The shader's per-geometry restore (slot 0x110, `thiscall(shader, state)`): `00be0e50` (the draw
/// family's) directly when the slot holds it; another target through the vtable.
#[inline(always)]
fn shader_restore(e: &mut Engine, shader: u32, state: u32) {
    if crate::slot_target(e, shader, slot::RESTORE) == 0x00be_0e50 {
        typed_call!(
            crate::draw::FAMILY,
            0x00be_0e50,
            crate::draw::passes::end_states(e, shader, state)
        );
    } else {
        e.vcall(shader, slot::RESTORE, &args![state]);
    }
}

/// The shader's draw (slot 0x6c, `thiscall(shader, geometry, partition, buffer data, state)`):
/// `00e812f0` (the render family's `prepare_streams`) directly when the slot holds it, as every
/// `NiD3DShader` class has; another target through the vtable.
#[inline(always)]
fn shader_draw(
    e: &mut Engine,
    shader: u32,
    geometry: impl OneWord,
    partition: impl OneWord,
    buffer: impl OneWord,
    state: impl OneWord,
) -> u32 {
    let (g, p, b, s) = (
        geometry.one_word(),
        partition.one_word(),
        buffer.one_word(),
        state.one_word(),
    );
    if crate::slot_target(e, shader, slot::DRAW) == 0x00e8_12f0 {
        typed_call!(
            crate::render::FAMILY,
            0x00e8_12f0,
            crate::render::stream::prepare_streams(e, Ptr::new(shader), g, p, b, s)
        )
    } else {
        e.vcall(shader, slot::DRAW, &args![g, p, b, s]).u32()
    }
}

/// The geometry's immediate draw with the renderer (slot 0xe0, `OnlyRenderImmediate`): the triangle
/// shape's, the segmented shape's and the particles' (the draw family's) and the triangle strips' (the
/// dispatch family's) directly when the slot holds them; another class's through the vtable.
#[inline(always)]
fn render_immediate(e: &mut Engine, geometry: u32, renderer: u32) {
    match crate::slot_target(e, geometry, slot::RENDER_IMMEDIATE) {
        0x00a7_4600 => typed_call!(
            crate::draw::FAMILY,
            0x00a7_4600,
            crate::draw::shapes::tri_shape_render_immediate(
                e,
                Ptr::new(geometry),
                Ptr::new(renderer)
            )
        ),
        0x00a8_7130 => typed_call!(
            crate::draw::FAMILY,
            0x00a8_7130,
            crate::draw::shapes::segmented_render_immediate(
                e,
                Ptr::new(geometry),
                Ptr::new(renderer)
            )
        ),
        0x00a9_b1f0 => typed_call!(
            crate::draw::FAMILY,
            0x00a9_b1f0,
            crate::draw::shapes::particles_render_immediate(
                e,
                Ptr::new(geometry),
                Ptr::new(renderer)
            )
        ),
        0x00a7_1d70 => typed_call!(
            crate::dispatch::FAMILY,
            0x00a7_1d70,
            crate::dispatch::scene::tri_strips_render_immediate(
                e,
                Ptr::new(geometry),
                Ptr::new(renderer)
            )
        ),
        _ => {
            e.vcall(geometry, slot::RENDER_IMMEDIATE, &args![renderer]);
        }
    }
}

/// The shader's partition set-up (slot 0x64, eight words): the skinned shader classes' bone set-up
/// (`00b7a2c0`, the draw family's) directly when the slot holds it; another shader's through the vtable.
#[inline(always)]
fn partition_setup(e: &mut Engine, shader: u32, words: [u32; 8]) {
    if crate::slot_target(e, shader, slot::PARTITION) == 0x00b7_a2c0 {
        let [a, b, c, d, f, g, h, i] = words;
        let _: u32 = typed_call!(
            crate::draw::FAMILY,
            0x00b7_a2c0,
            crate::draw::skin::bone_setup(e, Ptr::new(shader), a, b, c, d, f, g, h, i)
        );
    } else {
        e.vcall(shader, slot::PARTITION, &words);
    }
}

/// A drawn partition to the renderer's partition draw (`00e6d310`, the draw family's, `thiscall(renderer,
/// the drawn buffer data, the partition, the data)`).
#[inline(always)]
fn partition_drawn(e: &mut Engine, renderer: u32, drawn: u32, partition: u32, data: u32) {
    typed_call!(
        crate::draw::FAMILY,
        0x00e6_d310,
        crate::draw::packed::draw_partition(e, Ptr::new(renderer), drawn, partition, data)
    );
}

/// A geometry's word-answering slot with no arguments (0x30, 0x34): the leaf `00acbb70` (0) that
/// every geometry class holds there, directly; another target through the vtable.
#[inline(always)]
fn geometry_word(e: &mut Engine, geometry: u32, slot: u32) -> u32 {
    if crate::slot_target(e, geometry, slot) == 0x00ac_bb70 {
        typed_call!(leaf::FAMILY, 0x00ac_bb70, leaf::zero_of_this(e, geometry))
    } else {
        e.vcall(geometry, slot, &[]).u32()
    }
}

/// The comparison of two nodes' items: the alpha sort's by a Rust call when it is the one asked for.
#[inline(always)]
fn compare_items(e: &mut Engine, compare: u32, p: u32, q: u32) -> i32 {
    let (a, b) = (p.wrapping_add(8), q.wrapping_add(8));
    if compare == COMPARE_BY_DEPTH {
        typed_call!(FAMILY, 0x00b9_8b20, compare_by_depth(e, a, b))
    } else {
        e.call(compare, &args![a, b]).i32()
    }
}

// Typed from 00b99630
/// `thiscall(list, compare)`: a merge sort of the list in place (runs doubling), the item with the
/// comparison above 0 for the second run going first; the spare chain is released first.
pub fn sort_list(e: &mut Engine, this: Ptr<PassList>, compare: u32) {
    typed_call!(FAMILY, 0x00b6_3e90, release_spare(e, this));
    if vget(e, this, PassList::pHead) == 0 {
        return;
    }
    let mut run = 1u32;
    let mut tail;
    loop {
        let mut p = vget(e, this, PassList::pHead);
        vset(e, this, PassList::pHead, 0);
        tail = 0u32;
        let mut merges = 0u32;
        while p != 0 {
            merges += 1;
            let mut p_size = 0i32;
            let mut q = p;
            let mut i = 0u32;
            if run != 0 {
                loop {
                    q = e.global::<u32>(q);
                    p_size += 1;
                    if q == 0 {
                        break;
                    }
                    i += 1;
                    if i >= run {
                        break;
                    }
                }
            }
            #[allow(clippy::cast_possible_wrap)]
            let mut q_size = run as i32;
            loop {
                let second_first = |e: &mut Engine, p: u32, q: u32, q_size: i32| {
                    q_size != 0 && q != 0 && compare_items(e, compare, p, q) > 0
                };
                let from_second = if p_size > 0 {
                    second_first(e, p, q, q_size)
                } else {
                    if q_size <= 0 || q == 0 {
                        break;
                    }
                    p_size == 0 || second_first(e, p, q, q_size)
                };
                let next;
                if from_second {
                    next = q;
                    q = e.global::<u32>(q);
                    q_size -= 1;
                } else {
                    next = p;
                    p = e.global::<u32>(p);
                    p_size -= 1;
                }
                if tail != 0 {
                    e.set_global(tail, next);
                } else {
                    vset(e, this, PassList::pHead, next);
                }
                e.set_global(next.wrapping_add(4), tail);
                tail = next;
            }
            p = q;
        }
        run = run.wrapping_add(run);
        e.set_global(tail, 0u32);
        if merges == 1 {
            break;
        }
    }
    vset(e, this, PassList::pTail, tail);
}

/// A node for `list`: the spare chain's first, or a new one from the pool allocator.
#[inline(always)]
fn take_node(e: &mut Engine, list: Ptr<PassList>) -> Ptr<PassNode> {
    let spare = e.get(list, PassList::pSpare);
    if spare == 0 {
        // The node pool's allocation (`0043a010`, `thiscall` on the list's count).
        let count = Ptr::new(list.addr().wrapping_add(PassList::uiCount.off));
        let node: Ptr = typed_call!(
            crate::draw_f::FAMILY,
            0x0043_a010,
            crate::draw_f::helpers::allocate_node(e, count)
        );
        Ptr::new(node.addr())
    } else {
        let next = e.global::<u32>(spare);
        e.set(list, PassList::pSpare, next);
        Ptr::new(spare)
    }
}

// Typed from 00b99840
/// `thiscall(list, item pointer)`: appends the item.
pub fn list_append(e: &mut Engine, this: Ptr<PassList>, item: u32) {
    let node = take_node(e, this);
    let value = e.global::<u32>(item);
    e.set(node, PassNode::pItem, value);
    e.set(node, PassNode::pNext, 0);
    let tail = vget(e, this, PassList::pTail);
    e.set(node, PassNode::pPrev, tail);
    let tail = vget(e, this, PassList::pTail);
    let count = vget(e, this, PassList::uiCount).wrapping_add(1);
    if tail != 0 {
        e.set(Ptr::<PassNode>::new(tail), PassNode::pNext, node.addr());
        vset(e, this, PassList::uiCount, count);
    } else {
        vset(e, this, PassList::uiCount, count);
        vset(e, this, PassList::pHead, node.addr());
    }
    vset(e, this, PassList::pTail, node.addr());
}

// Typed from 00b99890
/// `thiscall(list, item pointer)`: prepends the item.
pub fn list_prepend(e: &mut Engine, this: Ptr<PassList>, item: u32) {
    let node = take_node(e, this);
    let value = e.global::<u32>(item);
    e.set(node, PassNode::pItem, value);
    let head = vget(e, this, PassList::pHead);
    e.set(node, PassNode::pNext, head);
    e.set(node, PassNode::pPrev, 0);
    let head = vget(e, this, PassList::pHead);
    let count = vget(e, this, PassList::uiCount).wrapping_add(1);
    if head != 0 {
        e.set(Ptr::<PassNode>::new(head), PassNode::pPrev, node.addr());
        vset(e, this, PassList::uiCount, count);
    } else {
        vset(e, this, PassList::uiCount, count);
        vset(e, this, PassList::pTail, node.addr());
    }
    vset(e, this, PassList::pHead, node.addr());
}

// Typed from 00b9abc0
/// `thiscall(holder, pass, prepend)`: a holder with bit 0 keeps the pass in its list (first or last);
/// another hands it to its batch renderer's `AddPass` (made with the holder's size on first use). The
/// holder's count goes up either way.
pub fn add_pass(e: &mut Engine, this: Ptr<PassHolder>, pass: Ptr<RenderPass>, prepend: u32) {
    if vget8(e, this, PassHolder::ucFlags) & 1 != 0 {
        e.with_stack(4, |e, slot| {
            e.set_global(slot.addr(), pass.addr());
            let list = this.at(PassHolder::kList);
            if low8(prepend) != 0 {
                typed_call!(FAMILY, 0x00b9_9890, list_prepend(e, list, slot.addr()));
            } else {
                typed_call!(FAMILY, 0x00b9_9840, list_append(e, list, slot.addr()));
            }
        });
    } else {
        if e.get(this, PassHolder::pRenderer).is_null() {
            let block = e.call(OPERATOR_NEW, &args![0x98u32]).u32();
            let renderer = if block != 0 {
                let size = e.get(this, PassHolder::uiRendererSize);
                e.call(RENDERER_CONSTRUCT, &args![block, size, 100u32])
                    .u32()
            } else {
                0
            };
            e.set(this, PassHolder::pRenderer, Ptr::new(renderer));
        }
        let pass_type = u32::from(e.get(pass, RenderPass::usPassEnum));
        let renderer = e.get(this, PassHolder::pRenderer);
        e.vcall(renderer.addr(), slot::ADD_PASS, &args![pass, pass_type]);
    }
    let count = vget16(e, this, PassHolder::usCount).wrapping_add(1);
    vset16(e, this, PassHolder::usCount, count);
}

// Typed from 00b9b3b0
/// `thiscall(renderer, pass, group, prepend)`: [`add_pass`] to the holder of register group `group`.
pub fn register_pass(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass: Ptr<RenderPass>,
    group: u32,
    prepend: u32,
) -> u32 {
    let holder = Ptr::<PassHolder>::new(e.global::<u32>(elem_addr(
        this,
        BSBatchRenderer::pGroups,
        group,
        4,
    )));
    typed_call!(FAMILY, 0x00b9_abc0, add_pass(e, holder, pass, prepend));
    0
}

// Typed from 00b9b3d0
/// `thiscall(renderer, pass, flag)`: [`add_pass`] (appended) to the alpha holder of `flag`, the
/// grouped one while alphas are grouped (+0x34 above 0).
pub fn register_alpha(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass: Ptr<RenderPass>,
    flag: u32,
) -> u32 {
    let field = if e.get(this, BSBatchRenderer::iGroupingAlphas) > 0 {
        BSBatchRenderer::pGroupedAlphas.off
    } else {
        BSBatchRenderer::pAlphaGroups.off
    };
    let holder = Ptr::<PassHolder>::new(
        e.global::<u32>(
            this.addr()
                .wrapping_add(flag.wrapping_mul(4))
                .wrapping_add(field),
        ),
    );
    typed_call!(FAMILY, 0x00b9_abc0, add_pass(e, holder, pass, 0u32));
    0
}

/// Empties a list into its spare chain: the spare chain released to the pool, the nodes in use kept
/// as the new spare chain.
#[inline(always)]
fn reset_list(e: &mut Engine, list: Ptr<PassList>) {
    typed_call!(FAMILY, 0x00b6_3e90, release_spare(e, list));
    let head = vget(e, list, PassList::pHead);
    e.set(list, PassList::pSpare, head);
    vset(e, list, PassList::pHead, 0);
    vset(e, list, PassList::pTail, 0);
    vset(e, list, PassList::uiCount, 0);
}

// Typed from 00b9a490
/// `thiscall(holder)`: empties the holder's list and its batch renderer's groups; the count clears.
pub fn clear_holder(e: &mut Engine, this: Ptr<PassHolder>) {
    reset_list(e, this.at(PassHolder::kList));
    let renderer = e.get(this, PassHolder::pRenderer);
    if !renderer.is_null() {
        typed_call!(FAMILY, 0x00b9_9bc0, clear_renderer(e, renderer, 0u32));
    }
    vset16(e, this, PassHolder::usCount, 0);
}

/// The first item of the map's buckets from `start` on.
#[inline(always)]
fn map_item_from(e: &Engine, this: Ptr<BSBatchRenderer>, start: u32) -> u32 {
    let count = e.get(this, BSBatchRenderer::RenderPassMap_m_uiHashSize);
    let buckets = e.get(this, BSBatchRenderer::RenderPassMap_m_ppkHashTable);
    (start..count)
        .map(|i| e.global::<u32>(buckets.wrapping_add(i * 4)))
        .find(|item| *item != 0)
        .unwrap_or(0)
}

// Typed from 00b99a50
/// `thiscall(group, unused)`: empties each of the group's five lists that has passes; the total clears.
pub fn clear_group(e: &mut Engine, this: Ptr<PassGroup>, _unused: u32) {
    for index in 0..5u32 {
        let list = Ptr::<PassList>::new(this.addr().wrapping_add(index * 0x14));
        if vget(e, list, PassList::uiCount) != 0 {
            reset_list(e, list);
        }
    }
    vset(e, this, PassGroup::uiCount, 0);
}

// Typed from 00b99bc0
/// `thiscall(renderer, unused)`: empties the group of every pass type in the map, then the active
/// list.
pub fn clear_renderer(e: &mut Engine, this: Ptr<BSBatchRenderer>, _unused: u32) {
    let mut item = if e.get(this, BSBatchRenderer::RenderPassMap_m_uiHashSize) != 0 {
        map_item_from(e, this, 0)
    } else {
        0
    };
    while item != 0 {
        let link = e.global::<u32>(item);
        let group = e.global::<u32>(item.wrapping_add(8));
        let next = if link != 0 {
            link
        } else {
            let key = e.global::<u32>(item.wrapping_add(4));
            let map = this
                .addr()
                .wrapping_add(BSBatchRenderer::RenderPassMap_vtable.off);
            let index = e
                .vcall(map, slot::MAP_HASH, &args![key])
                .u32()
                .wrapping_add(1);
            if index < e.get(this, BSBatchRenderer::RenderPassMap_m_uiHashSize) {
                map_item_from(e, this, index)
            } else {
                0
            }
        };
        let groups = e.get(this, BSBatchRenderer::pRenderPassA);
        let group = Ptr::<PassGroup>::new(group.wrapping_mul(0x68).wrapping_add(groups));
        for index in 0..5u32 {
            let list = Ptr::<PassList>::new(group.addr() + index * 0x14);
            if vget(e, list, PassList::uiCount) != 0 {
                reset_list(e, list);
            }
        }
        vset(e, group, PassGroup::uiCount, 0);
        item = next;
    }
    let active = this.at(BSBatchRenderer::kActivePassIndexList);
    let words = |_: &Engine| vec![active.addr(), active.addr().wrapping_add(4)];
    crate::draw::detach::suspended_for(e, words, |e| e.call(ACTIVE_LIST_CLEAR, &args![active]));
}

// Typed from 00b63e90
/// `thiscall(list)`: returns the list's spare chain to the global node pool under the pool's lock
/// (taken once around the walk and once per node), and cuts the list after its tail. Answers the tail.
pub fn release_spare(e: &mut Engine, this: Ptr<PassList>) -> u32 {
    let mut node = e.get(this, PassList::pSpare);
    if node != 0 {
        e.call(LOCK_ENTER, &args![POOL_LOCK, 0u32]);
        loop {
            let current = node;
            node = e.global::<u32>(node);
            e.call(LOCK_ENTER, &args![POOL_LOCK, 0u32]);
            pool_give(e, current);
            if node == 0 {
                break;
            }
        }
        pool_unlock(e);
    }
    let tail = vget(e, this, PassList::pTail);
    e.set(this, PassList::pSpare, 0);
    if tail != 0 {
        e.set_global(tail, 0u32);
    }
    tail
}

/// A node back to the pool's free list, the lock held once less.
#[inline(always)]
pub(crate) fn pool_give(e: &mut Engine, node: u32) {
    e.set_global(node.wrapping_add(4), 0u32);
    let free = e.global::<u32>(POOL_FREE);
    e.set_global(node, free);
    let count = e.global::<u32>(POOL_LOCK_COUNT).wrapping_sub(1);
    e.set_global(POOL_LOCK_COUNT, count);
    e.set_global(POOL_FREE, node);
    if count == 0 {
        e.set_global(POOL_LOCK, 0u32);
    }
}

/// The pool's lock held once less.
#[inline(always)]
fn pool_unlock(e: &mut Engine) {
    let count = e.global::<u32>(POOL_LOCK_COUNT).wrapping_sub(1);
    e.set_global(POOL_LOCK_COUNT, count);
    if count == 0 {
        e.set_global(POOL_LOCK, 0u32);
    }
}

// Typed from 00b99c90
/// The `AddPass` of `BSBatchRenderer`, virtual slot 0x8c, `thiscall(renderer, pass, type)`: files the
/// pass under `pass_type` (or its own type when 0). The list: 4 when the shader property has bit 22 of
/// its second flags; else an alpha pass goes to 2 or 3 and another to 1 or 0, by whether the shade
/// property has both bits 10 and 11. A pass counts as alpha when its alpha property blends, its sort
/// byte is set and it isn't held back (1.0 above either alpha float, no bit 0, type not 2), and always
/// for the types 4, 5, 0xe, 0xf and 0x23a. A new type gets the next group index, a cleared group and
/// a place in the active list; the pass is prepended when its sort byte is set and the type isn't
/// 0x238 to 0x23e, else appended.
pub fn add_to_group(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass: Ptr<RenderPass>,
    pass_type: u32,
) {
    if pass.is_null() {
        return;
    }
    let geometry = e.get(pass, RenderPass::pGeometry);
    if geometry.is_null() {
        return;
    }
    let properties = e.get(geometry, NiGeometry::pShaderProperty);
    let shade = e.get(geometry, NiGeometry::pShadeProperty);
    let paired = !shade.is_null() && e.get(shade, NiProperty::m_usFlags) & 0xc00 == 0xc00;
    let alpha_property = Ptr::<NiProperty>::new(e.global::<u32>(elem_addr(
        geometry,
        NiGeometry::m_PropertyState,
        0,
        4,
    )));
    let mut alpha = false;
    if !alpha_property.is_null()
        && (e.get(alpha_property, NiProperty::m_usFlags) >> 9) & 1 != 0
        && e.get(pass, RenderPass::ucSortFlag) != 0
    {
        use super::layout::BSShaderProperty as P;
        alpha = true;
        let blocked = 1.0 > e.get(properties, P::fAlpha) || 1.0 > e.get(properties, P::fFadeAlpha);
        if blocked && e.get(alpha_property, NiProperty::m_usFlags) & 1 == 0 && pass_type != 2 {
            alpha = false;
        }
    }
    if matches!(pass_type, 0xe | 0xf | 4 | 5 | 0x23a) {
        alpha = true;
    }
    let list = if e.get(properties, super::layout::BSShaderProperty::ulFlags2) & 0x40_0000 != 0 {
        4
    } else if alpha {
        u32::from(!paired) + 2
    } else {
        u32::from(paired)
    };
    let mut key = u32::from(e.get(pass, RenderPass::usPassEnum));
    if pass_type != 0 {
        key = pass_type;
    }
    let map = this
        .addr()
        .wrapping_add(BSBatchRenderer::RenderPassMap_vtable.off);
    // The out slot is the stack word of the `pass_type` argument.
    let (found, value) = e.with_stack(4, |e, slot| {
        e.set_global(slot.addr(), pass_type);
        let found = e.call(MAP_GET_AT, &args![map, key, slot]).u8() != 0;
        (found, e.global::<u32>(slot.addr()))
    });
    let index = if found {
        value
    } else {
        let index = e.get(this, BSBatchRenderer::RenderPassMap_m_uiCount);
        e.call(MAP_SET_AT, &args![map, key, index]);
        let group = group_at(e, this, index);
        typed_call!(FAMILY, 0x00b9_9a50, clear_group(e, group, 0u32));
        index
    };
    let group = group_at(e, this, index);
    if vget(e, group, PassGroup::uiCount) == 0 {
        let active = this.at(BSBatchRenderer::kActivePassIndexList);
        typed_call!(
            FAMILY,
            0x00b9_9770,
            insert_active(e, active, key, PASS_TYPE_COMPARE)
        );
    }
    let group = group_at(e, this, index);
    if e.get(pass, RenderPass::ucSortFlag) != 0 && !(0x238..=0x23e).contains(&key) {
        typed_call!(FAMILY, 0x00b9_9a90, group_prepend(e, group, list, pass));
    } else {
        typed_call!(FAMILY, 0x00b9_9b00, group_append(e, group, list, pass));
    }
}

#[inline(always)]
fn group_at(e: &Engine, this: Ptr<BSBatchRenderer>, index: u32) -> Ptr<PassGroup> {
    let groups = e.get(this, BSBatchRenderer::pRenderPassA);
    Ptr::new(index.wrapping_mul(0x68).wrapping_add(groups))
}

// Typed from 00b99770
/// `thiscall(active list, type, compare)`: puts a pass type into the active list in the comparison's
/// order: into an empty head, after the last entry it compares above, or before the first it doesn't
/// (at the head by `00b98e20`). A type of 0 does nothing.
pub fn insert_active(e: &mut Engine, this: Ptr<ActivePass>, item: u32, compare: u32) {
    let value = item & 0xffff;
    if value == 0 {
        return;
    }
    let mut previous = 0u32;
    let mut node = this.addr();
    while node != 0 {
        let entry = Ptr::<ActivePass>::new(node);
        let current = vget16(e, entry, ActivePass::usItem);
        if current == 0 {
            vset16(e, entry, ActivePass::usItem, low16(value));
            break;
        }
        if e.call(compare, &args![item, u32::from(current)]).i32() > 0 {
            if vget(e, entry, ActivePass::pNext) == 0 {
                let added = new_entry(e, value);
                vset(e, entry, ActivePass::pNext, added);
                break;
            }
        } else {
            if previous == 0 {
                e.with_stack(4, |e, slot| {
                    e.set_global(slot.addr(), item);
                    e.call(ACTIVE_INSERT_FIRST, &args![this, slot]);
                });
            } else {
                let added = e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, 8u32]).u32();
                let before = Ptr::<ActivePass>::new(previous);
                if added != 0 {
                    let added = Ptr::<ActivePass>::new(added);
                    vset16(e, added, ActivePass::usItem, low16(value));
                    vset(e, added, ActivePass::pNext, 0);
                    vset(e, before, ActivePass::pNext, added.addr());
                    vset(e, added, ActivePass::pNext, node);
                } else {
                    // The exe links through the null block: `[0 + 4]` takes the node.
                    vset(e, before, ActivePass::pNext, 0);
                    e.set_global(4u32, node);
                }
            }
            break;
        }
        previous = node;
        node = vget(e, entry, ActivePass::pNext);
    }
}

/// A new active entry (eight bytes from the memory manager) with `value`, or 0.
#[inline(always)]
fn new_entry(e: &mut Engine, value: u32) -> u32 {
    let added = e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, 8u32]).u32();
    if added != 0 {
        let entry = Ptr::<ActivePass>::new(added);
        vset16(e, entry, ActivePass::usItem, low16(value));
        vset(e, entry, ActivePass::pNext, 0);
    }
    added
}

#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn low16(word: u32) -> u16 {
    word as u16
}

// Typed from 00b99a90
/// `thiscall(group, list, pass)`: prepends the pass to the group's list; both counts go up.
pub fn group_prepend(e: &mut Engine, this: Ptr<PassGroup>, list: u32, pass: Ptr<RenderPass>) {
    let list = Ptr::<PassList>::new(this.addr().wrapping_add(list.wrapping_mul(0x14)));
    let node = take_node(e, list);
    e.set(node, PassNode::pItem, pass.addr());
    let head = vget(e, list, PassList::pHead);
    e.set(node, PassNode::pNext, head);
    e.set(node, PassNode::pPrev, 0);
    let head = vget(e, list, PassList::pHead);
    if head != 0 {
        e.set(Ptr::<PassNode>::new(head), PassNode::pPrev, node.addr());
    } else {
        vset(e, list, PassList::pTail, node.addr());
    }
    vset(e, list, PassList::pHead, node.addr());
    count_up(e, this, list);
}

// Typed from 00b99b00
/// `thiscall(group, list, pass)`: appends the pass to the group's list; both counts go up.
pub fn group_append(e: &mut Engine, this: Ptr<PassGroup>, list: u32, pass: Ptr<RenderPass>) {
    let list = Ptr::<PassList>::new(this.addr().wrapping_add(list.wrapping_mul(0x14)));
    let node = take_node(e, list);
    e.set(node, PassNode::pItem, pass.addr());
    e.set(node, PassNode::pNext, 0);
    let tail = vget(e, list, PassList::pTail);
    e.set(node, PassNode::pPrev, tail);
    let tail = vget(e, list, PassList::pTail);
    if tail != 0 {
        e.set(Ptr::<PassNode>::new(tail), PassNode::pNext, node.addr());
    } else {
        vset(e, list, PassList::pHead, node.addr());
    }
    vset(e, list, PassList::pTail, node.addr());
    count_up(e, this, list);
}

#[inline(always)]
fn count_up(e: &mut Engine, group: Ptr<PassGroup>, list: Ptr<PassList>) {
    let count = vget(e, list, PassList::uiCount).wrapping_add(1);
    vset(e, list, PassList::uiCount, count);
    let total = vget(e, group, PassGroup::uiCount).wrapping_add(1);
    vset(e, group, PassGroup::uiCount, total);
}
