//! Selected original kNVSE24 API, official4640f0f (2023-12-06),
//! commands_animation.cpp::CreateCommands. No later40 timing/switch behavior
//! is implied. These commands execute synchronously in24; there is no queue.
use engine_core::prelude::*;
use nvse::{
    commands::{arg_in_buffer, dynamic_cast, rtti, Read},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
const PLAYER: u32 = 0x011d_ea3c;

pub(crate) fn install(e: &mut Engine) {
    super::loader::install(e);
    for (op, name, parameters, execute) in [
        (
            0x3933,
            "ActivateAnim",
            &[
                ("anim sequence path", 0, false),
                ("bIsFirstPerson", 1, false),
                ("iPriority", 1, true),
                ("bStartOver", 1, true),
                ("fWeight", 2, true),
                ("fEaseInTime", 2, true),
                ("time sync sequence path", 1, true),
            ][..],
            activate as AbiFn,
        ),
        (
            0x3934,
            "DeactivateAnim",
            &[("anim sequence path", 0, false), ("fEaseOutTime", 2, true)][..],
            deactivate as AbiFn,
        ),
    ] {
        let params = table::alloc(e, parameters.len() as u32 * 12);
        for (i, &(name, kind, optional)) in parameters.iter().enumerate() {
            let at = params + i as u32 * 12;
            let name = table::guest_str(e, name.as_bytes());
            e.mem.set_u32(at, name);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
        }
        let address = table::execute_address(op);
        e.register(address, execute);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short: "",
                needs_ref: true,
                param_count: parameters.len() as u16,
                params,
                execute: address,
                eval: 0,
                flags: 0,
                expression: false,
                returns: Returns::Default,
            },
        );
    }
}

fn third_person(e: &mut Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + 0x68);
    e.vcall(process, 0x1b8, &[]).u32() // BaseProcess::GetAnimData virtual6E
}
fn first_person(e: &Engine, actor: u32) -> bool {
    actor == e.mem.u32(PLAYER) && e.mem.u8(actor + 0x64c) == 0
}
fn activate(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = arg_in_buffer(e, &read, 0, 0x400, "ActivateAnim");
    let mut first = read.int(1, u32::MAX);
    let priority = read.int(2, 0);
    let start = read.int(3, 1);
    let weight = read.float(4, f32::MIN_POSITIVE);
    let ease = read.float(5, 0.0);
    // The original24 metadata mistakenly declares arg7 Integer, while its
    // extractor destination is a zeroed char[400]. Preserve that precise
    // contract: its four written bytes are interpreted as a NUL-ended path.
    // EM's supplied scripts never pass this argument; no corrected40 API is
    // silently advertised through24 metadata.
    let sync = read.int(6, 0).to_le_bytes();
    let sync = &sync[..sync.iter().position(|&b| b == 0).unwrap_or(sync.len())];
    let actor = dynamic_cast(e, a.this_obj, rtti::TES_OBJECT_REFR, rtti::ACTOR);
    if actor == 0 {
        return handled(true);
    }
    if first == u32::MAX {
        first = u32::from(first_person(e, actor));
    }
    // Original24 selects the player's1P data for any first-person request.
    let animation = if first != 0 {
        e.mem.u32(e.mem.u32(PLAYER) + 0x690)
    } else {
        third_person(e, actor)
    };
    if animation == 0 {
        return handled(true);
    }
    let sequence = super::loader::find_or_load(e, animation, &path);
    if sequence == 0 {
        return handled(true);
    }
    let weight = if weight == f32::MIN_POSITIVE {
        e.mem.f32(sequence + 0x1c)
    } else {
        weight
    };
    let sync_sequence = if sync.is_empty() {
        0
    } else {
        super::loader::find_or_load(e, animation, sync)
    };
    if !sync.is_empty() && sync_sequence == 0 {
        return handled(true);
    }
    let manager = e.mem.u32(sequence + 0x40);
    // No pre-deactivation/restart in this initial24 source. The actual game
    // manager/sequence decides whether this active sequence can activate.
    let result = e
        .call(
            0x0047_aab0,
            &args![
                manager,
                sequence,
                priority,
                start,
                weight,
                ease,
                sync_sequence
            ],
        )
        .bool();
    a.set_result(e, f64::from(u32::from(result)));
    handled(true)
}

fn find_active(e: &mut Engine, animation: u32, path: &[u8]) -> u32 {
    if let Some(cached) = super::loader::cached(e, animation, path) {
        return cached.sequence;
    }
    // Actor overrides enter through the scoped animation-map loader; this
    // fallback searches the native map after checking the loaded sequences.
    super::loader::find(e, e.mem.u32(animation + 0xd8), path)
}
fn deactivate(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = arg_in_buffer(e, &read, 0, 0x400, "DeactivateAnim");
    let ease = read.float(1, 0.0);
    let actor = dynamic_cast(e, a.this_obj, rtti::TES_OBJECT_REFR, rtti::ACTOR);
    if actor == 0 {
        return handled(true);
    }
    // GetAnimationByPath in24 asks ModelLoader even for a previously loaded
    // actor sequence, then searches third-person first, not the visible POV.
    let model = super::loader::kf(e, &path);
    if model == 0 {
        return handled(true);
    }
    let source = e.mem.u32(model + 4);
    if source == 0 {
        return handled(true);
    }
    let path = table::guest_text(e, e.mem.u32(source + 8));
    let animation = third_person(e, actor);
    if animation == 0 {
        return handled(true);
    }
    let mut sequence = find_active(e, animation, &path);
    if sequence == 0 && actor == e.mem.u32(PLAYER) {
        let first = e.mem.u32(actor + 0x690);
        // Source assumes the player has its first-person Animation when here.
        sequence = find_active(e, first, &path);
    }
    if sequence != 0 {
        let owner = e.mem.u32(sequence + 0x40);
        let result = e.call(0x0047_b220, &args![owner, sequence, ease]).bool();
        a.set_result(e, f64::from(u32::from(result)));
    }
    handled(true)
}

#[cfg(test)]
mod tests;
