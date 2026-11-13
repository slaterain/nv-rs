//! JIP LN NVSE's LN events (`internal/lutana.h` (JIP LN NVSE)): the twelve events a script
//! handles through xNVSE's `SetEventHandler` by name (`OnCellEnter`, `OnCellExit`,
//! `OnPlayerGrab`, `OnPlayerRelease`, `OnCrosshairOn`, `OnCrosshairOff`, `OnButtonDown`,
//! `OnButtonUp`, `OnKeyDown`, `OnKeyUp`, `OnControlDown`, `OnControlUp`), and the gamepad
//! triggers' two that JIP's gamepad commands set (step 3.6). xNVSE asks JIP for an event's mask
//! by name and hands it the handler (`GetLNEventMask`, `ProcessLNEventHandler`: JIP's DLL exports,
//! ordinals 10 and 11, which xNVSE looks up at its start); here the two are slots on xNVSE's plugin
//! state ([`nvse::plugins::LnEvents`]), filled at JIP's start. They are checked once each frame,
//! from JIP's listener at the `MainGameLoop` message ([`process`]): the keys and controls by
//! their raw states, the pad's buttons and triggers by JIP's pad state ([`crate::input`]), the
//! player's cell, grabbed reference and crosshair reference by what changed since the last
//! frame (none the first frame after a load).
//!
//! A handler keeps its script's lambda variables while it is set (`LambdaVarContext`), as in JIP.

use crate::game::{InterfaceManager, PlayerCharacter, TESForm, TESObjectREFR};
use crate::Jip;
use engine_core::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// `kValidKeyCode` (`internal/lutana.h` (JIP LN NVSE)), copied by script: 266 entries.
const VALID_KEY_CODE: [bool; 266] = [
    false, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, true, true, true, false, false, false, true, true, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, true, true, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, true, false, false, true, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    true, true, true, false, true, false, true, false, true, true, true, true, true, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, true, true, true, true, true, true, true, true, true, true,
];

/// `kValidFilterForm` (`internal/lutana.h` (JIP LN NVSE)), copied by script: 121 entries.
const VALID_FILTER_FORM: [bool; 121] = [
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, true, true, true, true, true, false,
    true, true, false, true, true, true, true, true, true, true, true, true, true, true, true,
    true, true, false, false, true, true, false, true, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, true, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, true, false, false, false, false, true,
    false, false, false, false, false, false, true, true, false, false, false, false,
];

/// `LNEvents` (`kLNEventID_*`): the events' ids, and the masks that are `1 << id`.
pub mod id {
    pub const CELL_ENTER: u8 = 0;
    pub const CELL_EXIT: u8 = 1;
    pub const PLAYER_GRAB: u8 = 2;
    pub const PLAYER_RELEASE: u8 = 3;
    pub const CROSSHAIR_ON: u8 = 4;
    pub const CROSSHAIR_OFF: u8 = 5;
    pub const BUTTON_DOWN: u8 = 6;
    pub const BUTTON_UP: u8 = 7;
    pub const KEY_DOWN: u8 = 8;
    pub const KEY_UP: u8 = 9;
    pub const CONTROL_DOWN: u8 = 10;
    pub const CONTROL_UP: u8 = 11;
    pub const TRIGGER_DOWN: u8 = 12;
    pub const TRIGGER_UP: u8 = 13;
    /// `kLNEventID_Max`: the events kept as lists (the first eight).
    pub const MAX: u8 = KEY_DOWN;
}

/// An event's mask (`kLNEventMask_*`).
pub const fn mask(event: u8) -> u32 {
    1 << event
}

const ON_KEY: u32 = mask(id::KEY_DOWN) | mask(id::KEY_UP);
const ON_CONTROL: u32 = mask(id::CONTROL_DOWN) | mask(id::CONTROL_UP);
const ON_BUTTON: u32 = mask(id::BUTTON_DOWN) | mask(id::BUTTON_UP);
const ON_TRIGGER: u32 = mask(id::TRIGGER_DOWN) | mask(id::TRIGGER_UP);

/// `s_LNEventNames`: the names xNVSE asks for (without case), and their masks.
const NAMES: [(&str, u32); 12] = [
    ("OnCellEnter", mask(id::CELL_ENTER)),
    ("OnCellExit", mask(id::CELL_EXIT)),
    ("OnPlayerGrab", mask(id::PLAYER_GRAB)),
    ("OnPlayerRelease", mask(id::PLAYER_RELEASE)),
    ("OnCrosshairOn", mask(id::CROSSHAIR_ON)),
    ("OnCrosshairOff", mask(id::CROSSHAIR_OFF)),
    ("OnButtonDown", mask(id::BUTTON_DOWN)),
    ("OnButtonUp", mask(id::BUTTON_UP)),
    ("OnKeyDown", mask(id::KEY_DOWN)),
    ("OnKeyUp", mask(id::KEY_UP)),
    ("OnControlDown", mask(id::CONTROL_DOWN)),
    ("OnControlUp", mask(id::CONTROL_UP)),
];

/// Form types: `BGSListForm`, `TESObjectCELL`, `TESCaravanMoney` (the last a crosshair filter by
/// type may name).
const LIST_FORM: u8 = 0x55;
const CELL: u8 = 0x39;
const CARAVAN_MONEY: u32 = 0x74;
/// `kMaxMacros`, `MAX_CONTROL_BIND`.
const MAX_MACROS: usize = 266;
const MAX_CONTROLS: usize = 28;
/// `g_thePlayer`, `g_interfaceManager` (their fields through [`PlayerCharacter`],
/// [`InterfaceManager`] and [`TESObjectREFR`]'s layouts).
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
/// The vtable entry that answers true and `IS_REFERENCE`'s slot.
const RETURN_TRUE: u32 = 0x008d_0360;
const VT_IS_REFERENCE: u32 = 0xf0;

/// One handler of a list event (`LNEventData`): its event, filter kind (0 none, 1 a reference, 2
/// a base form or cell, 3 a form list, 4 a form type or button mask), script and filter (the form,
/// the list's first node, or the number).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handler {
    pub event: u8,
    pub filter_type: u8,
    pub script: u32,
    pub filter: u32,
}

/// A key's or a control's handlers (`LNDInpuCallbacks`): sets by script address, as JIP's `Set`
/// keeps them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputHandlers {
    pub on_down: BTreeSet<u32>,
    pub on_up: BTreeSet<u32>,
}

impl InputHandlers {
    fn is_empty(&self) -> bool {
        self.on_down.is_empty() && self.on_up.is_empty()
    }
}

/// JIP's LN event state.
#[derive(Clone, Debug)]
pub struct State {
    /// `s_LNEvents`: the eight list events' handlers, in the order set.
    pub events: [Vec<Handler>; id::MAX as usize],
    /// `s_LNOnKeyEvents`, `s_LNOnControlEvents`: by key or control.
    pub keys: BTreeMap<u32, InputHandlers>,
    pub controls: BTreeMap<u32, InputHandlers>,
    /// `s_LNOnTriggerEvents`: left up, left down, right up, right down.
    pub triggers: [BTreeSet<u32>; 4],
    /// `s_LNEventFlags`: the events with a handler.
    pub flags: u32,
    /// `s_inputEventClear`: the input events with an emptied entry to drop.
    pub input_clear: u32,
    /// `s_gameLoadFlagLN`: the first frame after a load (or the first ever) reports no change.
    pub game_loaded: bool,
    /// What the last frame saw (JIP's function statics).
    last_keys: Vec<bool>,
    last_controls: [bool; MAX_CONTROLS],
    last_buttons: u16,
    last_left_trigger: bool,
    last_right_trigger: bool,
    last_cell: u32,
    last_grabbed: u32,
    last_grab_id: u32,
    last_crosshair: u32,
    last_crosshair_id: u32,
}

impl Default for State {
    fn default() -> Self {
        State {
            events: Default::default(),
            keys: BTreeMap::new(),
            controls: BTreeMap::new(),
            triggers: Default::default(),
            flags: 0,
            input_clear: 0,
            game_loaded: true,
            last_keys: vec![false; MAX_MACROS],
            last_controls: [false; MAX_CONTROLS],
            last_buttons: 0,
            last_left_trigger: false,
            last_right_trigger: false,
            last_cell: 0,
            last_grabbed: 0,
            last_grab_id: 0,
            last_crosshair: 0,
            last_crosshair_id: 0,
        }
    }
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().ln_events
}

/// JIP's two entry points in xNVSE's plugin state, when JIP answers "installed" (xNVSE finds
/// them in the DLL it loaded; without it every one of these names is no event).
pub fn install(e: &mut Engine, answers_installed: bool) {
    if answers_installed {
        nvse::plugins::set_ln_events(
            e,
            nvse::plugins::LnEvents {
                mask: get_ln_event_mask,
                handler: process_ln_event_handler,
            },
        );
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: GetLNEventMask.
/// The mask of an LN event's name, without case; 0 for none.
pub fn get_ln_event_mask(_: &mut Engine, name: &[u8]) -> u32 {
    NAMES
        .iter()
        .find(|(n, _)| n.as_bytes().eq_ignore_ascii_case(name))
        .map_or(0, |(_, m)| *m)
}

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

fn form_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

fn is_reference(e: &Engine, form: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + VT_IS_REFERENCE) == RETURN_TRUE
}

/// A handler's lambda variables kept (`LambdaVarContext`'s construction) or given up.
fn capture(e: &mut Engine, script: u32) {
    nvse::lambda::save_variables(e, script);
}

fn release(e: &mut Engine, script: u32) {
    nvse::lambda::unsave_variables(e, script);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: SetInputEventHandler.
/// A key's or a control's handler added or removed (`key`: the filter number as a signed
/// integer). Removing with a negative key takes the script out of every entry; adding needs a
/// valid key (`kValidKeyCode`) or a control below 28. An entry left empty is dropped at the next
/// frame.
fn set_input_handler(e: &mut Engine, event_mask: u32, script: u32, key: i32, add: bool) -> bool {
    let on_key = event_mask & ON_KEY != 0;
    let on_up = event_mask == mask(id::KEY_UP) || event_mask == mask(id::CONTROL_UP);
    let group = if on_key { ON_KEY } else { ON_CONTROL };
    if key < 0 {
        if add {
            return false;
        }
        let mut result = false;
        let entries: Vec<u32> = {
            let s = state(e);
            let map = if on_key { &s.keys } else { &s.controls };
            map.keys().copied().collect()
        };
        for k in entries {
            let (erased, empty) = {
                let s = state(e);
                let map = if on_key { &mut s.keys } else { &mut s.controls };
                let h = map.get_mut(&k).expect("listed");
                let set = if on_up { &mut h.on_up } else { &mut h.on_down };
                (set.remove(&script), h.is_empty())
            };
            if erased {
                release(e, script);
            }
            result |= erased;
            if empty {
                state(e).input_clear |= group;
            }
        }
        return result;
    }
    let key = key as u32;
    if on_key {
        if !VALID_KEY_CODE.get(key as usize).copied().unwrap_or(false) {
            return false;
        }
    } else if key > 27 {
        return false;
    }
    if add {
        let (new_entry, inserted) = {
            let s = state(e);
            let map = if on_key { &mut s.keys } else { &mut s.controls };
            let new_entry = !map.contains_key(&key);
            let h = map.entry(key).or_default();
            let set = if on_up { &mut h.on_up } else { &mut h.on_down };
            (new_entry, set.insert(script))
        };
        if new_entry {
            state(e).flags |= group;
        }
        if inserted {
            capture(e, script);
        }
        inserted
    } else {
        let found = {
            let s = state(e);
            let map = if on_key { &mut s.keys } else { &mut s.controls };
            map.get_mut(&key).map(|h| {
                let set = if on_up { &mut h.on_up } else { &mut h.on_down };
                (set.remove(&script), h.is_empty())
            })
        };
        let Some((erased, empty)) = found else {
            return false;
        };
        if erased {
            release(e, script);
        }
        if empty {
            state(e).input_clear |= group;
        }
        erased
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: LNEventFinder::operator==.
/// Whether `stored` is the handler a set (`probe`, `remove` false) or a removal (`remove` true)
/// names: the same script; then, for a removal, any stored handler without a filter; otherwise the
/// same filter kind and, for the first six events or a removal, the same filter, for the buttons a
/// shared bit.
fn finds(probe: &Handler, remove: bool, stored: &Handler) -> bool {
    if probe.script != stored.script {
        return false;
    }
    if remove && stored.filter_type == 0 {
        return true;
    }
    if probe.filter_type != stored.filter_type {
        return false;
    }
    if probe.event < id::BUTTON_DOWN || remove {
        return probe.filter == stored.filter;
    }
    probe.filter & stored.filter != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: ProcessLNEventHandler.
/// A handler of an LN event added or removed, as xNVSE hands it over (the event's mask, the
/// script, whether to add, the `source` filter form, the number after the name's colon): a form
/// list filters by its forms (not an empty one); a cell event takes only a cell; a reference
/// filters by itself; another form by its base form, of a type a filter may be
/// (`kValidFilterForm`). Without a form, the crosshair events take a form type, the buttons a
/// button mask (which adding needs). Adding the same handler again, or removing one not set,
/// answers false.
pub fn process_ln_event_handler(
    e: &mut Engine,
    event_mask: u32,
    script: u32,
    add: bool,
    form: u32,
    number: u32,
) -> bool {
    if event_mask >= mask(id::KEY_DOWN) {
        return set_input_handler(e, event_mask, script, number as i32, add);
    }
    if event_mask == 0 {
        return false;
    }
    let event = event_mask.trailing_zeros() as u8;
    let mut h = Handler {
        event,
        filter_type: 0,
        script,
        filter: 0,
    };
    if form != 0 {
        if form_type(e, form) == LIST_FORM {
            let head = form + 0x18;
            if e.mem.u32(head) == 0 {
                return false;
            }
            h.filter = head;
            h.filter_type = 3;
        } else if event <= id::CELL_EXIT {
            if form_type(e, form) != CELL {
                return false;
            }
            h.filter_type = 2;
            h.filter = form;
        } else if is_reference(e, form) {
            h.filter_type = 1;
            h.filter = form;
        } else {
            let t = usize::from(form_type(e, form));
            if !VALID_FILTER_FORM.get(t).copied().unwrap_or(false) {
                return false;
            }
            h.filter_type = 2;
            h.filter = form;
        }
    } else if event >= id::CROSSHAIR_ON {
        if number != 0 {
            h.filter = number;
            if event <= id::CROSSHAIR_OFF {
                if number > CARAVAN_MONEY
                    || !VALID_FILTER_FORM
                        .get(number as usize)
                        .copied()
                        .unwrap_or(false)
                {
                    return false;
                }
            } else if number > 0xffff {
                return false;
            }
            h.filter_type = 4;
        } else if event >= id::BUTTON_DOWN && add {
            return false;
        }
    }
    let list = usize::from(event);
    if add {
        if state(e).events[list].iter().any(|s| finds(&h, false, s)) {
            return false;
        }
        capture(e, script);
        let s = state(e);
        s.events[list].push(h);
        s.flags |= event_mask;
    } else {
        // `Vector::Remove(Matcher)` (JIP's `containers.h`): from the end, EVERY match goes (a
        // stored handler without a filter and the one with this filter both), each one's lambda
        // variables given up as it is destroyed; false only when none matched.
        let mut removed = 0;
        let mut at = state(e).events[list].len();
        while at > 0 {
            at -= 1;
            if finds(&h, true, &state(e).events[list][at]) {
                state(e).events[list].remove(at);
                release(e, script);
                removed += 1;
            }
        }
        if removed == 0 {
            return false;
        }
        let s = state(e);
        if s.events[list].is_empty() {
            s.flags &= !event_mask;
        }
    }
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: LNEventData::EvalFilter.
fn passes(e: &Engine, h: &Handler, refr: u32, base: u32) -> bool {
    match h.filter_type {
        0 => true,
        1 => h.filter == refr,
        2 => h.filter == base,
        3 => {
            let mut node = h.filter;
            while node != 0 {
                let data = e.mem.u32(node);
                if data == refr || data == base {
                    return true;
                }
                node = e.mem.u32(node + 4);
            }
            false
        }
        4 => base != 0 && h.filter == u32::from(form_type(e, base)),
        _ => false,
    }
}

/// Calls a handler's script with one argument (`CallFunction(callback, nullptr, 1, arg)`).
fn call(e: &mut Engine, script: u32, arg: u32) {
    crate::script_call::call(e, script, 0, vec![arg]);
}

/// Calls each of a set's scripts with `arg` (`EventCallbackScripts::InvokeEvents`), over a copy.
fn invoke(e: &mut Engine, scripts: &BTreeSet<u32>, arg: u32) {
    for script in scripts.clone() {
        call(e, script, arg);
    }
}

/// Calls the handlers of list event `event` that pass the filter with `arg`, over a copy.
fn dispatch(e: &mut Engine, event: u8, refr: u32, base: u32, arg: u32) {
    let handlers = state(e).events[usize::from(event)].clone();
    for h in handlers {
        if passes(e, &h, refr, base) {
            call(e, h.script, arg);
        }
    }
}

/// The input part of [`process`] for the keys (`keys`) or the controls: emptied entries dropped
/// first; then each entry whose raw state changed since the last frame calls its down or up
/// handlers with the key or control.
fn process_input(e: &mut Engine, keys: bool) {
    let group = if keys { ON_KEY } else { ON_CONTROL };
    if state(e).flags & group == 0 {
        return;
    }
    if state(e).input_clear & group != 0 {
        let s = state(e);
        s.input_clear &= !group;
        let map = if keys { &mut s.keys } else { &mut s.controls };
        map.retain(|_, h| !h.is_empty());
        if map.is_empty() {
            s.flags &= !group;
        }
    }
    let entries: Vec<(u32, InputHandlers)> = {
        let s = state(e);
        let map = if keys { &s.keys } else { &s.controls };
        map.iter().map(|(k, h)| (*k, h.clone())).collect()
    };
    for (k, h) in entries {
        let current = if keys {
            crate::input::is_key_pressed_raw(e, k)
        } else {
            crate::input::is_control_pressed_raw(e, k)
        };
        let last = {
            let s = state(e);
            if keys {
                &mut s.last_keys[k as usize]
            } else {
                &mut s.last_controls[k as usize]
            }
        };
        if *last == current {
            continue;
        }
        if current {
            invoke(e, &h.on_down, k);
        } else {
            invoke(e, &h.on_up, k);
        }
        let s = state(e);
        if keys {
            s.last_keys[k as usize] = current;
        } else {
            s.last_controls[k as usize] = current;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/lutana.h: LN_ProcessEvents.
/// One frame's check, when any LN event has a handler: the keys; with a ready controller its
/// buttons (each handler's mask of buttons gone down or up) and triggers; then, in game mode with
/// the player in a cell, the cell left and entered, the reference released (if it is still the
/// form its id names) and grabbed, the crosshair reference lost (the same check) and gained (all
/// these not reported the first frame after a load); last the controls.
pub fn process(e: &mut Engine) {
    if state(e).flags == 0 {
        return;
    }
    process_input(e, true);
    if crate::input::controller_ready(e) {
        let pad = e.ext::<Jip>().input.pad;
        if state(e).flags & ON_BUTTON != 0 {
            let last = state(e).last_buttons;
            let changes = pad.buttons ^ last;
            if changes != 0 {
                let down = changes & pad.buttons;
                if down != 0 {
                    for h in state(e).events[usize::from(id::BUTTON_DOWN)].clone() {
                        let out = u32::from(down) & h.filter;
                        if out != 0 {
                            call(e, h.script, out);
                        }
                    }
                }
                let up = changes & last;
                if up != 0 {
                    for h in state(e).events[usize::from(id::BUTTON_UP)].clone() {
                        let out = u32::from(up) & h.filter;
                        if out != 0 {
                            call(e, h.script, out);
                        }
                    }
                }
                state(e).last_buttons = pad.buttons;
            }
        }
        if state(e).flags & ON_TRIGGER != 0 {
            let left = pad.left_trigger != 0;
            if state(e).last_left_trigger != left {
                let set = state(e).triggers[usize::from(left)].clone();
                invoke(e, &set, 0);
                state(e).last_left_trigger = left;
            }
            let right = pad.right_trigger != 0;
            if state(e).last_right_trigger != right {
                let set = state(e).triggers[usize::from(right) + 2].clone();
                invoke(e, &set, 1);
                state(e).last_right_trigger = right;
            }
        }
    }
    let manager = Ptr::<InterfaceManager>::new(e.mem.u32(INTERFACE_MANAGER));
    if e.get(manager, InterfaceManager::currentMode) > 1 {
        return;
    }
    let player = e.mem.u32(PLAYER);
    let cell = e.get(
        Ptr::<TESObjectREFR>::new(player),
        TESObjectREFR::pParentCell,
    );
    if cell == 0 {
        return;
    }
    let loaded = std::mem::replace(&mut state(e).game_loaded, false);
    let last_cell = state(e).last_cell;
    if last_cell != cell {
        if !loaded {
            if state(e).flags & mask(id::CELL_EXIT) != 0 {
                dispatch(e, id::CELL_EXIT, 0, last_cell, last_cell);
            }
            if state(e).flags & mask(id::CELL_ENTER) != 0 {
                dispatch(e, id::CELL_ENTER, 0, cell, cell);
            }
        }
        state(e).last_cell = cell;
    }
    let grabbed = e.get(
        Ptr::<PlayerCharacter>::new(player),
        PlayerCharacter::pGrabbedObject,
    );
    let (last, last_id) = (state(e).last_grabbed, state(e).last_grab_id);
    if last != grabbed {
        if !loaded {
            if last != 0
                && state(e).flags & mask(id::PLAYER_RELEASE) != 0
                && last == crate::cosave::lookup_form(e, last_id)
            {
                let base = base_form(e, last);
                dispatch(e, id::PLAYER_RELEASE, last, base, last);
            }
            if grabbed != 0 && state(e).flags & mask(id::PLAYER_GRAB) != 0 {
                let base = base_form(e, grabbed);
                dispatch(e, id::PLAYER_GRAB, grabbed, base, grabbed);
            }
        }
        state(e).last_grabbed = grabbed;
        if grabbed != 0 {
            let id = form_id(e, grabbed);
            state(e).last_grab_id = id;
        }
    }
    let crosshair = e.get(manager, InterfaceManager::pActivateRef);
    let (last, last_id) = (state(e).last_crosshair, state(e).last_crosshair_id);
    if last != crosshair {
        if !loaded {
            if last != 0
                && state(e).flags & mask(id::CROSSHAIR_OFF) != 0
                && last == crate::cosave::lookup_form(e, last_id)
            {
                let base = crate::overrides::ref_base_form(e, last);
                dispatch(e, id::CROSSHAIR_OFF, last, base, last);
            }
            if crosshair != 0 && state(e).flags & mask(id::CROSSHAIR_ON) != 0 {
                let base = crate::overrides::ref_base_form(e, crosshair);
                dispatch(e, id::CROSSHAIR_ON, crosshair, base, crosshair);
            }
        }
        state(e).last_crosshair = crosshair;
        if crosshair != 0 {
            let id = form_id(e, crosshair);
            state(e).last_crosshair_id = id;
        }
    }
    process_input(e, false);
}

/// A reference's base form as JIP reads it for the grab events (`baseForm`, `+0x20`).
fn base_form(e: &Engine, refr: u32) -> u32 {
    e.get(
        Ptr::<TESObjectREFR>::new(refr),
        TESObjectREFR::pObjectReference,
    )
}

/// The pre-load part (`DoPreLoadGameHousekeeping`: `s_gameLoadFlagLN = true`): the first frame
/// after the load reports no change.
pub fn pre_load(e: &mut Engine) {
    state(e).game_loaded = true;
}

#[cfg(test)]
#[path = "ln_events_tests.rs"]
mod tests;
