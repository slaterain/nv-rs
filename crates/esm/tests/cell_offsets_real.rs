//! The cell offset tables `esm::cell_offsets` builds from a plugin's records, against the
//! tables the game's own masters carry (`OFST`, made by Bethesda's tools): for every world space
//! of every master in the Data folder that has one, the built table must be the stored one,
//! entry for entry. Files without tables (a plugin, a master whose tables were stripped) are
//! built too and their tables counted, with the time each file took.
//!
//! ```text
//! NV_DATA="C:\...\Fallout New Vegas\Data" cargo test -p esm --test cell_offsets_real -- --ignored --nocapture
//! ```

use esm::cell_offsets::world_offsets;
use std::fs::File;
use std::path::PathBuf;
use std::time::Instant;

#[test]
#[ignore = "needs the player's FalloutNV.esm (set NV_DATA)"]
fn built_tables_are_the_masters_own() {
    let data = PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA: the game's Data folder"));
    let mut names: Vec<PathBuf> = std::fs::read_dir(&data)
        .expect("the Data folder")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("esm") || e.eq_ignore_ascii_case("esp"))
        })
        .collect();
    names.sort();
    assert!(
        names
            .iter()
            .any(|p| p.file_name().is_some_and(|n| n == "FalloutNV.esm")),
        "FalloutNV.esm is in {}",
        data.display()
    );
    let mut compared = 0;
    for path in names {
        let start = Instant::now();
        let worlds = world_offsets(File::open(&path).unwrap(), true).unwrap();
        let took = start.elapsed();
        let name = path.file_name().unwrap().to_string_lossy();
        for world in &worlds {
            let cells = world
                .built
                .as_ref()
                .map(|t| t.iter().filter(|&&o| o != 0).count());
            println!(
                "{name}: world {:08x} at {:#x}, grid {:?}, own table {}, built {:?}",
                world.form_id,
                world.record_offset,
                world.grid(),
                world.stored.as_ref().map_or(0, Vec::len),
                cells,
            );
            let Some(stored) = &world.stored else {
                continue;
            };
            let built = world
                .built
                .as_ref()
                .unwrap_or_else(|why| panic!("{name} {:08x}: not built: {why:?}", world.form_id));
            assert_eq!(
                built.len(),
                stored.len(),
                "{name} {:08x}: table size",
                world.form_id
            );
            let differ: Vec<usize> = (0..built.len())
                .filter(|&i| built[i] != stored[i])
                .collect();
            assert!(
                differ.is_empty(),
                "{name} {:08x}: {} of {} entries differ, first at {} (built {:#x}, own {:#x})",
                world.form_id,
                differ.len(),
                built.len(),
                differ[0],
                built[differ[0]],
                stored[differ[0]],
            );
            compared += 1;
        }
        // As the game asks: only what it would use, the rest not built (and not read).
        let start = Instant::now();
        let asked = world_offsets(File::open(&path).unwrap(), false).unwrap();
        let took_asked = start.elapsed();
        assert_eq!(asked.len(), worlds.len());
        for (asked, all) in asked.iter().zip(&worlds) {
            if let Ok(table) = &asked.built {
                assert_eq!(Ok(table), all.built.as_ref(), "{name} {:08x}", all.form_id);
            }
        }
        let given = asked.iter().filter(|w| w.built.is_ok()).count();
        println!(
            "{name}: {} world space records, all built in {took:.2?}; the {given} the game would \
             use in {took_asked:.2?}",
            worlds.len()
        );
    }
    println!("{compared} tables the same as the masters' own");
    assert!(compared > 0);
}
