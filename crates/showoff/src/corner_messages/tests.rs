use super::*;
use nvse::{functions, table, vars};
const DATA: u32 = 0x0300_0000;
const SCRIPT: u32 = DATA + 0x1000;
const TEXT: u32 = DATA + 0x2000;
const ICON: u32 = DATA + 0x3000;
const SOUND: u32 = DATA + 0x4000;
#[derive(Clone, Debug, PartialEq)]
struct Call {
    this: u32,
    text: Vec<u8>,
    icon: Vec<u8>,
    sound: Vec<u8>,
    time: f64,
    event: Vec<u8>,
}
#[derive(Default)]
struct Seen(Vec<Call>);
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011c_3000, 0x1000);
    table::install(&mut e);
    functions::install(&mut e);
    install(&mut e);
    e.mem.set_u32(functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    let mut bytes = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = [
        0x0d, 0, 0, 0, 0, 0, 1, 4, 1, 0, 2, 2, 0, 2, 3, 0, 2, 4, 0, 0,
    ];
    bytes.extend_from_slice(&(block.len() as u16).to_le_bytes());
    bytes.extend(block);
    bytes.resize(0x40, 0);
    // A genuine return-zero statement. Native DispatchEvent deliberately
    // ignores this UDF return rather than inventing a queue veto.
    bytes.extend([1, 4, 0, b'B', 0]);
    e.mem.write(SCRIPT + 0x100, &bytes);
    e.mem.set_u8(SCRIPT + 4, 0x11);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_1000);
    e.mem.set_u32(SCRIPT + 0x30, SCRIPT + 0x100);
    e.mem.set_u32(SCRIPT + 0x20, bytes.len() as u32);
    e.register(functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        let mut next = 0;
        for id in [4, 3, 2, 1] {
            let local = e.mem.alloc(0x10);
            e.mem.set_u32(local, id);
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, local);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        e.mem.set_u32(list + 0xc, next);
        list.into_ret()
    });
    e.register(functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(functions::SCRIPT_EXECUTE, |e, a| {
        let read = |e: &Engine, id| {
            let at = nvse::args::event_list_variable(e, a[2], id);
            e.mem.f64(at + 8)
        };
        let call = Call {
            this: a[1],
            text: vars::string(e, read(e, 1) as u32).unwrap_or_default(),
            icon: vars::string(e, read(e, 2) as u32).unwrap_or_default(),
            sound: vars::string(e, read(e, 3) as u32).unwrap_or_default(),
            time: read(e, 4),
            event: events::current_event_name(e),
        };
        e.ext::<Seen>().0.push(call);
        let rec = table::record_of(e, functions::OP_SET_FUNCTION_VALUE);
        e.with_stack(12, |e, f| {
            e.mem.set_u32(f.addr() + 8, 0x40);
            e.call(
                e.mem.u32(rec + 0x18),
                &[
                    e.mem.u32(rec + 0x14),
                    e.mem.u32(a[0] + 0x30),
                    0,
                    0,
                    a[0],
                    a[2],
                    f.addr(),
                    f.addr() + 8,
                ],
            );
        });
        Ret::default()
    });
    assert!(events::set_handler(
        &mut e,
        EVENT,
        events::Callback::new(SCRIPT),
        1,
        None
    ));
    e.mem.write(TEXT, b"&sUActnVats\x96unformatted\0");
    e.mem.write(ICON, b"Interface\\Icons\\warning.dds\0");
    e.mem.write(SOUND, b"UIPickpocketFail\0");
    e
}
#[test]
fn all_three_real_sites_dispatch_synchronously_with_native_strings_and_float_bits() {
    let mut e = engine();
    let event = events::find(&mut e, EVENT).unwrap();
    let info = events::state_info(&mut e, event);
    assert_eq!(
        info.params,
        [
            events::param::STRING,
            events::param::STRING,
            events::param::STRING,
            events::param::FLOAT
        ]
    );
    assert_eq!(info.flags, 0);
    assert!(events::find(&mut e, b"ShowOff:OnShowCornerMessage").is_none());
    for (i, site) in [APPEND_FIRST, APPEND_QUEUED, INSERT_URGENT]
        .into_iter()
        .enumerate()
    {
        let time = 1.234_567_9f32 + i as f32;
        assert_eq!(e.site_owners(site.addr()), [Owner::ShowOff]);
        assert_eq!(
            e.site(site.addr(), &[TEXT, ICON, SOUND, time.to_bits()]),
            None
        );
        assert_eq!(
            e.ext::<Seen>().0.len(),
            i + 1,
            "immediate even before NVSE's main-thread tick"
        );
        assert_eq!(
            e.ext::<Seen>().0[i],
            Call {
                this: 0,
                text: b"&sUActnVats\x96unformatted".to_vec(),
                icon: b"Interface\\Icons\\warning.dds".to_vec(),
                sound: b"UIPickpocketFail".to_vec(),
                time: f64::from(time),
                event: b"ShowOff:OnQueueCornerMessage".to_vec(),
            }
        );
    }
    assert!(events::current_event_name(&mut e).is_empty());
}
#[test]
fn event_survives_load_and_preserves_empty_strings_nan_and_negative_zero() {
    let mut e = engine();
    events::clear_flush_on_load(&mut e);
    e.mem.set_u8(TEXT, 0);
    e.mem.set_u8(ICON, 0);
    e.mem.set_u8(SOUND, 0);
    assert_eq!(
        e.site(
            APPEND_FIRST.addr(),
            &[TEXT, ICON, SOUND, (-0.0f32).to_bits()]
        ),
        None
    );
    let call = &e.ext::<Seen>().0[0];
    assert!(call.text.is_empty() && call.icon.is_empty() && call.sound.is_empty());
    assert_eq!(call.time.to_bits(), (-0.0f64).to_bits());
    assert_eq!(
        e.site(APPEND_QUEUED.addr(), &[TEXT, ICON, SOUND, 0x7fc1_2345]),
        None
    );
    assert!(e.ext::<Seen>().0[1].time.is_nan());
}
