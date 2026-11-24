use super::*;
use nvse::{functions, table};
const DATA: u32 = 0x0300_0000;
const PLAYER_REF: u32 = DATA + 0x1000;
const PROCESS: u32 = DATA + 0x2000;
const CONTROLLER: u32 = DATA + 0x3000;
const OTHER_CONTROLLER: u32 = DATA + 0x4000;
const FIRER: u32 = DATA + 0x5000;
const WEAPON: u32 = DATA + 0x6000;
const FIRE_SCRIPT: u32 = DATA + 0x7000;
const JUMP_SCRIPT: u32 = DATA + 0x7200;
const POV_SCRIPT: u32 = DATA + 0x7400;
#[derive(Debug, PartialEq)]
struct Call {
    script: u32,
    this: u32,
    word: u32,
    number: f64,
}
#[derive(Default)]
struct Calls(Vec<Call>);
fn function(e: &mut Engine, at: u32, kind: Option<u8>) {
    let mut code = vec![0x1d, 0, 0, 0, 0x10, 0];
    let mut block = vec![0x0d, 0, 0, 0, 0, 0, 1, u8::from(kind.is_some())];
    if let Some(kind) = kind {
        block.extend([1, 0, kind]);
    }
    code.extend_from_slice(&(block.len() as u16).to_le_bytes());
    code.extend(block);
    code.resize(0x40, 0);
    e.mem.write(at + 0x100, &code);
    e.mem.set_u8(at + 4, 0x11);
    e.mem.set_u32(at + 0xc, at - DATA);
    e.mem.set_u32(at + 0x30, at + 0x100);
    e.mem.set_u32(at + 0x20, code.len() as u32);
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    table::install(&mut e);
    functions::install(&mut e);
    install(&mut e);
    e.mem.set_u32(functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    e.mem.set_u32(PLAYER, PLAYER_REF);
    e.mem.set_u32(PLAYER_REF + 0xc, 0x14);
    e.mem.set_u32(PLAYER_REF + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x138, CONTROLLER);
    e.mem.set_u32(FIRER + 0xc, 0x0500_5000);
    e.mem.set_u32(WEAPON + 0xc, 0x0500_6000);
    function(&mut e, FIRE_SCRIPT, Some(4));
    function(&mut e, JUMP_SCRIPT, None);
    function(&mut e, POV_SCRIPT, Some(1));
    e.register(functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        let local = e.mem.alloc(0x10);
        e.mem.set_u32(local, 1);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, local);
        e.mem.set_u32(list + 0xc, node);
        list.into_ret()
    });
    e.register(functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(functions::SCRIPT_EXECUTE, |e, a| {
        let local = nvse::args::event_list_variable(e, a[2], 1);
        let call = Call {
            script: a[0],
            this: a[1],
            word: e.mem.u32(local + 8),
            number: e.mem.f64(local + 8),
        };
        e.ext::<Calls>().0.push(call);
        Ret::default()
    });
    for (name, script) in [
        (b"ShowOff:OnFireWeapon".as_slice(), FIRE_SCRIPT),
        (b"ShowOff:OnPlayerJump".as_slice(), JUMP_SCRIPT),
        (b"ShowOff:OnPOVChange".as_slice(), POV_SCRIPT),
    ] {
        assert!(events::set_handler(
            &mut e,
            name,
            events::Callback::new(script),
            1,
            None
        ));
    }
    e
}
#[test]
fn source_typed_registry_flags_context_and_native_integer_binding_are_real() {
    let mut e = engine();
    for (name, params, flags) in [
        (
            b"ShowOff:OnFireWeapon".as_slice(),
            vec![events::param::BASE_FORM],
            events::FLAG_FLUSH_ON_LOAD,
        ),
        (b"ShowOff:OnPlayerJump".as_slice(), vec![], 0),
        (
            b"ShowOff:OnPOVChange".as_slice(),
            vec![events::param::INT],
            0,
        ),
    ] {
        let id = events::find(&mut e, name).unwrap();
        let info = events::state_info(&mut e, id);
        assert_eq!(info.params, params);
        assert_eq!(info.flags, flags);
    }
    assert_eq!(e.site(FIRE.addr(), &[FIRER, WEAPON]), None);
    let call = &e.ext::<Calls>().0[0];
    assert_eq!(call.script, FIRE_SCRIPT);
    assert_eq!(call.this, FIRER);
    assert_eq!(call.word, 0x0500_6000);
    for word in [0u32, 1, 0x8000_0001, u32::MAX] {
        assert_eq!(e.site(POV.addr(), &[word]), None);
        let call = e.ext::<Calls>().0.last().unwrap();
        assert_eq!(call.script, POV_SCRIPT);
        assert_eq!(call.this, PLAYER_REF);
        assert_eq!(call.number, f64::from(word as i32));
    }
    assert!(events::current_event_name(&mut e).is_empty());
}
#[test]
fn jump_compares_the_actual_high_process_controller_and_not_a_cached_owner() {
    let mut e = engine();
    assert_eq!(e.site(JUMP.addr(), &[OTHER_CONTROLLER]), None);
    assert!(e.ext::<Calls>().0.is_empty());
    e.site(JUMP.addr(), &[CONTROLLER]);
    assert_eq!(
        e.ext::<Calls>().0[0],
        Call {
            script: JUMP_SCRIPT,
            this: PLAYER_REF,
            word: 0,
            number: 0.0
        }
    );
    e.mem.set_u32(PROCESS + 0x138, OTHER_CONTROLLER);
    e.site(JUMP.addr(), &[CONTROLLER]);
    assert_eq!(e.ext::<Calls>().0.len(), 1);
    e.site(JUMP.addr(), &[OTHER_CONTROLLER]);
    assert_eq!(e.ext::<Calls>().0.len(), 2);
}
#[test]
fn only_fire_callbacks_flush_on_load_and_notifications_cannot_veto_native_calls() {
    let mut e = engine();
    events::clear_flush_on_load(&mut e);
    assert_eq!(e.site(FIRE.addr(), &[FIRER, WEAPON]), None);
    assert!(e.ext::<Calls>().0.is_empty());
    assert_eq!(e.site(JUMP.addr(), &[CONTROLLER]), None);
    assert_eq!(e.site(POV.addr(), &[1]), None);
    assert_eq!(e.ext::<Calls>().0.len(), 2);
    assert_eq!(e.site_owners(FIRE.addr()), [Owner::ShowOff]);
    assert_eq!(e.site_owners(JUMP.addr()), [Owner::ShowOff]);
    assert_eq!(e.site_owners(POV.addr()), [Owner::ShowOff]);
}
