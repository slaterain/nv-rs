//! ShowOff f45d069 SO_fn_Files.cpp::IniToNVSE and four commands used by
//! MCM Extender: SetINIValC, SetINIIntC, GetINIString_Cached, SaveCachedIni.
//! Writes alter SimpleIni's cache only; saving is explicit. The retained document
//! follows the shipped SimpleIni.h::FindEntry/AddEntry/Save (load order, first
//! spelling/comments, last value, spaces and Windows CRLF, no BOM).
use super::{full_path, relative_name, with_args, Cache, Values};
use engine_core::{mod_files, prelude::*};
use nvse::{
    expr::Ty,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};

#[derive(Clone, Default)]
struct Entry {
    key: Vec<u8>,
    value: Vec<u8>,
    comment: Vec<u8>,
}
#[derive(Clone, Default)]
struct Section {
    name: Vec<u8>,
    comment: Vec<u8>,
    entries: Vec<Entry>,
}
#[derive(Clone, Default)]
pub(super) struct Document {
    comment: Vec<u8>,
    sections: Vec<Section>,
}
fn cstr(v: &[u8]) -> &[u8] {
    v.split(|&b| b == 0).next().unwrap_or_default()
}
fn space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
fn trim_end(v: &[u8]) -> &[u8] {
    &v[..v.iter().rposition(|&b| !space(b)).map_or(0, |i| i + 1)]
}
fn newline(v: &[u8], at: &mut usize) {
    if v.get(*at) == Some(&b'\r') {
        *at += 1;
        if v.get(*at) == Some(&b'\n') {
            *at += 1;
        }
    } else if v.get(*at) == Some(&b'\n') {
        *at += 1;
    }
}
fn comment(v: &[u8], at: &mut usize, allow_blank: bool) -> Vec<u8> {
    let mut out = Vec::new();
    while v.get(*at).is_some_and(|b| matches!(b, b';' | b'#')) {
        let start = *at;
        while *at < v.len() && !matches!(v[*at], b'\r' | b'\n') {
            *at += 1;
        }
        out.extend_from_slice(&v[start..*at]);
        newline(v, at);
        if v.get(*at).is_some_and(|b| matches!(b, b';' | b'#')) {
            out.push(b'\n');
            continue;
        }
        if allow_blank {
            let mut look = *at;
            let mut lines = 0;
            while look < v.len() && space(v[look]) {
                if matches!(v[look], b'\r' | b'\n') {
                    newline(v, &mut look);
                    lines += 1;
                } else {
                    look += 1;
                }
            }
            if v.get(look).is_some_and(|b| matches!(b, b';' | b'#')) {
                out.extend(std::iter::repeat(b'\n').take(lines + 1));
                *at = look;
                continue;
            }
        }
        break;
    }
    out
}
impl Document {
    fn section(&mut self, name: &[u8], comment: Vec<u8>) -> usize {
        if let Some(i) = self
            .sections
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(name))
        {
            return i;
        }
        self.sections.push(Section {
            name: name.to_vec(),
            comment,
            entries: Vec::new(),
        });
        self.sections.len() - 1
    }
    fn set(&mut self, section: &[u8], key: &[u8], value: &[u8], comment: &[u8]) -> i32 {
        let i = self.section(cstr(section), Vec::new());
        let section = &mut self.sections[i];
        let key = cstr(key);
        if let Some(entry) = section
            .entries
            .iter_mut()
            .find(|e| e.key.eq_ignore_ascii_case(key))
        {
            entry.value = cstr(value).to_vec();
            return 1;
        }
        let mut comment = cstr(comment).to_vec();
        if !comment.is_empty() && !matches!(comment[0], b';' | b'#') {
            comment.insert(0, b';');
        }
        section.entries.push(Entry {
            key: key.to_vec(),
            value: cstr(value).to_vec(),
            comment,
        });
        2
    }
    fn parse(bytes: &[u8]) -> Self {
        let bytes = cstr(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes));
        let mut doc = Self::default();
        let mut at = 0;
        doc.comment = comment(bytes, &mut at, false);
        let mut section = Some(Vec::new());
        let mut note = Vec::new();
        while at < bytes.len() {
            while at < bytes.len() && space(bytes[at]) {
                at += 1;
            }
            if at == bytes.len() {
                break;
            }
            if matches!(bytes[at], b';' | b'#') {
                note = comment(bytes, &mut at, true);
                continue;
            }
            if bytes[at] == b'[' {
                at += 1;
                while at < bytes.len() && space(bytes[at]) {
                    at += 1;
                }
                let start = at;
                while at < bytes.len() && !matches!(bytes[at], b']' | b'\r' | b'\n') {
                    at += 1;
                }
                if bytes.get(at) != Some(&b']') {
                    section = None;
                    continue;
                }
                let name = trim_end(&bytes[start..at]).to_vec();
                doc.section(&name, std::mem::take(&mut note));
                section = Some(name);
                at += 1;
                while at < bytes.len() && !matches!(bytes[at], b'\r' | b'\n') {
                    at += 1;
                }
                continue;
            }
            let start = at;
            while at < bytes.len() && !matches!(bytes[at], b'=' | b'\r' | b'\n') {
                at += 1;
            }
            if bytes.get(at) != Some(&b'=') {
                continue;
            }
            if at == start {
                while at < bytes.len() && !matches!(bytes[at], b'\r' | b'\n') {
                    at += 1;
                }
                continue;
            }
            let key = trim_end(&bytes[start..at]);
            at += 1;
            while at < bytes.len() && matches!(bytes[at], b' ' | b'\t') {
                at += 1;
            }
            let start = at;
            while at < bytes.len() && !matches!(bytes[at], b'\r' | b'\n') {
                at += 1;
            }
            if let Some(section) = &section {
                doc.set(section, key, trim_end(&bytes[start..at]), &note);
            }
            note.clear();
        }
        doc
    }
    fn values(&self) -> Values {
        self.sections
            .iter()
            .flat_map(|s| {
                s.entries.iter().map(move |e| {
                    (
                        (s.name.to_ascii_lowercase(), e.key.to_ascii_lowercase()),
                        e.value.clone(),
                    )
                })
            })
            .collect()
    }
    fn from_values(values: &Values) -> Self {
        // Only needed by synthetic fixtures that directly seed the old cache.
        let mut pairs = values.iter().collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.cmp(b.0));
        let mut doc = Self::default();
        for ((s, k), v) in pairs {
            doc.set(s, k, v, &[]);
        }
        doc
    }
    fn bytes(&self) -> Vec<u8> {
        fn text(out: &mut Vec<u8>, v: &[u8]) {
            let mut at = 0;
            while at < v.len() {
                let start = at;
                while at < v.len() && !matches!(v[at], b'\r' | b'\n') {
                    at += 1;
                }
                out.extend_from_slice(&v[start..at]);
                out.extend_from_slice(b"\r\n");
                newline(v, &mut at);
            }
        }
        let mut out = Vec::new();
        let mut need = false;
        if !self.comment.is_empty() {
            text(&mut out, &self.comment);
            need = true;
        }
        for section in &self.sections {
            if !section.comment.is_empty() {
                if need {
                    out.extend_from_slice(b"\r\n\r\n");
                }
                text(&mut out, &section.comment);
                need = false;
            }
            if need {
                out.extend_from_slice(b"\r\n\r\n");
            }
            if !section.name.is_empty() {
                out.push(b'[');
                out.extend_from_slice(&section.name);
                out.extend_from_slice(b"]\r\n");
            }
            for entry in &section.entries {
                if !entry.comment.is_empty() {
                    out.extend_from_slice(b"\r\n");
                    text(&mut out, &entry.comment);
                }
                out.extend_from_slice(&entry.key);
                out.extend_from_slice(b" = ");
                out.extend_from_slice(&entry.value);
                out.extend_from_slice(b"\r\n");
            }
            need = true;
        }
        out
    }
}

/// Both caches are protected in the same order, values then document; no engine
/// call or file write runs with either lock held. These are shared free slots.
pub(super) fn values(e: &mut Engine, name: &[u8]) -> Option<Values> {
    let key = name.to_ascii_lowercase();
    if let Some(values) = e.ext_free::<Cache>().0.lock().unwrap().get(&key).cloned() {
        return Some(values);
    }
    let bytes = mod_files::read(e, &full_path(name))?;
    let doc = Document::parse(&bytes);
    let values = doc.values();
    let cache = e.ext_free::<Cache>();
    let mut all = cache.0.lock().unwrap();
    let mut docs = cache.1.lock().unwrap();
    all.entry(key.clone()).or_insert_with(|| values.clone());
    docs.entry(key).or_insert(doc);
    Some(values)
}
pub(super) fn clear(e: &mut Engine, name: &[u8]) -> bool {
    let cache = e.ext_free::<Cache>();
    let mut all = cache.0.lock().unwrap();
    let mut docs = cache.1.lock().unwrap();
    docs.remove(&name.to_ascii_lowercase());
    all.remove(&name.to_ascii_lowercase()).is_some()
}
fn split(value: &[u8]) -> Option<(&[u8], &[u8])> {
    let i = value
        .iter()
        .position(|&b| matches!(b, b':' | b'/' | b'\\'))?;
    (i + 1 < value.len()).then(|| (cstr(&value[..i]), cstr(&value[i + 1..])))
}
/// SimpleIni.h::SetDoubleValue uses %f, then removes only trailing zeroes
/// (an integral double retains its dot); its fixed buffer is 64 bytes.
fn float_text(value: f64) -> Vec<u8> {
    let mut out = if value.is_nan() {
        let mantissa = value.to_bits() & ((1 << 52) - 1);
        let word = if mantissa & (1 << 51) == 0 {
            "nan(snan)"
        } else if value.is_sign_negative() && mantissa == 1 << 51 {
            "nan(ind)"
        } else {
            "nan"
        };
        format!("{}{word}", if value.is_sign_negative() { "-" } else { "" })
    } else {
        format!("{value:.6}")
    };
    assert!(
        out.len() < 64,
        "ShowOff SetDoubleValue: %f exceeds its 64-byte C buffer"
    );
    while out.ends_with('0') {
        out.pop();
    }
    out.into_bytes()
}
fn set(e: &mut Engine, words: &[u32], integer: bool) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    with_args(e, &a, |e, ev, args| {
        if !(2..=4).contains(&args.len())
            || !ev.can_convert(e, &args[0], Ty::String)
            || (!ev.can_convert(e, &args[1], Ty::Number)
                && (integer || !ev.can_convert(e, &args[1], Ty::String)))
            || args
                .iter()
                .skip(2)
                .any(|arg| !ev.can_convert(e, arg, Ty::String))
        {
            return;
        }
        let section_key = ev.get_string(e, &args[0]);
        let Some((section, key)) = split(&section_key) else {
            return;
        };
        let value = if integer {
            nvse::args::f64_to_i32(ev.get_number(e, &args[1]))
                .to_string()
                .into_bytes()
        } else if ev.can_convert(e, &args[1], Ty::String) {
            ev.get_string(e, &args[1])
        } else {
            float_text(ev.get_number(e, &args[1]))
        };
        let given = args.get(2).map_or_else(Vec::new, |t| ev.get_string(e, t));
        let Some(name) = relative_name(e, a.script, given) else {
            return;
        };
        let note = args.get(3).map_or_else(Vec::new, |t| ev.get_string(e, t));
        // The source loads uncached files outside the mutex; the actual update
        // is performed against the current cached document while holding it.
        let cached = e
            .ext_free::<Cache>()
            .0
            .lock()
            .unwrap()
            .contains_key(&name.to_ascii_lowercase());
        let parsed = if cached {
            Document::default()
        } else {
            mod_files::read(e, &full_path(&name))
                .map(|b| Document::parse(&b))
                .unwrap_or_default()
        };
        let cache = e.ext_free::<Cache>();
        let mut all = cache.0.lock().unwrap();
        let mut docs = cache.1.lock().unwrap();
        let cache_key = name.to_ascii_lowercase();
        let initial = all.get(&cache_key).map_or(parsed, Document::from_values);
        let doc = docs.entry(cache_key.clone()).or_insert(initial);
        let result = doc.set(section, key, &value, &note);
        all.insert(cache_key, doc.values());
        drop(docs);
        drop(all);
        a.set_result(e, f64::from(result));
    });
    handled(true)
}
fn set_value(e: &mut Engine, words: &[u32]) -> Ret {
    set(e, words, false)
}
fn set_integer(e: &mut Engine, words: &[u32]) -> Ret {
    set(e, words, true)
}
fn get_string(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let mut result = Vec::new();
    with_args(e, &a, |e, ev, args| {
        if args.is_empty()
            || args.len() > 3
            || args.iter().any(|arg| !ev.can_convert(e, arg, Ty::String))
        {
            return;
        }
        result = args.get(2).map_or_else(Vec::new, |t| ev.get_string(e, t));
        let section_key = ev.get_string(e, &args[0]);
        let Some((section, key)) = split(&section_key) else {
            return;
        };
        let given = args.get(1).map_or_else(Vec::new, |t| ev.get_string(e, t));
        let Some(name) = relative_name(e, a.script, given) else {
            return;
        };
        if let Some(ini) = values(e, &name) {
            if let Some(value) = ini.get(&(section.to_ascii_lowercase(), key.to_ascii_lowercase()))
            {
                result = value.clone();
            }
        }
    });
    nvse::commands::string::assign_to_string_var(e, &a, &result);
    handled(true)
}
fn save(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let mut handled_ok = true;
    with_args(e, &a, |e, ev, args| {
        if args.len() > 1
            || args
                .first()
                .is_some_and(|t| !ev.can_convert(e, t, Ty::String))
        {
            return;
        }
        let given = args.first().map_or_else(Vec::new, |t| ev.get_string(e, t));
        let Some(name) = relative_name(e, a.script, given) else {
            handled_ok = false;
            return;
        };
        let doc = {
            let cache = e.ext_free::<Cache>();
            let all = cache.0.lock().unwrap();
            let docs = cache.1.lock().unwrap();
            docs.get(&name.to_ascii_lowercase()).cloned().or_else(|| {
                all.get(&name.to_ascii_lowercase())
                    .map(Document::from_values)
            })
        };
        if let Some(doc) = doc {
            let saved = mod_files::write(e, &full_path(&name), &doc.bytes());
            a.set_result(e, f64::from(u8::from(saved)));
        }
    });
    handled(handled_ok)
}
pub(super) fn install(e: &mut Engine) {
    table::grow_to(e, 0x3d3b);
    for (op, name, short, spec, returns, handler) in [
        (
            0x3d26,
            "SetINIValue_Cached",
            "SetINIValC",
            &[
                (0x8004, false),
                (0x8005, false),
                (0x8004, true),
                (0x8004, true),
            ][..],
            Returns::Default,
            set_value as AbiFn,
        ),
        (
            0x3d3a,
            "SetINIInteger_Cached",
            "SetINIIntC",
            &[
                (0x8004, false),
                (0x8001, false),
                (0x8004, true),
                (0x8004, true),
            ][..],
            Returns::Default,
            set_integer,
        ),
        (
            0x3d2a,
            "GetINIStringOrDefault_Cached",
            "GetINIString_Cached",
            &[(0x8004, false), (0x8004, true), (0x8004, true)][..],
            Returns::String,
            get_string,
        ),
        (
            0x3d2d,
            "SaveCachedIniFile",
            "SaveCachedIni",
            &[(0x8004, true)][..],
            Returns::Default,
            save,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(
                e,
                if kind == 0x8004 {
                    b"string"
                } else if kind == 0x8005 {
                    b"float or string"
                } else {
                    b"int"
                },
            );
            e.mem.set_u32(at, text);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
        }
        let execute = table::execute_address(op);
        e.register(execute, handler);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: spec.len() as u16,
                params,
                execute,
                eval: 0,
                flags: 0,
                expression: true,
                returns,
            },
        );
    }
}
#[cfg(test)]
pub(crate) mod tests;
