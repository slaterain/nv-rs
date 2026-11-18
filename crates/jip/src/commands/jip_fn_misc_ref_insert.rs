//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE), its node insertion and model attachment:
//! `InsertNode`, `AttachModel` and `ModelHasBlock`, with JIP's maps of what each form gets
//! (`s_insertNodeMap`, `s_attachModelMap`, `internal/hooks.h`) and the functions that apply them
//! to a model (`DoInsertNode`, `DoAttachModel` and their helpers).
//!
//! A command registers its entry in the form's map (a base form, or a reference made in the
//! editor) and, asked to (`doInsert` bit 2) on a reference with 3D, applies it to the loaded
//! model now (the player's first-person model too). Every model the game builds later gets the
//! entries through JIP's model hooks (H08 `CreateObjectNodeHook`, H10 the biped and weapon slots):
//! those call [`do_insert_objects`], built here. `InsertNode` and `AttachModel` are registered with
//! those rows ([`AWAITING_H08_H10`]): without them an entry on a base form would never reach a
//! model. `ModelHasBlock` reads the maps and the 3D and is registered now.
//!
//! The maps: per form, per node name (a fixed string of the game's string table, or none for the
//! root; JIP's `Map<NiFixedString, ..>` orders them by the string's address), per object name (a
//! text, ordered by a case-sensitive byte compare, JIP's `StrCompareCS`) the name of what was
//! put in (a fixed string: the new node's name, or the attached model's root name once one was
//! made; 0 before). Each fixed string held here holds one count of the game's table, given back
//! when its entry goes. The maps are not saved: JIP rebuilds them by the mods' scripts each run.

use super::nif_nodes as nif;
use super::{type_of, with_guest_text, Handlers};
use crate::game_patches::gl::lights as gl_lights;
use crate::game_patches::gp;
use engine_core::prelude::*;
use nvse::{handled, CommandArgs};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x28df, model_has_block),
];

/// `InsertNode` and `AttachModel`, built but not registered until JIP's model hooks (rows H08
/// and H10) are in: without them an entry on a base form would never reach a model the game
/// builds. The rows' pull request adds this list to `commands::ALL`.
#[rustfmt::skip]
pub static AWAITING_H08_H10: Handlers = &[
    (0x28d3, insert_node),
    (0x28d4, attach_model),
];

/// `kHookFormFlag6_InsertNode` and `kHookFormFlag6_AttachModel` (`internal/hooks.h`) in a form's
/// `jipFormFlags6` (+6): set while the form has entries in the map, tested by the model hooks.
pub const INSERT_NODE: u8 = 1 << 6;
pub const ATTACH_MODEL: u8 = 1 << 7;

/// One node name's entries (`DataStrings`): object name, the put-in name (fixed string or 0).
pub type DataStrings = Vec<(Vec<u8>, u32)>;

/// A form's entries (`NodeNamesMap`): the node name (fixed string, 0 for the root) and its data,
/// in JIP's order.
pub type NodeNames = Vec<(u32, DataStrings)>;

#[derive(Default)]
struct Maps {
    insert: HashMap<u32, NodeNames>,
    attach: HashMap<u32, NodeNames>,
    /// Requests from another thread waiting for the main loop (`MainLoopAddCallback`), by key.
    pending: HashMap<u32, Request>,
    next_key: u32,
}

/// JIP's two maps and `s_insertObjects` (whether either has any form). Made at install.
#[derive(Default)]
pub struct InsertObjects {
    maps: Mutex<Maps>,
    any: AtomicBool,
}

fn maps(e: &mut Engine) -> std::sync::MutexGuard<'_, Maps> {
    e.ext_free::<InsertObjects>()
        .maps
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `s_insertObjects`: whether any form has entries (the model hooks' first test).
pub fn any_insert_objects(e: &mut Engine) -> bool {
    e.ext_free::<InsertObjects>().any.load(Ordering::Relaxed)
}

/// A copy of `form`'s entries in the insert (`insert`) or attach map, for the model hooks.
pub fn node_names(e: &mut Engine, insert: bool, form: u32) -> Option<NodeNames> {
    let m = maps(e);
    let map = if insert { &m.insert } else { &m.attach };
    map.get(&form).cloned()
}

/// Makes the slot (install, before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<InsertObjects>();
}

// ---- the game's objects and functions ----

/// `TESObjectREFR::baseForm` (+0x20), a form's id (+0xC) and JIP's flag byte (+6).
const REFR_BASE: u32 = 0x20;
const FORM_ID: u32 = 0x0c;
/// A form's vtable slots: `IsReference` (+0xF0, `return true` `008D0360` for a reference),
/// `IsBoundObject` (+0xE8), `GetModelPath`-bearing model (`TESModel` via `GetModelPath` below).
const VT_IS_REFERENCE: u32 = 0xf0;
const VT_IS_BOUND_OBJECT: u32 = 0xe8;
const RETURN_TRUE: u32 = 0x008d_0360;
/// An `NiAVObject`: name (+8), parent (+0x18), collision object (+0x1C), flags (+0x30; JIP tests
/// its fourth byte +0x33: 0x20 lights below, 0x40 lights made ready, 0x80 JIP's own block); a
/// node's children (`NiTArray`: data +0xA0, count +0xA6); its vtable: `AttachChild` (+0xDC: the
/// child, a bool), the child set at an index (+0xE0: the index, the child), `RemoveObject`
/// (+0xE8), the transform update (+0xC0: the update data), `return this` at +0xC for a node.
const PARENT: u32 = 0x18;
const COLLISION: u32 = 0x1c;
const FLAGS_HIGH: u32 = 0x33;
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;
const VT_ATTACH_CHILD: u32 = 0xdc;
const VT_SET_AT: u32 = 0xe0;
const VT_REMOVE_OBJECT: u32 = 0xe8;
const VT_UPDATE: u32 = 0xc0;
const LIGHTS_BELOW: u8 = 0x20;
const LIGHTS_READY: u8 = 0x40;
const JIP_BLOCK: u8 = 0x80;
/// `NiNode::Create`'s game half (`00A5F030`, a new empty node), the string table's
/// `AddString` (`00A5B690`), `ModelLoader` (`*011C3B3C`): its map's find (`004493D0`), a
/// model's queued load (`0043C6E0` the task's constructor: the task, the path, 0, 0, 1, 0; its
/// flag byte +0x3C bit 0x10, `0043CCF0` on the task made, `0043D180` the run, `0043C830` the
/// destructor; the model it loaded at +0x30 of the 0x44-byte task) and `LoadModel`
/// (`00447080`: the loader, the path, 0, 1, 0, 0, 1). A model: its count +8, root +0xC.
const NODE_CREATE: u32 = 0x00a5_f030;
const ADD_STRING: u32 = 0x00a5_b690;
const MODEL_LOADER: u32 = 0x011c_3b3c;
const MODEL_MAP_FIND: u32 = 0x0044_93d0;
const MODEL_TASK_CONSTRUCT: u32 = 0x0043_c6e0;
const MODEL_TASK_FLAG: u32 = 0x0043_ccf0;
const MODEL_TASK_RUN: u32 = 0x0043_d180;
const MODEL_TASK_DESTRUCT: u32 = 0x0043_c830;
const MODEL_TASK_SIZE: u32 = 0x44;
const MODEL_TASK_MODEL: u32 = 0x30;
const LOAD_MODEL: u32 = 0x0044_7080;
const MODEL_REFS: u32 = 0x08;
const MODEL_ROOT: u32 = 0x0c;
/// JIP's `meshesPath[0x100]` (`data\meshes\` then the path) and the command's text buffer
/// (`Pool_CAlloc(0x100)`: the form, the mode word, then the text from +8).
const MESHES: &[u8] = b"data\\meshes\\";
const MESHES_BUFFER: usize = 0x100;
const TEXT_BUFFER: usize = 0x100 - 8;
/// `ModelHasBlock`'s `buffer[0x80]` (a `^` first).
const BLOCK_BUFFER: usize = 0x80 - 1;
/// `kCommandInfo_InsertNode.numParams` (and `AttachModel`'s, `ModelHasBlock`'s): the leading
/// arguments and xNVSE's 21 format parameters.
const INSERT_PARAMS: u32 = 23;
const HAS_BLOCK_PARAMS: u32 = 22;
/// `kInventoryType`.
use super::forms::INVENTORY_TYPE;

fn is_reference(e: &Engine, form: u32) -> bool {
    form != 0 && {
        let vtbl = e.mem.u32(form);
        vtbl != 0 && e.mem.u32(vtbl + VT_IS_REFERENCE) == RETURN_TRUE
    }
}

fn is_bound_object(e: &mut Engine, form: u32) -> bool {
    let vtbl = e.mem.u32(form);
    let f = e.mem.u32(vtbl + VT_IS_BOUND_OBJECT);
    e.call(f, &[form]).eax & 0xff != 0
}

fn is_node(e: &Engine, object: u32) -> bool {
    nif::is_node(e, object)
}

fn flags_high(e: &Engine, object: u32) -> u8 {
    e.mem.u8(object + FLAGS_HIGH)
}

fn set_flags_high(e: &Engine, object: u32, bits: u8) {
    let f = flags_high(e, object);
    e.mem.set_u8(object + FLAGS_HIGH, f | bits);
}

/// The text of a fixed string (empty for 0).
fn fixed_text(e: &Engine, s: u32) -> Vec<u8> {
    if s == 0 {
        Vec::new()
    } else {
        nvse::table::guest_text(e, s)
    }
}

/// `GetNiFixedString(text)`: the table's string with one count more.
fn fixed(e: &mut Engine, text: &[u8]) -> u32 {
    with_guest_text(e, text, |e, at| e.call(ADD_STRING, &[at]).eax)
}

/// A fixed string's count down (`NiFixedString`'s release); nothing for 0.
fn release(e: &Engine, s: u32) {
    super::jip_fn_misc_ref_nodes_w::release_name(e, s);
}

/// A fixed string's count up (`lock inc [s-8]`); nothing for 0.
fn add_ref(e: &Engine, s: u32) {
    if s == 0 {
        return;
    }
    if let Some(count) = e.mem.atomic_u32(s.wrapping_sub(8)) {
        count.fetch_add(1, Ordering::SeqCst);
    }
}

/// `NiNode::GetBlockByName` with a fixed string: the first block below `node` named so (0 for a
/// name 0).
fn block_by_name(e: &Engine, node: u32, name: u32) -> u32 {
    if name == 0 || node == 0 {
        return 0;
    }
    let text = fixed_text(e, name);
    let block = nif::get_block(e, node, &text);
    if block == node {
        // `GetBlockByName` looks only below the node.
        let mut found = 0;
        for child in nif::children(e, node) {
            found = nif::get_block(e, child, &text);
            if found != 0 {
                break;
            }
        }
        found
    } else {
        block
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::Create (asm).
/// A new node (the game's constructor) named `name` (its count raised).
fn node_create(e: &mut Engine, name: u32) -> u32 {
    let node = e.call(NODE_CREATE, &[]).eax;
    if name != 0 {
        add_ref(e, name);
        e.mem.set_u32(node + nif::NAME, name);
    }
    node
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiAVObject::GetIndex
// (asm).
/// The object's place in its parent's children as JIP counts it: from the last down, the place
/// plus one where it is found; 0 with no parent, no children or not found.
fn index_in_parent(e: &Engine, object: u32) -> u32 {
    let parent = e.mem.u32(object + PARENT);
    if parent == 0 {
        return 0;
    }
    let data = e.mem.u32(parent + CHILDREN);
    let mut i = u32::from(e.mem.u16(parent + CHILD_COUNT));
    while i != 0 {
        if e.mem.u32(data + 4 * (i - 1)) == object {
            return i;
        }
        i -= 1;
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoInsertNode (asm).
/// For an object name without `^`: under `target` (a node), a new node named `name_str` (flagged
/// JIP's), unless the root already has a block of that name. With `^`: a node named `name_str`
/// becomes `target`'s parent: a block of that name under the root (a node, not already its
/// parent) takes it, else a new one takes `target`'s place in its parent and then `target`;
/// nothing when `target` is the root.
pub fn do_insert_node(e: &mut Engine, target: u32, object_name: &[u8], name_str: u32, root: u32) {
    if object_name.first() != Some(&b'^') {
        if !is_node(e, target) || block_by_name(e, root, name_str) != 0 {
            return;
        }
        let node = node_create(e, name_str);
        set_flags_high(e, node, JIP_BLOCK);
        e.vcall(target, VT_ATTACH_CHILD, &[node, 1]);
        return;
    }
    if root == target {
        return;
    }
    let found = block_by_name(e, root, name_str);
    if found == 0 {
        let node = node_create(e, name_str);
        set_flags_high(e, node, JIP_BLOCK);
        let index = index_in_parent(e, target);
        let parent = e.mem.u32(target + PARENT);
        e.vcall(parent, VT_SET_AT, &[index, node]);
        e.vcall(node, VT_ATTACH_CHILD, &[target, 1]);
    } else if e.mem.u32(target + PARENT) != found && is_node(e, found) {
        e.vcall(found, VT_ATTACH_CHILD, &[target, 1]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: LoadModelCopy (asm).
/// A copy of the model at `path` (`meshes\` relative): found in the loader's map, else loaded
/// through a queued model task run at once; its count raised when it is 0, its lights made ready
/// once (`InitPointLights`), then copied (`CreateCopy`). 0 when it does not load.
pub fn load_model_copy(e: &mut Engine, path: &[u8]) -> u32 {
    let model = with_guest_text(e, path, |e, text| {
        let map = e.mem.u32(e.mem.u32(MODEL_LOADER));
        let found = e.with_stack(4, |e, out| {
            e.mem.set_u32(out.addr(), 0);
            let ok = e.call(MODEL_MAP_FIND, &args![map, text, out.addr()]).eax & 0xff != 0;
            (ok, e.mem.u32(out.addr()))
        });
        if found.0 {
            return found.1;
        }
        e.with_stack(MODEL_TASK_SIZE, |e, task| {
            let task = task.addr();
            let made = e
                .call(
                    MODEL_TASK_CONSTRUCT,
                    &args![task, text, 0u32, 0u32, 1u32, 0u32],
                )
                .eax;
            let flag = e.mem.u8(made + 0x3c);
            e.mem.set_u8(made + 0x3c, flag | 0x10);
            e.call(MODEL_TASK_FLAG, &args![made]);
            e.call(MODEL_TASK_RUN, &args![task]);
            e.call(MODEL_TASK_DESTRUCT, &args![task]);
            e.mem.u32(task + MODEL_TASK_MODEL)
        })
    });
    if model == 0 {
        return 0;
    }
    if e.mem.u32(model + MODEL_REFS) == 0 {
        if let Some(count) = e.mem.atomic_u32(model + MODEL_REFS) {
            count.fetch_add(1, Ordering::SeqCst);
        }
    }
    let node = e.mem.u32(model + MODEL_ROOT);
    if node == 0 {
        return 0;
    }
    if flags_high(e, node) & LIGHTS_READY == 0 {
        set_flags_high(e, node, LIGHTS_READY);
        gl_lights::init_point_lights(e, node, node);
    }
    gp::create_copy(e, node)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiObjectNET::SetName
// (asm).
/// The object renamed to `text` (the old name's count given back unless it is the same string).
fn set_name(e: &mut Engine, object: u32, text: &[u8]) {
    let s = fixed(e, text);
    let old = e.mem.u32(object + nif::NAME);
    if s == old {
        release(e, s);
    } else {
        release(e, old);
        e.mem.set_u32(object + nif::NAME, s);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: AppendNameSuffix and
// AppendBlockNameSuffixes (asm).
/// Every block's name (the node's and all below) gets `suffix` after its first 0x20 bytes at
/// most; a block with no name or an empty one is left.
fn append_suffixes(e: &mut Engine, node: u32, suffix: &[u8]) {
    let name = e.mem.u32(node + nif::NAME);
    if name != 0 {
        let len = e.mem.u32(name - 4);
        if len != 0 {
            let keep = len.min(0x20) as usize;
            let mut text = e.mem.bytes(name, keep as u32);
            text.extend_from_slice(suffix);
            set_name(e, node, &text);
        }
    }
    if is_node(e, node) {
        let data = e.mem.u32(node + CHILDREN);
        let count = u32::from(e.mem.u16(node + CHILD_COUNT));
        for i in 0..count {
            let child = e.mem.u32(data + 4 * i);
            if child != 0 {
                append_suffixes(e, child, suffix);
            }
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::RemoveCollision
// (asm).
/// The node's collision object let go (its count down, deleted at 0) and the same for every node
/// below.
fn remove_collision(e: &mut Engine, node: u32) {
    let col = e.mem.u32(node + COLLISION);
    if col != 0 {
        e.mem.set_u32(node + COLLISION, 0);
        let left = match e.mem.atomic_u32(col + 4) {
            Some(c) => c.fetch_sub(1, Ordering::SeqCst).wrapping_sub(1),
            None => 0,
        };
        if left as i32 <= 0 {
            e.vcall(col, 4, &[]);
        }
    }
    let data = e.mem.u32(node + CHILDREN);
    let count = u32::from(e.mem.u16(node + CHILD_COUNT));
    for i in 0..count {
        let child = e.mem.u32(data + 4 * i);
        if child != 0 && is_node(e, child) {
            remove_collision(e, child);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoAttachModel (asm).
/// The model `model_path` (a `*suffix*` first: every block name of the copy gets the suffix, 16
/// bytes at most) copied ([`load_model_copy`]) and attached under `target`: the copy's root name
/// kept in `name_slot` when it holds none (a count taken), its collision removed, the target's
/// transforms updated, the copy flagged JIP's; when it has lights below, its parents up to the
/// root are marked so (stopping at one already marked). Answers the copy, 0 when it did not load.
pub fn do_attach_model(
    e: &mut Engine,
    target: u32,
    model_path: &[u8],
    name_slot: &mut u32,
    root: u32,
) -> u32 {
    let (path, suffix) = if model_path.first() == Some(&b'*') {
        let rest = &model_path[1..];
        let at = rest.iter().position(|&b| b == b'*').unwrap_or(rest.len());
        let len = at.min(0x10);
        let after = (at + 1).min(rest.len());
        (&rest[after..], Some(&rest[..len]))
    } else {
        (model_path, None)
    };
    let node = load_model_copy(e, path);
    if node == 0 {
        return 0;
    }
    if let Some(suffix) = suffix.filter(|s| !s.is_empty()) {
        append_suffixes(e, node, suffix);
    }
    if *name_slot == 0 {
        let name = e.mem.u32(node + nif::NAME);
        add_ref(e, name);
        *name_slot = name;
    }
    remove_collision(e, node);
    e.vcall(target, VT_ATTACH_CHILD, &[node, 1]);
    e.with_stack(0xc, |e, data| {
        for i in 0..3 {
            e.mem.set_u32(data.addr() + 4 * i, 0);
        }
        e.vcall(target, VT_UPDATE, &[data.addr()]);
    });
    set_flags_high(e, node, JIP_BLOCK);
    if flags_high(e, node) & LIGHTS_BELOW != 0 {
        let mut at = node;
        loop {
            at = e.mem.u32(at + PARENT);
            if at == 0 || flags_high(e, at) & LIGHTS_BELOW != 0 {
                break;
            }
            set_flags_high(e, at, LIGHTS_BELOW);
            if at == root {
                break;
            }
        }
    }
    node
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoInsertNodes, DoAttachModels
// and DoInsertObjects (asm).
/// What the model hooks call for a model being built: for each of the two forms (the second may
/// be 0) flagged with entries, the insert map's then the attach map's entries applied under
/// `root` (an entry's node by name, the root for none; the attach map's only to a node).
pub fn do_insert_objects(e: &mut Engine, form1: u32, form2: u32, root: u32) {
    let forms = [form1, form2];
    for (insert, flag) in [(true, INSERT_NODE), (false, ATTACH_MODEL)] {
        for &form in forms.iter().filter(|&&f| f != 0) {
            if e.mem.u8(form + 6) & flag == 0 {
                continue;
            }
            let Some(names) = node_names(e, insert, form) else {
                continue;
            };
            for (node_name, data) in names {
                let dest = if node_name == 0 || e.mem.u32(root + nif::NAME) == node_name {
                    root
                } else {
                    let found = block_by_name(e, root, node_name);
                    // The attach map's block found by name must be a node.
                    if found == 0 || (!insert && !is_node(e, found)) {
                        continue;
                    }
                    found
                };
                for (object, value) in data {
                    if insert {
                        do_insert_node(e, dest, &object, value, root);
                    } else {
                        let mut slot = value;
                        do_attach_model(e, dest, &object, &mut slot, root);
                        if slot != value {
                            set_value(e, form, node_name, &object, slot);
                        }
                    }
                }
            }
        }
    }
}

/// The attach map's put-in name for one entry, kept when it had none (the hook path's
/// `*pDataStr` write); a count taken for it by [`do_attach_model`] is given back otherwise.
fn set_value(e: &mut Engine, form: u32, node_name: u32, object: &[u8], value: u32) {
    let mut m = maps(e);
    let stored = m
        .attach
        .get_mut(&form)
        .and_then(|n| n.iter_mut().find(|(k, _)| *k == node_name))
        .and_then(|(_, d)| d.iter_mut().find(|(k, _)| k == object));
    match stored {
        Some((_, v)) if *v == 0 => *v = value,
        _ => {
            drop(m);
            release(e, value);
        }
    }
}

/// One registration (`dataStr`): the form, the mode byte (`doInsert`: bit 1 add, bit 2 apply
/// now; 3 both), the flag (insert or attach) and the text (`[node|]object`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub form: u32,
    pub mode: u8,
    pub flag: u8,
    pub text: Vec<u8>,
}

/// The insertion position of `key` in a sorted list by `cmp`.
fn sorted_find<K, V>(
    list: &[(K, V)],
    key: &K,
    cmp: impl Fn(&K, &K) -> std::cmp::Ordering,
) -> Result<usize, usize> {
    list.binary_search_by(|(k, _)| cmp(k, key))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// RegisterInsertObject.
/// The request added to (mode bit 1) or taken from the form's map, and applied to or taken off
/// the reference's 3D now (mode bit 2). A reference made in game or of an inventory item is only
/// ever applied now (mode 3, with 3D), never kept; a base form must be a bound object. An attach
/// needs its model file (`data\meshes\`, loose or archived). False when nothing was done.
pub fn register_insert_object(e: &mut Engine, req: &Request) -> bool {
    let form = req.form;
    let refr = if is_reference(e, form) { form } else { 0 };
    let mut root = if refr != 0 {
        nif::ref_ni_node(e, refr)
    } else {
        0
    };
    let mut modify = true;
    if refr != 0 {
        let base = e.mem.u32(refr + REFR_BASE);
        let created = e.mem.u32(refr + FORM_ID) >> 24 == 0xff;
        if created || INVENTORY_TYPE.get(usize::from(type_of(e, base))) == Some(&true) {
            if req.mode != 3 || root == 0 {
                return false;
            }
            modify = false;
        }
    } else if form == 0 || !is_bound_object(e, form) {
        return false;
    }
    let text = &req.text[..req
        .text
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(req.text.len())];
    let (node_name, object) = match text.iter().position(|&b| b == b'|') {
        Some(i) => (Some(&text[..i]), &text[i + 1..]),
        None => (None, text),
    };
    if object.is_empty() {
        return false;
    }
    let insert = req.flag == INSERT_NODE;
    if req.mode & 1 != 0 {
        if !insert {
            let suffix = if object[0] == b'*' {
                match object
                    .get(2..)
                    .and_then(|r| r.iter().position(|&b| b == b'*'))
                {
                    Some(i) => &object[2 + i + 1..],
                    None => return false,
                }
            } else {
                object
            };
            let mut path = MESHES.to_vec();
            path.extend_from_slice(&suffix[..suffix.len().min(MESHES_BUFFER - MESHES.len() - 1)]);
            if super::ln_fn_utility::file_exists_ex(e, &mut path, false) == 0 {
                return false;
            }
        }
        // `NiFixedString blockName(nodeName)`: a temporary count, given back at the end.
        let block = node_name.map_or(0, |n| fixed(e, n));
        let mut temp = 0u32;
        let stored = if modify {
            let mut m = maps(e);
            let map = if insert { &mut m.insert } else { &mut m.attach };
            let new_form = !map.contains_key(&form);
            let names = map.entry(form).or_default();
            let mut new_key = false;
            let at = match sorted_find(names, &block, |a, b| a.cmp(b)) {
                Ok(i) => i,
                Err(i) => {
                    new_key = true;
                    names.insert(i, (block, Vec::new()));
                    i
                }
            };
            let data = &mut names[at].1;
            let key = object.to_vec();
            let added = match sorted_find(data, &key, |a, b| a.as_slice().cmp(b.as_slice())) {
                Ok(_) => false,
                Err(i) => {
                    data.insert(i, (key, 0));
                    true
                }
            };
            drop(m);
            if new_key {
                // The map's key holds a count of its own.
                add_ref(e, block);
            }
            if new_form {
                crate::world_hooks::set_jip_flag(e, form, req.flag, true);
            }
            if !added {
                release(e, block);
                return false;
            }
            true
        } else {
            false
        };
        let mut value = 0u32;
        if insert {
            let name = if object[0] == b'^' {
                &object[1..]
            } else {
                object
            };
            value = fixed(e, name);
        }
        if root != 0 && req.mode & 2 != 0 {
            let use_root = block == 0 || e.mem.u32(root + nif::NAME) == block;
            let target = if use_root {
                root
            } else {
                block_by_name(e, root, block)
            };
            if target != 0 {
                if insert {
                    do_insert_node(e, target, object, value, root);
                } else {
                    let made = do_attach_model(e, target, object, &mut value, root);
                    if made != 0 && flags_high(e, made) & LIGHTS_BELOW != 0 {
                        gl_lights::add_point_lights(e, made);
                    }
                }
            }
            if nif::is_player(e, refr) {
                root = gp::first_person_node(e);
                if root != 0 {
                    let target = if use_root {
                        root
                    } else {
                        block_by_name(e, root, block)
                    };
                    if target != 0 {
                        if insert {
                            do_insert_node(e, target, object, value, root);
                        } else {
                            do_attach_model(e, target, object, &mut value, root);
                        }
                    }
                }
            }
        }
        if stored {
            set_entry_value(e, insert, form, block, object, value);
        } else {
            temp = value;
        }
        release(e, temp);
        release(e, block);
    } else {
        let block = node_name.map_or(0, |n| fixed(e, n));
        let found = {
            let m = maps(e);
            let map = if insert { &m.insert } else { &m.attach };
            map.get(&form)
                .and_then(|n| n.iter().find(|(k, _)| *k == block))
                .and_then(|(_, d)| d.iter().find(|(k, _)| k.as_slice() == object))
                .map(|(_, v)| *v)
        };
        release(e, block);
        let Some(value) = found else {
            return false;
        };
        if root != 0 && req.mode & 2 != 0 && value != 0 && (!insert || object[0] != b'^') {
            let object_node = block_by_name(e, root, value);
            if object_node != 0 {
                let parent = e.mem.u32(object_node + PARENT);
                e.vcall(parent, VT_REMOVE_OBJECT, &[object_node]);
            }
            if nif::is_player(e, refr) {
                root = gp::first_person_node(e);
                if root != 0 {
                    let object_node = block_by_name(e, root, value);
                    if object_node != 0 {
                        let parent = e.mem.u32(object_node + PARENT);
                        e.vcall(parent, VT_REMOVE_OBJECT, &[object_node]);
                    }
                }
            }
        }
        let (gone, form_empty) = {
            let mut m = maps(e);
            let map = if insert { &mut m.insert } else { &mut m.attach };
            let mut gone = vec![value];
            let mut form_empty = false;
            if let Some(names) = map.get_mut(&form) {
                if let Some(at) = names.iter().position(|(k, _)| *k == block) {
                    names[at].1.retain(|(k, _)| k.as_slice() != object);
                    if names[at].1.is_empty() {
                        gone.push(names.remove(at).0);
                    }
                }
                form_empty = names.is_empty();
            }
            if form_empty {
                map.remove(&form);
            }
            (gone, form_empty)
        };
        for s in gone {
            release(e, s);
        }
        if form_empty {
            crate::world_hooks::set_jip_flag(e, form, req.flag, false);
        }
    }
    let any = {
        let m = maps(e);
        !m.insert.is_empty() || !m.attach.is_empty()
    };
    e.ext_free::<InsertObjects>()
        .any
        .store(any, Ordering::Relaxed);
    true
}

/// The stored entry's put-in name set (the register path's `*pDataStr`).
fn set_entry_value(e: &mut Engine, insert: bool, form: u32, block: u32, object: &[u8], value: u32) {
    let mut m = maps(e);
    let map = if insert { &mut m.insert } else { &mut m.attach };
    if let Some((_, v)) = map
        .get_mut(&form)
        .and_then(|n| n.iter_mut().find(|(k, _)| *k == block))
        .and_then(|(_, d)| d.iter_mut().find(|(k, _)| k.as_slice() == object))
    {
        *v = value;
    }
}

/// The main loop's step for requests made on another thread (JIP's `MainLoopAddCallback(
/// RegisterInsertObject, dataStr)`): the request kept under `key`.
pub fn run_pending(e: &mut Engine, key: u32) {
    let req = maps(e).pending.remove(&key);
    if let Some(req) = req {
        register_insert_object(e, &req);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_InsertNode_Execute and Cmd_AttachModel_Execute.
/// The form, the mode word and the text read (the text after the form and mode in JIP's
/// 0x100-byte block); the mode's low byte kept, the flag set; applied now on the main thread
/// (result 1 when done), or, when it asks for the 3D on another thread, at the next main loop.
fn insert_or_attach(e: &mut Engine, w: &[u32], flag: u8, command: &str) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 2, INSERT_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let form = f
        .leading
        .first()
        .and_then(nvse::args::Arg::form)
        .unwrap_or(0);
    let mode = f.leading.get(1).and_then(nvse::args::Arg::int).unwrap_or(0) as u8;
    let text = super::formatted_in_buffer(e, &f.text, TEXT_BUFFER, command);
    let req = Request {
        form,
        mode,
        flag,
        text,
    };
    let main = engine_core::thread_host::current_id(e) == engine_core::thread_host::MAIN_THREAD_ID;
    if mode & 2 == 0 || main {
        if register_insert_object(e, &req) {
            a.set_result(e, 1.0);
        }
    } else {
        let key = {
            let mut m = maps(e);
            m.next_key = m.next_key.wrapping_add(1);
            let key = m.next_key;
            m.pending.insert(key, req);
            key
        };
        crate::main_loop::add_native(e, crate::main_loop::Native::RegisterInsertObject, key);
    }
    handled(true)
}

fn insert_node(e: &mut Engine, w: &[u32]) -> Ret {
    insert_or_attach(e, w, INSERT_NODE, "InsertNode")
}

fn attach_model(e: &mut Engine, w: &[u32]) -> Ret {
    insert_or_attach(e, w, ATTACH_MODEL, "AttachModel")
}

/// `kXData_ExtraLeveledCreature` and its base form (+0x10); an actor base's `IsActor` slot
/// (+0xF8, `return true`).
const EXTRA_LEVELED_CREATURE: u32 = 0x2e;
const LEVELED_BASE: u32 = 0x10;
const VT_IS_ACTOR_BASE: u32 = 0xf8;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetBaseForm2 (asm).
/// The reference's base form; for a base made in game (`0xFF` file), an actor base's leveled
/// creature extra's base (0 without one), else 0.
fn base_form2(e: &mut Engine, refr: u32) -> u32 {
    let base = e.mem.u32(refr + REFR_BASE);
    if base == 0 || e.mem.u8(base + 0xf) != 0xff {
        return base;
    }
    if e.mem.u32(e.mem.u32(base) + VT_IS_ACTOR_BASE) != RETURN_TRUE {
        return 0;
    }
    let extra = super::extra::refr_extra(e, refr, EXTRA_LEVELED_CREATURE);
    if extra == 0 {
        0
    } else {
        e.mem.u32(extra + LEVELED_BASE)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetModelPath (asm).
/// The text of the form's model (`kBaseComponentOffsets`' byte 3: its `TESModel`, the path's
/// text at +4); `None` for a type without one, no text or an empty one.
fn model_path_of(e: &mut Engine, form: u32) -> Option<Vec<u8>> {
    let model = super::model_component(e, form);
    if model == 0 {
        return None;
    }
    let text = e.mem.u32(model + 4);
    if text == 0 || e.mem.u8(text) == 0 {
        return None;
    }
    Some(nvse::table::guest_text(e, text))
}

/// Whether `form`'s insert map entries hold `name` or `^name` as an object name.
fn insert_map_has(e: &mut Engine, form: u32, name: &[u8]) -> bool {
    let mut caret = vec![b'^'];
    caret.extend_from_slice(name);
    let m = maps(e);
    m.insert.get(&form).is_some_and(|names| {
        names
            .iter()
            .any(|(_, d)| d.iter().any(|(k, _)| k.as_slice() == name || *k == caret))
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ModelHasBlock_Execute.
/// `ModelHasBlock form "name"`: 1 when the reference's model has a block of that name, or the
/// form's (then the reference's base form's) inserted nodes name it (with or without `^`), or,
/// for a form with no 3D that is a bound object, its model file (loaded through the game's
/// `LoadModel`) has it.
fn model_has_block(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, HAS_BLOCK_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let mut form = f
        .leading
        .first()
        .and_then(nvse::args::Arg::form)
        .unwrap_or(0);
    let name = super::formatted_in_buffer(e, &f.text, BLOCK_BUFFER, "ModelHasBlock");
    let refr = if is_reference(e, form) { form } else { 0 };
    let mut root = if refr != 0 { nif::ni_node(e, refr) } else { 0 };
    if root != 0 && nif::get_block(e, root, &name) != 0 {
        a.set_result(e, 1.0);
        return handled(true);
    }
    if form != 0 && insert_map_has(e, form, &name) {
        a.set_result(e, 1.0);
        return handled(true);
    }
    if refr != 0 {
        form = base_form2(e, refr);
        if form == 0 {
            return handled(true);
        }
        if insert_map_has(e, form, &name) {
            a.set_result(e, 1.0);
            return handled(true);
        }
    }
    if root == 0 && form != 0 && is_bound_object(e, form) {
        if let Some(path) = model_path_of(e, form) {
            root = with_guest_text(e, &path, |e, text| {
                let loader = e.mem.u32(MODEL_LOADER);
                e.call(
                    LOAD_MODEL,
                    &args![loader, text, 0u32, 1u32, 0u32, 0u32, 1u32],
                )
                .eax
            });
            if root != 0 && nif::get_block(e, root, &name) != 0 {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_insert_tests.rs"]
mod tests;
