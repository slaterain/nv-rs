//! ShowOfff45d069 SO_fn_Math.cpp45–61 and easing.h (Robert Penner
//! equations credited upstream). All ten author curve families/modes, including
//! the source's specific Circ bounds; no blanket input clamp is introduced.
use engine_core::prelude::*;
use nvse::{
    commands::{arg_in_buffer, Read},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::f64::consts::PI;
const OP: u32 = 0x3d12;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OP + 1);
    let p = table::alloc(e, 36);
    for (i, (name, kind)) in [("double", 0x2c), ("string", 0), ("int", 1)]
        .into_iter()
        .enumerate()
    {
        let at = p + i as u32 * 12;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, kind);
        e.mem.set_u32(at + 8, 0);
    }
    let execute = table::execute_address(OP);
    e.register(execute, command);
    table::add_plugin_record(
        e,
        OP,
        &PluginRecord {
            name: "ApplyEasing",
            short: "Ease",
            needs_ref: false,
            param_count: 3,
            params: p,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
fn family(name: &[u8]) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    [
        b"sin".as_slice(),
        b"quad",
        b"cub",
        b"quart",
        b"quint",
        b"circ",
        b"expo",
        b"back",
        b"elastic",
        b"bounce",
    ]
    .into_iter()
    .position(|prefix| name.starts_with(prefix))
}
fn min_one(x: f64) -> f64 {
    if 1.0 < x {
        1.0
    } else {
        x
    }
} // std::min<T>(x,1), including NaN.
fn bounce_out(mut x: f64) -> f64 {
    let n = 7.5625;
    let d = 2.75;
    if x < 1.0 / d {
        return n * x * x;
    }
    if x < 2.0 / d {
        x -= 1.5 / d;
        return n * x * x + 0.75;
    }
    if x < 2.5 / d {
        x -= 2.25 / d;
        return n * x * x + 0.9375;
    }
    x -= 2.625 / d;
    n * x * x + 0.984375
}
fn ease(mut x: f64, family: usize, mode: u32) -> f64 {
    match (family, mode) {
        (0, 0) => 1.0 - ((x * PI) / 2.0).cos(),
        (0, 1) => ((x * PI) / 2.0).sin(),
        (0, _) => -((PI * x).cos() - 1.0) / 2.0,
        (1, 0) => x * x,
        (1, 1) => 1.0 - (1.0 - x) * (1.0 - x),
        (1, _) => {
            if x < 0.5 {
                2.0 * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powf(2.0) / 2.0
            }
        }
        (2, 0) => x * x * x,
        (2, 1) => 1.0 - (1.0 - x).powf(3.0),
        (2, _) => {
            if x < 0.5 {
                4.0 * x * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powf(3.0) / 2.0
            }
        }
        (3, 0) => x * x * x * x,
        (3, 1) => 1.0 - (1.0 - x).powf(4.0),
        (3, _) => {
            if x < 0.5 {
                8.0 * x * x * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powf(4.0) / 2.0
            }
        }
        (4, 0) => x * x * x * x * x,
        (4, 1) => 1.0 - (1.0 - x).powf(5.0),
        (4, _) => {
            if x < 0.5 {
                16.0 * x * x * x * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powf(5.0) / 2.0
            }
        }
        (5, 0) => {
            x = min_one(x.powf(2.0));
            1.0 - (1.0 - x).sqrt()
        }
        (5, 1) => {
            x -= 1.0;
            x = min_one(x.powf(2.0));
            (1.0 - x).sqrt()
        }
        (5, _) => {
            if x < 0.5 {
                if x < 0.0 {
                    x = 0.0;
                }
                (1.0 - (1.0 - (2.0 * x).powf(2.0)).sqrt()) / 2.0
            } else {
                x = -2.0 * x + 2.0;
                x = min_one(x.powf(2.0));
                ((1.0 - x).sqrt() + 1.0) / 2.0
            }
        }
        (6, 0) => {
            if x == 0.0 {
                0.0
            } else {
                2.0f64.powf(10.0 * x - 10.0)
            }
        }
        (6, 1) => {
            if x == 1.0 {
                1.0
            } else {
                1.0 - 2.0f64.powf(-10.0 * x)
            }
        }
        (6, _) => {
            if x == 0.0 {
                0.0
            } else if x == 1.0 {
                1.0
            } else if x < 0.5 {
                2.0f64.powf(20.0 * x - 10.0) / 2.0
            } else {
                (2.0 - 2.0f64.powf(-20.0 * x + 10.0)) / 2.0
            }
        }
        (7, mode) => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            let c2 = c1 * 1.525;
            match mode {
                0 => c3 * x * x * x - c1 * x * x,
                1 => 1.0 + c3 * (x - 1.0).powf(3.0) + c1 * (x - 1.0).powf(2.0),
                _ => {
                    if x < 0.5 {
                        ((2.0 * x).powf(2.0) * ((c2 + 1.0) * 2.0 * x - c2)) / 2.0
                    } else {
                        ((2.0 * x - 2.0).powf(2.0) * ((c2 + 1.0) * (x * 2.0 - 2.0) + c2) + 2.0)
                            / 2.0
                    }
                }
            }
        }
        (8, mode) => {
            let c4 = (2.0 * PI) / 3.0;
            let c5 = (2.0 * PI) / 4.5;
            if x == 0.0 {
                0.0
            } else if x == 1.0 {
                1.0
            } else {
                match mode {
                    0 => -2.0f64.powf(10.0 * x - 10.0) * ((x * 10.0 - 10.75) * c4).sin(),
                    1 => 2.0f64.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0,
                    _ => {
                        if x < 0.5 {
                            -(2.0f64.powf(20.0 * x - 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0
                        } else {
                            (2.0f64.powf(-20.0 * x + 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0
                                + 1.0
                        }
                    }
                }
            }
        }
        (9, 0) => 1.0 - bounce_out(1.0 - x),
        (9, 1) => bounce_out(x),
        (9, _) => {
            if x < 0.5 {
                (1.0 - bounce_out(1.0 - 2.0 * x)) / 2.0
            } else {
                (1.0 + bounce_out(2.0 * x - 1.0)) / 2.0
            }
        }
        _ => unreachable!(),
    }
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    if r.ok {
        let mode = r.int(2, u32::MAX);
        if mode <= 2 {
            let name = arg_in_buffer(e, &r, 1, 64, "ApplyEasing");
            if let Some(family) = family(&name) {
                if let Some(input) = r.args.first().and_then(|v| v.double()) {
                    a.set_result(e, ease(input, family, mode));
                }
            }
        }
    }
    handled(true)
}
#[cfg(test)]
mod tests;
