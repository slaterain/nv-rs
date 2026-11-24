//! kNVSE24 game_types.cpp::GetActorRealAnimGroup and the actor/form
//! layouts consulted by commands_animation.cpp::PickAnimation.
use engine_core::prelude::*;
use nvse::commands::{dynamic_cast, rtti};
pub(crate) const PLAYER: u32 = 0x011d_ea3c;
pub(crate) fn weapon_info(e: &mut Engine, actor: u32) -> u32 {
    e.vcall(e.mem.u32(actor + 0x68), 0x148, &[]).u32()
}
pub(crate) fn weapon(e: &mut Engine, actor: u32) -> u32 {
    let info = weapon_info(e, actor);
    if info == 0 {
        0
    } else {
        e.mem.u32(info + 8)
    }
}
pub(crate) fn third(e: &mut Engine, actor: u32) -> u32 {
    e.vcall(e.mem.u32(actor + 0x68), 0x1b8, &[]).u32()
}
pub(crate) fn first(e: &Engine, animation: u32) -> bool {
    let p = e.mem.u32(PLAYER);
    p != 0 && animation == e.mem.u32(p + 0x690)
}
pub(crate) fn slot(e: &Engine, group: u16) -> u32 {
    e.mem.u32(0x0119_77d8 + u32::from(group & 0xff) * 0x24 + 8)
}
pub(crate) fn real_group(e: &mut Engine, actor: u32, group: u8) -> u16 {
    let weapon = weapon(e, actor);
    let process = e.mem.u32(actor + 0x68);
    let hand = if weapon != 0 {
        e.mem
            .u32(0x0118_a838 + u32::from(e.mem.u8(weapon + 0xf4)) * 4) as u8
    } else if e.mem.u8(process + 0x134) != 0 || e.mem.u8(process + 0x135) != 0 {
        1
    } else {
        0
    };
    let mover = e.mem.u32(actor + 0x190);
    let flags = e.vcall(mover, 0x20, &[]).u32();
    let movement = if flags & 0x800 != 0 {
        2
    } else if flags & 0x2000 != 0 {
        3
    } else if flags & 0x400 != 0 {
        1
    } else {
        0
    };
    let power =
        e.call(0x008b_a3e0, &args![actor]).bool() || e.call(0x008b_a410, &args![actor]).bool();
    ((movement << 12) + (u32::from(power) << 15) + (u32::from(hand) << 8) + u32::from(group)) as u16
}
pub(crate) fn npc(e: &mut Engine, actor: u32) -> u32 {
    dynamic_cast(e, e.mem.u32(actor + 0x20), rtti::TES_FORM, rtti::TES_NPC)
}
pub(crate) fn actor_base(e: &mut Engine, actor: u32) -> u32 {
    let base = e.mem.u32(actor + 0x20);
    if e.mem.u8(base + 0xf) == 0xff {
        let extra = e.call(0x0041_0220, &args![actor + 0x44, 0x2eu32]).u32();
        if extra != 0 {
            return e.mem.u32(extra + 0x10);
        }
    }
    base
}
/// Source custom order: None,Human,gender; Hurt can replace all three;
/// attached weapon-mod folders are then appended and checked in reverse.
pub(crate) fn custom_order(
    e: &mut Engine,
    animation: u32,
    actor: u32,
    group: u16,
    mod_stacks: [bool; 3],
) -> Vec<usize> {
    let mut order = vec![0];
    let npc = npc(e, actor);
    if actor == e.mem.u32(PLAYER) || npc != 0 {
        order.push(7);
    }
    if npc != 0 {
        order.push(if e.mem.u32(npc + 0x34) & 1 == 0 { 1 } else { 2 });
    }
    let left = e.vcall(actor + 0xa4, 8, &args![0x1du32]).i32();
    if left <= 0 || (e.vcall(actor + 0xa4, 8, &args![0x1eu32]).i32() <= 0 && slot(e, group) == 1) {
        let map = engine_core::animation_maps::selected(animation, e.mem.u32(animation + 0xdc));
        let sequence = e.with_stack(4, |e, out| {
            e.mem.set_u32(out.addr(), 0);
            if !e.call(0x0049_c390, &args![map, group, out]).bool() {
                return 0;
            }
            let base = e.mem.u32(out.addr());
            e.vcall(base, 0x10, &args![u32::MAX]).u32()
        });
        if sequence != 0 {
            let path = nvse::table::guest_text(e, e.mem.u32(sequence + 8));
            if path.windows(6).any(|w| w.eq_ignore_ascii_case(b"\\hurt\\")) {
                order.clear();
                order.push(6);
            }
        }
    }
    let info = weapon_info(e, actor);
    if info != 0 {
        let lists = e.mem.u32(info);
        if lists != 0 {
            let list = e.mem.u32(lists);
            if list != 0 {
                let extra = e.call(0x0041_0220, &args![list, 0x8du32]).u32();
                if extra != 0 {
                    let flags = e.mem.u8(extra + 0xc);
                    for (i, &present) in mod_stacks.iter().enumerate() {
                        if present && flags & (1 << i) != 0 {
                            order.push(3 + i);
                        }
                    }
                }
            }
        }
    }
    order
}
