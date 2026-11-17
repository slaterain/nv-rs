//! What the crate's tests share: an engine with xNVSE's table (over a stand-in of the exe's two
//! tables) and a call through a command's record, as `EvaluateLine` makes it.

use engine_core::prelude::*;

/// A call's guest data: the compiled line, its offset word, the result, the script and its
/// event list (nothing in them: the identity commands read plain strings).
pub const DATA: u32 = 0x0300_0000;
pub const OFFSET: u32 = 0x0300_0f00;
pub const RESULT: u32 = 0x0300_0f08;
pub const SCRIPT: u32 = 0x0300_1000;
pub const LIST: u32 = 0x0300_2000;

/// The console's open byte (`0x011DEA2E`) and its line flag in the thread's TLS block (`+0x268`):
/// both set is the console mode JIP's echo asks for.
const CONSOLE_OPEN: u32 = 0x011d_ea2e;

/// An engine with xNVSE's table installed over empty stand-ins of the exe's console and script
/// tables, and room for a call's data; JIP not yet installed.
pub fn engine() -> Engine {
    let mut e = Engine::new();
    prepare(&mut e);
    e
}

/// xNVSE's table and JIP answering "installed" put into an engine a test already made (an
/// engine crate's, with its own stand-ins and mappings): what [`engine`] does (a page mapped
/// twice keeps its bytes), then [`crate::install_as`]. A command is then called through its
/// record with [`call`]; the call's data is at [`DATA`] (64 KB).
pub fn install_into(e: &mut Engine) {
    install_into_as(e, true);
}

/// [`install_into`] with JIP answering "installed" or not (`answers_installed`): the engine
/// crates' tests of a translation with JIP present but not installed.
pub fn install_into_as(e: &mut Engine, answers_installed: bool) {
    prepare(e);
    crate::install_as(e, answers_installed);
}

/// What [`engine`] does to a new engine.
fn prepare(e: &mut Engine) {
    e.map(0x0118_e000, 0x9000);
    e.map(CONSOLE_OPEN & !0xfff, 0x1000);
    e.map(DATA, 0x1_0000);
    // The processes' vtables (HighProcess 0x01087864, MiddleHighProcess 0x0108904C), where JIP
    // writes its hit-data routines once installed, and the player's MagicTarget vtable slot
    // 0x0108A9B4 (game_patches::gp, G122).
    e.map(0x0108_7000, 0x4000);
    // `ExtraTimeLeft`'s vtable (0x010158FC), whose two slots JIP's ExtraJIP framework writes
    // (step 3.9 GH1).
    e.map(0x0101_5000, 0x1000);
    // The queued references' vtables (0x01016BA4 to 0x01016D7C), where JIP writes its attaches
    // (game_patches::gp).
    e.map(0x0101_6000, 0x1000);
    nvse::table::install(e);
    // The platform's XInputGetState at its import slot, as the host registers it (no controller:
    // ERROR_DEVICE_NOT_CONNECTED), for JIP's wrap there.
    e.register(0x00fd_f394, |_, _| Ret {
        eax: 0x48f,
        ..Ret::default()
    });
    // `ExtraDataList::GetCanNotWear` (`00418b10`, JIP's wrap there), when the test engine has no
    // translation of it: the exe's test, `BaseExtraList::HasExtra(0x3E)` (`0040fe80`), the bit of
    // type 0x3E in the presence bits at `+8` (byte `+0xF`, bit `0x40`).
    // `00482090` (a container's items' health, JIP's wrap there: group GH1, G50), when the test
    // engine has no translation of it: a stand-in that does nothing (JIP's wrap replaces it).
    if !e.is_registered(0x0048_2090) {
        e.register(0x0048_2090, |_, _| Ret::default());
    }
    if !e.is_registered(0x0041_8b10) {
        e.register(0x0041_8b10, |e, a| Ret {
            eax: u32::from(e.mem.u8(a[0] + 0xf) & 0x40 != 0),
            ..Ret::default()
        });
    }
    // `TESImageSpaceModifier::GetGetHit` (`005d2860`, JIP's wrap there once installed: step 3.9
    // GS, G123), when the test engine has no translation of it: no "get hit" modifier (the
    // exe's answer while its image space byte is 0).
    if !e.is_registered(0x005d_2860) {
        e.register(0x005d_2860, |_, _| Ret::default());
    }
    // Group GC's wraps (G109 `0058e9d0`, G111 `00646260`), when the test engine has no
    // translation of them: the impact data set's slot for the material (the exe's answer with no
    // override set), and `fDamageToWeaponValue`'s value (`0x11CF154`, the exe's with no ammo).
    if !e.is_registered(0x0058_e9d0) {
        e.register(0x0058_e9d0, |e, a| Ret {
            eax: e
                .mem
                .u32(a[0].wrapping_add(a[1].wrapping_mul(4)).wrapping_add(0x1c)),
            ..Ret::default()
        });
    }
    if !e.is_registered(0x0064_6260) {
        e.map(0x011c_f000, 0x1000);
        e.register(0x0064_6260, |e, _| f32::into_ret(e.mem.f32(0x011c_f154)));
    }
    // G84's and G85's wraps (`00646d00`, `00646360`), when the test engine has no translation of
    // them: answers no test reads (JIP's wraps replace them, never handing the call on).
    for at in [0x0064_6d00, 0x0064_6360] {
        if !e.is_registered(at) {
            e.register(at, |_, _| f32::into_ret(-1.0));
        }
    }
    // Group GX's wraps (G59 `00ee820a`, the `XInputSetState` thunk; H20 `00a239e0`, the mouse
    // axis read), when the test engine has no translation of them: a marker answer (0x5a5a).
    for at in [0x00ee_820a, 0x00a2_39e0] {
        if !e.is_registered(at) {
            e.register(at, |_, _| Ret {
                eax: 0x5a5a,
                ..Ret::default()
            });
        }
    }
    // O1's wrap (`009b5a30`, `HitData::ReduceDamage`): a stand-in that leaves the hit as it is.
    if !e.is_registered(0x009b_5a30) {
        e.register(0x009b_5a30, |_, _| Ret::default());
    }
    // O5's wrap (`00867a40`, `GameTimeGlobals::Update`) when the test engine has no translation
    // of it: a marker answer (0x5a5a).
    if !e.is_registered(0x0086_7a40) {
        e.register(0x0086_7a40, |_, _| Ret {
            eax: 0x5a5a,
            ..Ret::default()
        });
    }
    // Group GW's G113 and D05 wraps (`00595560`, `004f4f90`, `005e3630`) when the test engine has
    // no translation of them: marker answers (0x5a5b, 0x5a5c, 0x5a5d).
    if !e.is_registered(0x0059_5560) {
        e.register(0x0059_5560, |_, _| 0x5a5bu32.into_ret());
    }
    if !e.is_registered(0x004f_4f90) {
        e.register(0x004f_4f90, |_, _| 0x5a5cu32.into_ret());
    }
    if !e.is_registered(0x005e_3630) {
        e.register(0x005e_3630, |_, _| 0x5a5du32.into_ret());
    }
    // G108's wraps (`00af01e0`, `00af0290`, `BSWin32GameSound`'s frequency set and read): marker
    // answers (0x5a5e, 7.0).
    if !e.is_registered(0x00af_01e0) {
        e.register(0x00af_01e0, |_, _| 0x5a5eu32.into_ret());
    }
    if !e.is_registered(0x00af_0290) {
        e.register(0x00af_0290, |_, _| 7.0f32.into_ret());
    }
    // Group GF: `TESNPC`'s model vtable (0x0104A1A4), whose set slot `+0x18` JIP writes (G40),
    // the two skeleton settings (0x011D2718, 0x011D3E44), and `00601c30` (G39's wrap) when the
    // test engine has no translation of it: the exe's answer, the female setting's text for a
    // female NPC (`+0x34` bit 0 of the NPC, the model being at `+0xDC`), else the male one's.
    e.map(0x0104_a000, 0x1000);
    e.map(0x011d_2000, 0x2000);
    // `004f15a0` (G48's wrap) when the test engine has no translation of it: the exe's, the
    // word's low byte stored at `+4` alone, the form answered.
    if !e.is_registered(0x004f_15a0) {
        e.register(0x004f_15a0, |e, a| {
            e.mem.set_u8(a[0].wrapping_add(4), a[1] as u8);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
    }
    if !e.is_registered(0x0060_1c30) {
        e.register(0x0060_1c30, |e, a| {
            let female = e.mem.u8(a[0].wrapping_sub(0xdc).wrapping_add(0x34)) & 1 != 0;
            Ret {
                eax: e.mem.u32(if female { 0x011d_271c } else { 0x011d_3e48 }),
                ..Ret::default()
            }
        });
    }
}

/// Opens or closes the console as the echo sees it.
pub fn console(e: &mut Engine, open: bool) {
    e.mem.set_u8(CONSOLE_OPEN, u8::from(open));
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, u8::from(open));
}

/// A string argument as the compiler writes it: its length, then its bytes.
pub fn s(text: &[u8]) -> Vec<u8> {
    let mut b = (text.len() as u16).to_le_bytes().to_vec();
    b.extend_from_slice(text);
    b
}

/// The eight handler words of a call to `opcode` with `args`, its line written at [`DATA`] and
/// the result set to 0 (as xNVSE zeroes it before an extender command runs).
pub fn line(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> [u32; 8] {
    e.mem.set_u16(DATA, opcode as u16);
    let bytes: Vec<u8> = args.concat();
    e.mem.set_u16(DATA + 2, bytes.len() as u16 + 2);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(DATA + 6 + i as u32, *b);
    }
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = nvse::table::record_of(e, opcode);
    assert_ne!(record, 0, "{opcode:04x}");
    let params = e.mem.u32(record + 0x14);
    [params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]
}

/// The execute word of `opcode`'s record.
pub fn execute_of(e: &Engine, opcode: u32) -> u32 {
    e.mem.u32(nvse::table::record_of(e, opcode) + 0x18)
}

/// Calls `opcode` through its record's execute with `args`: the handler's `al` and the result.
pub fn call(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> (u32, f64) {
    let words = line(e, opcode, args);
    let execute = execute_of(e, opcode);
    let ok = e.call(execute, &words).eax & 0xff;
    (ok, e.mem.f64(RESULT))
}

/// Two co-saves the real game wrote with xNVSE 6.4.9 and JIP LN NVSE 57.30 (the scratch copy
/// `game-xnvse`, 2026-11-07: `nvsecos1.nvse`, and `nvsecos4.nvse`, byte for byte `nvsecos2.nvse`).
/// JIP's section in each holds only `JPGF` (no flags, a cap of 1.0): no co-save of JIP's other
/// records, or of JIP with JohnnyGuitar NVSE, has been made yet.
pub const REAL_COSAVES: [(&str, &[&str]); 2] = [
    (
        "nvsecos1",
        &[
            "4e5653450100000006000400d02000040200000000140000060000001c01000053444f4d00000000b20000000a0d0046",
            "616c6c6f75744e562e65736d0d00446561644d6f6e65792e65736d1000486f6e6573744865617274732e65736d11004f",
            "6c64576f726c64426c7565732e65736d10004c6f6e65736f6d65526f61642e65736d0e0054726962616c5061636b2e65",
            "736d11004d657263656e6172795061636b2e65736d0f00436c61737369635061636b2e65736d0f004361726176616e50",
            "61636b2e65736d150047756e52756e6e657273417273656e616c2e65736d535652410200000000000000525652410200",
            "000022000000ff03000000010101000000ff0200000001000000000000f83f0100000000000004404556524102000000",
            "000000005356545300000000000000004556545300000000000000000022000001000000140000004a50474601000000",
            "08000000000000000000803f",
        ],
    ),
    (
        "nvsecos4",
        &[
            "4e5653450100000006000400d020000402000000001400000b000000d401000053444f4d00000000b20000000a0d0046",
            "616c6c6f75744e562e65736d0d00446561644d6f6e65792e65736d1000486f6e6573744865617274732e65736d11004f",
            "6c64576f726c64426c7565732e65736d10004c6f6e65736f6d65526f61642e65736d0e0054726962616c5061636b2e65",
            "736d11004d657263656e6172795061636b2e65736d0f00436c61737369635061636b2e65736d0f004361726176616e50",
            "61636b2e65736d150047756e52756e6e657273417273656e616c2e65736d535652410200000000000000525652410200",
            "0000280000000205000000030001000000010200000001004101000000000000f03f0100620100000000000000405256",
            "5241020000002700000001020000000101010000000104000000010000000000000a4003010078023412000104050000",
            "0052565241020000002b00000003090000000100010000000302000000000000000000f0bf0214000000000000000000",
            "254003030074656e45565241020000000000000053565453000000000000000052565453000000000c00000000010000",
            "000500616c70686152565453000000000d000000010400000006004dfc6c6c657252565453000000000b000000040700",
            "00000400666f75724556545300000000000000000022000001000000140000004a504746010000000800000000000000",
            "0000803f",
        ],
    ),
];

pub fn unhex(lines: &[&str]) -> Vec<u8> {
    let hex: String = lines.concat();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// Script reference `index` (from 1) is `form`: the reference list at the script's `+0x44`, each
/// entry's form at `+8` (as `nvse::args` reads an `r` argument).
pub fn script_ref(e: &mut Engine, index: u16, form: u32) {
    let i = u32::from(index);
    let ref_var = 0x0300_a000 + i * 0x10;
    e.mem.set_u32(ref_var + 8, form);
    e.mem.set_u32(ref_var + 0xc, 0);
    let mut node = SCRIPT + 0x44;
    for at in 1..i {
        let next = e.mem.u32(node + 4);
        node = if next == 0 {
            let made = 0x0300_b000 + (at + 1) * 8;
            e.mem.set_u32(node + 4, made);
            made
        } else {
            next
        };
    }
    e.mem.set_u32(node, ref_var);
}

/// A form argument: script reference `index` (`r`).
pub fn form_arg(index: u16) -> Vec<u8> {
    let mut b = vec![b'r'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}

/// An integer argument (`n`).
pub fn int_arg(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// A 16-bit code argument (actor value, animation group, ...) as the compiler writes it: the
/// two bytes alone, no type byte.
pub fn short_arg(v: u16) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// xNVSE's record of key `k`, for a test to change; the keys' lock-free copy
/// (`nvse::input::publish_key_states`, which a device read writes) is written when it goes, so
/// JIP's key reads (which read that copy) see the change.
pub fn key(e: &mut Engine, k: u32) -> KeyEdit<'_> {
    KeyEdit { e, k: k as usize }
}

/// [`key`]'s guard.
pub struct KeyEdit<'a> {
    e: &'a mut Engine,
    k: usize,
}

impl std::ops::Deref for KeyEdit<'_> {
    type Target = nvse::input::KeyInfo;
    fn deref(&self) -> &nvse::input::KeyInfo {
        &self
            .e
            .ext_get::<nvse::Nvse>()
            .expect("xNVSE is installed")
            .input
            .keys[self.k]
    }
}

impl std::ops::DerefMut for KeyEdit<'_> {
    fn deref_mut(&mut self) -> &mut nvse::input::KeyInfo {
        &mut self.e.ext::<nvse::Nvse>().input.keys[self.k]
    }
}

impl Drop for KeyEdit<'_> {
    fn drop(&mut self) {
        nvse::input::publish_key_states(self.e);
    }
}

/// Runs `f` on `e` while a second engine of the run (a real thread's: `share_slots_for_test`)
/// holds JIP's shared slot. A path that asks for JIP's slot stops the test (two engines on one
/// thread), so `f` finishing shows the path does not take it: the hooks' seams that run on the AI
/// and loader threads.
pub fn with_jips_slot_held<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(
        other.ext::<crate::Jip>().installed,
        "the other engine holds JIP's state"
    );
    let r = f(e);
    engine_core::extend::release_held_for_test();
    r
}

/// Runs `f` on `e` while a second engine of the run (a real thread's: `share_slots_for_test`)
/// holds xNVSE's shared slot. A path that asks for xNVSE's slot stops the test (two engines on
/// one thread), so `f` finishing shows the path does not take it: a condition evaluated on an AI
/// thread.
pub fn with_nvses_slot_held<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    // Shared as `nvse::install` shares it (a test engine may hold it as its own).
    e.share_ext::<nvse::Nvse>();
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    let _ = other.ext::<nvse::Nvse>();
    let r = f(e);
    engine_core::extend::release_held_for_test();
    r
}
