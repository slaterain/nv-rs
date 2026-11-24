//! ShowOff f45d069 SO_fn_Misc.cpp::Cmd_ToANSIChar_Execute (3D66).
//!
//! The host provides actual layout text observed on its native window thread.
//! This implements the pressed-key path used by MCM Extender's search input.
//! Unobserved keys, dead keys and unsupported modifier combinations are
//! unavailable; no US keyboard table or complete ShowOff version is invented.
use engine_core::prelude::*;
use nvse::{commands::Read, handled, table, CommandArgs};

const OPCODE: u32 = 0x3d66;
const WIDE_CHAR_TO_MULTI_BYTE: u32 = 0x00fd_f170;

pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 24);
    for (i, (name, optional)) in [("scan code", false), ("ignore shift", true)]
        .into_iter()
        .enumerate()
    {
        let at = params + i as u32 * 12;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, 1);
        e.mem.set_u32(at + 8, u32::from(optional));
    }
    let execute = table::execute_address(OPCODE);
    e.register(execute, execute_command);
    table::add_plugin_record(
        e,
        OPCODE,
        &table::PluginRecord {
            name: "ToANSIChar",
            short: "",
            needs_ref: false,
            param_count: 2,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: table::Returns::String,
        },
    );
}

fn execute_command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let read = Read::of(e, &a);
    if !read.ok {
        nvse::commands::string::assign_to_string_var(e, &a, b"");
        return handled(true);
    }
    let Some(text) = engine_core::key_text::get(read.int(0, 0), read.int(1, 0) != 0) else {
        // As in the source's failed keyboard-state/map/dead-key queries, no
        // string assignment occurs. Empty text is not a guessed character.
        return handled(true);
    };
    if text.is_empty() || text.len() > 16 {
        return handled(true);
    }
    let bytes = e.with_stack(64, |e, base| {
        for (i, unit) in text.iter().enumerate() {
            e.mem.set_u16(base.addr() + i as u32 * 2, *unit);
        }
        let out = base.addr() + 32;
        let count = e
            .call(
                WIDE_CHAR_TO_MULTI_BYTE,
                &args![0u32, 0u32, base, text.len() as u32, out, 16u32, 0u32, 0u32],
            )
            .u32();
        if count == 0 || count > 16 {
            return None;
        }
        // An extra byte avoids the original 16-byte buffer's terminator
        // overrun when the conversion fills it. CString semantics retained.
        e.mem.set_u8(out + count, 0);
        Some(e.mem.cstr(out))
    });
    if let Some(bytes) = bytes {
        nvse::commands::string::assign_to_string_var(e, &a, &bytes);
    }
    handled(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    type EventText = Option<(u32, Vec<u16>, Vec<u16>)>;
    thread_local! { static TEXT: RefCell<EventText> = const { RefCell::new(None) }; }
    fn provider(scan: u32, ignore: bool) -> Option<Vec<u16>> {
        TEXT.with(|slot| {
            slot.borrow().as_ref().filter(|v| v.0 == scan).map(|v| {
                if ignore {
                    v.2.clone()
                } else {
                    v.1.clone()
                }
            })
        })
    }
    const DATA: u32 = 0x0300_0000;
    const RESULT: u32 = DATA + 0x100;
    const OFFSET: u32 = RESULT + 8;
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(DATA, 0x1000);
        e.map(0x0118_e000, 0x9000);
        table::install(&mut e);
        install(&mut e);
        engine_core::key_text::install(provider);
        e.register(WIDE_CHAR_TO_MULTI_BYTE, |e, a| {
            assert_eq!(a[0..2], [0, 0]);
            assert_eq!(a[5..], [16, 0, 0]);
            // Explicit measured ACP1252 fixture. The product calls the real
            // portable NLS conversion, not this fixture or a UTF-8 cast.
            for i in 0..a[3] {
                let unit = e.mem.u16(a[2] + 2 * i);
                assert!(unit <= 255);
                e.mem.set_u8(a[4] + i, unit as u8);
            }
            a[3].into_ret()
        });
        e
    }
    fn call(e: &mut Engine, scan: u32, ignore: Option<u32>) -> f64 {
        let mut data = vec![];
        data.extend_from_slice(&(if ignore.is_some() { 2u16 } else { 1u16 }).to_le_bytes());
        for value in std::iter::once(scan).chain(ignore) {
            data.push(b'n');
            data.extend_from_slice(&value.to_le_bytes());
        }
        e.mem.set_u16(DATA, OPCODE as u16);
        e.mem.set_u16(DATA + 2, data.len() as u16);
        e.mem.write(DATA + 4, &data);
        e.mem.set_u32(OFFSET, 4);
        e.mem.set_f64(RESULT, -1234.0);
        let record = table::record_of(e, OPCODE);
        let params = e.mem.u32(record + 0x14);
        assert_eq!(
            e.call(
                e.mem.u32(record + 0x18),
                &args![params, DATA, 0u32, 0u32, 0u32, 0u32, RESULT, OFFSET]
            )
            .u8(),
            1
        );
        e.mem.f64(RESULT)
    }
    #[test]
    fn record_and_pressed_layout_characters_preserve_optional_shift_and_acp() {
        let mut e = engine();
        let record = table::record_of(&e, OPCODE);
        assert_eq!(table::opcode_by_name(&e, b"ToANSIChar"), Some(OPCODE));
        assert_eq!(table::return_type(&e, OPCODE), table::Returns::String);
        assert_eq!(e.mem.u16(record + 0x10), 0);
        assert_eq!(e.mem.u16(record + 0x12), 2);
        let params = e.mem.u32(record + 0x14);
        assert_eq!((e.mem.u32(params + 4), e.mem.u32(params + 8)), (1, 0));
        assert_eq!((e.mem.u32(params + 16), e.mem.u32(params + 20)), (1, 1));
        TEXT.with(|slot| *slot.borrow_mut() = Some((3, vec![b'2' as u16], vec![0xe9])));
        let id = call(&mut e, 3, None) as u32;
        assert_eq!(nvse::vars::string(&e, id), Some(b"2".to_vec()));
        let id = call(&mut e, 3, Some(1)) as u32;
        assert_eq!(nvse::vars::string(&e, id), Some(vec![0xe9]));
        assert_eq!(
            call(&mut e, 4, None),
            -1234.0,
            "unobserved key does not invent a character"
        );
        TEXT.with(|slot| *slot.borrow_mut() = None);
        assert_eq!(
            call(&mut e, 3, None),
            -1234.0,
            "unavailable/dead key leaves result untouched"
        );
    }
}
