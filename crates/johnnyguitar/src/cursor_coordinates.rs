//! Selected JG59049a0 JIPFixes.cpp::CursorPosUICords GetCursorPos extension.
//! Only the getter's optional UI-coordinate argument is ported. This does not
//! advertise the complete bFixJIP family (GetJohnnyPatch8 remains false).
use engine_core::prelude::*;
use nvse::{
    args::Arg,
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
const OP: u32 = 0x2638;
const MANAGER: u32 = 0x011d_8a80;
const PIXEL_SIZE: u32 = 0x011d_8a48;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OP + 1);
    let params = table::alloc(e, 24);
    for (i, (name, kind, optional)) in [("axis", 8, 0), ("UI coordinates", 1, 1)]
        .into_iter()
        .enumerate()
    {
        let p = params + i as u32 * 12;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(p, name);
        e.mem.set_u32(p + 4, kind);
        e.mem.set_u32(p + 8, optional);
    }
    let execute = table::execute_address(OP);
    e.register(execute, command);
    table::add_plugin_record(
        e,
        OP,
        &PluginRecord {
            name: "GetCursorPos",
            short: "",
            needs_ref: false,
            param_count: 2,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
fn command(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let axis = match r.args.first() {
            Some(Arg::Byte(b)) => *b,
            _ => 0,
        };
        let manager = e.mem.u32(MANAGER);
        let mut position = e.mem.f32(
            manager
                + if matches!(axis, b'X' | b'x') {
                    0x38
                } else {
                    0x40
                },
        );
        if r.int(1, 0) != 0 {
            position *= e.mem.f32(PIXEL_SIZE);
        }
        a.set_result(e, f64::from(position));
    }
    handled(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    const DATA: u32 = 0x0300_0000;
    fn query(e: &mut Engine, axis: u8, scale: Option<u32>) -> f64 {
        let mut data = vec![
            0x38,
            0x26,
            0,
            0,
            if scale.is_some() { 2 } else { 1 },
            0,
            axis,
        ];
        if let Some(v) = scale {
            data.push(b'n');
            data.extend_from_slice(&v.to_le_bytes());
        }
        let len = (data.len() - 4) as u16;
        data[2..4].copy_from_slice(&len.to_le_bytes());
        e.mem.write(DATA, &data);
        e.mem.set_u32(DATA + 0x100, 4);
        e.mem.set_f64(DATA + 0x108, -1.0);
        let rec = table::record_of(e, OP);
        e.call(
            e.mem.u32(rec + 0x18),
            &[
                e.mem.u32(rec + 0x14),
                DATA,
                0,
                0,
                0,
                0,
                DATA + 0x108,
                DATA + 0x100,
            ],
        );
        e.mem.f64(DATA + 0x108)
    }
    #[test]
    fn optional_ui_coordinates_compile_metadata_and_real_axis_integer_decoding() {
        let mut e = Engine::new();
        e.map(DATA, 0x2000);
        e.map(0x011d_8000, 0x1000);
        e.map(0x0118_e000, 0x9000);
        table::install(&mut e);
        install(&mut e);
        e.mem.set_u32(MANAGER, DATA + 0x1000);
        e.mem.set_f32(DATA + 0x1038, 640.5);
        e.mem.set_f32(DATA + 0x1040, 360.25);
        e.mem.set_f32(PIXEL_SIZE, 0.75);
        let rec = table::record_of(&e, OP);
        let p = e.mem.u32(rec + 0x14);
        assert_eq!(e.mem.u16(rec + 0x12), 2);
        assert_eq!(
            (
                e.mem.u32(p + 4),
                e.mem.u32(p + 8),
                e.mem.u32(p + 16),
                e.mem.u32(p + 20)
            ),
            (8, 0, 1, 1)
        );
        assert_eq!(query(&mut e, b'X', None), 640.5);
        assert_eq!(query(&mut e, b'x', Some(1)), f64::from(640.5f32 * 0.75));
        assert_eq!(
            query(&mut e, b'Y', Some(u32::MAX)),
            f64::from(360.25f32 * 0.75)
        );
        assert_eq!(query(&mut e, b'Z', Some(0)), 360.25);
    }
}
