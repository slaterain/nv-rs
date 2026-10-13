//! `fallout/misc/saveload/bgssaveloadstructs.cpp` (Xbox PDB source unit), subsystem `fallout/misc`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00853500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSNumericIDIndex::GetNumericID` (Xbox PDB): reads the big-endian
/// three-byte index. Bit 23 denotes a created form; other values use the
/// save manager's reverse map.
pub fn get_numeric_id(e: &mut Engine, this: Ptr) -> u32 {
    let index = ((e.mem.u8(this.addr()) as u32) << 16)
        | ((e.mem.u8(this.addr().wrapping_add(1)) as u32) << 8)
        | e.mem.u8(this.addr().wrapping_add(2)) as u32;
    if index & 0x800000 != 0 {
        (index & 0x7fffff) | 0xff000000
    } else {
        let manager = e.mem.u32(0x011ddf38);
        let map = e.call(0x0044ddc0, &args![manager]).u32();
        e.call(0x00846d20, &args![map, index]).u32()
    }
}

// Translated from 00853570 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSNumericIDIndex::SetNumericID` (Xbox PDB): resolves the save index,
/// marks created IDs with bit 23 and writes exactly three bytes.
pub fn set_numeric_id(e: &mut Engine, this: Ptr, id: u32) {
    let manager = e.mem.u32(0x011ddf38);
    let map = e.call(0x0044ddc0, &args![manager]).u32();
    let mut index = e.call(0x00846c90, &args![map, id]).u32();
    if index >> 24 == 0xff {
        index |= 0x800000;
    }
    e.mem.set_u8(this.addr(), (index >> 16) as u8);
    e.mem
        .set_u8(this.addr().wrapping_add(1), (index >> 8) as u8);
    e.mem.set_u8(this.addr().wrapping_add(2), index as u8);
}

/// The byte table `008535e0` fills: `0x79` bytes at `011de360`, indexed by
/// the values of the dword table at `011a2428`.
const INDEX_BYTES: u32 = 0x011d_e360;
/// The number of entries of the dword table, kept in a global the loop
/// re-reads every pass.
const INDEX_COUNT: u32 = 0x011d_e3e0;
/// The dword table at `011a2428`.
const INDEX_OFFSETS: u32 = 0x011a_2428;

// Translated from 008535e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (the initializer `BGSSaveLoadGame::Create`
/// runs last): fills the 0x79 bytes at `011de360` with 0xff (`memset`),
/// stores 0x37 in `011de3e0`, then for `i` from 0 while `i < [011de3e0]`
/// writes the byte `i` at `011de360 + table[i]` (`table` is the dword array
/// at `011a2428`).
pub fn fn_008535e0(e: &mut Engine) {
    e.call(0x0040_3d30, &args![INDEX_BYTES, 0xffu32, 0x79u32]);
    e.set_global::<u32>(INDEX_COUNT, 0x37);
    let mut i = 0u32;
    while i < e.global::<u32>(INDEX_COUNT) {
        let offset = e.mem.u32(INDEX_OFFSETS.wrapping_add(i.wrapping_mul(4)));
        e.mem.set_u8(offset.wrapping_add(INDEX_BYTES), i as u8);
        i += 1;
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00853500, get_numeric_id(Ptr) -> u32),
        entry!(0x00853570, set_numeric_id(Ptr, u32)),
        entry!(0x008535e0, fn_008535e0()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_ids_decode_without_a_save_manager_and_preserve_memory() {
        // Generated native vectors, FalloutNV.exe 1.4.0.525 SHA256
        // 19406942e48724d797300c4ea6be9ac69a32f670b8c35db09279a2258422739f,
        // Ryzen AI 9 HX 370, FCW027F/MXCSR1F80: all three outputs/guards agree.
        let mut e = Engine::new();
        let index = e.mem.alloc(5);
        for (bytes, expected) in [
            ([0x80, 0, 0], 0xff000000),
            ([0x81, 0x23, 0x45], 0xff012345),
            ([0xff, 0xff, 0xff], 0xff7fffff),
        ] {
            let data = [0xaa, bytes[0], bytes[1], bytes[2], 0xbb];
            e.mem.write(index, &data);
            assert_eq!(e.call(0x00853500, &args![index + 1]).u32(), expected);
            assert_eq!(e.mem.bytes(index, 5), data);
        }
    }

    #[test]
    fn ordinary_indices_use_the_returned_map_and_preserve_full_result() {
        let mut e = Engine::new();
        e.map(0x011ddf38, 4);
        e.mem.set_u32(0x011ddf38, 0x1010);
        let index = e.mem.alloc(3);
        e.mem.write(index, &[0x12, 0x34, 0x56]);
        e.register_double(0x0044ddc0, |_, a| {
            assert_eq!(a, &[0x1010]);
            0x2020u32.into_ret()
        });
        e.register_double(0x00846d20, |_, a| {
            assert_eq!(a, &[0x2020, 0x123456]);
            0xabcdef01u32.into_ret()
        });
        assert_eq!(e.call(0x00853500, &args![index]).u32(), 0xabcdef01);
    }

    #[test]
    fn writing_marks_created_ids_after_mapping_and_touches_only_three_bytes() {
        let mut e = Engine::new();
        e.map(0x011ddf38, 4);
        e.mem.set_u32(0x011ddf38, 0x1010);
        let index = e.mem.alloc(5);
        e.register_double(0x0044ddc0, |_, a| {
            assert_eq!(a, &[0x1010]);
            0x2020u32.into_ret()
        });
        for (mapped, bytes) in [
            (0x00123456u32, [0x12, 0x34, 0x56]),
            (0xff012345, [0x81, 0x23, 0x45]),
            (0xfe012345, [0x01, 0x23, 0x45]),
            (0xffffffff, [0xff, 0xff, 0xff]),
        ] {
            e.mem.write(index, &[0xaa; 5]);
            e.register_double(0x00846c90, move |_, a| {
                assert_eq!(a, &[0x2020, 0x778899]);
                mapped.into_ret()
            });
            e.call(0x00853570, &args![index + 1, 0x778899u32]);
            assert_eq!(
                e.mem.bytes(index, 5),
                [0xaa, bytes[0], bytes[1], bytes[2], 0xaa]
            );
        }
    }
}

#[cfg(test)]
mod index_table_tests {
    use super::*;

    #[test]
    fn the_index_bytes_are_filled_through_the_offset_table() {
        let mut e = Engine::new();
        e.register(0x0040_3d30, |e, a| {
            assert_eq!((a[0], a[1], a[2]), (INDEX_BYTES, 0xff, 0x79));
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            Ret::default()
        });
        e.map(INDEX_BYTES, 0x100);
        e.map(INDEX_COUNT, 4);
        e.map(INDEX_OFFSETS, 0x100);
        // Offsets 0, 2, 4, ... (the first 0x37 entries); the rest stay 0xff.
        for i in 0..0x37u32 {
            e.mem.set_u32(INDEX_OFFSETS + 4 * i, 2 * i);
        }
        e.set_global::<u32>(INDEX_COUNT, 5);
        e.call(0x008535e0, &args![]);
        assert_eq!(e.global::<u32>(INDEX_COUNT), 0x37, "the count is reset");
        assert_eq!(e.mem.u8(INDEX_BYTES), 0);
        assert_eq!(e.mem.u8(INDEX_BYTES + 2), 1);
        assert_eq!(e.mem.u8(INDEX_BYTES + 4), 2);
        assert_eq!(e.mem.u8(INDEX_BYTES + 0x6c), 0x36);
        assert_eq!(e.mem.u8(INDEX_BYTES + 1), 0xff, "unused bytes stay 0xff");
        assert_eq!(e.mem.u8(INDEX_BYTES + 0x78), 0xff);
    }
}
