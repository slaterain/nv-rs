//! Crash guards (`NV_CRASH_GUARDS`), a row of the `fixes` preset ([`crate::fixes`], docs/FIXES.md,
//! "Crash guards"): places where FalloutNV.exe 1.4.0.525 reads through a missing object and the
//! game crashes, found by lStewieAl's Tweaks and built here from the exe. In the `vanilla` preset
//! nothing is installed and the port stops where the original crashes.
//!
//! Each fault is a guard site in its translation (`engine::guard`, the table
//! `crates/engine-core/data/guard_sites.tsv`): the site passes the words it holds there and takes
//! the path the row answers for. This module is the row's answer at every site: whether the
//! fault is there (a null object, a form of the wrong type). What the guarded path is (a skip, a
//! substituted 0, an early end) is the site's own code, written from the exe around the
//! instruction the plugin patches.

use crate::fixes::Fix;
use engine::prelude::*;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "crash guards",
    env: "NV_CRASH_GUARDS",
    addresses: &[],
    summary: "where the original reads through a missing object and crashes (docs/FIXES.md, \
              \"Crash guards\"), the game goes on as for an empty one",
    install,
};

/// The form type of ammunition (`TESAmmo`, the byte at form +4).
const FORM_TYPE_AMMO: u8 = 0x29;

/// The sites this row answers at, in the table's order (a test holds the two together).
pub const SITES: &[u32] = &[
    0x004b_d48d,
    0x004b_d4b5,
    0x004b_d4dd,
    0x004f_269f,
    0x0057_aba4,
    0x0064_529d,
    0x0065_a125,
    0x0075_f78e,
    0x008b_fbc1,
    0x009b_9387,
    0x00a3_4fcb,
    0x00a3_5003,
];

/// Turns the guard sites on for the run.
pub fn install(e: &mut Engine) {
    engine::guard::install(e, SITES, guard);
}

/// The row's answer at site `at` with its words: true takes the guarded path.
fn guard(e: &mut Engine, at: u32, words: &[u32]) -> bool {
    match at {
        // A missing object: a weapon's mod, the player's cell, a reference's parent cell, a
        // mesh's geometry, the player name's text, an item's owner, a limb node, a sequence's
        // owner.
        0x004b_d48d | 0x004b_d4b5 | 0x004b_d4dd | 0x004f_269f | 0x0057_aba4 | 0x0065_a125
        | 0x0075_f78e | 0x008b_fbc1 | 0x009b_9387 | 0x00a3_4fcb | 0x00a3_5003 => words[0] == 0,
        // A form taken as ammunition that is no ammunition (a throwing weapon).
        0x0064_529d => e.mem.u8(words[0] + 4) != FORM_TYPE_AMMO,
        _ => panic!("guard site {at:08x} is not one of the crash guards' (guard_sites.tsv)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::{Path, PathBuf};

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn table() -> Vec<(u32, String)> {
        let text = std::fs::read_to_string(repo().join("crates/engine-core/data/guard_sites.tsv"))
            .expect("the guard sites' table");
        text.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let cols: Vec<&str> = l.trim_end_matches('\r').split('\t').collect();
                assert_eq!(cols.len(), 4, "{l}");
                (u32::from_str_radix(cols[0], 16).unwrap(), cols[2].to_string())
            })
            .collect()
    }

    fn sources(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|x| x == "rs") {
                out.push((path.clone(), std::fs::read_to_string(&path).unwrap()));
            }
        }
    }

    /// Every `guard_site!(e, 0x...)` in the engine crates, by its address.
    fn sites_in_code() -> Vec<u32> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(repo().join("crates")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("engine") && path.join("src").is_dir() {
                sources(&path.join("src"), &mut files);
            }
        }
        let mut found = Vec::new();
        for (path, text) in &files {
            if path.ends_with("guard.rs") {
                continue;
            }
            for part in text.split("guard_site!(").skip(1) {
                let address = part
                    .split(',')
                    .nth(1)
                    .unwrap_or_else(|| panic!("{}: a guard site without its address", path.display()))
                    .trim()
                    .trim_start_matches("0x")
                    .replace('_', "");
                found.push(u32::from_str_radix(&address, 16).unwrap_or_else(|_| {
                    panic!("{}: a guard site's address is not a literal", path.display())
                }));
            }
        }
        found.sort_unstable();
        found
    }

    #[test]
    fn the_table_is_the_code_and_the_row() {
        let table = table();
        let addresses: Vec<u32> = table.iter().map(|(a, _)| *a).collect();
        let mut sorted = addresses.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, addresses, "the table is sorted, one row a site");
        assert_eq!(sites_in_code(), addresses, "the translations' sites are the table's");
        let ours: Vec<u32> = table
            .iter()
            .filter(|(_, row)| row == "crash guards")
            .map(|(a, _)| *a)
            .collect();
        assert_eq!(ours, SITES, "the row answers at its sites");
    }

    // -----------------------------------------------------------------------------------------
    // Each site driven to its instruction with the faulting input: the original (no row) stops
    // at the bad read, the row goes on.
    // -----------------------------------------------------------------------------------------

    /// An engine whose every translated function but `keep` answers 0 with no effect, so a test
    /// runs the function under test and the reads it makes itself.
    fn isolated(keep: &[u32]) -> Engine {
        let mut e = Engine::new();
        let mut list = Vec::new();
        engine::units::funcs(&mut list);
        for (address, _) in list {
            if !keep.contains(&address) {
                e.register(address, |_, _| Ret::default());
            }
        }
        e
    }

    fn zeroed(e: &mut Engine, size: u32) -> u32 {
        let block = e.mem.alloc(size);
        for offset in (0..size).step_by(4) {
            e.mem.set_u32(block + offset, 0);
        }
        block
    }

    /// Test functions for virtual slots, past `.text`.
    const TEST_FUNCTIONS: u32 = 0x00ff_4000;

    /// An object of `size` zeroed bytes whose virtual slots answer the given words.
    fn object(e: &mut Engine, next: &mut u32, size: u32, slots: &[(u32, u32)]) -> u32 {
        let object = zeroed(e, size);
        let vtable = zeroed(e, 0x400);
        e.mem.set_u32(object, vtable);
        for &(slot, answer) in slots {
            let at = TEST_FUNCTIONS + *next * 4;
            *next += 1;
            e.register_double(at, move |_, _| answer.into_ret());
            e.mem.set_u32(vtable + slot, at);
        }
        object
    }

    /// Runs `f` without and with the row: the original's stop message, and the row's result.
    fn both<T>(mut make: impl FnMut() -> Engine, f: impl Fn(&mut Engine) -> T) -> (String, T) {
        let mut e = make();
        let stop = match catch_unwind(AssertUnwindSafe(|| f(&mut e))) {
            Ok(_) => panic!("the original went on at the fault"),
            Err(stop) => stop
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| stop.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        };
        let mut e = make();
        install(&mut e);
        (stop, f(&mut e))
    }

    fn stops_at_null(stop: &str) {
        assert!(
            stop.contains("read of unmapped memory at 0000") || stop.contains("write of unmapped memory at 0000"),
            "the original stops reading through null: {stop}"
        );
    }

    #[test]
    fn credits_with_no_save_loaded_close() {
        let (stop, flags) = both(
            || {
                let mut e = isolated(&[0x0075_f750]);
                // `strlen` as the C runtime has it: it reads the text.
                e.register(0x0044_a670, |e, a| {
                    let mut n = 0;
                    while e.mem.u8(a[0] + n) != 0 {
                        n += 1;
                    }
                    n.into_ret()
                });
                e.register(0x004a_4080, |_, _| 1u32.into_ret());
                let start = zeroed(&mut e, 0x200);
                e.mem.set_u32(start + 0x1a8, 0x4000_0000);
                e.set_global(0x011d_aac0, start);
                let saves = zeroed(&mut e, 0x40);
                e.set_global(0x011d_e134, saves);
                e
            },
            |e| {
                e.call(0x0075_f750, &args![]);
                let start = e.global::<u32>(0x011d_aac0);
                e.mem.u32(start + 0x1a8)
            },
        );
        stops_at_null(&stop);
        assert_eq!(flags, 0, "the row clears the flag and closes the menu");
    }

    #[test]
    fn steal_alarm_with_no_owner() {
        let (stop, ()) = both(
            || {
                let mut e = isolated(&[0x008b_fa40, 0x0040_1170]);
                let player = zeroed(&mut e, 0x800);
                e.set_global(0x011d_ea3c, player);
                let word = zeroed(&mut e, 4);
                e.register_double(0x0043_d4d0, move |_, _| word.into_ret());
                e.register_double(0x0040_3e20, move |_, _| word.into_ret());
                e
            },
            |e| {
                let mut next = 0;
                let taken_from = object(e, &mut next, 0x100, &[(0x100, 0)]);
                let player = e.global::<u32>(0x011d_ea3c);
                e.call(0x008b_fa40, &args![player, taken_from, 0x1234u32, 1u32, 0u32, 0u32]);
            },
        );
        stops_at_null(&stop);
    }

    #[test]
    fn dismemberment_with_no_limb_node() {
        let (stop, cleared) = both(
            || {
                let mut e = isolated(&[0x009b_90b0, 0x008d_01e0]);
                e.set_global(0x011f_201c, 1u32);
                e.register(0x0057_3090, |_, _| 1u32.into_ret());
                e
            },
            |e| {
                let mut next = 0;
                let extra = zeroed(e, 0x40);
                e.register_double(0x0042_e8c0, move |_, _| extra.into_ret());
                let part_node = object(e, &mut next, 0x100, &[(0x9c, 0)]);
                let actor = object(e, &mut next, 0x100, &[(0x100, 1), (0x1d0, part_node)]);
                let node = object(e, &mut next, 0x100, &[(0xc, 0)]);
                let p = object(e, &mut next, 0x200, &[(0x1d0, node)]);
                let impact = zeroed(e, 0x40);
                e.mem.set_u32(impact, actor);
                e.mem.set_u32(p + 0x88, impact);
                e.mem.set_u8(p + 0x90, 1);
                e.mem.set_u32(p + 0x150, 3);
                e.call(0x009b_90b0, &args![p]);
                e.mem.u8(p + 0x90) == 0
            },
        );
        stops_at_null(&stop);
        assert!(cleared, "the row ends the cut as for a missing limb");
    }

    #[test]
    fn controller_sequence_with_no_owner() {
        for (ease_in, state) in [(1.0f32, 2u32), (0.0, 1)] {
            let (stop, (activated, now)) = both(
                || isolated(&[0x00a3_4f20]),
                |e| {
                    let sequence = zeroed(e, 0x80);
                    let activated = e
                        .call(
                            0x00a3_4f20,
                            &args![sequence, 0u32, 0u32, 1.0f32, ease_in, 0u32, 0u32],
                        )
                        .bool();
                    (activated, e.mem.u32(sequence + 0x44))
                },
            );
            stops_at_null(&stop);
            assert!(activated);
            assert_eq!(now, state);
        }
    }

    #[test]
    fn face_gen_mesh_with_no_geometry() {
        let (stop, loaded) = both(
            || {
                let mut e = isolated(&[0x0065_9ce0, 0x0045_6650]);
                e.register(0x0040_1000, |e, a| zeroed(e, a[0]).into_ret());
                // The tri file reads, and holds 5 model vertices.
                e.register(0x00ab_5ec0, |_, _| 1u32.into_ret());
                e.register(0x0065_c700, |_, _| 5u32.into_ret());
                e
            },
            |e| {
                let model = zeroed(e, 0x10);
                e.call(
                    0x0065_9ce0,
                    &args![model, 0x1000u32, 0u32, 0x2000u32, -1i32, 0u32],
                )
                .bool()
            },
        );
        stops_at_null(&stop);
        assert!(loaded);
    }

    #[test]
    fn weapon_damage_with_a_throwing_weapon_as_ammo() {
        let (stop, ()) = both(
            || {
                let mut e = isolated(&[0x0064_50f0, 0x0059_a030]);
                let player = zeroed(&mut e, 0x800);
                e.set_global(0x011d_ea3c, player);
                e
            },
            |e| {
                let weapon = zeroed(e, 0x400);
                e.mem.set_u8(weapon + 4, 0x28);
                let thrown = zeroed(e, 0x400);
                e.mem.set_u8(thrown + 4, 0x28);
                // What the effect list reads as its first entry in the weapon form.
                e.mem.set_u32(thrown + 0xd4, 0x0000_0100);
                e.call(0x0064_50f0, &args![weapon, 1.0f32, 0u32, thrown]);
            },
        );
        stops_at_null(&stop);
    }

    #[test]
    fn multibound_reference_with_no_cell() {
        let (stop, ()) = both(
            || isolated(&[0x0057_ab70, 0x008d_6f30, 0x0054_5960]),
            |e| {
                let mut next = 0;
                let reference = object(e, &mut next, 0x100, &[(0x1d0, 0)]);
                e.register(0x004b_9ba0, |_, _| 1u32.into_ret());
                e.register_double(0x006b_7f20, move |e, a| {
                    e.mem.set_u32(a[1], 0);
                    e.mem.set_u32(a[2], reference);
                    e.mem.set_u32(a[3], 1);
                    Ret::default()
                });
                e.call(0x0057_ab70, &args![]);
            },
        );
        stops_at_null(&stop);
    }

    #[test]
    fn item_value_with_a_removed_weapon_mod() {
        let (stop, _) = both(
            || {
                let mut e = isolated(&[0x004b_d400]);
                e.register(0x0041_8ba0, |_, _| 1u32.into_ret());
                e.register(0x0042_e560, |_, _| 1u32.into_ret());
                e
            },
            |e| {
                let weapon = zeroed(e, 0x400);
                e.register_double(0x00ec_43fb, move |_, _| weapon.into_ret());
                let extra = zeroed(e, 0x20);
                let node = zeroed(e, 8);
                e.mem.set_u32(node, extra);
                let entry = zeroed(e, 0x10);
                e.mem.set_u32(entry, node);
                e.mem.set_u32(entry + 8, weapon);
                e.call(0x004b_d400, &args![entry]).f32()
            },
        );
        stops_at_null(&stop);
    }

    #[test]
    fn region_check_with_the_player_in_no_cell() {
        let (stop, found) = both(
            || {
                let mut e = isolated(&[0x004f_2640, 0x0054_7110]);
                let player = zeroed(&mut e, 0x800);
                e.set_global(0x011d_ea3c, player);
                e
            },
            |e| e.call(0x004f_2640, &args![0x1234u32]).bool(),
        );
        stops_at_null(&stop);
        assert!(!found);
    }
}
