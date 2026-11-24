use super::*;
const DATA: u32 = 0x0300_0000;
const CONTROL: u32 = DATA + 0x5000;
const KF: u32 = CONTROL + 0x100;
const SOURCE: u32 = CONTROL + 0x200;
const GROUP: u32 = CONTROL + 0x300;
const BASE_VT: u32 = CONTROL + 0x400;
const MAP_VT: u32 = CONTROL + 0x500;
const USES: u32 = CONTROL + 4;
const LOADS: u32 = CONTROL + 8;
const BASES_FREED: u32 = CONTROL + 12;
const FAIL_LOAD: u32 = CONTROL + 16;
const MAPS_FREED: u32 = CONTROL + 20;
const NATIVE: u32 = DATA + 0x7000;

fn node(e: &Engine, map: u32, key: u16) -> u32 {
    let mut n = e
        .mem
        .u32(e.mem.u32(map + 8) + (u32::from(key) % e.mem.u32(map + 4)) * 4);
    while n != 0 {
        if e.mem.u32(n + 4) == u32::from(key) {
            return n;
        }
        n = e.mem.u32(n);
    }
    0
}
pub(crate) fn fixture() -> Engine {
    let mut e = super::super::tests::engine();
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(MODEL_LOADER, CONTROL);
    e.mem.set_u32(KF + 4, SOURCE);
    e.mem.set_u32(KF + 8, GROUP);
    let name = nvse::table::guest_str(&mut e, b"EM_NoBob.kf");
    e.mem.set_u32(SOURCE + 8, name);
    e.mem.set_u16(GROUP + 0x10, 2);
    e.register(LOAD_KF, |e, w| {
        assert_eq!(w[0], CONTROL);
        e.mem.set_u32(USES, e.mem.u32(USES) + 1);
        Ret {
            eax: KF,
            ..Ret::default()
        }
    });
    e.register(0x0040_1000, |e, w| Ret {
        eax: e.mem.alloc(w[0]),
        ..Ret::default()
    });
    e.register(0x0049_c050, |e, w| {
        assert_eq!(w[1], 0x65);
        let buckets = e.mem.alloc(w[1] * 4);
        e.mem.set_u32(w[0], MAP_VT);
        e.mem.set_u32(w[0] + 4, w[1]);
        e.mem.set_u32(w[0] + 8, buckets);
        Ret {
            eax: w[0],
            ..Ret::default()
        }
    });
    e.register(MAP_GET, |e, w| {
        let n = node(e, w[0], w[1] as u16);
        if n == 0 {
            return Ret::default();
        }
        e.mem.set_u32(w[2], e.mem.u32(n + 8));
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.register(0x0049_c250, |e, w| {
        let bucket = e.mem.u32(w[0] + 8) + (w[1] % e.mem.u32(w[0] + 4)) * 4;
        let n = e.mem.u32(bucket);
        assert_eq!(e.mem.u32(n + 4), w[1]);
        e.mem.set_u32(bucket, e.mem.u32(n));
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.register(0x0049_0500, |e, w| {
        assert_eq!(w[1], KF);
        assert_eq!(w[2], 0);
        let map = animation_maps::selected(w[0], e.mem.u32(w[0] + 0xdc));
        assert_ne!(map, e.mem.u32(w[0] + 0xdc));
        e.mem.set_u32(LOADS, e.mem.u32(LOADS) + 1);
        if e.mem.u32(FAIL_LOAD) != 0 {
            return Ret::default();
        }
        let sequence = e.mem.alloc(0x78);
        let base = e.mem.alloc(8);
        let entry = e.mem.alloc(12);
        let id = e.mem.u16(GROUP + 0x10);
        e.mem.set_u32(sequence + 8, e.mem.u32(SOURCE + 8));
        e.mem.set_u32(sequence + 0x74, GROUP);
        e.mem.set_u32(sequence + 0x40, e.mem.u32(w[0] + 0xd8));
        e.mem.set_f32(sequence + 0x1c, 0.75);
        e.mem.set_u32(base, BASE_VT);
        e.mem.set_u32(base + 4, sequence);
        let bucket = e.mem.u32(map + 8) + (u32::from(id) % e.mem.u32(map + 4)) * 4;
        e.mem.set_u32(entry, e.mem.u32(bucket));
        e.mem.set_u32(entry + 4, u32::from(id));
        e.mem.set_u32(entry + 8, base);
        e.mem.set_u32(bucket, entry);
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.mem.set_u32(BASE_VT + 0x10, NATIVE);
    e.register(NATIVE, |e, w| {
        assert_eq!(w[1], u32::MAX);
        Ret {
            eax: e.mem.u32(w[0] + 4),
            ..Ret::default()
        }
    });
    e.mem.set_u32(BASE_VT, NATIVE + 4);
    e.register(NATIVE + 4, |e, w| {
        assert_eq!(w[1], 1);
        e.mem.set_u32(BASES_FREED, e.mem.u32(BASES_FREED) + 1);
        e.mem.set_u32(USES, e.mem.u32(USES) - 1);
        let seq = e.mem.u32(w[0] + 4);
        e.site(0x004e_eb4b, &[seq]);
        Ret::default()
    });
    e.register(0x0049_c080, |e, w| {
        assert_eq!(w[1], 1);
        e.mem.set_u32(MAPS_FREED, e.mem.u32(MAPS_FREED) + 1);
        Ret::default()
    });
    e.mem.set_u32(MAP_VT + 4, NATIVE + 8);
    e.register(NATIVE + 8, |_, _| Ret::default());
    e.mem.set_u32(MAP_VT + 8, NATIVE + 12);
    e.register(NATIVE + 12, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e
}
pub(crate) fn animation(e: &mut Engine) -> (u32, u32) {
    let a = e.mem.alloc(0x140);
    let m = e.mem.alloc(0x80);
    let buckets = e.mem.alloc(4);
    e.mem.set_u32(m + 0x50, MAP_VT);
    e.mem.set_u32(m + 0x58, buckets);
    e.mem.set_u32(a + 0xd8, m);
    e.mem.set_u32(a + 0xdc, 0x76543210);
    (a, m)
}

#[test]
fn twenty_four_load_uses_original_add_animation_private_map_and_raw_cache() {
    let mut e = fixture();
    let (a, _) = animation(&mut e);
    let l = load(&mut e, a, b"EM_NoBob.kf");
    assert_ne!(l.sequence, 0);
    assert_eq!(e.mem.u32(LOADS), 1);
    assert_eq!(e.mem.u32(a + 0xdc), 0x76543210);
    assert_eq!(animation_maps::selected(a, 9), 9);
    assert_eq!(load(&mut e, a, b"EM_NoBob.kf"), l);
    assert_eq!(e.mem.u32(USES), 1);
    assert_eq!(e.mem.u32(LOADS), 1);
    e.site(0x0048_fb82, &[a, 0]);
    assert_eq!(e.mem.u32(BASES_FREED), 1);
    assert_eq!(e.mem.u32(MAPS_FREED), 1);
    assert!(cached(&e, a, b"EM_NoBob.kf").is_none());
    release(&mut e, a);
    assert_eq!(e.mem.u32(BASES_FREED), 1);
}
#[test]
fn same_group_unlinks_previous_base_without_destroying_it_until_teardown() {
    let mut e = fixture();
    let (a, _) = animation(&mut e);
    let one = load(&mut e, a, b"EM_NoBob.kf");
    let name = nvse::table::guest_str(&mut e, b"Other.kf");
    e.mem.set_u32(SOURCE + 8, name);
    let two = load(&mut e, a, b"Other.kf");
    assert_ne!(one.base, two.base);
    assert_eq!(e.mem.u32(BASES_FREED), 0);
    assert_eq!(cached(&e, a, b"EM_NoBob.kf"), Some(one));
    release(&mut e, a);
    assert_eq!(e.mem.u32(BASES_FREED), 2);
}
#[test]
fn failed_native_load_and_unwind_restore_private_selection_and_original_pointer() {
    let mut e = fixture();
    let (a, _) = animation(&mut e);
    e.mem.set_u32(FAIL_LOAD, 1);
    assert_eq!(load(&mut e, a, b"EM_NoBob.kf"), Loaded::default());
    assert_eq!(animation_maps::selected(a, 9), 9);
    assert_eq!(e.mem.u32(a + 0xdc), 0x76543210);
    assert!(cached(&e, a, b"EM_NoBob.kf").is_none());
    e.register(0x0049_0500, |_, _| panic!("native load stopped"));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        load(&mut e, a, b"EM_NoBob.kf")
    }));
    assert!(r.is_err());
    assert_eq!(animation_maps::selected(a, 9), 9);
    assert_eq!(e.mem.u32(a + 0xdc), 0x76543210);
}
#[test]
fn native_sequence_destruction_forgets_its_raw_cached_context() {
    let mut e = fixture();
    let (a, _) = animation(&mut e);
    let l = load(&mut e, a, b"EM_NoBob.kf");
    e.site(0x004e_eb4b, &[l.sequence]);
    assert!(cached(&e, a, b"EM_NoBob.kf").is_none());
}
