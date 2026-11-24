//! StewieGetValue / StewVal, read from Tweaks 10.05's DLL (SHA256
//! 47b92393b48c3d90bd8c1e3c1224c55126aca90f766329b24b8959ec5d20c478).
//! Registration 10081a16..10081bdb: eighth command at 2139, ambiguous return (5).
//! Descriptor 100fb16c; execute 10062ff0..10063289; type jump table 1006328c.
use engine_core::prelude::*;
use nvse::{
    expr::{Eval, Tok, Ty},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};

pub fn install(e: &mut Engine) {
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"string");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 0x8004);
    e.mem.set_u32(params + 8, 0);
    table::grow_to(e, 0x213a);
    let execute = table::execute_address(0x2139);
    e.register(execute, get_value);
    table::add_plugin_record(
        e,
        0x2139,
        &PluginRecord {
            name: "StewieGetValue",
            short: "StewVal",
            needs_ref: false,
            param_count: 1,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: true,
            returns: Returns::Ambiguous,
        },
    );
}
fn value(e: &mut Engine, query: &[u8]) -> Option<Tok> {
    // snprintf's 512-byte destination. The type selector is case-insensitive.
    let query = &query[..query.len().min(511)];
    let query = &query[..query.iter().position(|&b| b == 0).unwrap_or(query.len())];
    let split = query
        .iter()
        .position(|&b| matches!(b, b':' | b'/' | b'\\'))?;
    let (key, section) = (&query[..split], &query[split + 1..]);
    let key = std::str::from_utf8(key).ok()?;
    let section = std::str::from_utf8(section).ok()?;
    let state = e.ext::<crate::Stewie>();
    match key.as_bytes().first()?.to_ascii_lowercase() {
        b'b' | b'i' | b'u' => {
            let raw = state.settings.raw(section, key);
            let n = crate::ini::long_value(raw.as_deref(), -19997);
            (n != -19997).then(|| Tok::number(f64::from(n)))
        }
        b'f' => {
            let raw = state.settings.raw(section, key)?;
            if raw.is_empty() || raw.len() >= 64 {
                return None;
            }
            let (n, used, _) = nvse::cnum::strtod(&raw);
            // The DLL's SimpleIni double reader (1005f7d0) refuses trailing bytes.
            (used == raw.len() && n != -19997.).then(|| Tok::number(n))
        }
        b's' => state.settings.raw(section, key).map(Tok::Str),
        _ => None,
    }
}
fn get_value(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.);
    let mut ev = Eval::new(e, &a);
    let mut found = false;
    nvse::expr::with_evaluator(e, |e| {
        if let Some(args) = ev.extract_args(e) {
            if args.len() == 1 && ev.can_convert(e, &args[0], Ty::String) {
                let query = ev.get_string(e, &args[0]);
                if let Some(result) = value(e, &query) {
                    ev.assign_result(e, &result);
                    found = true;
                }
            }
        }
    });
    nvse::expr::report(e, &ev);
    if !found && nvse::commands::is_console_mode(e) {
        nvse::commands::console_print(e, "Failed to find tweaks setting.", &[]);
    }
    handled(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_obeys_type_prefix_separator_sentinel_and_effective_settings() {
        let mut e = Engine::new();
        e.ext::<crate::Stewie>().settings=crate::settings::Settings::new(&[("test.ini",b"[Tweaks]\niMaxCharacterLevel=42\nbRobotCompanionsHealWithScrapMetal=1\nfTest=1.25\nuTest=0xffffffff\nsTest=hello\nfBad=2x\niMissing=-19997\n")],true);
        assert_eq!(
            value(&mut e, b"iMaxCharacterLevel:Tweaks"),
            Some(Tok::number(42.))
        );
        assert_eq!(
            value(&mut e, b"bRobotCompanionsHealWithScrapMetal/Tweaks"),
            Some(Tok::number(1.))
        );
        assert_eq!(value(&mut e, b"fTest\\Tweaks"), Some(Tok::number(1.25)));
        assert_eq!(
            value(&mut e, b"sTest:Tweaks"),
            Some(Tok::Str(b"hello".to_vec()))
        );
        for q in [
            b"fBad:Tweaks".as_slice(),
            b"iMissing:Tweaks",
            b"qTest:Tweaks",
            b"no_separator",
        ] {
            assert!(value(&mut e, q).is_none())
        }
    }
    #[test]
    fn compiled_expression_calls_register_alias_and_preserve_string_results() {
        const DATA: u32 = 0x03000000;
        const RESULT: u32 = DATA + 0x1000;
        const OFFSET: u32 = RESULT + 8;
        let mut e = Engine::new();
        e.map(0x0118e000, 0x9000);
        e.map(DATA, 0x10000);
        nvse::table::install(&mut e);
        install(&mut e);
        e.ext::<crate::Stewie>().settings = crate::settings::Settings::new(
            &[(
                "test.ini",
                b"[Tweaks]\niMaxCharacterLevel=42\nsLabel=hello\n",
            )],
            true,
        );
        assert_eq!(table::opcode_by_name(&e, b"StewVal"), Some(0x2139));
        assert_eq!(table::return_type(&e, 0x2139), Returns::Ambiguous);
        for (query, expected) in [
            ("iMaxCharacterLevel:Tweaks", Some(42.)),
            ("sLabel:Tweaks", None),
        ] {
            let token = [
                vec![b'S'],
                (query.len() as u16).to_le_bytes().to_vec(),
                query.as_bytes().to_vec(),
            ]
            .concat();
            let body = [
                vec![1],
                ((token.len() + 2) as u16).to_le_bytes().to_vec(),
                token,
            ]
            .concat();
            e.mem.set_u16(DATA, 0x2139);
            e.mem.set_u16(DATA + 2, body.len() as u16);
            e.mem.write(DATA + 4, &body);
            e.mem.set_u32(OFFSET, 4);
            let rec = table::record_of(&e, 0x2139);
            let words = [e.mem.u32(rec + 0x14), DATA, 0, 0, 0, 0, RESULT, OFFSET];
            let returned = nvse::expr::with_evaluator(&mut e, |e| {
                assert_eq!(e.call(e.mem.u32(rec + 0x18), &words).eax & 0xff, 1);
                nvse::expr::expected_return_type(e)
            });
            let result = e.mem.f64(RESULT);
            if let Some(n) = expected {
                assert_eq!(result, n)
            } else {
                assert_eq!(
                    nvse::vars::string(&e, result as u32),
                    Some(b"hello".to_vec())
                );
                assert_eq!(returned, Some(Returns::String));
            }
        }
    }
}
