use super::*;
use std::{cell::RefCell, collections::BTreeMap};
type Files = BTreeMap<Vec<u8>, Vec<mod_files::Listed>>;
thread_local! {static FOLDERS:RefCell<Files>=const{RefCell::new(BTreeMap::new())};}
fn norm(p: &[u8]) -> Vec<u8> {
    p.to_ascii_lowercase()
}
fn unavailable(_: &Engine, _: &[u8]) -> Result<Vec<u8>, u32> {
    Err(mod_files::ERROR_FILE_NOT_FOUND)
}
fn no_write(_: &mut Engine, _: &[u8], _: &[u8]) -> Result<(), u32> {
    Err(mod_files::ERROR_NOT_SUPPORTED)
}
static CALLS: mod_files::ModFileCalls = mod_files::ModFileCalls {
    read: unavailable,
    exists: |_, _| false,
    directory_exists: |_, p| FOLDERS.with(|f| f.borrow().contains_key(&norm(p))),
    create_directory: |_, _| Err(mod_files::ERROR_NOT_SUPPORTED),
    write: no_write,
    append: no_write,
    read_head: |e, p, _| unavailable(e, p),
    list: |_, p| FOLDERS.with(|f| f.borrow().get(&norm(p)).cloned().unwrap_or_default()),
    name_key: |p| {
        p.iter()
            .map(|b| u16::from(b.to_ascii_uppercase()))
            .collect()
    },
};
#[test]
fn command_folder_recurses_one_variant_batch_and_json_uses_pov_subdirectories() {
    mod_files::install(&CALLS);
    let mut e = Engine::new();
    FOLDERS.with(|f| {
        let mut f = f.borrow_mut();
        for (name, items) in [
            (
                b"Data\\Meshes\\Demo".as_slice(),
                vec![("Top.KF", false), ("readme.txt", false), ("Nested", true)],
            ),
            (b"Data\\Meshes\\Demo\\Nested", vec![("Other.kf", false)]),
            (
                b"Data\\Meshes\\AnimGroupOverride\\Prone",
                vec![("_male", true), ("_1stperson", true), ("Ignored", true)],
            ),
            (
                b"Data\\Meshes\\AnimGroupOverride\\Prone\\_male",
                vec![("sneak.kf", false)],
            ),
            (
                b"Data\\Meshes\\AnimGroupOverride\\Prone\\_1stperson",
                vec![("idle.kf", false)],
            ),
        ] {
            f.insert(
                norm(name),
                items
                    .into_iter()
                    .map(|(name, is_folder)| mod_files::Listed {
                        name: name.as_bytes().to_vec(),
                        is_folder,
                    })
                    .collect(),
            );
        }
    });
    assert_eq!(
        command_files(&mut e, b"Demo"),
        [b"Demo\\Top.KF".to_vec(), b"Demo\\Nested\\Other.kf".to_vec()]
    );
    assert_eq!(
        command_files(&mut e, b"Missing.kf"),
        [b"Missing.kf".to_vec()]
    );
    let groups = json_folder(&mut e, b"Prone");
    assert_eq!(groups.len(), 2);
    assert!(!groups[0].0);
    assert!(groups[1].0);
    assert_eq!(
        groups[0].1,
        [b"AnimGroupOverride\\Prone\\_male\\sneak.kf".to_vec()]
    );
    FOLDERS.with(|f| f.borrow_mut().clear());
}
#[test]
fn filename_group_resolution_uses_actual_group_table_and_native_prefix_outputs() {
    let mut e = crate::tests::engine();
    let name = table::guest_str(&mut e, b"Aim");
    e.mem.set_u32(0x0119_77d8 + 17 * 0x24, name);
    let sneak = table::guest_str(&mut e, b"sneak");
    e.mem.set_u32(0x0119_7798, sneak);
    let hand = table::guest_str(&mut e, b"1hp");
    e.mem.set_u32(0x0119_77a8, hand);
    e.register(0x005f_38d0, |e, w| {
        assert_eq!(
            table::guest_text(e, w[0]),
            b"Folder\\pasneak1hpAim_order_2.kf"
        );
        e.mem.set_u32(w[1], 1);
        e.mem.set_u32(w[2], 2);
        e.mem.set_u32(w[3], 1);
        Ret::default()
    });
    assert_eq!(
        group_id(&mut e, b"Folder\\pasneak1hpAim_order_2.kf"),
        Ok(0x9211)
    );
}
#[test]
fn condition_text_is_the_original_function_source_not_a_success_stub() {
    assert_eq!(
        condition_source("GetVariable value Quest == 1"),
        b"begin function{}\nSetFunctionValue (GetVariable value Quest == 1)\nend\n"
    );
    assert_eq!(
        condition_source("SetFunctionValue 1%rReturn"),
        b"begin function{}\nSetFunctionValue 1\r\nReturn\nend\n"
    );
    // Compilation is deliberately performed only by compile_script_ex in the
    // production loader. This test asserts source construction, not a fake
    // successful compiler or execution of external quest records.
}
