//! xNVSE's plugin interface, for the plugins nv-rs builds in (`PluginManager`,
//! `NVSEMessagingInterface` and `NVSESerializationInterface` (xNVSE)). xNVSE knows a built-in
//! as it knows a plugin DLL it loaded: by its name and version (the plugin list that
//! `GetPluginInfoByName`, `IsPluginInstalled` and `GetPluginVersion` read, for a built-in that
//! answers "installed"), by its opcode base (the header of its co-save section), by the listener
//! it registers for xNVSE's messages and by its co-save callbacks. Built-ins come in xNVSE's
//! order of plugins: by DLL file name, without case (`PluginManager::Init`).
//!
//! The messages go out where xNVSE sends them (`Dispatch_Message`), after the events, which
//! xNVSE's dispatch also tells first ([`crate::events`]): the save and load moments
//! ([`crate::saveload`]) and the exits.

use crate::cosave::{Chunk, OutOfRange, Reader};
use crate::Nvse;
use engine_core::prelude::*;

/// `NVSEMessagingInterface`'s message types (`PluginAPI.h` (xNVSE)).
pub mod msg {
    pub const POST_LOAD: u32 = 0;
    pub const EXIT_GAME: u32 = 1;
    pub const EXIT_TO_MAIN_MENU: u32 = 2;
    pub const LOAD_GAME: u32 = 3;
    pub const SAVE_GAME: u32 = 4;
    pub const SCRIPT_PRECOMPILE: u32 = 5;
    pub const PRE_LOAD_GAME: u32 = 6;
    pub const EXIT_GAME_CONSOLE: u32 = 7;
    pub const POST_LOAD_GAME: u32 = 8;
    pub const POST_POST_LOAD: u32 = 9;
    pub const RUNTIME_SCRIPT_ERROR: u32 = 10;
    pub const DELETE_GAME: u32 = 11;
    pub const RENAME_GAME: u32 = 12;
    pub const RENAME_NEW_GAME: u32 = 13;
    pub const NEW_GAME: u32 = 14;
    pub const DELETE_GAME_NAME: u32 = 15;
    pub const RENAME_GAME_NAME: u32 = 16;
    pub const RENAME_NEW_GAME_NAME: u32 = 17;
    pub const DEFERRED_INIT: u32 = 18;
    pub const CLEAR_SCRIPT_DATA_CACHE: u32 = 19;
    pub const MAIN_GAME_LOOP: u32 = 20;
}

/// A message as a listener gets it (`NVSEMessagingInterface::Message`): its type, its data (a
/// guest word: the address of a text for the save, load, delete and rename messages, the
/// success byte for `PostLoadGame`, 0 for none) and the data's length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub data: u32,
    pub len: u32,
}

/// The records a built-in writes into its co-save section (`OpenRecord`, `WriteRecord`,
/// `WriteRecordData` and `WriteRecord8` to `WriteRecord64` (xNVSE)): a chunk a record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Records {
    pub chunks: Vec<Chunk>,
}

impl Records {
    /// `OpenRecord`: a new record, which the writes after it go into.
    pub fn open(&mut self, kind: u32, version: u32) {
        self.chunks.push(Chunk {
            kind,
            version,
            data: Vec::new(),
        });
    }

    /// `WriteRecord`: a new record holding `bytes`.
    pub fn record(&mut self, kind: u32, version: u32, bytes: &[u8]) {
        self.open(kind, version);
        self.data(bytes);
    }

    /// `WriteRecordData`: bytes into the open record (none open: nothing, as xNVSE writes only
    /// into an open chunk).
    pub fn data(&mut self, bytes: &[u8]) {
        if let Some(c) = self.chunks.last_mut() {
            c.data.extend_from_slice(bytes);
        }
    }

    pub fn u8(&mut self, v: u8) {
        self.data(&[v]);
    }
    pub fn u16(&mut self, v: u16) {
        self.data(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.data(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.data(&v.to_le_bytes());
    }
}

/// A listener for xNVSE's messages (`RegisterListener(handle, "NVSE", ...)`).
pub type Listener = fn(&mut Engine, &Message);
/// A save callback: the built-in's records for the co-save being written.
pub type SaveCallback = fn(&mut Engine, &mut Records);
/// A load or pre-load callback: the built-in's records of the co-save being read (none when the
/// co-save has no section of it), through xNVSE's reader.
pub type LoadCallback = fn(&mut Engine, &mut Reader) -> Result<(), OutOfRange>;

/// A plugin nv-rs builds in, as xNVSE knows it.
#[derive(Clone, Copy, Debug)]
pub struct BuiltIn {
    /// The file name of the plugin's DLL: xNVSE loads plugins in the order of their names.
    pub file: &'static str,
    /// `PluginInfo::name` and `::version`.
    pub name: &'static str,
    pub version: u32,
    /// The first opcode it asked for (`SetOpcodeBase`): its co-save section's header.
    pub opcode_base: u32,
    /// Whether it is in the plugin list (answers "installed").
    pub listed: bool,
    pub listener: Option<Listener>,
    pub save: Option<SaveCallback>,
    pub load: Option<LoadCallback>,
    pub pre_load: Option<LoadCallback>,
    pub new_game: Option<fn(&mut Engine)>,
}

/// The built-ins on the engine, in xNVSE's order of plugins.
#[derive(Default)]
pub struct State {
    pub built_ins: Vec<BuiltIn>,
}

/// Registers a built-in, in xNVSE's order of plugins (`InstallPlugins` after `std::sort` of the
/// DLL paths with `_stricmp`). A second registration of the same opcode base is refused.
pub fn register(e: &mut Engine, plugin: BuiltIn) {
    let list = &mut e.ext::<Nvse>().plugins.built_ins;
    assert!(
        list.iter().all(|p| p.opcode_base != plugin.opcode_base),
        "{}: registered twice",
        plugin.name
    );
    let key = plugin.file.to_ascii_lowercase();
    let at = list
        .iter()
        .position(|p| p.file.to_ascii_lowercase() > key)
        .unwrap_or(list.len());
    list.insert(at, plugin);
}

/// The built-ins, in xNVSE's order (a copy: their callbacks take the engine).
pub fn built_ins(e: &Engine) -> Vec<BuiltIn> {
    e.ext_get::<Nvse>()
        .map(|n| n.plugins.built_ins.clone())
        .unwrap_or_default()
}

/// The built-in whose co-save section has `opcode_base` (`LookupHandleFromBaseOpcode`).
pub fn by_opcode_base(e: &Engine, opcode_base: u32) -> Option<BuiltIn> {
    built_ins(e)
        .into_iter()
        .find(|p| p.opcode_base == opcode_base)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), PluginManager.cpp:
// PluginManager::GetInfoByName.
/// The listed built-in named `name` (compared as `_stricmp` does in the C locale: A to Z
/// against a to z, every other byte as it is): its name and version.
pub fn listed(e: &Engine, name: &[u8]) -> Option<(&'static str, u32)> {
    built_ins(e)
        .into_iter()
        .filter(|p| p.listed)
        .find(|p| p.name.as_bytes().eq_ignore_ascii_case(name))
        .map(|p| (p.name, p.version))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), PluginManager.cpp:
// PluginManager::Dispatch_Message (to the plugins' listeners; the callers tell the events first).
/// Sends a message to every built-in's listener, in their order.
pub fn dispatch(e: &mut Engine, kind: u32, data: u32, len: u32) {
    let message = Message { kind, data, len };
    for p in built_ins(e) {
        if let Some(listen) = p.listener {
            listen(e, &message);
        }
    }
}

/// [`dispatch`] of a message whose data is a text of the guest's (its address and length).
pub fn dispatch_guest_text(e: &mut Engine, kind: u32, text: u32) {
    if built_ins(e).iter().all(|p| p.listener.is_none()) {
        return;
    }
    let len = crate::table::guest_text(e, text).len() as u32;
    dispatch(e, kind, text, len);
}

/// [`dispatch`] of a message whose data is a host text, copied to the guest stack for the call
/// (xNVSE's `std::string` buffers), with the text's length.
pub fn dispatch_text(e: &mut Engine, kind: u32, text: &[u8]) {
    dispatch_text_as(e, kind, text, text.len() as u32);
}

/// [`dispatch_text`] with the length xNVSE gives (`RuntimeScriptError` says 4, a pointer's
/// size, whatever the text).
pub fn dispatch_text_as(e: &mut Engine, kind: u32, text: &[u8], len: u32) {
    if built_ins(e).iter().all(|p| p.listener.is_none()) {
        return;
    }
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        for (i, b) in text.iter().enumerate() {
            e.mem.set_u8(at.addr() + i as u32, *b);
        }
        e.mem.set_u8(at.addr() + text.len() as u32, 0);
        dispatch(e, kind, at.addr(), len);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static HEARD: RefCell<Vec<(&'static str, Message, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    }

    fn plugin(file: &'static str, name: &'static str, base: u32, listed: bool) -> BuiltIn {
        BuiltIn {
            file,
            name,
            version: base,
            opcode_base: base,
            listed,
            listener: None,
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        }
    }

    fn heard_a(e: &mut Engine, m: &Message) {
        let text = crate::table::guest_text(e, m.data);
        HEARD.with(|h| h.borrow_mut().push(("a", *m, text)));
    }

    fn heard_b(_: &mut Engine, m: &Message) {
        HEARD.with(|h| h.borrow_mut().push(("b", *m, Vec::new())));
    }

    /// Built-ins sort by their DLL's name without case; the same base twice is refused; only
    /// listed ones are in the plugin list, found by name without case.
    #[test]
    fn built_ins_in_xnvses_order() {
        let mut e = Engine::new();
        register(&mut e, plugin("johnnyguitarnvse.dll", "JG", 0x3100, true));
        register(&mut e, plugin("jip_nvse.dll", "JIP LN NVSE", 0x2200, false));
        register(&mut e, plugin("ShowOffNVSE.dll", "ShowOff", 0x3c93, true));
        register(&mut e, plugin("hot_reload.dll", "Hot", 0x2000, true));
        let files: Vec<&str> = built_ins(&e).iter().map(|p| p.file).collect();
        assert_eq!(
            files,
            [
                "hot_reload.dll",
                "jip_nvse.dll",
                "johnnyguitarnvse.dll",
                "ShowOffNVSE.dll"
            ]
        );
        assert_eq!(by_opcode_base(&e, 0x2200).unwrap().name, "JIP LN NVSE");
        assert!(by_opcode_base(&e, 0x2600).is_none());
        assert_eq!(listed(&e, b"jg"), Some(("JG", 0x3100)));
        assert_eq!(listed(&e, b"JIP LN NVSE"), None, "not listed");
        let twice = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            register(&mut e, plugin("x.dll", "X", 0x2200, true))
        }));
        assert!(twice.is_err());
    }

    /// A message goes to every listener in the built-ins' order, a host text through the guest
    /// stack with its length.
    #[test]
    fn messages_reach_the_listeners() {
        HEARD.with(|h| h.borrow_mut().clear());
        let mut e = Engine::new();
        dispatch_text(&mut e, msg::DELETE_GAME, b"nobody listens");
        let mut b = plugin("b.dll", "B", 0x2100, false);
        b.listener = Some(heard_b);
        let mut a = plugin("a.dll", "A", 0x2000, false);
        a.listener = Some(heard_a);
        register(&mut e, b);
        register(&mut e, a);
        dispatch(&mut e, msg::NEW_GAME, 0, 0);
        dispatch_text(&mut e, msg::SAVE_GAME, b"Save 1.fos");
        let heard = HEARD.with(|h| std::mem::take(&mut *h.borrow_mut()));
        let order: Vec<(&str, u32)> = heard.iter().map(|(w, m, _)| (*w, m.kind)).collect();
        assert_eq!(
            order,
            [
                ("a", msg::NEW_GAME),
                ("b", msg::NEW_GAME),
                ("a", msg::SAVE_GAME),
                ("b", msg::SAVE_GAME)
            ]
        );
        assert_eq!(heard[2].2, b"Save 1.fos");
        assert_eq!(heard[2].1.len, 10);
    }

    /// Records: each `open` starts a chunk the writes go into; `record` opens one with its bytes.
    #[test]
    fn records_are_chunks() {
        let mut r = Records::default();
        r.data(b"lost");
        r.record(0x4a50_4746, 1, &[1, 2]);
        r.u8(3);
        r.open(0x4a50_5356, 9);
        r.u16(0x0504);
        r.u32(0x0908_0706);
        r.u64(0x1110_0f0e_0d0c_0b0a);
        assert_eq!(r.chunks.len(), 2);
        assert_eq!(
            (r.chunks[0].kind, r.chunks[0].version, &r.chunks[0].data[..]),
            (0x4a50_4746, 1, &[1u8, 2, 3][..])
        );
        assert_eq!(
            r.chunks[1].data,
            [4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11]
        );
    }
}
