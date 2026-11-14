//! A stand-in game for the new compiler's tests: a few event blocks, commands, forms and enum
//! names, the errors printed collected.

use super::ast::{BlockInfo, CmdInfo, ParamInfo, Parse};
use super::Env;
use crate::compiler::{tt, vt, NVSE_ARRAY, NVSE_FORM, NVSE_NUMBER, NVSE_STRING};
use crate::newlang::passes::types::{FORM_GLOBAL, FORM_QUEST, FORM_REFR};

/// A script's variables: name and type.
pub type Vars = Vec<(Vec<u8>, u8)>;

/// A stand-in form.
#[derive(Clone, Debug)]
pub struct Form {
    pub id: u32,
    pub name: &'static [u8],
    pub ty: u8,
    /// A script form's type.
    pub script_type: Option<u16>,
    /// The variables of its script (a reference's base form's, a quest's), when it has one.
    pub script_vars: Option<Vec<(&'static [u8], u8, u16)>>,
    /// The parameter kinds it fits.
    pub fits: Vec<u32>,
}

pub struct TestEnv {
    pub errors: Vec<Vec<u8>>,
    pub blocks: Vec<(&'static [u8], BlockInfo)>,
    pub commands: Vec<CmdInfo>,
    pub forms: Vec<Form>,
    pub plugins: Vec<&'static [u8]>,
    pub enums: Vec<(u32, &'static [u8], u32, u32)>,
    pub script: Option<(u16, Vars)>,
    pub written_vars: Option<Vec<(Vec<u8>, u8, u32)>>,
}

fn p(name: &str, kind: u32, optional: bool) -> ParamInfo {
    ParamInfo {
        name: name.as_bytes().to_vec(),
        kind,
        optional,
    }
}

/// A command record.
pub fn cmd(name: &str, opcode: u16, params: Vec<ParamInfo>, parse: Parse, ret: u8) -> CmdInfo {
    CmdInfo {
        opcode,
        long_name: name.as_bytes().to_vec(),
        needs_parent: false,
        params,
        parse,
        return_type: ret,
        plugin: if opcode >= 0x2000 {
            Some((b"JIP LN NVSE".to_vec(), 57))
        } else if opcode >= 0x1400 {
            Some((b"NVSE".to_vec(), 6))
        } else {
            None
        },
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        use Parse::*;
        let opt_int = || BlockInfo {
            opcode: 0,
            params: vec![p("int", 1, true)],
        };
        TestEnv {
            errors: Vec::new(),
            blocks: vec![
                (
                    b"GameMode",
                    BlockInfo {
                        opcode: 0,
                        params: vec![],
                    },
                ),
                (
                    b"MenuMode",
                    BlockInfo {
                        opcode: 1,
                        params: vec![p("int", 1, true)],
                    },
                ),
                (
                    b"OnActivate",
                    BlockInfo {
                        opcode: 2,
                        params: vec![p("ref", 4, true)],
                    },
                ),
                (
                    b"OnAdd",
                    BlockInfo {
                        opcode: 3,
                        params: vec![p("ref", 4, false)],
                    },
                ),
                (b"ScriptEffectStart", opt_int()),
            ],
            commands: vec![
                // A vanilla command reading an actor value and a number.
                cmd(
                    "GetAV",
                    0x100e,
                    vec![p("actor value", 5, false), p("int", 1, true)],
                    Default,
                    tt::INVALID,
                ),
                cmd(
                    "GetPos",
                    0x1006,
                    vec![p("axis", 8, false)],
                    Default,
                    tt::INVALID,
                ),
                cmd(
                    "AddItem",
                    0x1002,
                    vec![p("item", 0x32, false), p("count", 1, false)],
                    Default,
                    tt::INVALID,
                ),
                cmd(
                    "MoveTo",
                    0x1003,
                    vec![p("object reference", 4, false)],
                    Default,
                    tt::INVALID,
                ),
                cmd("GetSelf", 0x10ce, vec![], Default, tt::FORM),
                cmd(
                    "Print",
                    0x1600,
                    vec![p("string", NVSE_STRING, false)],
                    Other,
                    tt::INVALID,
                ),
                cmd(
                    "ar_Size",
                    0x1561,
                    vec![p("array", NVSE_ARRAY, false)],
                    Other,
                    tt::NUMBER,
                ),
                cmd(
                    "ToNumber",
                    0x1601,
                    vec![p("string", NVSE_STRING, false)],
                    Other,
                    tt::NUMBER,
                ),
                cmd("JipThing", 0x2100, vec![], Default, tt::NUMBER),
                cmd("JipOther", 0x2101, vec![], Default, tt::NUMBER),
                cmd(
                    "Call",
                    0x1545,
                    vec![p("function", NVSE_FORM, false)]
                        .into_iter()
                        .chain((0..15).map(|_| p("arg", 0, true)))
                        .collect(),
                    Call,
                    tt::INVALID,
                ),
                cmd(
                    "CallAfterSeconds",
                    0x1602,
                    vec![
                        p("time", NVSE_NUMBER, false),
                        p("function", NVSE_FORM, false),
                    ]
                    .into_iter()
                    .chain((0..15).map(|_| p("arg", 0, true)))
                    .collect(),
                    Other,
                    tt::INVALID,
                ),
                cmd(
                    "SetFunctionValue",
                    0x1546,
                    vec![p("value", 0, false)],
                    Other,
                    tt::INVALID,
                ),
                CmdInfo {
                    needs_parent: true,
                    ..cmd("GetDisabled", 0x1007, vec![], Default, tt::INVALID)
                },
                cmd(
                    "ShowMessage",
                    0x1008,
                    vec![p("message", 0x31, false)],
                    Default,
                    tt::INVALID,
                ),
            ],
            forms: vec![
                Form {
                    id: 0x14,
                    name: b"PlayerRef",
                    ty: FORM_REFR,
                    script_type: None,
                    script_vars: None,
                    fits: vec![4, 6],
                },
                Form {
                    id: 0x100,
                    name: b"Caps001",
                    ty: 0x1f,
                    script_type: None,
                    script_vars: None,
                    fits: vec![0x32],
                },
                Form {
                    id: 0x101,
                    name: b"GameHour",
                    ty: FORM_GLOBAL,
                    script_type: None,
                    script_vars: None,
                    fits: vec![0x13],
                },
                Form {
                    id: 0x102,
                    name: b"MyQuest",
                    ty: FORM_QUEST,
                    script_type: None,
                    script_vars: Some(vec![(b"stage", vt::INTEGER, 3)]),
                    fits: vec![0x0e],
                },
                Form {
                    id: 0x103,
                    name: b"MyUDF",
                    ty: 0x11,
                    script_type: Some(0),
                    script_vars: None,
                    fits: vec![],
                },
                Form {
                    id: 0x104,
                    name: b"QuestScript",
                    ty: 0x11,
                    script_type: Some(1),
                    script_vars: None,
                    fits: vec![],
                },
                Form {
                    id: 0x105,
                    name: b"NoScriptQuest",
                    ty: FORM_QUEST,
                    script_type: None,
                    script_vars: None,
                    fits: vec![],
                },
                Form {
                    id: 0x106,
                    name: b"SomeWeapon",
                    ty: 0x28,
                    script_type: None,
                    script_vars: None,
                    fits: vec![],
                },
            ],
            plugins: vec![b"JIP LN NVSE"],
            enums: vec![(5, b"Health", 16, 2), (8, b"X", b'X' as u32, 1)],
            script: None,
            written_vars: None,
        }
    }
}

impl TestEnv {
    /// The errors printed, as text.
    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
            .into_iter()
            .map(|e| String::from_utf8_lossy(&e).into_owned())
            .collect()
    }

    fn form(&self, at: u32) -> Option<&Form> {
        self.forms.iter().find(|f| f.id == at)
    }
}

impl Env for TestEnv {
    fn error(&mut self, message: &[u8]) {
        self.errors.push(message.to_vec());
    }
    fn block(&mut self, name: &[u8]) -> Option<BlockInfo> {
        self.blocks
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, b)| b.clone())
    }
    fn plugin_listed(&mut self, name: &[u8]) -> bool {
        self.plugins.iter().any(|p| p.eq_ignore_ascii_case(name))
    }
    fn command(&mut self, name: &[u8]) -> Option<CmdInfo> {
        self.commands
            .iter()
            .find(|c| c.long_name.eq_ignore_ascii_case(name))
            .cloned()
    }
    fn form_by_name(&mut self, name: &[u8]) -> u32 {
        self.forms
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .map_or(0, |f| f.id)
    }
    fn form_by_id(&mut self, id: u32) -> u32 {
        self.form(id).map_or(0, |f| f.id)
    }
    fn form_type(&mut self, form: u32) -> u8 {
        self.form(form).map_or(0, |f| f.ty)
    }
    fn script_form_type(&mut self, form: u32) -> Option<u16> {
        self.form(form).and_then(|f| f.script_type)
    }
    fn form_script_variable(&mut self, form: u32, name: &[u8]) -> Option<Option<(u8, u16)>> {
        let vars = self.form(form)?.script_vars.as_ref()?;
        Some(
            vars.iter()
                .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
                .map(|&(_, t, i)| (t, i)),
        )
    }
    fn editor_id(&mut self, form: u32) -> Vec<u8> {
        self.form(form).map(|f| f.name.to_vec()).unwrap_or_default()
    }
    fn vanilla_enum(&mut self, kind: u32, text: &[u8]) -> (u32, u32) {
        self.enums
            .iter()
            .find(|(k, n, _, _)| *k == kind && n.eq_ignore_ascii_case(text))
            .map_or((u32::MAX, 2), |&(_, _, v, l)| (v, l))
    }
    fn form_matches_param(&mut self, form: u32, kind: u32) -> bool {
        self.form(form).is_some_and(|f| f.fits.contains(&kind))
    }
    fn script_variables(&mut self) -> Option<Vec<(Vec<u8>, u8)>> {
        self.script.as_ref().map(|(_, v)| v.clone())
    }
    fn set_script_variables(&mut self, vars: &[(Vec<u8>, u8, u32)]) {
        self.written_vars = Some(vars.to_vec());
    }
    fn script_type(&mut self) -> Option<u16> {
        self.script.as_ref().map(|(t, _)| *t)
    }
}
