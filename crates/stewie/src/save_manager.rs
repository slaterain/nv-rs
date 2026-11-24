//! Save Manager's timer and rotating autosaves, owned Tweaks 10.05 DLL:
//! 10050CF0, 10050FD0, 10051080..10051131, 10051160, 10051630,
//! 1007629C/10076301. Private evidence: SAVE-MANAGER-LIFECYCLE.json.
//! Manual save/reload hotkeys:10051230..100513B1, incremental10050E40;
//! event triggers, pause-menu option and active-save highlighting10051030..10051630.
use crate::settings::Settings;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::{mod_files, prelude::*};
use nvse::plugins::{self, msg, Message};

const RUN: WrapSlot = WrapSlot::new(Owner::Stewie, 0x0085_0a40);
const COUNTDOWN: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0085_1dae);
const LOADING: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0085_1dec);
const TRAVEL: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0051_8e41);
const MENU_OPEN: SiteSlot = SiteSlot::new(Owner::Stewie, 0x007c_b88b);
const MENU_CLOSE: SiteSlot = SiteSlot::new(Owner::Stewie, 0x007c_b65d);
const CONSOLE_OPEN: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0071_d665);
const CONSOLE_CLOSE: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0071_d720);
const CLOCK: u32 = 0x011f_63a8;
const MANAGER: u32 = 0x011d_e134;
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE: u32 = 0x011d_8a80;
const CAN_SAVE: u32 = 0x0085_0fe0;
const SAVE: u32 = 0x0085_03b0;
const PAUSE_CHOOSE: u32 = engine_core::extend::ROUTINE_BASE + 0xf500;

#[derive(Clone, Copy, Default)]
struct Config {
    timer: i32,
    max: i32,
    min_interval: u32,
    periodic_fullsave: bool,
    hide_message: bool,
    prevent_combat: bool,
    prevent_godmode: bool,
    all_saves_reset: bool,
    create_key: u32,
    incremental_key: u32,
    reload_key: u32,
    incremental_max: i32,
    slot_interval: i32,
    location: bool,
    quest: bool,
    exit: bool,
    pre_level: bool,
    pickpocket: bool,
    craft: bool,
    prevent_script: bool,
    pause_button: bool,
    highlight: bool,
}

#[derive(Default)]
struct State {
    config: Config,
    index: i32,
    last: u32,
    pause_start: u32,
    menu: bool,
    console: bool,
    timed: bool,
    // Native Setup caches Player+68 after the player exists (1008B8D4).
    // Host installation is earlier, so the same cache is bound at DeferredInit.
    process: Option<u32>,
    incremental_index: i32,
    last_slot_change: u32,
    reload_requested: bool,
    quickload_requested: bool,
    last_reload_input_tick: u32,
    last_load_tick: u32,
    craft_count: u32,
    exit_menu_checks: bool,
}

/// 100639A0: GetLongValue -> CVTDQ2PS, MULSS(1000), CVTTSS2SI.
fn milliseconds(seconds: i32) -> i32 {
    let value = seconds as f32 * 1000.0;
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

pub(crate) fn install(e: &mut Engine, settings: &Settings) {
    if settings.long("Tweaks", "bImprovedAutoSave", 0) == 0 {
        return;
    }
    let section = "Save Manager";
    let get = |key, default| settings.long(section, key, default);
    *e.ext::<State>() = State {
        config: Config {
            timer: milliseconds(get("iAutoSaveTimer", 0)),
            max: get("iMaxAutoSaveCount", 3),
            min_interval: milliseconds(get("iMinAutosaveInterval", 0)) as u32,
            periodic_fullsave: get("bPeriodicFullsave", 0) != 0,
            hide_message: get("bHideAutosaveMessage", 0) != 0,
            prevent_combat: get("bPreventAutosaveInCombat", 0) != 0,
            prevent_godmode: get("bPreventAutosaveInTGM", 0) != 0,
            all_saves_reset: get("bAllSavesResetAutosaveTimer", 1) != 0,
            create_key: get("iCreateSaveKey", 0) as u32,
            incremental_key: get("iIncrementalSaveKey", 0) as u32,
            reload_key: get("iReloadCurrentSaveKey", 0) as u32,
            incremental_max: get("iMaxIncrementalSaveCount", 5),
            slot_interval: milliseconds(get("iIncrementalSaveSlotChangeInterval", 0)),
            location: get("bSaveOnLocationDiscovered", 0) != 0,
            quest: get("bSaveOnQuestCompleted", 0) != 0,
            exit: get("bSaveOnExitGame", 0) != 0,
            pre_level: get("bSavePreLevelUp", 0) != 0,
            pickpocket: get("bSaveOnPickpocket", 0) != 0,
            craft: get("bSaveOnCraft", 0) != 0,
            prevent_script: get("bPreventScriptedSaves", 0) != 0,
            pause_button: get("bReplaceContinueWithQuicksave", 0) != 0,
            highlight: get("bHighlightActiveSave", 1) != 0,
        },
        index: get("_iAutoSaveIndex", 0),
        incremental_index: get("_iIncrementalSaveIndex", 0),
        ..State::default()
    };
    e.share_ext::<State>();
    let config = e.ext::<State>().config;
    for (on, addr, handler) in [
        (
            config.location,
            0x0077_9529,
            queue_one as engine_core::extend::SiteFn,
        ),
        (config.quest, 0x0060_ca6c, queue_one),
        (config.pre_level, 0x0077_d6de, pre_level),
        (config.pickpocket, 0x0075_b24a, pickpocket),
        (config.exit, 0x005b_6cb0, on_exit),
        (config.exit, 0x0085_1050, exit_menu_check),
        (config.hide_message, 0x0085_0415, hide_notice),
    ] {
        if on {
            e.on_site(SiteSlot::new(Owner::Stewie, addr), handler);
        }
    }
    if config.craft {
        install_recipe_hooks(e);
    }
    if config.highlight {
        e.on_site(SiteSlot::new(Owner::Stewie, 0x007d_6931), highlight_save);
    }
    if config.pause_button {
        e.register(PAUSE_CHOOSE, pause_choose);
        for (addr, handler) in [
            (0x007c_c7c2, pause_option as engine_core::extend::SiteFn),
            (0x007c_bdaf, pause_is_save),
            (0x007c_fd3c, pause_can_load),
            (0x0070_f7e2, pause_active),
        ] {
            e.on_site(SiteSlot::new(Owner::Stewie, addr), handler);
        }
    }
    if config.prevent_script {
        // Source installer1005172D disables the original script autosave gate.
        e.mem.set_u32(0x011d_e2a0, 0);
        e.on_site(SiteSlot::new(Owner::Stewie, 0x005c_54b7), scripted_save);
    }
    if config.create_key != 0 || config.incremental_key != 0 {
        e.on_site(SiteSlot::new(Owner::Stewie, 0x0094_0c78), manual_keys);
    }
    if config.reload_key != 0 {
        for address in [0x0094_039b, 0x0094_2869, 0x0094_7ab8] {
            e.on_site(SiteSlot::new(Owner::Stewie, address), reload_input);
        }
        e.on_site(SiteSlot::new(Owner::Stewie, 0x0086_eac2), dispatch_load);
    }
    e.wrap(RUN, run_autosave);
    e.on_site(COUNTDOWN, countdown);
    e.on_site(LOADING, loading);
    e.on_site(TRAVEL, travel);
    e.on_site(MENU_OPEN, menu_open);
    e.on_site(MENU_CLOSE, menu_close);
    e.on_site(CONSOLE_OPEN, console_open);
    e.on_site(CONSOLE_CLOSE, console_close);
    if e.ext::<State>().config.timer > 1000 {
        plugins::listen_per_frame(e);
    }
}

pub(crate) fn listen(e: &mut Engine, message: &Message) {
    if e.ext_get::<State>().is_none() {
        return;
    }
    match message.kind {
        msg::DEFERRED_INIT => {
            let player = e.mem.u32(PLAYER);
            e.ext::<State>().process = Some(e.mem.u32(player + 0x68));
        }
        msg::POST_LOAD_GAME | msg::NEW_GAME => {
            let now = e.mem.u32(CLOCK);
            let state = e.ext::<State>();
            state.last = now;
            if state.pause_start != 0 {
                state.pause_start = now;
            }
            let manager = e.mem.u32(MANAGER);
            e.mem.set_u8(manager + 0xf, 0);
        }
        msg::SAVE_GAME => {
            let now = e.mem.u32(CLOCK);
            let state = e.ext::<State>();
            if state.config.all_saves_reset || state.timed {
                state.last = now;
            }
            state.timed = false;
        }
        msg::MAIN_GAME_LOOP => timer(e),
        _ => {}
    }
}

fn timer(e: &mut Engine) {
    let (config, last, process) = {
        let state = e.ext::<State>();
        (state.config, state.last, state.process)
    };
    let Some(process) = process.filter(|_| config.timer > 1000) else {
        return;
    };
    let now = e.mem.u32(CLOCK);
    if now < last.wrapping_add(config.timer as u32) {
        return;
    }
    let manager = e.mem.u32(MANAGER);
    if e.mem.u8(manager + 0xf) != 0 {
        return;
    }
    let interface = e.mem.u32(INTERFACE);
    if e.mem.u32(interface + 0xc) != 1 || e.mem.u32(interface + 0x4bc) != 0 {
        return;
    }
    if !e.call(CAN_SAVE, &args![manager, false]).bool() || e.vcall(process, 0x404, &[]).bool() {
        return;
    }
    let player = e.mem.u32(PLAYER);
    let combat =
        config.prevent_combat && e.mem.u8(player + 0xdf0) != 0 && e.mem.u8(player + 0xdf1) == 0;
    let godmode =
        config.prevent_godmode && (e.mem.u8(0x011e_07ba) != 0 || e.mem.u8(0x011e_0898) != 0);
    if combat || godmode {
        e.ext::<State>().last = now.wrapping_sub(config.timer as u32).wrapping_add(3000);
        return;
    }
    e.mem.set_u8(manager + 0xf, 5);
    e.ext::<State>().timed = true;
}

fn pause(e: &mut Engine, menu: bool, open: bool) {
    let now = e.mem.u32(CLOCK);
    let state = e.ext::<State>();
    if menu {
        state.menu = open;
    } else {
        state.console = open;
    }
    if open {
        if state.pause_start == 0 {
            state.pause_start = now;
        }
    } else if state.pause_start != 0 && !(state.menu || state.console) {
        state.last = state.last.wrapping_add(now.wrapping_sub(state.pause_start));
        state.pause_start = 0;
    }
}
fn menu_open(e: &mut Engine, _: &[u32]) -> Option<u32> {
    pause(e, true, true);
    None
}
fn menu_close(e: &mut Engine, _: &[u32]) -> Option<u32> {
    pause(e, true, false);
    None
}
fn console_open(e: &mut Engine, _: &[u32]) -> Option<u32> {
    pause(e, false, true);
    None
}
fn console_close(e: &mut Engine, _: &[u32]) -> Option<u32> {
    pause(e, false, false);
    None
}

fn countdown(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let manager = words[0];
    let blocked = e.mem.u32(0x011d_a0c0) != 0 || e.mem.u32(0x011d_9514) != 0;
    let count = if blocked {
        255
    } else {
        e.mem.u8(manager + 0xf).wrapping_sub(1)
    };
    e.mem.set_u8(manager + 0xf, count);
    Some(0)
}
fn loading(e: &mut Engine, words: &[u32]) -> Option<u32> {
    if words[1] != 0 {
        e.mem.set_u8(words[0] + 0xf, 120);
    }
    None
}
fn travel(e: &mut Engine, words: &[u32]) -> Option<u32> {
    e.mem.set_u8(words[0] + 0xf, 32);
    Some(0)
}

fn run_autosave(e: &mut Engine, words: &[u32]) -> Ret {
    let manager = words[0];
    e.mem.set_u16(manager + 0xe, 0);
    let (config, last) = {
        let state = e.ext::<State>();
        (state.config, state.last)
    };
    if e.mem.u32(CLOCK).wrapping_sub(last) < config.min_interval {
        return 0u32.into_ret();
    }
    // Native replacement uses these game fields, not a host-clock sky step.
    let world = e.mem.u32(0x011d_ea10);
    let sky = e.mem.u32(world + 0x68);
    let delta = e.mem.u32(0x011d_ea30);
    e.call(0x0063_ac70, &args![sky, delta]);
    u32::from(rotate_autosave(e, manager)).into_ret()
}
fn rotate_autosave(e: &mut Engine, manager: u32) -> bool {
    let config = e.ext::<State>().config;
    let fullsave = {
        let state = e.ext::<State>();
        if state.index >= config.max {
            state.index = 0;
            config.periodic_fullsave
        } else {
            false
        }
    };
    if fullsave {
        let saved = e.call(SAVE, &args![manager, 0u32, u32::MAX, 0u32]).bool();
        if saved && !config.hide_message {
            e.call(
                0x0070_52f0,
                &args![0x0106_14e0u32, 0u32, 0u32, 0u32, 1f32, 0u32],
            );
        }
        saved
    } else {
        let index = {
            let state = e.ext::<State>();
            state.index = state.index.wrapping_add(1);
            state.index
        };
        let name = format!("Autosave - {index}");
        let saved = e.with_stack(name.len() as u32 + 1, |e, text| {
            e.mem.set_cstr(text.addr(), name.as_bytes());
            e.call(SAVE, &args![manager, text, u32::MAX, 0u32]).bool()
        });
        if saved {
            // Preserve the author's main-only index key through the portable
            // mod-file layer: a port-owned copy, never the installed INI.
            let _ = mod_files::write_profile_string(
                e,
                crate::INI,
                b"Save Manager",
                b"_iAutoSaveIndex",
                index.to_string().as_bytes(),
            );
        }
        saved
    }
}

/// Owned10051330/10051390: dispatched after the player's original menu-button
/// update. Query the game's OSInputGlobals pressed edge, not NVSE raw/disabled keys.
fn manual_keys(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let config = e.ext::<State>().config;
    if config.create_key != 0
        && e.call(
            0x00a2_4180,
            &args![e.mem.u32(0x011f_35cc), config.create_key, 1u32],
        )
        .bool()
        && e.call(SAVE, &args![e.mem.u32(MANAGER), 0u32, u32::MAX, 0u32])
            .bool()
    {
        save_notice(e);
    }
    if config.incremental_key != 0
        && e.call(
            0x00a2_4180,
            &args![e.mem.u32(0x011f_35cc), config.incremental_key, 1u32],
        )
        .bool()
    {
        incremental_save(e);
    }
    None
}
fn save_notice(e: &mut Engine) {
    e.call(
        0x0070_52f0,
        &args![0x0106_14e0u32, 0u32, 0u32, 0u32, 1f32, 0u32],
    );
}
/// 10050E40: signed strict elapsed comparison and pre-save slot advancement.
/// The interval clock/index survive load/new-game callbacks, as in the owned DLL.
fn incremental_save(e: &mut Engine) -> bool {
    let now = e.mem.u32(CLOCK);
    let index = {
        let state = e.ext::<State>();
        if (now.wrapping_sub(state.last_slot_change) as i32) > state.config.slot_interval {
            state.last_slot_change = now;
            if state.incremental_index >= state.config.incremental_max {
                state.incremental_index = 0;
            }
            state.incremental_index = state.incremental_index.wrapping_add(1);
        }
        state.incremental_index
    };
    let player_name = e.call(0x0055_d520, &args![e.mem.u32(PLAYER)]).u32();
    let mut name = e.mem.cstr(player_name);
    name.extend_from_slice(format!(" - Save {index}").as_bytes());
    // Source formatting buffer is MAX_PATH260, including the terminating NUL.
    name.truncate(259);
    let saved = e.with_stack(260, |e, text| {
        e.mem.set_cstr(text.addr(), &name);
        e.call(SAVE, &args![e.mem.u32(MANAGER), text, u32::MAX, 0u32])
            .bool()
    });
    if saved {
        let _ = mod_files::write_profile_string(
            e,
            crate::INI,
            b"Save Manager",
            b"_iIncrementalSaveIndex",
            index.to_string().as_bytes(),
        );
        // Unlike periodic full autosaves, this source path is unconditional on
        // bHideAutosaveMessage and uses the same native save-completed notice.
        save_notice(e);
    }
    saved
}
/// 10051230 replaces only three vanilla quickload input reads, always returning0.
/// A reload edge wins over the original quickload query once per game tick.
fn reload_input(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let tick = e.mem.u32(0x011a_2fe0);
    let key = e.ext::<State>().config.reload_key;
    let pressed = e.call(0x00a2_4180, &args![words[0], key, 1u32]).bool();
    if pressed
        && e.mem.u8(0x011e_0be5) == 0
        && e.mem.u8(0x011d_ea2e) == 0
        && tick != e.ext::<State>().last_reload_input_tick
    {
        let state = e.ext::<State>();
        state.last_reload_input_tick = tick;
        state.reload_requested = true;
    } else {
        let quickload = e.call(0x00a2_4660, words).bool();
        e.ext::<State>().quickload_requested |= quickload;
    }
    Some(0)
}
/// 100512B0 at86EAC2: run pending load before the original object's update phase.
/// The source replaces the getter with its known singleton, even with no request.
fn dispatch_load(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let (reload, quickload, previous) = {
        let state = e.ext::<State>();
        (
            state.reload_requested,
            state.quickload_requested,
            state.last_load_tick,
        )
    };
    let tick = e.mem.u32(0x011a_2fe0);
    if (reload || quickload)
        && e.mem.u8(0x011e_0be5) == 0
        && e.mem.u8(0x011d_ea2e) == 0
        && tick != previous
    {
        e.ext::<State>().last_load_tick = tick;
        e.mem.set_u8(0x011e_0be5, 1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.call(
                if reload { 0x0085_12f0 } else { 0x0085_09f0 },
                &args![e.mem.u32(MANAGER)],
            );
            if !reload {
                e.mem.set_u32(0x011a_3b34, (-1f32).to_bits());
            }
        }));
        e.mem.set_u8(0x011e_0be5, 0);
        if let Err(error) = result {
            std::panic::resume_unwind(error);
        }
        let state = e.ext::<State>();
        state.reload_requested = false;
        state.quickload_requested = false;
    }
    Some(0x011d_effc)
}

fn queue(e: &mut Engine, frames: u8) {
    e.mem.set_u8(e.mem.u32(MANAGER) + 0xf, frames);
}
fn queue_one(e: &mut Engine, _: &[u32]) -> Option<u32> {
    queue(e, 1);
    None
}
fn pre_level(e: &mut Engine, _: &[u32]) -> Option<u32> {
    // Owned10051050: unsigned strict comparison of wrapping(now-10000),
    // not an elapsed>=10sec simplification across zero/wrap.
    if e.mem.u32(CLOCK).wrapping_sub(10000) > e.ext::<State>().last {
        queue(e, 1);
    }
    None
}
fn pickpocket(e: &mut Engine, words: &[u32]) -> Option<u32> {
    if e.mem.u8(words[0] + 0x88) != 0 && e.mem.u8(words[0] + 0x89) == 0 {
        queue(e, 1);
    }
    None
}
fn on_exit(e: &mut Engine, _: &[u32]) -> Option<u32> {
    // Owned10051030 permanently NOPs851050 then runs rotation directly,
    // without the min-interval/Sky work in RunAutosave. NOP forces normal
    // menu checks even for autosaves; it does not bypass save eligibility.
    e.ext::<State>().exit_menu_checks = true;
    rotate_autosave(e, e.mem.u32(MANAGER));
    None
}
fn exit_menu_check(e: &mut Engine, _: &[u32]) -> Option<u32> {
    e.ext::<State>().exit_menu_checks.then_some(0)
}
fn hide_notice(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0)
}
/// Owned10003AB0's exact console-open/TLS behavior; unlike NVSE's helper,
/// the owned DLL returns the nonzero console-open byte if TLS is null.
fn scripted_save(e: &mut Engine, _: &[u32]) -> Option<u32> {
    if e.mem.u8(0x011d_ea2e) == 0 {
        return Some(0);
    }
    let tls = e.tls();
    Some(u32::from(tls == 0 || e.mem.u8(tls + 0x268) != 0))
}
#[derive(Default)]
struct RecipeHooks(bool);
/// One OwnerStewie dispatcher composes two independent features at the exact
/// original call sites, keeping SaveManager before and MenuSearch after calls.
pub(super) fn install_recipe_hooks(e: &mut Engine) {
    if e.ext::<RecipeHooks>().0 {
        return;
    }
    e.ext::<RecipeHooks>().0 = true;
    e.on_site(SiteSlot::new(Owner::Stewie, 0x0072_72f2), |e, words| {
        recipe_lifecycle(e, true, words[0]);
        crate::menu_search::recipe_lifecycle(e, true, words[0]);
        None
    });
    e.on_site(SiteSlot::new(Owner::Stewie, 0x0072_6f2a), |e, words| {
        recipe_lifecycle(e, false, words[0]);
        crate::menu_search::recipe_lifecycle(e, false, words[0]);
        None
    });
}
fn recipe_lifecycle(e: &mut Engine, opening: bool, phase: u32) {
    if phase != 0 || !e.ext_get::<State>().is_some_and(|s| s.config.craft) {
        return;
    }
    let count = e.mem.u32(e.mem.u32(0x011c_6dd0) + 4);
    if opening {
        e.ext::<State>().craft_count = count;
    } else if count != e.ext::<State>().craft_count {
        queue(e, 20);
    }
}

/// Owned10051450 inserts a pause-only option before Continue and removes
/// Continue's pause mode. The normal native Option destructor owns the heap block.
fn pause_option(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (array, original) = (words[0], words[1]);
    let option = e.call(0x00aa_3e40, &args![0x011f_6238u32, 16u32]).u32();
    e.mem.set_u32(option, 0x0107_6ee0);
    e.mem.set_u32(option + 4, e.mem.u32(0x011d_3280));
    e.mem.set_u32(option + 8, PAUSE_CHOOSE);
    e.mem.set_u32(option + 0xc, 2);
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), option);
        e.call(0x007c_b2e0, &args![array, cell]);
        e.mem
            .set_u32(original + 0xc, e.mem.u32(original + 0xc) & !2);
        e.mem.set_u32(cell.addr(), original);
        e.call(0x007c_b2e0, &args![array, cell]);
    });
    Some(1)
}
fn pause_is_save(e: &mut Engine, words: &[u32]) -> Option<u32> {
    (e.mem.u32(words[0] + 8) == PAUSE_CHOOSE).then_some(1)
}
fn pause_can_load(e: &mut Engine, words: &[u32]) -> Option<u32> {
    if words[0] != 0 {
        e.call(
            0x00a0_12d0,
            &args![words[0], words[1], (words[2] as f32).to_bits(), 1u32],
        );
    }
    Some(1)
}
fn pause_active(e: &mut Engine, words: &[u32]) -> Option<u32> {
    Some(u32::from(words[0] == e.mem.u32(0x011d_aac0)))
}
fn pause_choose(e: &mut Engine, _: &[u32]) -> Ret {
    if incremental_save(e) {
        e.call(0x007c_e7a0, &args![1u32]);
    } else {
        let sound = nvse::table::guest_str(e, b"UIMenuCancel");
        e.with_stack(8, |e, buffer| {
            let handle = e
                .call(
                    0x00ad_7550,
                    &args![e.mem.u32(0x011f_6d98), buffer, sound, 0x121u32],
                )
                .u32();
            e.call(0x00ad_8830, &args![handle, 0u32]);
        });
    }
    Ret::default()
}
fn highlight_save(e: &mut Engine, words: &[u32]) -> Option<u32> {
    e.call(
        0x00a0_12d0,
        &args![
            words[0],
            words[2],
            (u32::from(words[3] as u8) as f32).to_bits(),
            1u32
        ],
    );
    let current = e.mem.u32(e.mem.u32(MANAGER) + 0x14);
    if current != 0 && e.mem.cstr(e.mem.u32(words[1])) == e.mem.cstr(current) {
        e.call(
            0x00a0_12d0,
            &args![words[0], 0xff4u32, 0f32.to_bits(), 1u32],
        );
    }
    Some(1)
}

#[cfg(test)]
mod tests;
