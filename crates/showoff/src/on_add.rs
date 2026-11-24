//! ShowOff f45d069, Events/ShowOffEvents.h:1455-1639 (`OnAddAlt`).
//! The five original call sites preserve their order relative to vanilla action
//! marking and inventory mutation. The world reference and inventory info are
//! independent flat contexts, with the original same-kind nesting limitations.
//! Each engine keeps its own
//! transient context; no borrowed extender slot lives across script dispatch.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::{events, handled, invref, table, CommandArgs};

const EVENT: &[u8] = b"ShowOff:OnAdd";
const OPCODE: u32 = 0x3d3b;
const ADD_ITEM: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_5091);
const EQUIP_FOR_REF: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_4b19);
const PICKUP: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_4c28);
const ROCK_IT: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_4d00);
const PROJECTILE: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_4f03);

#[derive(Clone, Copy, Default)]
struct Context {
    world_ref: u32,
    owner: u32,
    item: u32,
    extra: u32,
    count: i32,
    cached: u32,
}

pub(crate) fn install(e: &mut Engine) {
    events::register_native(
        e,
        "ShowOff:OnAdd",
        &[events::param::BASE_FORM, events::param::REFERENCE],
        0,
    );
    table::grow_to(e, OPCODE + 1);
    let execute = table::execute_address(OPCODE);
    e.register(execute, get_added_item_ref);
    table::add_plugin_record(
        e,
        OPCODE,
        &table::PluginRecord {
            name: "GetAddedItemRefShowOff",
            short: "GetAddedItemRefSO",
            needs_ref: false,
            param_count: 0,
            params: 0,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: table::Returns::Form,
        },
    );
    e.on_site(ADD_ITEM, inventory_added);
    e.on_site(EQUIP_FOR_REF, inventory_added);
    e.on_site(PICKUP, world_added);
    e.on_site(ROCK_IT, inventory_added);
    e.on_site(PROJECTILE, inventory_added);
}

/// The source's lazy InvRefCreatingInfo::GetInvRef. Its temporary entry and
/// reference remain owned by xNVSE until normal main-loop/load cleanup.
fn item_ref(e: &mut Engine) -> u32 {
    let context = *e.ext::<Context>();
    if context.world_ref != 0 {
        return context.world_ref;
    }
    if context.cached != 0 {
        return context.cached;
    }
    if context.item == 0 || context.owner == 0 {
        return 0;
    }
    let reference =
        invref::create_entry(e, context.owner, context.item, context.count, context.extra);
    e.ext::<Context>().cached = reference;
    reference
}

fn get_added_item_ref(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let reference = item_ref(e);
    if reference != 0 {
        e.mem.set_u32(a.result, e.mem.u32(reference + 0xc));
    }
    handled(true)
}

/// The two source overloads reset only their own global: world entry leaves
/// inventory info alone; inventory Fill leaves the world ref and cached invref
/// alone, and its Reset clears inventory info. No nested context is saved or
/// restored. A caught port stop must also take this reset path.
fn with_context<R>(e: &mut Engine, context: Context, run: impl FnOnce(&mut Engine) -> R) -> R {
    let world = context.world_ref != 0;
    if world {
        e.ext::<Context>().world_ref = context.world_ref;
    } else {
        let current = e.ext::<Context>();
        current.owner = context.owner;
        current.item = context.item;
        current.extra = context.extra;
        current.count = context.count;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(e)));
    let current = e.ext::<Context>();
    if world {
        current.world_ref = 0;
    } else {
        *current = Context {
            world_ref: current.world_ref,
            ..Context::default()
        };
    }
    match result {
        Ok(value) => value,
        Err(stop) => std::panic::resume_unwind(stop),
    }
}

/// Payload owner/item/extra/count, immediately before the two inventory calls
/// or after the projectile branches' vanilla SetActionFlag calls. Count bits
/// preserve the original signed countDelta (RockIt) and literal1 (projectile).
fn inventory_added(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (owner, item, extra, count) = (words[0], words[1], words[2], words[3] as i32);
    with_context(
        e,
        Context {
            owner,
            item,
            extra,
            count,
            ..Context::default()
        },
        |e| events::dispatch(e, EVENT, 0, &[item, owner]),
    );
    None
}

/// Payload owner/world-reference, just after vanilla SetActionFlag at574c28.
fn world_added(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (owner, reference) = (words[0], words[1]);
    let item = e.mem.u32(reference + 0x20);
    with_context(
        e,
        Context {
            world_ref: reference,
            ..Context::default()
        },
        |e| events::dispatch(e, EVENT, reference, &[item, owner]),
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATA: u32 = 0x0300_0000;
    const OWNER: u32 = DATA + 0x1000;
    const ITEM: u32 = DATA + 0x1100;
    const WORLD: u32 = DATA + 0x1200;
    const EXTRA: u32 = DATA + 0x1300;
    const SCRIPT: u32 = DATA + 0x2000;
    const SEEN: u32 = DATA + 0x3000;
    const RESULT: u32 = DATA + 0x3100;
    const REF_VT: u32 = DATA + 0x4000;
    const HEAP_TOP: u32 = DATA + 0x4ff0;
    const NEXT_ID: u32 = DATA + 0x4ff4;
    const MODE: u32 = DATA + 0x4ff8;

    fn alloc(e: &mut Engine, size: u32) -> u32 {
        let at = e.mem.u32(HEAP_TOP).max(DATA + 0x5000);
        e.mem.set_u32(HEAP_TOP, at + size.next_multiple_of(0x10));
        e.mem.write(at, &vec![0; size as usize]);
        at
    }

    fn setup() -> Engine {
        let mut e = Engine::new();
        e.map(DATA, 0x20000);
        e.map(0x0118_e000, 0x9000);
        for page in [0x011c_3000, 0x011f_6000, 0x011d_a000] {
            e.map(page, 0x1000);
        }
        nvse::table::install(&mut e);
        nvse::functions::install(&mut e);
        install(&mut e);
        e.register(0x0040_1000, |e, a| Ret {
            eax: alloc(e, a[0]),
            ..Ret::default()
        });
        e.register(0x0055_a2f0, |e, a| {
            let id = e.mem.u32(NEXT_ID).max(0xff00_1000);
            e.mem.set_u32(NEXT_ID, id + 1);
            e.mem.set_u32(a[0], REF_VT);
            e.mem.set_u32(a[0] + 0xc, id);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        e.mem.set_u32(REF_VT + 0x10, DATA + 0x4100);
        e.register(DATA + 0x4100, |e, _| {
            e.mem.set_u32(SEEN + 32, e.mem.u32(SEEN + 32) + 1);
            Ret::default()
        });
        e.register(0x0040_1030, |_, _| Ret::default());
        e.register(0x0041_0220, |_, _| Ret::default());
        // Real typed event filter validation asks TESForm::IsReference at +f0.
        for form in [ITEM, OWNER, WORLD] {
            e.mem.set_u32(form, REF_VT);
        }
        e.mem.set_u32(REF_VT + 0xf0, DATA + 0x4fe0);
        e.register(DATA + 0x4fe0, |_, a| u32::from(a[0] != ITEM).into_ret());
        e.mem.set_u32(ITEM + 0xc, 0x1001);
        e.mem.set_u32(OWNER + 0xc, 0x1002);
        e.mem.set_u32(WORLD + 0xc, 0x1003);
        e.mem.set_u32(WORLD + 0x20, ITEM);
        e.register(0x0048_39c0, |e, a| Ret {
            eax: [OWNER, ITEM, WORLD, SCRIPT]
                .into_iter()
                .find(|&form| e.mem.u32(form + 0xc) == a[0])
                .unwrap_or(0),
            ..Ret::default()
        });
        e
    }

    fn helper(e: &mut Engine) -> u32 {
        let rec = table::record_of(e, OPCODE);
        let words = [0, 0, 0, 0, 0, 0, RESULT, 0];
        e.mem.set_f64(RESULT, -1234.0);
        assert_eq!(e.call(e.mem.u32(rec + 0x18), &words).eax & 0xff, 1);
        assert_eq!(e.mem.u32(RESULT + 4), 0);
        e.mem.u32(RESULT)
    }

    #[test]
    fn helper_registration_is_a_form_and_context_is_lazy_and_cached_until_normal_cleanup() {
        let mut e = setup();
        assert_eq!(
            table::opcode_by_name(&e, b"GetAddedItemRefSO"),
            Some(OPCODE)
        );
        let rec = table::record_of(&e, OPCODE);
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), 0);
        assert_eq!(table::return_type(&e, OPCODE), table::Returns::Form);
        assert_eq!(helper(&mut e), 0);
        assert_eq!(e.mem.u32(HEAP_TOP), 0);
        let id = with_context(
            &mut e,
            Context {
                owner: OWNER,
                item: ITEM,
                extra: EXTRA,
                count: -7,
                ..Context::default()
            },
            |e| {
                assert_eq!(e.mem.u32(HEAP_TOP), 0, "no eager reference allocation");
                let id = helper(e);
                assert_eq!(helper(e), id);
                let reference = invref::get(e, id).unwrap();
                assert_eq!(reference.container, OWNER);
                assert_eq!(reference.data.item, ITEM);
                assert_eq!(reference.data.xdata, EXTRA);
                let entry = reference.data.entry;
                assert_eq!(e.mem.u32(entry + invref::ENTRY_COUNT_DELTA) as i32, -7);
                id
            },
        );
        assert_eq!(helper(&mut e), 0);
        assert!(
            invref::get(&mut e, id).is_some(),
            "context reset does not destroy inventory reference"
        );
        // Avoid deferred stack-writeback in this allocator/constructor fixture;
        // clear still exercises actual temp entry/reference ownership cleanup.
        invref::set_removed(&mut e, id);
        invref::clear(&mut e);
        assert!(invref::get(&mut e, id).is_none());
        assert_eq!(e.mem.u32(SEEN + 32), 1);
    }

    #[test]
    fn world_context_returns_actual_reference_without_allocating_and_flat_reset_matches_source() {
        let mut e = setup();
        with_context(
            &mut e,
            Context {
                world_ref: WORLD,
                ..Context::default()
            },
            |e| {
                assert_eq!(helper(e), 0x1003);
                assert_eq!(e.mem.u32(HEAP_TOP), 0);
                with_context(e, Context::default(), |e| assert_eq!(helper(e), 0x1003));
                assert_eq!(
                    helper(e),
                    0x1003,
                    "inventory Reset leaves world context alone"
                );
                with_context(
                    e,
                    Context {
                        world_ref: WORLD,
                        ..Context::default()
                    },
                    |e| {
                        assert_eq!(helper(e), 0x1003);
                    },
                );
                assert_eq!(
                    helper(e),
                    0,
                    "nested world event clears outer world context"
                );
            },
        );
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_context(
                &mut e,
                Context {
                    world_ref: WORLD,
                    ..Context::default()
                },
                |_| panic!("stopped handler"),
            );
        }));
        assert!(stopped.is_err());
        assert_eq!(helper(&mut e), 0);
    }

    #[test]
    fn inventory_fill_preserves_cached_ref_then_nested_reset_clears_outer_inventory_info() {
        let mut e = setup();
        with_context(
            &mut e,
            Context {
                owner: OWNER,
                item: ITEM,
                count: 7,
                ..Context::default()
            },
            |e| {
                let outer = helper(e);
                with_context(
                    e,
                    Context {
                        owner: OWNER,
                        item: ITEM,
                        count: 99,
                        ..Context::default()
                    },
                    |e| assert_eq!(helper(e), outer, "Fill does not clear the cached invref"),
                );
                assert_eq!(
                    helper(e),
                    0,
                    "Reset clears outer inventory info, without restoration"
                );
            },
        );
    }

    /// Native event -> real xNVSE parameter binding -> Script::Execute double.
    fn add_listener(e: &mut Engine) {
        let data = SCRIPT + 0x100;
        let mut code = vec![0x1d, 0, 0, 0, 0x10, 0];
        let block = [0x0d, 0, 0, 0, 0, 0, 1, 2, 1, 0, 4, 2, 0, 4];
        code.extend_from_slice(&(block.len() as u16).to_le_bytes());
        code.extend(block);
        code.resize(0x40, 0);
        e.mem.write(data, &code);
        e.mem.set_u8(SCRIPT + 4, 0x11);
        e.mem.set_u32(SCRIPT + 0xc, 0x1004);
        e.mem.set_u32(SCRIPT + 0x30, data);
        e.mem.set_u32(SCRIPT + 0x20, code.len() as u32);
        e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
            let list = alloc(e, 0x14);
            e.mem.set_u32(list, a[0]);
            let mut next = 0;
            for id in [2, 1] {
                let local = alloc(e, 0x10);
                e.mem.set_u32(local, id);
                let node = alloc(e, 8);
                e.mem.set_u32(node, local);
                e.mem.set_u32(node + 4, next);
                next = node;
            }
            e.mem.set_u32(list + 0xc, next);
            Ret {
                eax: list,
                ..Ret::default()
            }
        });
        e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
        e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
            let one = nvse::args::event_list_variable(e, a[2], 1);
            let two = nvse::args::event_list_variable(e, a[2], 2);
            e.mem.set_u32(SEEN, e.mem.u32(SEEN) + 1);
            e.mem.set_u32(SEEN + 4, e.mem.u32(one + 8));
            e.mem.set_u32(SEEN + 8, e.mem.u32(two + 8));
            e.mem.set_u32(SEEN + 12, a[1]);
            if e.mem.u32(MODE) != 0 {
                let id = helper(e);
                assert_eq!(helper(e), id);
                e.mem.set_u32(SEEN + 16, id);
            }
            Ret::default()
        });
        let mut callback = events::Callback::new(SCRIPT);
        callback.filters.insert(1, nvse::vars::Data::Form(0x1001));
        callback.filters.insert(2, nvse::vars::Data::Form(0x1002));
        assert!(events::set_handler(e, EVENT, callback, 1, None));
    }

    #[test]
    fn all_five_sites_dispatch_two_typed_arguments_and_preserve_world_calling_reference() {
        let mut e = setup();
        add_listener(&mut e);
        e.mem.set_u32(MODE, 1);
        for (slot, count) in [
            (ADD_ITEM, 7),
            (EQUIP_FOR_REF, 3),
            (ROCK_IT, 2),
            (PROJECTILE, 1),
        ] {
            assert_eq!(e.site(slot.addr(), &[OWNER, ITEM, EXTRA, count]), None);
            assert_eq!(e.mem.u32(SEEN + 4), 0x1001);
            assert_eq!(e.mem.u32(SEEN + 8), 0x1002);
            assert_eq!(e.mem.u32(SEEN + 12), 0);
            let id = e.mem.u32(SEEN + 16);
            let entry = invref::get(&mut e, id).unwrap().data.entry;
            assert_eq!(e.mem.u32(entry + invref::ENTRY_COUNT_DELTA), count);
            assert_eq!(helper(&mut e), 0);
        }
        assert_eq!(e.site(PICKUP.addr(), &[OWNER, WORLD]), None);
        assert_eq!(e.mem.u32(SEEN + 12), WORLD);
        assert_eq!(e.mem.u32(SEEN + 16), 0x1003);
        assert_eq!(e.mem.u32(SEEN), 5);
        assert_eq!(helper(&mut e), 0);
        assert_eq!(e.site(ADD_ITEM.addr(), &[WORLD, ITEM, EXTRA, 1]), None);
        assert_eq!(
            e.mem.u32(SEEN),
            5,
            "different owner fails the native event filter"
        );
        assert_eq!(helper(&mut e), 0);
    }
}
