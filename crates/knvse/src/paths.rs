//! kNVSE24 4640f0f: OverrideAnimsFromScript/GetAnimGroupId and
//! file_animations.cpp's JSON entries. Game file paths use the existing
//! portable mod-files seam, retaining recursive loose-file enumeration.
use engine_core::{mod_files, prelude::*};
use nvse::table;

pub(crate) const ROOT: &[u8] = b"Data\\Meshes\\AnimGroupOverride";
pub(crate) fn join(base: &[u8], name: &[u8]) -> Vec<u8> {
    [base, b"\\", name].concat()
}
pub(crate) fn extension(path: &[u8], ext: &[u8]) -> bool {
    path.len() >= ext.len() && path[path.len() - ext.len()..].eq_ignore_ascii_case(ext)
}
fn walk(e: &mut Engine, folder: &[u8], out: &mut Vec<Vec<u8>>) {
    for item in mod_files::list(e, folder) {
        let child = join(folder, &item.name);
        if item.is_folder {
            walk(e, &child, out);
        } else if extension(&item.name, b".kf") {
            out.push(child[b"Data\\Meshes\\".len()..].to_vec());
        }
    }
}
/// Each command invocation is one variant batch, including all nested KFs.
pub(crate) fn command_files(e: &mut Engine, path: &[u8]) -> Vec<Vec<u8>> {
    let full = join(b"Data\\Meshes", path);
    if !mod_files::directory_exists(e, &full) {
        return vec![path.to_vec()];
    }
    let mut out = Vec::new();
    walk(e, &full, &mut out);
    out
}
pub(crate) fn json_folder(e: &mut Engine, folder: &[u8]) -> Vec<(bool, Vec<Vec<u8>>)> {
    let full = join(ROOT, folder);
    let mut groups = Vec::new();
    for item in mod_files::list(e, &full) {
        if item.is_folder
            && (item.name.eq_ignore_ascii_case(b"_male")
                || item.name.eq_ignore_ascii_case(b"_1stperson"))
        {
            let first = item.name.eq_ignore_ascii_case(b"_1stperson");
            let mut files = Vec::new();
            walk(e, &join(&full, &item.name), &mut files);
            groups.push((first, files));
        }
    }
    groups
}
fn starts(text: &[u8], prefix: &[u8]) -> bool {
    text.get(..prefix.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
}
fn simple_id(e: &Engine, name: &[u8]) -> Option<u8> {
    let end = name.iter().position(|&b| b == b'_').unwrap_or(name.len());
    let name = &name[..end];
    let end = name.iter().position(|&b| b == b' ').unwrap_or(name.len());
    let name = &name[..end];
    (0..245u32)
        .find(|&id| {
            table::guest_text(e, e.mem.u32(0x0119_77d8 + id * 0x24)).eq_ignore_ascii_case(name)
        })
        .map(|id| id as u8)
}
fn file_stem(path: &[u8]) -> &[u8] {
    let name = path
        .rsplit(|&b| b == b'\\' || b == b'/')
        .next()
        .unwrap_or(path);
    &name[..name.iter().rposition(|&b| b == b'.').unwrap_or(name.len())]
}
fn base_name<'a>(e: &Engine, mut name: &'a [u8]) -> &'a [u8] {
    if starts(name, b"pa") {
        name = &name[2..];
    }
    for id in 0..3 {
        let p = table::guest_text(e, e.mem.u32(0x0119_7798 + id * 4));
        if starts(name, &p) {
            name = &name[p.len()..];
            break;
        }
    }
    if starts(name, b"mt") {
        return &name[2..];
    }
    for id in 0..11 {
        let p = table::guest_text(e, e.mem.u32(0x0119_77a8 + id * 4));
        if starts(name, &p) {
            name = &name[p.len()..];
            break;
        }
    }
    name
}
fn full_id(e: &mut Engine, path: &[u8], base: u8) -> u16 {
    super::loader::with_path(e, path, |e, text| {
        e.with_stack(12, |e, words| {
            e.mem.write(words.addr(), &[0; 12]);
            e.call(
                0x005f_38d0,
                &args![text, words, words.addr() + 4, words.addr() + 8],
            );
            (u32::from(base)
                + (e.mem.u32(words.addr()) << 12)
                + (e.mem.u32(words.addr() + 4) << 8)
                + (e.mem.u32(words.addr() + 8) << 15)) as u16
        })
    })
}
/// Resolve the filename first; unknown stems use the actual game BSStream
/// sequence reader, as24 does, rather than guessing a group from a substring.
pub(crate) fn group_id(e: &mut Engine, path: &[u8]) -> Result<u16, String> {
    if let Some(id) = simple_id(e, base_name(e, file_stem(path))) {
        return Ok(full_id(e, path, id));
    }
    let full = [b"Meshes\\".as_slice(), path].concat();
    let base = super::loader::with_path(e, &full, |e, text| {
        e.with_stack(1496, |e, frame| {
            let file = e
                .call(0x00af_df20, &args![text, 0u32, u32::MAX, 1u32])
                .u32();
            if file == 0 {
                return None;
            }
            let stream = frame.addr();
            let result = stream + 1492;
            e.call(0x0043_cfd0, &args![stream]);
            if !e.call(0x00c3_a8a0, &args![stream, text, file]).bool() {
                e.call(0x0043_d090, &args![stream]);
                return None;
            }
            e.call(0x0063_3c90, &args![result, 0u32]);
            e.call(0x00a3_5700, &args![stream, 0u32, result]);
            let sequence = e.mem.u32(result);
            let id = if sequence == 0 {
                None
            } else {
                simple_id(e, &table::guest_text(e, e.mem.u32(sequence + 8)))
            };
            e.call(0x0043_d090, &args![stream]);
            // Owned temporary NiPointer is released in the native port;24 left
            // this pointer on its stack without a destructor (a model leak).
            e.call(0x0045_cec0, &args![result]);
            id
        })
    });
    base.map(|id| full_id(e, path, id)).ok_or_else(|| {
        format!(
            "kNVSE24: cannot resolve animation group for {}",
            String::from_utf8_lossy(path)
        )
    })
}

pub(crate) fn condition_source(text: &str) -> Vec<u8> {
    let body = if text.to_ascii_lowercase().contains("setfunctionvalue") {
        text.replace("%r", "\r\n").replace("%R", "\r\n")
    } else {
        format!("SetFunctionValue ({text})")
    };
    format!("begin function{{}}\n{body}\nend\n").into_bytes()
}
pub(crate) struct JsonEntry {
    pub folder: Vec<u8>,
    pub form: u32,
    pub condition: u32,
    pub poll: bool,
    pub priority: i32,
}
fn mod_index(e: &Engine, name: &[u8]) -> Option<u8> {
    let handler = e.mem.u32(0x011c_3f2c);
    if handler == 0 {
        return None;
    }
    let mut node = handler + 0x210;
    while node != 0 {
        let info = e.mem.u32(node);
        if info != 0 && table::guest_text(e, info + 0x20).eq_ignore_ascii_case(name) {
            return Some(e.mem.u8(info + 0x40c));
        }
        node = e.mem.u32(node + 4);
    }
    None
}
pub(crate) fn read_json(e: &mut Engine) -> Vec<JsonEntry> {
    let mut entries = Vec::new();
    for item in mod_files::list(e, ROOT) {
        if item.is_folder || !extension(&item.name, b".json") {
            continue;
        }
        let path = join(ROOT, &item.name);
        let Some(bytes) = mod_files::read(e, &path) else {
            continue;
        };
        let parsed = serde_json::from_slice::<serde_json::Value>(&bytes);
        let Ok(serde_json::Value::Array(array)) = parsed else {
            eprintln!(
                "kNVSE24: invalid JSON array {}",
                String::from_utf8_lossy(&path)
            );
            continue;
        };
        for value in array {
            let Some(object) = value.as_object() else {
                eprintln!("kNVSE24: expected JSON object");
                continue;
            };
            let Some(folder) = object.get("folder").and_then(|v| v.as_str()) else {
                eprintln!("kNVSE24: missing folder");
                continue;
            };
            let priority = object.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let mod_name = object.get("mod").and_then(|v| v.as_str()).unwrap_or("");
            let index = if mod_name.is_empty() {
                None
            } else {
                let Some(i) = mod_index(e, mod_name.as_bytes()) else {
                    eprintln!("kNVSE24: mod not loaded: {mod_name}");
                    continue;
                };
                Some(i)
            };
            let forms = match object.get("form") {
                Some(serde_json::Value::Array(a)) => {
                    a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
                }
                Some(v) => v.as_str().into_iter().collect(),
                None => Vec::new(),
            };
            let parsed = forms
                .iter()
                .map(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16))
                .collect::<Result<Vec<_>, _>>();
            let Ok(ids) = parsed else {
                eprintln!("kNVSE24: invalid hexadecimal form in JSON");
                continue;
            };
            let mut condition = 0;
            let mut poll = false;
            if let Some(text) = object.get("condition").and_then(|v| v.as_str()) {
                let source = condition_source(text);
                let name = format!("kNVSEConditionScript_{folder}");
                condition = nvse::compile_script::compile_script_ex(
                    e,
                    &source,
                    Some(name.as_bytes()),
                    false,
                );
                poll = object
                    .get("pollCondition")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if condition == 0 {
                    eprintln!("kNVSE24: failed compiling condition for {folder}; conditional entry skipped");
                    continue;
                }
            }
            if let Some(index) = index.filter(|_| !ids.is_empty()) {
                for id in ids {
                    let id = (u32::from(index) << 24) | (id & 0xffffff);
                    let form = e.call(0x0048_39c0, &args![id]).u32();
                    if form != 0 {
                        entries.push(JsonEntry {
                            folder: folder.as_bytes().to_vec(),
                            form,
                            condition,
                            poll,
                            priority,
                        });
                    } else {
                        eprintln!("kNVSE24: form {id:08X} not found");
                    }
                }
            } else {
                entries.push(JsonEntry {
                    folder: folder.as_bytes().to_vec(),
                    form: 0,
                    condition,
                    poll,
                    priority,
                });
            }
        }
    }
    entries.sort_by_key(|entry| entry.priority);
    entries
}

#[cfg(test)]
mod tests;
