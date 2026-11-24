//! ShowOfff45d069 SO_fn_Gameplay.cpp349–378,430–476;
//! MenuHooks.cpp76–158 and decoding.h1576–1659. Native perk browser support.
use engine_core::{
    extend::{Owner, SiteSlot},
    prelude::*,
};
use nvse::{
    commands::{arg_in_buffer, Read},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::{
    cell::Cell,
    sync::atomic::{AtomicBool, Ordering},
};
const MENU: u32 = 0x011d_9fdc;
const PLAYER: u32 = 0x011d_ea3c;
#[derive(Default)]
struct BrowserHooks(AtomicBool);
fn hooks_on(e: &Engine) -> bool {
    e.ext_get::<BrowserHooks>()
        .is_some_and(|s| s.0.load(Ordering::Acquire))
}
thread_local! {static FORCE_LIST:Cell<bool>=const{Cell::new(false)};}
fn with_forced_list<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            FORCE_LIST.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(FORCE_LIST.with(|v| v.replace(true)));
    f()
}
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<BrowserHooks>();
    table::grow_to(e, 0x3cec);
    for (op, name, short, specs, execute) in [
        (
            0x3ce6,
            "GetLevelUpMenuPoints",
            "",
            vec![("int", 1), ("int", 1)],
            points as AbiFn,
        ),
        (
            0x3ceb,
            "ShowPerkMenu",
            "IfIDecideToGoWithYourFunctionWhatAreThePerks",
            vec![("int", 1), ("string", 0)],
            show as AbiFn,
        ),
    ] {
        let params = table::alloc(e, 24);
        for (i, (name, kind)) in specs.into_iter().enumerate() {
            let at = params + i as u32 * 12;
            let name = table::guest_str(e, name.as_bytes());
            e.mem.set_u32(at, name);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, 1);
        }
        let address = table::execute_address(op);
        e.register(address, execute);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: 2,
                params,
                execute: address,
                eval: 0,
                flags: 0,
                expression: false,
                returns: Returns::Default,
            },
        );
    }
    e.on_site(SiteSlot::new(Owner::ShowOff, 0x0078_4f26), |_, _| {
        FORCE_LIST.with(Cell::get).then_some(1)
    });
    e.on_site(SiteSlot::new(Owner::ShowOff, 0x0078_5087), |e, _| {
        if !hooks_on(e) {
            return None;
        }
        let player = e.mem.u32(PLAYER);
        let level = u32::from(e.call(0x0087_f9f0, &args![player]).u16());
        Some(u32::from(level % e.mem.u32(0x011c_d078) == 0))
    });
    e.on_site(SiteSlot::new(Owner::ShowOff, 0x0078_5acc), |e, a| {
        if !hooks_on(e) {
            return None;
        }
        Some(u32::from(
            e.mem.u32(a[0] + 0x60) != 0 || e.mem.u32(a[0] + 0x58) != 0,
        ))
    });
    e.on_site(SiteSlot::new(Owner::ShowOff, 0x0078_5b0f), |e, _| {
        if !hooks_on(e) {
            return None;
        }
        let menu = e.mem.u32(MENU);
        if e.mem.u32(menu + 0x60) != 1 {
            Some(u32::from(e.mem.u32(menu + 0x58) < e.mem.u32(menu + 0x60)))
        } else {
            e.call(0x0078_7850, &args![menu + 0x94, 1u32]);
            Some(1)
        }
    });
    e.on_site(SiteSlot::new(Owner::ShowOff, 0x0078_5b2c), |e, _| {
        hooks_on(e).then_some(1)
    });
}
fn points(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let menu = e.mem.u32(MENU);
    if menu != 0 {
        let perks = r.int(0, 0) != 0;
        let assigned = r.int(1, 0) != 0;
        let value = if perks && !assigned && e.mem.u32(menu + 0xc4) == 0 {
            0
        } else {
            e.mem
                .u32(menu + if assigned { 0x54 } else { 0x5c } + u32::from(perks) * 4)
        };
        a.set_result(e, f64::from(value));
    }
    handled(true)
}
fn list_count(e: &Engine, head: u32) -> u32 {
    if e.mem.u32(head) == 0 {
        return 0;
    }
    let mut count = 1;
    let mut node = e.mem.u32(head + 4);
    while node != 0 {
        count += 1;
        node = e.mem.u32(node + 4);
    }
    count
}
fn show(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let count = r.int(0, u32::MAX) as i32;
    let title = arg_in_buffer(e, &r, 1, 0x1000, "ShowPerkMenu");
    // Source installs these safety hooks at delayed plugin init. This partial
    // port activates that family on explicit browser use; query-only installs
    // keep vanilla UI behavior. State is initialized before real threads.
    e.ext_get::<BrowserHooks>()
        .unwrap()
        .0
        .store(true, Ordering::Release);
    with_forced_list(|| {
        // Source temporarily patches784F26 for the entire command. The Rust
        // scope changes only this thread's exact native branch, with unwind
        // and reentrant restoration; no executable/guest code is modified.
        e.call(0x0070_6270, &[]);
        let menu = e.mem.u32(MENU);
        if menu == 0 {
            return;
        }
        if e.mem.u32(menu + 0xc4) == 0 {
            let root = e.mem.u32(menu + 4);
            e.call(0x00a0_12d0, &args![root, 0xfa3u32, 1.0f32, 1u32]);
            e.call(0x0078_5830, &args![menu, 2u32]);
            return;
        }
        if count > -1 {
            let mut selectable = 0u32;
            let mut node = menu + 0xc4;
            while node != 0 {
                let perk = e.mem.u32(node);
                if perk != 0 {
                    let player = e.mem.u32(PLAYER);
                    if e.call(0x0078_5150, &args![perk, player]).bool()
                        && e.call(0x0087_f9f0, &args![player]).u16()
                            >= u16::from(e.mem.u8(perk + 0x39))
                    {
                        selectable += 1;
                    }
                }
                node = e.mem.u32(node + 4);
            }
            let count = (count as u32)
                .min(selectable)
                .min(list_count(e, menu + 0xc4));
            e.mem.set_u32(menu + 0x60, count);
        }
        e.call(0x0078_5830, &args![menu, 1u32]);
        let back = e.mem.u32(menu + 0x4c);
        e.call(0x00a0_12d0, &args![back, 0xfa3u32, 0.0f32, 1u32]);
        e.mem.set_u32(menu + 0x5c, 0);
        if !title.is_empty() {
            e.with_stack(title.len() as u32 + 1, |e, text| {
                e.mem.write(text.addr(), &title);
                e.mem.set_u8(text.addr() + title.len() as u32, 0);
                let tile = e.mem.u32(menu + 0x2c);
                e.call(0x00a0_1350, &args![tile, 0xfc4u32, text, 1u32]);
            });
        }
        a.set_result(e, 1.0);
    });
    handled(true)
}
#[cfg(test)]
mod tests;
