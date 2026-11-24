//! Selected commands from korri123/kNVSE at
//! 9efea363ca2a209241e45e8b1fb518c3eff88b3f (source version 40).
//! `IsAnimPlayingAlt` follows that source. The selected original24 playback
//! family follows official4640f0f, as documented in `playback`. No kNVSE
//! plugin/version is advertised. Actor/folder overrides follow the same24 pin;
//! unsupported text-key/variant families are diagnosed explicitly.
use engine_core::prelude::*;
use nvse::{
    commands::{dynamic_cast, rtti, Read},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};

// main.cpp: SetOpcodeBase(3920), nine REG_CMD entries, then CreateCommands.
// commands_animation.cpp: IsAnimPlayingAlt is CreateCommands' 28th entry.
const IS_ANIM_PLAYING_ALT: u32 = 0x3944;
const PLAYER: u32 = 0x011d_ea3c;
// commands_animation.cpp: g_animGroupInfos; NiNodes.h: AnimGroupInfo (24 bytes hex).
const GROUP_INFOS: u32 = 0x0119_77d8;
const GROUP_COUNT: u32 = 245;
const GROUP_INFO_SIZE: u32 = 0x24;
const GROUP_SEQUENCE_TYPE: u32 = 8;
// GameObjects.h/.cpp, GameProcess.h and NiNodes.h (upstream xNVSE layouts).
const PROCESS: u32 = 0x68;
const THIRD_PERSON_CONSISTENT: u32 = 0x64c;
const FIRST_PERSON_ANIMATION: u32 = 0x690;
const PROCESS_ANIMATION: u32 = 0x1c0;
const VT_GET_ANIMATION: u32 = 0x1e4;
const CURRENT_SEQUENCES: u32 = 0xe0;
const SEQUENCE_STATE: u32 = 0x44;
const SEQUENCE_GROUP: u32 = 0x74;
const GROUP_ID: u32 = 0x10;

/// Install this command without listing the incomplete kNVSE plugin.
pub fn install(e: &mut Engine) {
    table::grow_to(e, IS_ANIM_PLAYING_ALT + 1);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"anim group");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 0x0a); // kParamType_AnimationGroup: immediate UInt16.
    e.mem.set_u32(params + 8, 0);
    let execute = table::execute_address(IS_ANIM_PLAYING_ALT);
    e.register(execute, is_anim_playing_alt);
    table::add_plugin_record(
        e,
        IS_ANIM_PLAYING_ALT,
        &PluginRecord {
            name: "IsAnimPlayingAlt",
            short: "",
            needs_ref: true,
            param_count: 1,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
    timing24::install(e);
    overrides::install(e);
    playback::install(e);
    let specs = [
        ("first person", 1, false),
        ("enable", 1, false),
        ("animation path", 0, false),
        ("poll condition", 1, true),
        ("condition", 0x3d, true),
    ];
    let params = table::alloc(e, 12 * specs.len() as u32);
    for (i, (name, kind, optional)) in specs.iter().enumerate() {
        let at = params + i as u32 * 12;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, *kind);
        e.mem.set_u32(at + 8, u32::from(*optional));
    }
    let execute = table::execute_address(0x3922);
    e.register(execute, overrides::command);
    table::add_plugin_record(
        e,
        0x3922,
        &PluginRecord {
            name: "SetActorAnimationPath",
            short: "",
            needs_ref: false,
            param_count: 5,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
    nvse::plugins::register(
        e,
        nvse::plugins::BuiltIn {
            file: "kNVSE.dll",
            name: "kNVSE",
            version: 24,
            opcode_base: 0x3920,
            listed: false,
            listener: Some(listener),
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        },
    );
    nvse::plugins::listen_per_frame(e);
}

fn listener(e: &mut Engine, message: &nvse::plugins::Message) {
    match message.kind {
        nvse::plugins::msg::DEFERRED_INIT => overrides::deferred(e),
        nvse::plugins::msg::POST_LOAD_GAME => {
            timing24::post_load(e);
            overrides::post_load(e);
        }
        nvse::plugins::msg::MAIN_GAME_LOOP => {
            if !e.call(0x0070_2360, &[]).bool() {
                timing24::tick(e);
                overrides::tick(e);
                timing24::misc(e);
            }
        }
        nvse::plugins::msg::EXIT_GAME | nvse::plugins::msg::EXIT_GAME_CONSOLE => {
            timing24::shutdown(e)
        }
        _ => {}
    }
}

/// commands_animation.cpp::CreateCommands, IsAnimPlayingAlt lambda. This
/// inspects the actual sequence in its group-info slot, rather than the saved
/// Animation::group IDs (which need not describe an active sequence).
fn is_anim_playing_alt(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let group = read.short(0, u32::MAX);
    let actor = dynamic_cast(e, a.this_obj, rtti::TES_OBJECT_REFR, rtti::ACTOR);
    if actor == 0 {
        return handled(true);
    }
    let animation = if actor == e.mem.u32(PLAYER) {
        // PlayerCharacter::GetAnimData() uses the consistent POV byte, not
        // the current/requested camera bytes at +64a/+64b.
        if e.mem.u8(actor + THIRD_PERSON_CONSISTENT) == 0 {
            e.mem.u32(actor + FIRST_PERSON_ANIMATION)
        } else {
            let process = e.mem.u32(actor + PROCESS);
            e.mem.u32(process + PROCESS_ANIMATION)
        }
    } else {
        e.vcall(actor, VT_GET_ANIMATION, &[]).u32()
    };
    if animation == 0 || group >= GROUP_COUNT {
        return handled(true);
    }
    let slot = e
        .mem
        .u32(GROUP_INFOS + GROUP_INFO_SIZE * group + GROUP_SEQUENCE_TYPE);
    let sequence = e.mem.u32(animation + CURRENT_SEQUENCES + 4 * slot);
    if sequence != 0 {
        let anim_group = e.mem.u32(sequence + SEQUENCE_GROUP);
        // TESAnimGroup::GetBaseGroupID casts its UInt16 groupID to the
        // UInt8 AnimGroupID enum; high movement/weapon bits are ignored.
        if anim_group != 0
            && u32::from(e.mem.u8(anim_group + GROUP_ID)) == group
            && e.mem.u32(sequence + SEQUENCE_STATE) != 0
        {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

#[cfg(test)]
engine_core::assemble!();
mod actors;
mod loader;
mod overrides;
mod paths;
mod playback;
#[cfg(test)]
mod tests;
mod timing24;
