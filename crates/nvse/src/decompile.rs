//! xNVSE's script decompiler (`ScriptAnalyzer.cpp`, `ScriptAnalyzer.h` (xNVSE)): compiled script
//! data read back as script text, line by line, for `DecompileScript`, `HasScriptCommand` and the
//! plugin API; and the expression evaluator's line text (`ExpressionEvaluator::GetLineText`,
//! `GetVariablesText`, `ScriptUtils.cpp` (xNVSE)), which `Assert` prints and which decompiles a
//! lambda through the analyzer.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). Texts are bytes, as xNVSE's
//! `std::string`s are. Where xNVSE would read through a null pointer (a token that failed to read
//! asked for its text, a parameter type past its tables), the port answers the text named at that
//! place instead of stopping the game.
//!
//! The analyzer reads the exe's own tables: the statement records (`Begin`, `If`, ...,
//! `0x0118CB50`), the event block records (`0x0118E2F0`), the compiler's operator table
//! (`0x0118CAD0`), the actor value infos (`0x011D61C8`), the animation groups (`0x011977D8`), the
//! miscellaneous statistics' names (`0x01189280`) and the critical stages' (`0x0119BBB0`).

use crate::args::{event_list_variable, script_ref, CLASSIFY};
use crate::expr::{self, Eval, Tok, Ty};
use crate::functions::script_data;
use crate::table::{guest_text, record_of};
use engine_core::prelude::*;

type B = Vec<u8>;

/// `g_scriptStatementCommandInfos`: 16 records for codes `0x10..=0x1F`.
const STATEMENTS: u32 = 0x0118_cb50;
/// `g_eventBlockCommandInfos`: 38 records.
const EVENT_BLOCKS: u32 = 0x0118_e2f0;
const EVENT_BLOCK_COUNT: u32 = 38;
/// `g_gameScriptOperators`: 16 `ScriptOperator`s of 8 bytes (code, precedence `+4`, text `+5`).
const GAME_OPERATORS: u32 = 0x0118_cad0;
/// `g_actorValues`: `ActorValueInfo *` by code; `infoName` at `+0x38`.
const ACTOR_VALUES: u32 = 0x011d_61c8;
/// `g_animGroups`: 245 `AnimGroupInfo`s of `0x24` bytes, the name first.
const ANIM_GROUPS: u32 = 0x0119_77d8;
const ANIM_GROUP_COUNT: i32 = 245;
const MISC_STATS: u32 = 0x0118_9280;
const CRITICAL_STAGES: u32 = 0x0119_bbb0;
/// The game's parse routines (`g_gameParseCommands`; xNVSE's `Cmd_Default_Parse` is the port's
/// [`crate::table::DEFAULT_PARSE`]) and the message box ones among them.
const GAME_PARSERS: [u32; 5] = [
    0x005b_1ba0,
    0x005b_3c70,
    0x005b_3ca0,
    0x005b_3c40,
    0x005b_3cd0,
];
const MESSAGE_BOX_PARSERS: [u32; 4] = [0x005b_3cd0, 0x005b_3c40, 0x005b_3c70, 0x005b_3ca0];
const LOOKUP_FORM: u32 = 0x0048_39c0;
const FORM_HEAP_FREE: u32 = 0x0040_1030;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_GLOBAL: u32 = 0x0118_3ad4;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_TES_QUEST: u32 = 0x0118_6500;
/// `TESForm::GetName` (the editor id the game keeps), virtual `+0x130`.
const GET_NAME: u32 = 0x130;

/// Statement codes (`ScriptStatementCode`).
pub mod code {
    pub const BEGIN: u32 = 0x10;
    pub const END: u32 = 0x11;
    pub const SET_TO: u32 = 0x15;
    pub const IF: u32 = 0x16;
    pub const ELSE: u32 = 0x17;
    pub const ELSE_IF: u32 = 0x18;
    pub const END_IF: u32 = 0x19;
    pub const REFERENCE_FUNCTION: u32 = 0x1c;
    pub const SCRIPT_NAME: u32 = 0x1d;
    pub const MAX: u32 = 0x1f;
}

/// How deep lambdas inside lambdas are decompiled (on this thread), and the limit.
const MAX_LAMBDA_DEPTH: u32 = 16;
thread_local! {
    static LAMBDA_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// xNVSE's commands the decompiler names.
const OP_WHILE: u32 = 0x153b;
const OP_LOOP: u32 = 0x153c;
const OP_FOR_EACH: u32 = 0x153d;
const OP_FOR_EACH_ALT: u32 = 0x1670;
const OP_PUSH_CONTEXT: u32 = 0x154d;
const OP_POP_CONTEXT: u32 = 0x154e;

/// What the root analyzer (`GetRootAnalyzer`, the first of `g_analyzerStack`) keeps: the
/// variables seen holding arrays and strings, and whether a line used the compiler override.
/// `active` is false while no analyzer runs (the evaluator's line text outside a decompile).
#[derive(Default)]
pub struct Root {
    active: bool,
    arrays: Vec<u32>,
    strings: Vec<u32>,
    compiler_override: bool,
}

impl Root {
    fn none() -> Root {
        Root::default()
    }
    fn analyzer() -> Root {
        Root {
            active: true,
            ..Root::default()
        }
    }
    /// `RegisterNVSEVar`.
    fn register(&mut self, info: u32, var_type: u8) {
        if info == 0 || !self.active {
            return;
        }
        let set = match var_type {
            3 => &mut self.arrays,
            2 => &mut self.strings,
            _ => return,
        };
        if !set.contains(&info) {
            set.push(info);
        }
    }
}

// ---- the game's structures ----

fn cast(e: &mut Engine, object: u32, to: u32) -> u32 {
    if object == 0 {
        return 0;
    }
    e.call(DYNAMIC_CAST, &args![object, 0u32, RTTI_TES_FORM, to, 0u32])
        .u32()
}

fn lookup_form(e: &mut Engine, id: u32) -> u32 {
    e.call(LOOKUP_FORM, &args![id]).u32()
}

/// `String::CStr`: the text, empty without data or length.
fn string_at(e: &Engine, s: u32) -> B {
    let data = e.mem.u32(s);
    if data == 0 || e.mem.u16(s + 4) == 0 {
        return B::new();
    }
    guest_text(e, data)
}

/// `TESForm::GetName`, the `+0x130` virtual function; empty for none.
fn form_name(e: &mut Engine, form: u32) -> B {
    let at = e.vcall(form, GET_NAME, &args![]).u32();
    if at == 0 {
        B::new()
    } else {
        guest_text(e, at)
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.cpp: TESForm::GetEditorID.
/// The names the game does not keep for a few of its own forms, else `GetName`.
fn editor_id(e: &mut Engine, form: u32) -> B {
    let name: &[u8] = match e.mem.u32(form + 0xc) {
        0x7 => b"Player",
        0x14 => b"PlayerRef",
        0xa => b"LockPick",
        0x2d => b"MaleAdult01Default",
        0x2e => b"FemaleAdult01Default",
        0x33 => b"RadiationMarker",
        0x3d => b"DefaultCombatStyle",
        0x147 => b"PipBoyLight",
        0x163 => b"HelpManual",
        0x1f5 => b"DefaultWaterExplosion",
        0x1f6 => b"GasTrapDummy",
        _ => return form_name(e, form),
    };
    name.to_vec()
}

/// `Script::GetVariableInfo`: the `VariableInfo` of local `idx` (`varList`, `+0x4C`); 0 for none.
fn variable_info(e: &Engine, script: u32, idx: u32) -> u32 {
    if script == 0 {
        return 0;
    }
    let mut node = script + 0x4c;
    while node != 0 {
        let info = e.mem.u32(node);
        if info != 0 && e.mem.u32(info) == idx {
            return info;
        }
        node = e.mem.u32(node + 4);
    }
    0
}

/// `VariableInfo::IsReferenceType`: a reference of the script's list names this local.
fn is_reference_type(e: &Engine, info: u32, script: u32) -> bool {
    let idx = e.mem.u32(info);
    let mut node = script + 0x44;
    while node != 0 {
        let r = e.mem.u32(node);
        if r != 0 && e.mem.u32(r + 0xc) == idx {
            return true;
        }
        node = e.mem.u32(node + 4);
    }
    false
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp:
// Script::RefVariable::GetReferencedScript.
/// A quest's script, or a reference's script; 0 for neither.
fn referenced_script(e: &mut Engine, ref_var: u32) -> u32 {
    let form = e.mem.u32(ref_var + 8);
    if form == 0 {
        return 0;
    }
    if e.mem.u8(form + 4) == 0x47 {
        return e.mem.u32(form + 0x1c);
    }
    let refr = cast(e, form, RTTI_TES_OBJECT_REFR);
    if refr != 0 {
        return extra_script(e, refr);
    }
    0
}

/// `TESObjectREFR::GetExtraScript`'s script (`ExtraScript +0xC`); 0 for none.
fn extra_script(e: &mut Engine, refr: u32) -> u32 {
    const EXTRA_GET_BY_TYPE: u32 = 0x0041_0220;
    let extra = e
        .call(EXTRA_GET_BY_TYPE, &args![refr + 0x44, 0x0du32])
        .u32();
    if extra == 0 {
        0
    } else {
        e.mem.u32(extra + 0xc)
    }
}

/// A command record's parameter `i` (12 bytes each, read as xNVSE reads it, past the count too).
fn param_type(e: &Engine, record: u32, i: usize) -> u32 {
    e.mem.u32(e.mem.u32(record + 0x14) + 12 * i as u32 + 4)
}

fn has_optional_param(e: &Engine, record: u32) -> bool {
    let params = e.mem.u32(record + 0x14);
    (0..u32::from(e.mem.u16(record + 0x12))).any(|i| e.mem.u32(params + 12 * i + 8) != 0)
}

fn long_name(e: &Engine, record: u32) -> B {
    guest_text(e, e.mem.u32(record))
}

fn unk_opcode(opcode: u32) -> B {
    format!("UNK_OPCODE_{opcode:04X}").into_bytes()
}

/// `TokenListToString`: each text after a space.
fn token_list(parts: impl IntoIterator<Item = B>) -> B {
    let mut out = B::new();
    for p in parts {
        out.push(b' ');
        out.extend(p);
    }
    out
}

/// `UnformatString`: a new line as `%r`, a quote as `%q`.
fn unformat(s: &[u8]) -> B {
    let mut out = B::new();
    for &b in s {
        match b {
            b'\n' => out.extend_from_slice(b"%r"),
            b'"' => out.extend_from_slice(b"%q"),
            _ => out.push(b),
        }
    }
    out
}

fn quoted(s: &[u8]) -> B {
    let mut out = vec![b'"'];
    out.extend_from_slice(s);
    out.push(b'"');
    out
}

fn paren(s: &[u8]) -> B {
    let mut out = vec![b'('];
    out.extend_from_slice(s);
    out.push(b')');
    out
}

/// `FormatString("%g", v)`.
fn g(e: &Engine, v: f64) -> B {
    crate::commands::printf::ucrt_format(e, b"%g", &v.to_le_bytes())
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp: DoubleToString.
/// `%.10f` with its trailing zeros and then its point taken off.
fn double_to_string(e: &Engine, v: f64) -> B {
    let mut s = crate::commands::printf::ucrt_format(e, b"%.10f", &v.to_le_bytes());
    while s.last() == Some(&b'0') {
        s.pop();
    }
    if s.last() == Some(&b'.') {
        s.pop();
    }
    s
}

fn is_space(c: u8) -> bool {
    c == b' ' || (0x09..=0x0d).contains(&c)
}

// ---- the iterator ----

/// `ScriptIterator`.
#[derive(Clone, Copy, Debug)]
struct Iter {
    opcode: u32,
    length: u16,
    ref_idx: u16,
    script: u32,
    cur: u32,
    start: u32,
    /// No read goes past here: the script data's end, or a line's (xNVSE has no bound; a
    /// corrupt script fails its line instead of reading on).
    limit: u32,
    /// A read was refused at [`Iter::limit`].
    bad: bool,
}

impl Iter {
    fn new(e: &Engine, script: u32) -> Iter {
        let data = script_data(e, script);
        let mut it = Iter {
            opcode: 0,
            length: 0,
            ref_idx: 0,
            script,
            cur: data,
            start: data,
            limit: data.wrapping_add(e.mem.u32(script + 0x20)),
            bad: false,
        };
        it.read_line(e);
        it
    }
    /// How many bytes lie before [`Iter::limit`] (a look ahead, not a read).
    fn left(&self) -> u32 {
        self.limit.saturating_sub(self.cur)
    }
    /// Whether `n` more bytes lie before [`Iter::limit`]; when not, the read is refused.
    fn room(&mut self, n: u32) -> bool {
        let ok = u64::from(self.cur) + u64::from(n) <= u64::from(self.limit);
        if !ok {
            self.bad = true;
        }
        ok
    }
    fn r8(&mut self, e: &Engine) -> u8 {
        if !self.room(1) {
            return 0;
        }
        let v = e.mem.u8(self.cur);
        self.cur = self.cur.wrapping_add(1);
        v
    }
    fn r16(&mut self, e: &Engine) -> u16 {
        if !self.room(2) {
            return 0;
        }
        let v = e.mem.u16(self.cur);
        self.cur = self.cur.wrapping_add(2);
        v
    }
    fn r32(&mut self, e: &Engine) -> u32 {
        if !self.room(4) {
            return 0;
        }
        let v = e.mem.u32(self.cur);
        self.cur = self.cur.wrapping_add(4);
        v
    }
    fn rf64(&mut self, e: &Engine) -> f64 {
        if !self.room(8) {
            return 0.0;
        }
        let v = e.mem.f64(self.cur);
        self.cur = self.cur.wrapping_add(8);
        v
    }
    /// `ReadLine`: a reference call (`0x1C`) reads its reference first.
    fn read_line(&mut self, e: &Engine) {
        if script_data(e, self.script) == 0 {
            return;
        }
        // xNVSE reads the header past the data's end at the last line and then stops (`End`);
        // here a header that does not fit before the end stops the iterator without the read.
        let header = if self.cur <= self.limit
            && self.limit - self.cur >= 2
            && e.mem.u16(self.cur) == code::REFERENCE_FUNCTION as u16
        {
            8
        } else {
            4
        };
        if u64::from(self.cur) + header > u64::from(self.limit) {
            self.cur = self.limit.wrapping_add(1);
            return;
        }
        self.opcode = u32::from(self.r16(e));
        if self.opcode == code::REFERENCE_FUNCTION {
            self.ref_idx = self.r16(e);
            self.opcode = u32::from(self.r16(e));
            self.length = self.r16(e);
        } else {
            self.ref_idx = 0;
            self.length = self.r16(e);
        }
        self.start = self.cur;
    }
    fn advance(&mut self, e: &Engine) {
        self.cur = self.cur.wrapping_add(u32::from(self.length));
        self.read_line(e);
    }
    fn end(&self, e: &Engine) -> bool {
        let data = script_data(e, self.script);
        self.cur > data.wrapping_add(e.mem.u32(self.script + 0x20))
    }
    fn ref_var(&mut self, e: &Engine) -> u32 {
        let idx = self.r16(e);
        script_ref(e, self.script, u32::from(idx))
    }
    fn calling_ref(&self, e: &Engine) -> u32 {
        if self.ref_idx == 0 {
            0
        } else {
            script_ref(e, self.script, u32::from(self.ref_idx))
        }
    }
    /// `ReadVariable`: `r` and a reference of the list first for another script's local; the
    /// type byte; the local's info for `f` and `s`. 0 when it does not resolve.
    fn read_variable(&mut self, e: &mut Engine, form_out: &mut u32, type_out: &mut u8) -> u32 {
        let mut script = self.script;
        if !self.room(1) {
            return 0;
        }
        if e.mem.u8(self.cur) == b'r' {
            self.cur = self.cur.wrapping_add(1);
            let ref_var = self.ref_var(e);
            if ref_var == 0 {
                return 0;
            }
            let form = e.mem.u32(ref_var + 8);
            if form == 0 {
                return 0;
            }
            script = referenced_script(e, ref_var);
            if script == 0 {
                return 0;
            }
            *form_out = form;
        }
        let t = self.r8(e);
        *type_out = t;
        if t == b'f' || t == b's' {
            let idx = self.r16(e);
            return variable_info(e, script, u32::from(idx));
        }
        0
    }
    /// `ReadStringLiteral`: its length word, at most `0x200`.
    fn read_string(&mut self, e: &Engine) -> Option<B> {
        let len = self.r16(e);
        if len > 0x200 || !self.room(u32::from(len)) {
            return None;
        }
        let s = e.mem.bytes(self.cur, u32::from(len));
        self.cur = self.cur.wrapping_add(u32::from(len));
        Some(s)
    }
    /// `ReadNumericConstant`: `strtod` over what is left of the expression (at most `0x200`).
    fn read_number(&mut self, e: &Engine, end: u32) -> Option<f64> {
        let len = end.wrapping_sub(self.cur);
        if len > 0x200 {
            return None;
        }
        let mut buf = e.mem.bytes(self.cur, len);
        if let Some(z) = buf.iter().position(|&b| b == 0) {
            buf.truncate(z);
        }
        let (v, used) = strtod(&buf);
        if used == 0 {
            return None;
        }
        self.cur = self.cur.wrapping_add(used as u32);
        Some(v)
    }
}

/// The C runtime's `strtod`: its value and how many bytes it took (0: no number).
pub(crate) fn strtod(s: &[u8]) -> (f64, usize) {
    let mut i = 0;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let start = i;
    let mut j = i;
    if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
        j += 1;
    }
    let neg = j > start && s[start] == b'-';
    let rest = &s[j..];
    let lower: Vec<u8> = rest.iter().take(8).map(u8::to_ascii_lowercase).collect();
    if lower.starts_with(b"infinity") || lower.starts_with(b"inf") {
        let n = if lower.starts_with(b"infinity") { 8 } else { 3 };
        let v = if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        return (v, j + n);
    }
    if lower.starts_with(b"nan") {
        return (f64::NAN, j + 3);
    }
    if lower.starts_with(b"0x") {
        let mut k = j + 2;
        let mut v = 0f64;
        let mut digits = 0;
        while k < s.len() && s[k].is_ascii_hexdigit() {
            v = v * 16.0 + f64::from((s[k] as char).to_digit(16).unwrap_or(0));
            k += 1;
            digits += 1;
        }
        if digits > 0 {
            return (if neg { -v } else { v }, k);
        }
        // "0x" with no digit: the "0" alone.
        return (if neg { -0.0 } else { 0.0 }, j + 1);
    }
    let mut k = j;
    let mut digits = 0;
    while k < s.len() && s[k].is_ascii_digit() {
        k += 1;
        digits += 1;
    }
    if k < s.len() && s[k] == b'.' {
        k += 1;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return (0.0, 0);
    }
    if k < s.len() && (s[k] == b'e' || s[k] == b'E') {
        let mut m = k + 1;
        if m < s.len() && (s[m] == b'+' || s[m] == b'-') {
            m += 1;
        }
        if m < s.len() && s[m].is_ascii_digit() {
            while m < s.len() && s[m].is_ascii_digit() {
                m += 1;
            }
            k = m;
        }
    }
    let text = String::from_utf8_lossy(&s[start..k]).into_owned();
    let text = text
        .strip_suffix('.')
        .map_or(text.clone(), |t| format!("{t}.0"));
    (text.parse::<f64>().unwrap_or(0.0), k)
}

// ---- tokens ----

/// An operand (`OperandToken` and its kinds).
#[derive(Clone, Debug)]
enum Operand {
    /// `ScriptVariableToken`: the script, the local's info and the form of `form.local`.
    Var {
        script: u32,
        info: u32,
        form: u32,
    },
    Str(B),
    Num(f64),
    /// `RefToken`: an entry of the script's reference list (0: none).
    Ref {
        script: u32,
        ref_var: u32,
    },
    /// `GlobalVariableToken`.
    Global {
        ref_var: u32,
        global: u32,
    },
    /// `InlineExpressionToken`: the evaluator's tokens (none when they did not read).
    Inline {
        script: u32,
        tokens: Option<Vec<Tok>>,
    },
    Call(Box<Call>),
}

impl Operand {
    fn error(&self) -> bool {
        match self {
            Operand::Var { script, info, .. } => *info == 0 || *script == 0,
            Operand::Ref { ref_var, .. } => *ref_var == 0,
            Operand::Global { ref_var, global } => *ref_var == 0 || *global == 0,
            Operand::Inline { tokens, .. } => tokens.is_none(),
            Operand::Call(c) => c.error,
            Operand::Str(_) | Operand::Num(_) => false,
        }
    }

    fn global(e: &mut Engine, ref_var: u32) -> Operand {
        let form = if ref_var == 0 {
            0
        } else {
            e.mem.u32(ref_var + 8)
        };
        Operand::Global {
            ref_var,
            global: cast(e, form, RTTI_TES_GLOBAL),
        }
    }

    fn text(&self, e: &mut Engine, root: &mut Root) -> B {
        match self {
            Operand::Var { script, info, form } => var_text(e, root, *script, *info, *form),
            Operand::Str(s) => quoted(&unformat(s)),
            Operand::Num(v) => double_to_string(e, *v),
            Operand::Ref { script, ref_var } => ref_text(e, root, *script, *ref_var),
            Operand::Global { global, .. } => {
                if *global == 0 {
                    b"<failed to resolve ref list entry>".to_vec()
                } else {
                    string_at(e, global + 0x18)
                }
            }
            Operand::Inline { script, tokens } => match tokens {
                Some(t) => line_text(e, root, *script, t, true),
                None => b"<failed to decompile inline expression>".to_vec(),
            },
            Operand::Call(c) => c.text(e, root),
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
// ScriptVariableToken::ToString, GetBackupName.
/// A local's name (`form.name` for another script's), or its stand-in name when it has none.
fn var_text(e: &mut Engine, root: &Root, script: u32, info: u32, form: u32) -> B {
    if info == 0 {
        return b"<failed to get variable>".to_vec();
    }
    let mut name = string_at(e, info + 0x18);
    if name.is_empty() {
        let idx = e.mem.u32(info);
        let prefix: &str = if root.active && root.arrays.contains(&info) {
            "a"
        } else if root.active && root.strings.contains(&info) {
            "s"
        } else if idx == 0 {
            // xNVSE compares the local's id with `eVarType_Float` here.
            if is_reference_type(e, info, script) {
                "r"
            } else {
                "f"
            }
        } else {
            "i"
        };
        name = format!("{prefix}Var{idx}").into_bytes();
    }
    if form != 0 {
        let mut out = form_name(e, form);
        out.push(b'.');
        out.extend(name);
        return out;
    }
    name
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp: RefToken::ToString.
/// A reference of the list: the local it reads, or the form's editor id (`<id>` without one).
fn ref_text(e: &mut Engine, root: &Root, script: u32, ref_var: u32) -> B {
    if ref_var == 0 {
        return b"<failed to resolve ref list entry>".to_vec();
    }
    let var_idx = e.mem.u32(ref_var + 0xc);
    if var_idx != 0 && script != 0 {
        let info = variable_info(e, script, var_idx);
        if info == 0 {
            return format!("<failed to get var info index {var_idx}>").into_bytes();
        }
        return var_text(e, root, script, info, 0);
    }
    let form = e.mem.u32(ref_var + 8);
    if form != 0 {
        let id = editor_id(e, form);
        if id.is_empty() {
            return format!("<{:X}>", e.mem.u32(form + 0xc)).into_bytes();
        }
        return id;
    }
    b"<failed to resolve ref list entry>".to_vec()
}

/// `CommandCallToken`.
#[derive(Clone, Debug)]
struct Call {
    record: u32,
    opcode: u32,
    calling: Option<(u32, u32)>,
    args: Vec<Operand>,
    /// The evaluator's argument lists, for an expression-format command.
    expr: Option<(u32, Vec<Vec<Tok>>)>,
    error: bool,
}

/// `kExtractParam_*`.
mod extract {
    pub const STRING_LITERAL: u8 = 0;
    pub const INT: u8 = 1;
    pub const SHORT: u8 = 2;
    pub const BYTE: u8 = 3;
    pub const FLOAT: u8 = 4;
    pub const DOUBLE: u8 = 5;
    pub const FORM: u8 = 6;
    pub const SCRIPT_VARIABLE: u8 = 7;
}

impl Call {
    fn new(record: u32, opcode: u32, calling_ref: u32, script: u32) -> Call {
        Call {
            record,
            opcode,
            calling: (calling_ref != 0).then_some((script, calling_ref)),
            args: Vec::new(),
            expr: None,
            error: record == 0,
        }
    }

    fn read_variable_token(&mut self, e: &mut Engine, it: &mut Iter) -> bool {
        let mut form = 0;
        let mut t = 0;
        let info = it.read_variable(e, &mut form, &mut t);
        if info == 0 {
            return false;
        }
        self.args.push(Operand::Var {
            script: it.script,
            info,
            form,
        });
        true
    }

    fn read_form_token(&mut self, e: &mut Engine, it: &mut Iter) -> bool {
        if it.r8(e) == b'r' {
            let ref_var = it.ref_var(e);
            self.args.push(Operand::Ref {
                script: it.script,
                ref_var,
            });
            true
        } else {
            self.read_variable_token(e, it)
        }
    }

    fn read_numeric_token(&mut self, e: &mut Engine, it: &mut Iter) -> bool {
        match it.r8(e) {
            b'n' => {
                let v = it.r32(e) as i32;
                self.args.push(Operand::Num(f64::from(v)));
            }
            b'z' => {
                let v = it.rf64(e);
                self.args.push(Operand::Num(v));
            }
            b'G' => {
                let r = it.ref_var(e);
                self.args.push(Operand::global(e, r));
            }
            _ => {
                it.cur = it.cur.wrapping_sub(1);
                return self.read_variable_token(e, it);
            }
        }
        true
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
    // CommandCallToken::ParseGameArgs.
    fn parse_game_args(&mut self, e: &mut Engine, root: &mut Root, it: &mut Iter, n: u16) -> bool {
        if e.mem.u32(self.record + 0x14) == 0 {
            return true;
        }
        // More arguments than the command has parameters: a corrupt line (xNVSE reads on past
        // the parameter list).
        if n > e.mem.u16(self.record + 0x12) {
            return false;
        }
        for i in 0..usize::from(n) {
            if it.bad {
                return false;
            }
            let ty = param_type(e, self.record, i);
            let class = CLASSIFY.get(ty as usize).copied().unwrap_or(8);
            if it.left() >= 2 && e.mem.u16(it.cur) == 0xffff {
                it.cur = it.cur.wrapping_add(2);
                let (tokens, end) = read_tokens(e, it.script, it.cur, it.limit);
                it.cur = end;
                self.args.push(Operand::Inline {
                    script: it.script,
                    tokens,
                });
                continue;
            }
            match class {
                extract::STRING_LITERAL => {
                    if let Some(s) = it.read_string(e) {
                        self.args.push(Operand::Str(s));
                    }
                }
                extract::INT | extract::FLOAT | extract::DOUBLE => {
                    self.read_numeric_token(e, it);
                }
                extract::SHORT => {
                    let v = it.r16(e);
                    self.args.push(Operand::Num(f64::from(v)));
                }
                extract::BYTE => {
                    let v = it.r8(e);
                    self.args.push(Operand::Num(f64::from(v)));
                }
                extract::FORM | extract::SCRIPT_VARIABLE => {
                    let read = if class == extract::FORM {
                        self.read_form_token(e, it)
                    } else {
                        self.read_variable_token(e, it)
                    };
                    if !read {
                        return false;
                    }
                }
                _ => {}
            }
            // xNVSE tests the last argument read (with none yet it would read before the list).
            match self.args.last() {
                Some(a) if !a.error() => {}
                _ => return false,
            }
        }
        let _ = root;
        true
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
    // CommandCallToken::ParseCommandArgs.
    fn parse_args(&mut self, e: &mut Engine, root: &mut Root, mut it: Iter, len: u32) -> bool {
        if self.record == 0 {
            return false;
        }
        let execute = e.mem.u32(self.record + 0x18);
        let function = record_of(e, crate::functions::OP_FUNCTION);
        if function != 0 && execute == e.mem.u32(function + 0x18) {
            let info = crate::functions::read_info(e, it.script);
            if info.event_list != 0 {
                crate::functions::delete_event_list(e, info.event_list);
            }
            for p in &info.params {
                let var = variable_info(e, it.script, u32::from(p.idx));
                root.register(var, p.var_type);
                let token = Operand::Var {
                    script: it.script,
                    info: var,
                    form: 0,
                };
                let bad = token.error();
                self.args.push(token);
                if bad {
                    return false;
                }
            }
            return true;
        }
        let parse = e.mem.u32(self.record + 0x1c);
        let default = GAME_PARSERS.contains(&parse) || parse == crate::table::DEFAULT_PARSE;
        let mut over = false;
        if default && it.left() >= 2 && e.mem.u16(it.cur) > 0x7fff {
            over = true;
            it.cur = it.cur.wrapping_add(2);
            if root.active {
                root.compiler_override = true;
            }
        }
        if !default || over {
            if parse == crate::table::BEGIN_LOOP_PARSE {
                if !it.room(4) {
                    return false;
                }
                it.cur = it.cur.wrapping_add(4);
            }
            let mut at = it.cur;
            let mut lists = Vec::new();
            if parse == crate::table::CALL_PARSE {
                // The caller's version byte.
                at = at.wrapping_add(1);
                let (tokens, end) = read_tokens(e, it.script, at, it.limit);
                let Some(t) = tokens else {
                    return false;
                };
                lists.push(t);
                at = end;
            }
            if at >= it.limit {
                return false;
            }
            let n = e.mem.u8(at);
            at = at.wrapping_add(1);
            for _ in 0..n {
                let (tokens, end) = read_tokens(e, it.script, at, it.limit);
                let Some(t) = tokens else {
                    self.expr = Some((it.script, lists));
                    return false;
                };
                lists.push(t);
                at = end;
            }
            for t in &lists {
                register_vars(e, root, it.script, t);
            }
            self.expr = Some((it.script, lists));
            return true;
        }
        let n = it.r16(e);
        if !self.parse_game_args(e, root, &mut it, n) {
            return false;
        }
        if it.cur < it.start.wrapping_add(len) && MESSAGE_BOX_PARSERS.contains(&parse) {
            let n = it.r16(e);
            if n > 9 {
                return false;
            }
            for _ in 0..n {
                if !self.read_numeric_token(e, &mut it) {
                    return false;
                }
            }
        }
        !it.bad
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
    // CommandCallToken::ToString, StringForNumericParam.
    fn text(&self, e: &mut Engine, root: &mut Root) -> B {
        if self.record == 0 {
            return unk_opcode(self.opcode);
        }
        let mut out = match self.calling {
            Some((script, r)) => {
                let mut t = ref_text(e, root, script, r);
                t.push(b'.');
                t
            }
            None => B::new(),
        };
        let mut name = long_name(e, self.record);
        if name.is_empty() {
            name = unk_opcode(self.opcode);
        }
        let execute = e.mem.u32(self.record + 0x18);
        let function = record_of(e, crate::functions::OP_FUNCTION);
        if function != 0 && execute == e.mem.u32(function + 0x18) {
            let n = self.args.len();
            let mut parts = Vec::new();
            for (i, a) in self.args.iter().enumerate() {
                let mut t = a.text(e, root);
                if i != n - 1 {
                    t.push(b',');
                }
                parts.push(t);
            }
            let mut s = name;
            s.extend_from_slice(b" {");
            s.extend(token_list(parts));
            s.extend_from_slice(b" }");
            return s;
        }
        out.extend(name);
        if let Some((script, lists)) = &self.expr {
            let parts: Vec<B> = lists
                .iter()
                .map(|t| line_text(e, root, *script, t, false))
                .collect();
            out.extend(token_list(parts));
            return out;
        }
        let mut parts = Vec::new();
        let count = usize::from(e.mem.u16(self.record + 0x12));
        for (i, a) in self.args.iter().enumerate() {
            // A message box's numbers past the parameter list: xNVSE reads a type past the list;
            // here they have none (no name).
            let ty = if i < count {
                param_type(e, self.record, i)
            } else {
                0
            };
            if let Operand::Num(v) = a {
                let s = numeric_param_text(e, ty, crate::args::f64_to_i32(*v));
                if !s.is_empty() {
                    parts.push(s);
                    continue;
                }
            }
            parts.push(a.text(e, root));
        }
        out.extend(token_list(parts));
        out
    }
}

/// `StringForNumericParam`: the name a number stands for in a parameter of this type; empty for
/// none. An actor value or animation group the tables do not have (xNVSE reads a null name) has
/// none.
fn numeric_param_text(e: &mut Engine, ty: u32, v: i32) -> B {
    match ty {
        0x05 => {
            // xNVSE indexes the table with any value; a code past its 77 entries has no name here.
            if !(0..=76).contains(&v) {
                return B::new();
            }
            let info = e
                .mem
                .u32(ACTOR_VALUES.wrapping_add(4u32.wrapping_mul(v as u32)));
            if info == 0 {
                return B::new();
            }
            let name = e.mem.u32(info + 0x38);
            if name == 0 {
                B::new()
            } else {
                guest_text(e, name)
            }
        }
        0x08 => match v {
            0x58 => b"X".to_vec(),
            0x59 => b"Y".to_vec(),
            0x5a => b"Z".to_vec(),
            _ => b"<unknown axis>".to_vec(),
        },
        0x0a => {
            if !(0..ANIM_GROUP_COUNT).contains(&v) {
                return B::new();
            }
            let name = e.mem.u32(ANIM_GROUPS + 0x24 * v as u32);
            if name == 0 {
                B::new()
            } else {
                guest_text(e, name)
            }
        }
        0x12 => match v {
            0 => b"Male".to_vec(),
            1 => b"Female".to_vec(),
            _ => b"<invalid sex>".to_vec(),
        },
        0x29 if v < 43 => {
            let name = e
                .mem
                .u32(MISC_STATS.wrapping_add(4u32.wrapping_mul(v as u32)));
            quoted(&guest_text(e, name))
        }
        0x37 if v < 5 => {
            let name = e
                .mem
                .u32(CRITICAL_STAGES.wrapping_add(4u32.wrapping_mul(v as u32)));
            guest_text(e, name)
        }
        _ => B::new(),
    }
}

/// `ExpressionEvaluator(data, script, &offset)` with `GetTokens`: the tokens of the expression
/// at `at` (its length word first) and where it ends; `None` when a token does not read.
fn read_tokens(e: &mut Engine, script: u32, at: u32, limit: u32) -> (Option<Vec<Tok>>, u32) {
    let data = script_data(e, script);
    e.with_stack(16, |e, frame| {
        let off = frame.addr();
        e.mem.set_u32(off, at.wrapping_sub(data));
        let mut ev = scratch_eval(script, data, 0, off, at);
        ev.limit = limit;
        let tokens = ev.tokens(e);
        (tokens, ev.at)
    })
}

fn scratch_eval(script: u32, data: u32, event_list: u32, offset_ptr: u32, at: u32) -> Eval {
    Eval {
        params: 0,
        data,
        this_obj: 0,
        containing_obj: 0,
        script,
        event_list,
        result: 0,
        offset_ptr,
        at,
        errors: Vec::new(),
        suppress_errors: true,
        base_offset: None,
        inline: false,
        limit: u32::MAX,
        expr_end: u32::MAX,
        overrun: false,
        resolved_vars: None,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp: RegisterNVSEVars.
/// The analyzer notes the array and string variables an expression names, and those of the
/// expressions inside its commands.
fn register_vars(e: &mut Engine, root: &mut Root, script: u32, tokens: &[Tok]) {
    if !root.active {
        return;
    }
    for t in tokens {
        match t {
            Tok::Var {
                ty: Ty::ArrayVar,
                var_idx,
                ..
            } => {
                let info = variable_info(e, script, u32::from(*var_idx));
                root.register(info, 3);
            }
            Tok::Var {
                ty: Ty::StringVar,
                var_idx,
                ..
            } => {
                let info = variable_info(e, script, u32::from(*var_idx));
                root.register(info, 2);
            }
            Tok::Command { .. } => {
                let call = call_token(e, root, script, t);
                if let Some((s, lists)) = &call.expr {
                    for l in lists {
                        register_vars(e, root, *s, l);
                    }
                }
            }
            _ => {}
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp:
// ScriptToken::GetCallToken.
/// A command token of an expression read as a command call (its arguments parsed, the result of
/// the parse not looked at).
fn call_token(e: &mut Engine, root: &mut Root, script: u32, tok: &Tok) -> Call {
    let Tok::Command {
        ref_idx,
        opcode,
        offset,
        ..
    } = *tok
    else {
        return Call::new(0, u32::MAX, 0, 0);
    };
    let record = record_of(e, opcode);
    if record == 0 {
        return Call::new(0, u32::MAX, 0, 0);
    }
    let rec_opcode = e.mem.u32(record + 8);
    let calling = script_ref(e, script, u32::from(ref_idx));
    let mut call = Call::new(record, rec_opcode, calling, script);
    let data = script_data(e, script).wrapping_add(offset);
    let length = e.mem.u16(data.wrapping_sub(2));
    let it = Iter {
        opcode: rec_opcode,
        length,
        ref_idx,
        script,
        cur: data,
        start: data,
        // The token's own arguments (its length word counts itself), which the token's read
        // checked lie inside its expression.
        limit: data.wrapping_add(u32::from(length).saturating_sub(2)),
        bad: false,
    };
    call.parse_args(e, root, it, u32::from(length));
    call
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp:
// ScriptToken::GetVariableName.
/// An expression variable's name: the script's own local, or `form.name` of a reference's or a
/// quest's script; empty when it does not resolve.
fn variable_name(e: &mut Engine, root: &Root, script: u32, ref_idx: u16, var_idx: u16) -> B {
    if ref_idx == 0 {
        let info = variable_info(e, script, u32::from(var_idx));
        if info != 0 {
            return var_text(e, root, script, info, 0);
        }
        return B::new();
    }
    let ref_var = script_ref(e, script, u32::from(ref_idx));
    if ref_var == 0 {
        return B::new();
    }
    let form = e.mem.u32(ref_var + 8);
    let refr = cast(e, form, RTTI_TES_OBJECT_REFR);
    if refr != 0 {
        let s = extra_script(e, refr);
        if s != 0 {
            let info = variable_info(e, s, u32::from(var_idx));
            if info != 0 && e.vcall(refr, GET_NAME, &args![]).u32() != 0 {
                return var_text(e, root, s, info, form);
            }
        }
    } else {
        let quest = cast(e, form, RTTI_TES_QUEST);
        if quest != 0 {
            let s = e.mem.u32(quest + 0x1c);
            if s == 0 {
                return B::new();
            }
            let info = variable_info(e, s, u32::from(var_idx));
            if info != 0 {
                return var_text(e, root, s, info, form);
            }
        }
    }
    B::new()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionEvaluator::GetLineText (the tokens' form).
/// An expression written back from its postfix tokens: operators between their operands, a
/// composite operand in parentheses where a stronger operator takes it; the whole in parentheses
/// for an inline expression; empty when the tokens do not make one value.
pub fn line_text(e: &mut Engine, root: &mut Root, script: u32, tokens: &[Tok], inline: bool) -> B {
    let mut operands: Vec<B> = Vec::new();
    let mut operators: Vec<u8> = Vec::new();
    let mut composites: std::collections::HashSet<B> = std::collections::HashSet::new();
    for (i, tok) in tokens.iter().enumerate() {
        let last = i + 1 == tokens.len();
        if let Tok::Op(o) = *tok {
            let n = expr::operand_count(o);
            if operands.len() < n {
                return B::new();
            }
            if let (Some(&last_op), false) = (operators.last(), operands.is_empty()) {
                operators.pop();
                if expr::op_precedence(last_op) < expr::op_precedence(o) {
                    let k = operands.len();
                    if composites.contains(&operands[k - 1]) {
                        operands[k - 1] = paren(&operands[k - 1]);
                    }
                    if k > 1 && composites.contains(&operands[k - 2]) {
                        operands[k - 2] = paren(&operands[k - 2]);
                    }
                }
            }
            operators.push(o);
            let sym = expr::op_symbol(o).as_bytes();
            if n == 2 {
                let rh = operands.pop().unwrap_or_default();
                let lh = operands.pop().unwrap_or_default();
                let mut s = lh;
                if o == expr::op::LEFT_BRACKET {
                    s.push(b'[');
                    s.extend(rh);
                    s.push(b']');
                } else if matches!(
                    o,
                    expr::op::SLICE
                        | expr::op::EXPONENT
                        | expr::op::MEMBER_ACCESS
                        | expr::op::MAKE_PAIR
                        | expr::op::DOT
                ) {
                    s.extend_from_slice(sym);
                    s.extend(rh);
                } else {
                    s.push(b' ');
                    s.extend_from_slice(sym);
                    s.push(b' ');
                    s.extend(rh);
                }
                operands.push(s);
            } else if n == 1 {
                if let Some(top) = operands.last_mut() {
                    let mut s = sym.to_vec();
                    s.extend_from_slice(top);
                    *top = s;
                }
            }
            if let Some(top) = operands.last() {
                composites.insert(top.clone());
            }
            continue;
        }
        let text = match tok {
            Tok::Number { v, .. } => g(e, *v),
            Tok::Str(s) => quoted(&unformat(s)),
            Tok::Form(id) => {
                let form = if *id == 0 { 0 } else { lookup_form(e, *id) };
                if form == 0 {
                    b"<bad form>".to_vec()
                } else {
                    let name = editor_id(e, form);
                    if name.is_empty() {
                        format!("<{:X}>", e.mem.u32(form + 0xc)).into_bytes()
                    } else {
                        name
                    }
                }
            }
            Tok::Global(global) => {
                if *global == 0 {
                    b"<bad global>".to_vec()
                } else {
                    string_at(e, global + 0x18)
                }
            }
            Tok::Command { .. } => {
                let call = call_token(e, root, script, tok);
                if call.error {
                    b"<failed resolve cmd>".to_vec()
                } else {
                    let mut s = call.text(e, root);
                    let mut applied = false;
                    if call.expr.as_ref().is_some_and(|(_, l)| !l.is_empty()) && !last {
                        applied = true;
                        s = paren(&s);
                    }
                    if !applied && has_optional_param(e, call.record) && !last {
                        s = paren(&s);
                    }
                    s
                }
            }
            Tok::Var {
                ref_idx, var_idx, ..
            } => {
                let name = variable_name(e, root, script, *ref_idx, *var_idx);
                if name.is_empty() {
                    format!("<failed to get var name {ref_idx} {var_idx}>").into_bytes()
                } else {
                    name
                }
            }
            Tok::LambdaData { .. }
                if LAMBDA_DEPTH.with(std::cell::Cell::get) >= MAX_LAMBDA_DEPTH =>
            {
                // Lambdas nested deeper than this (xNVSE recurses until its stack runs out):
                // nothing decompiles.
                B::new()
            }
            Tok::LambdaData { data, len } => {
                LAMBDA_DEPTH.with(|d| d.set(d.get() + 1));
                let lambda = crate::lambda::new_script(e, *data, *len, script);
                let mut text = if root.active {
                    decompile_with(e, root, lambda, true)
                } else {
                    let mut own = Root::analyzer();
                    decompile_with(e, &mut own, lambda, true)
                };
                LAMBDA_DEPTH.with(|d| d.set(d.get() - 1));
                // `MakeUnique<Script>(...)`: the memory freed, no destructor run.
                e.call(FORM_HEAP_FREE, &args![lambda]);
                while text.first().is_some_and(|&c| is_space(c)) {
                    text.remove(0);
                }
                while text.last().is_some_and(|&c| is_space(c)) {
                    text.pop();
                }
                text
            }
            Tok::OptionalEmpty => b"_".to_vec(),
            _ => b"<can't decompile token>".to_vec(),
        };
        operands.push(text);
    }
    if operands.len() == 1 {
        let s = operands.pop().unwrap_or_default();
        return if inline { paren(&s) } else { s };
    }
    B::new()
}

/// `ScriptToken::GetVariableDataAsString` of a variable as the evaluation resolved it.
fn variable_data(e: &mut Engine, ev: &Eval, tok: &Tok) -> B {
    let Tok::Var { ty, local, .. } = tok else {
        return B::new();
    };
    match ty {
        Ty::NumericVar => {
            let v = if *local == 0 {
                0.0
            } else {
                ev.get_number(e, tok)
            };
            g(e, v)
        }
        Ty::RefVar => {
            let id = if *local == 0 { 0 } else { e.mem.u32(local + 8) };
            let form = if id == 0 { 0 } else { lookup_form(e, id) };
            if form != 0 {
                crate::commands::scripting_more::form_representation(e, form)
            } else if id != 0 {
                format!("invalid form ({id:X})").into_bytes()
            } else {
                b"uninitialized form (0)".to_vec()
            }
        }
        Ty::StringVar => {
            let s = if *local == 0 && !matches!(tok, Tok::Var { string, .. } if *string != 0) {
                B::new()
            } else {
                ev.get_string(e, tok)
            };
            quoted(&s)
        }
        Ty::ArrayVar => {
            let id = if *local == 0 {
                0
            } else {
                crate::args::f64_to_i32(e.mem.f64(local + 8)) as u32
            };
            let Some(arr) = crate::vars::array(e, id).map(|a| a.elements.clone()) else {
                return b"uninitialized array".to_vec();
            };
            use crate::vars::Container;
            let size = match &arr {
                Container::Array(v) => v.len(),
                Container::NumMap(v) => v.len(),
                Container::StrMap(v) => v.len(),
            };
            if size > 10 {
                return format!("array size {size}").into_bytes();
            }
            let mut s = b"array keys [".to_vec();
            if size > 0 {
                match &arr {
                    Container::Array(v) => {
                        for i in 0..v.len() {
                            s.extend(g(e, i as f64));
                            s.extend_from_slice(b", ");
                        }
                    }
                    Container::NumMap(v) => {
                        for (k, _) in v {
                            s.extend(g(e, *k));
                            s.extend_from_slice(b", ");
                        }
                    }
                    Container::StrMap(v) => {
                        for (k, _) in v {
                            s.push(b'"');
                            s.extend_from_slice(k);
                            s.extend_from_slice(b"\", ");
                        }
                    }
                }
                s.truncate(s.len() - 2);
            }
            s.push(b']');
            s
        }
        _ => B::new(),
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionEvaluator::GetVariablesText (the tokens' form).
/// `name=value` for each variable an expression reads (once each), and for a command called on a
/// reference variable that variable's form; separated by ", ".
fn variables_text(e: &mut Engine, ev: &Eval, tokens: &[Tok]) -> B {
    let mut out = B::new();
    let mut printed: Vec<(u16, u16)> = Vec::new();
    let mut root = Root::none();
    for tok in tokens {
        let key = match tok {
            Tok::Var {
                ref_idx, var_idx, ..
            } => (*ref_idx, *var_idx),
            Tok::Command { ref_idx, .. } => (*ref_idx, 0),
            _ => (0, 0),
        };
        if printed.contains(&key) {
            continue;
        }
        match tok {
            Tok::Var {
                ref_idx, var_idx, ..
            } => {
                // A variable the evaluation did not resolve (a short circuit passed over it, or it
                // failed) prints as unresolved: 0, "", no form, no array. xNVSE prints what the
                // line's cached token last held, from an earlier run of the line, or the same
                // when it never resolved; the port keeps no tokens between runs.
                let resolved = if ev.resolved_vars.as_ref().is_some_and(|v| v.contains(&key)) {
                    let mut scratch =
                        scratch_eval(ev.script, ev.data, ev.event_list, ev.offset_ptr, 0);
                    scratch.resolved(e, tok).unwrap_or_else(|| tok.clone())
                } else {
                    tok.clone()
                };
                out.extend(variable_name(e, &root, ev.script, *ref_idx, *var_idx));
                out.push(b'=');
                out.extend(variable_data(e, ev, &resolved));
                printed.push(key);
                out.extend_from_slice(b", ");
            }
            Tok::Command { ref_idx, .. } if *ref_idx != 0 => {
                let r = script_ref(e, ev.script, u32::from(*ref_idx));
                let var_idx = if r == 0 { 0 } else { e.mem.u32(r + 0xc) };
                if var_idx != 0 {
                    let info = variable_info(e, ev.script, var_idx);
                    if info != 0 && e.mem.u16(info + 0x18 + 6) != 0 {
                        let var = if ev.event_list == 0 {
                            0
                        } else {
                            event_list_variable(e, ev.event_list, e.mem.u32(info))
                        };
                        if var != 0 {
                            let name = string_at(e, info + 0x18);
                            let id = e.mem.u32(var + 8);
                            let form = if id == 0 { 0 } else { lookup_form(e, id) };
                            out.extend(name);
                            if form != 0 {
                                out.push(b'=');
                                out.extend(crate::commands::scripting_more::form_representation(
                                    e, form,
                                ));
                            } else if id != 0 {
                                out.extend(format!("=invalid form ({id:x})").into_bytes());
                            } else {
                                out.extend_from_slice(b"=uninitialized form (0)");
                            }
                        }
                    }
                }
                out.extend_from_slice(b", ");
            }
            _ => {}
        }
        root.active = false;
    }
    for _ in 0..2 {
        if out.pop().is_none() {
            break;
        }
    }
    out
}

/// `ResetCursor` and the command the evaluator runs for.
fn reset_cursor(e: &mut Engine, ev: &mut Eval) -> u32 {
    let base = ev.base_offset.unwrap_or(0);
    let offset = base.wrapping_add(4);
    e.mem.set_u32(ev.offset_ptr, offset);
    ev.at = ev.data.wrapping_add(offset);
    record_of(e, u32::from(e.mem.u16(ev.data.wrapping_add(base))))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionEvaluator::GetLineText().
/// The evaluator's whole line: the command's name, then each argument (in parentheses when there
/// are several and it has more than one token). An argument whose tokens do not read (xNVSE reads
/// through a null there) adds nothing.
pub fn evaluator_line_text(e: &mut Engine, ev: &mut Eval) -> B {
    let record = reset_cursor(e, ev);
    let n = ev.read_byte(e);
    let mut out = B::new();
    if record != 0 {
        out.extend(long_name(e, record));
        out.push(b' ');
    }
    let mut root = Root::none();
    for i in 0..n {
        if let Some(tokens) = ev.tokens(e) {
            let arg = line_text(e, &mut root, ev.script, &tokens, ev.inline);
            if n != 1 && tokens.len() > 1 {
                out.extend(paren(&arg));
            } else {
                out.extend(arg);
            }
        }
        if i != n - 1 {
            out.push(b' ');
        }
    }
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionEvaluator::GetVariablesText().
/// Each argument's variables, one argument a line.
pub fn evaluator_variables_text(e: &mut Engine, ev: &mut Eval) -> B {
    reset_cursor(e, ev);
    let n = ev.read_byte(e);
    let mut out = B::new();
    for i in 0..n {
        if let Some(tokens) = ev.tokens(e) {
            out.extend(variables_text(e, ev, &tokens));
        }
        if i != n - 1 {
            out.push(b'\n');
        }
    }
    out
}

// ---- lines ----

#[derive(Debug)]
enum ExprTok {
    /// A `ScriptOperator`'s address.
    Op(u32),
    Operand(Operand),
}

/// `Expression`: a `set` or `if` expression in the game's own form.
#[derive(Debug, Default)]
struct Expression {
    stack: Vec<ExprTok>,
}

fn operator_code(e: &Engine, op: u32) -> u32 {
    e.mem.u32(op)
}
fn operator_precedence(e: &Engine, op: u32) -> u8 {
    e.mem.u8(op + 4)
}
fn operator_text(e: &Engine, op: u32) -> B {
    let mut s = e.mem.bytes(op + 5, 3);
    if let Some(z) = s.iter().position(|&b| b == 0) {
        s.truncate(z);
    }
    s
}

/// `Expression::ReadOperator`: the longest of the game's operators at the position.
fn read_operator(e: &Engine, it: &mut Iter) -> u32 {
    // xNVSE compares an operator's characters with what follows even past the expression's end
    // (the next bytes of the script); here up to the script data's end, no further.
    let data_end = script_data(e, it.script).wrapping_add(e.mem.u32(it.script + 0x20));
    let byte_at = |at: u32| (at < data_end).then(|| e.mem.u8(at));
    let Some(c) = byte_at(it.cur) else {
        return 0;
    };
    if c <= 0x20 || is_space(c) || !c.is_ascii_punctuation() {
        return 0;
    }
    let start = it.cur;
    let mut best = 0;
    let mut best_len = 0;
    let mut ret = start;
    for k in 0..16 {
        let op = GAME_OPERATORS + 8 * k;
        let text = operator_text(e, op);
        let mut matched = 0;
        let mut ok = true;
        for (i, &ch) in text.iter().enumerate() {
            if byte_at(start.wrapping_add(i as u32)) != Some(ch) {
                ok = false;
                break;
            }
            matched += 1;
        }
        if ok && matched > best_len {
            best = op;
            best_len = matched;
            ret = start.wrapping_add(matched);
        }
    }
    it.cur = ret;
    best
}

impl Expression {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
    // Expression::ReadExpression.
    fn read(e: &mut Engine, root: &mut Root, it: &mut Iter) -> Option<Expression> {
        let mut x = Expression::default();
        let len = it.r16(e);
        // An expression longer than what is left of its line fails the line (a corrupt script).
        if it.bad || u64::from(it.cur) + u64::from(len) > u64::from(it.limit) {
            return None;
        }
        let end = it.cur.wrapping_add(u32::from(len));
        let line_limit = it.limit;
        it.limit = end;
        let mut ref_var = 0u32;
        while it.cur < end {
            let c = it.r8(e);
            if c <= 0x20 || is_space(c) {
                continue;
            }
            match c {
                b's' | b'l' | b'f' => {
                    let idx = it.r16(e);
                    let script = if ref_var != 0 {
                        referenced_script(e, ref_var)
                    } else {
                        it.script
                    };
                    let form = if ref_var != 0 {
                        e.mem.u32(ref_var + 8)
                    } else {
                        0
                    };
                    let info = variable_info(e, script, u32::from(idx));
                    x.stack
                        .push(ExprTok::Operand(Operand::Var { script, info, form }));
                    ref_var = 0;
                    continue;
                }
                b'G' => {
                    let r = it.ref_var(e);
                    x.stack.push(ExprTok::Operand(Operand::global(e, r)));
                    continue;
                }
                b'Z' => {
                    let r = it.ref_var(e);
                    x.stack.push(ExprTok::Operand(Operand::Ref {
                        script: it.script,
                        ref_var: r,
                    }));
                    continue;
                }
                b'r' => {
                    ref_var = it.ref_var(e);
                    continue;
                }
                b'n' | b'z' => return None,
                b'"' => {
                    if let Some(s) = it.read_string(e) {
                        x.stack.push(ExprTok::Operand(Operand::Str(s)));
                    }
                    // xNVSE goes on to the operator and number reads from here.
                }
                b'X' => {
                    let opcode = u32::from(it.r16(e));
                    let record = record_of(e, opcode);
                    let mut call = Call::new(record, opcode, ref_var, it.script);
                    let data_len = it.r16(e);
                    if u64::from(it.cur) + u64::from(data_len) > u64::from(it.limit) {
                        return None;
                    }
                    if !call.error
                        && data_len != 0
                        && !call.parse_args(e, root, *it, u32::from(data_len))
                    {
                        return None;
                    }
                    x.stack
                        .push(ExprTok::Operand(Operand::Call(Box::new(call))));
                    it.cur = it.cur.wrapping_add(u32::from(data_len));
                    ref_var = 0;
                    continue;
                }
                _ => {}
            }
            it.cur = it.cur.wrapping_sub(1);
            let op = read_operator(e, it);
            if op != 0 {
                x.stack.push(ExprTok::Op(op));
                continue;
            }
            let v = it.read_number(e, end)?;
            x.stack.push(ExprTok::Operand(Operand::Num(v)));
        }
        it.limit = line_limit;
        (!it.bad).then_some(x)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
    // Expression::ToString, OperatorToken::ToString.
    fn text(&self, e: &mut Engine, root: &mut Root) -> B {
        let mut operands: Vec<B> = Vec::new();
        let mut operators: Vec<u32> = Vec::new();
        let mut composites: std::collections::HashSet<B> = std::collections::HashSet::new();
        for (i, tok) in self.stack.iter().enumerate() {
            match tok {
                ExprTok::Op(op) => {
                    if let (Some(&last), false) = (operators.last(), operands.is_empty()) {
                        operators.pop();
                        if operator_precedence(e, *op) > operator_precedence(e, last) {
                            let k = operands.len();
                            if composites.contains(&operands[k - 1]) {
                                operands[k - 1] = paren(&operands[k - 1]);
                            }
                            if k > 1 && composites.contains(&operands[k - 2]) {
                                operands[k - 2] = paren(&operands[k - 2]);
                            }
                        }
                    }
                    operators.push(*op);
                    let result = if operator_code(e, *op) == 0xf {
                        match operands.pop() {
                            Some(top) => {
                                let mut s = vec![b'-'];
                                s.extend(top);
                                s
                            }
                            None => b"<invalid number of operands on stack>".to_vec(),
                        }
                    } else if operands.len() < 2 {
                        b"<invalid number of operands on stack>".to_vec()
                    } else {
                        let rh = operands.pop().unwrap_or_default();
                        let mut s = operands.pop().unwrap_or_default();
                        s.push(b' ');
                        s.extend(operator_text(e, *op));
                        s.push(b' ');
                        s.extend(rh);
                        s
                    };
                    composites.insert(result.clone());
                    operands.push(result);
                }
                ExprTok::Operand(o) => {
                    let mut s = o.text(e, root);
                    if i + 1 != self.stack.len() {
                        if let Operand::Call(c) = o {
                            if c.record != 0 && has_optional_param(e, c.record) {
                                s = paren(&s);
                            }
                        }
                    }
                    operands.push(s);
                }
            }
        }
        if operands.len() != 1 {
            let n = operands.len();
            let parts = operands.iter().enumerate().map(|(i, s)| {
                let mut p = vec![b'\''];
                p.extend_from_slice(s);
                p.push(b'\'');
                if i != n - 1 {
                    p.push(b',');
                }
                p
            });
            let mut out = b"; failed to decompile expression, bad expression stack:".to_vec();
            out.extend(token_list(parts));
            return out;
        }
        operands.pop().unwrap_or_default()
    }
}

#[derive(Debug)]
enum Kind {
    /// A statement that prints only its name.
    Plain,
    /// `scn`.
    Name,
    Begin {
        call: Call,
    },
    SetTo {
        var: Operand,
        expr: Expression,
    },
    Conditional {
        expr: Expression,
    },
    Command {
        call: Call,
    },
}

/// `ScriptLine`: one line, its record (`cmdInfo`, 0 for none) and whether it read.
#[derive(Debug)]
struct Line {
    record: u32,
    script: u32,
    kind: Kind,
    error: bool,
}

fn statement_record(code: u32) -> u32 {
    if code > code::MAX {
        return 0;
    }
    STATEMENTS.wrapping_add(code.wrapping_sub(0x10).wrapping_mul(0x28))
}

fn event_record(op: u32) -> u32 {
    if op >= EVENT_BLOCK_COUNT {
        return 0;
    }
    EVENT_BLOCKS + op * 0x28
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
// ScriptAnalyzer::ParseLine and the line classes' constructors.
fn parse_line(e: &mut Engine, root: &mut Root, it: &Iter) -> Line {
    let op = it.opcode;
    let script = it.script;
    // A line running past the data's end fails (a corrupt script); inside it, no read passes its
    // end.
    let line_end = u64::from(it.start) + u64::from(it.length);
    if line_end > u64::from(it.limit) {
        return Line {
            record: 0,
            script,
            kind: Kind::Plain,
            error: true,
        };
    }
    let mut bounded = *it;
    bounded.limit = line_end as u32;
    let it = &bounded;
    let statement = |kind: Kind, record: u32| Line {
        record,
        script,
        kind,
        error: record == 0,
    };
    if op <= code::MAX {
        let record = statement_record(op);
        match op {
            code::BEGIN => {
                let mut ctx = *it;
                let event_op = u32::from(ctx.r16(e));
                let event = event_record(event_op);
                let _end_jump = ctx.r32(e);
                let event_opcode = if event != 0 {
                    e.mem.u32(event + 8)
                } else {
                    u32::MAX
                };
                let mut call = Call::new(event, event_opcode, 0, script);
                let mut error = record == 0 || ctx.bad;
                if it.length > 6 {
                    if event == 0 {
                        error = true;
                    } else {
                        let sub = Iter {
                            opcode: event_opcode,
                            length: it.length,
                            ref_idx: 0,
                            script,
                            cur: ctx.cur,
                            start: ctx.cur,
                            limit: ctx.limit,
                            bad: false,
                        };
                        if !call.parse_args(e, root, sub, u32::from(it.length) - 6) {
                            error = true;
                        }
                    }
                }
                return Line {
                    record,
                    script,
                    kind: Kind::Begin { call },
                    error,
                };
            }
            0x11
            | 0x12
            | 0x13
            | 0x14
            | 0x1e
            | 0x1f
            | code::END_IF
            | code::REFERENCE_FUNCTION
            | code::ELSE => return statement(Kind::Plain, record),
            code::SET_TO => {
                if record == 0 {
                    return statement(Kind::Plain, 0);
                }
                let mut ctx = *it;
                let t = e.mem.u8(ctx.cur);
                let var = if t == b'r' || t == b's' || t == b'f' {
                    let mut form = 0;
                    let mut vt = 0;
                    let info = ctx.read_variable(e, &mut form, &mut vt);
                    Operand::Var { script, info, form }
                } else if t == b'G' {
                    ctx.cur = ctx.cur.wrapping_add(1);
                    let r = ctx.ref_var(e);
                    Operand::global(e, r)
                } else {
                    return Line {
                        record,
                        script,
                        kind: Kind::Plain,
                        error: true,
                    };
                };
                if var.error() {
                    return Line {
                        record,
                        script,
                        kind: Kind::Plain,
                        error: true,
                    };
                }
                let expr = Expression::read(e, root, &mut ctx);
                let error = expr.is_none() || ctx.bad;
                return Line {
                    record,
                    script,
                    kind: Kind::SetTo {
                        var,
                        expr: expr.unwrap_or_default(),
                    },
                    error,
                };
            }
            code::IF | code::ELSE_IF => {
                if record == 0 {
                    return statement(Kind::Plain, 0);
                }
                let mut ctx = *it;
                let _jump = ctx.r16(e);
                let expr = Expression::read(e, root, &mut ctx);
                let error = expr.is_none() || ctx.bad;
                return Line {
                    record,
                    script,
                    kind: Kind::Conditional {
                        expr: expr.unwrap_or_default(),
                    },
                    error,
                };
            }
            code::SCRIPT_NAME => return statement(Kind::Name, record),
            _ => {}
        }
    }
    let record = record_of(e, op);
    let mut call = Call::new(record, op, it.calling_ref(e), script);
    let mut error = record == 0;
    if !error && it.length != 0 && !call.parse_args(e, root, *it, u32::from(it.length)) {
        error = true;
    }
    Line {
        record,
        script,
        kind: Kind::Command { call },
        error,
    }
}

impl Line {
    fn opcode(&self, e: &Engine) -> u32 {
        e.mem.u32(self.record + 8)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp: the lines'
    // ToString.
    fn text(&self, e: &mut Engine, root: &mut Root) -> B {
        let name = if self.record == 0 {
            B::new()
        } else {
            long_name(e, self.record)
        };
        match &self.kind {
            Kind::Plain => name,
            Kind::Name => {
                let mut s = name;
                s.push(b' ');
                s.extend(form_name(e, self.script));
                s
            }
            Kind::Begin { call } => {
                let mut s = name;
                s.push(b' ');
                if root.active && root.compiler_override {
                    s.push(b'_');
                }
                s.extend(call.text(e, root));
                s
            }
            Kind::SetTo { var, expr } => {
                let mut s = b"Set ".to_vec();
                s.extend(var.text(e, root));
                s.extend_from_slice(b" To ");
                s.extend(expr.text(e, root));
                s
            }
            Kind::Conditional { expr } => {
                let mut s = name;
                s.push(b' ');
                s.extend(expr.text(e, root));
                s
            }
            Kind::Command { call } => call.text(e, root),
        }
    }

    fn call(&self) -> Option<&Call> {
        match &self.kind {
            Kind::Command { call } => Some(call),
            _ => None,
        }
    }
}

/// `ScriptAnalyzer` with `Parse`: every line of the script; `None` when a line did not read (the
/// analyzer's `error`) or the script has no data.
fn parse(e: &mut Engine, root: &mut Root, script: u32) -> (Vec<Line>, bool) {
    let mut lines = Vec::new();
    if script == 0 || script_data(e, script) == 0 {
        return (lines, true);
    }
    let mut it = Iter::new(e, script);
    let mut error = false;
    while !it.end(e) {
        let line = parse_line(e, root, &it);
        error |= line.error;
        lines.push(line);
        it.advance(e);
    }
    (lines, error)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp:
// ScriptAnalyzer::DecompileScript.
fn decompile_with(e: &mut Engine, root: &mut Root, script: u32, is_lambda: bool) -> B {
    let (lines, error) = parse(e, root, script);
    if error {
        return B::new();
    }
    let add = [
        code::IF,
        code::BEGIN,
        OP_WHILE,
        OP_FOR_EACH,
        OP_FOR_EACH_ALT,
    ];
    let min = [code::END_IF, code::END, OP_LOOP];
    let neutral = [code::ELSE, code::ELSE_IF];
    let mut header = B::new();
    let mut text = B::new();
    let mut tabs: i32 = 0;
    let mut last = 0u32;
    for line in &lines {
        let op = line.opcode(e);
        if op == OP_PUSH_CONTEXT || op == OP_POP_CONTEXT || (op == code::SCRIPT_NAME && is_lambda) {
            continue;
        }
        let is_min = min.contains(&op);
        let is_add = add.contains(&op);
        if is_add && !text.ends_with(b"\n\n") && !add.contains(&last) && !neutral.contains(&last) {
            text.push(b'\n');
        }
        if is_min {
            tabs -= 1;
        }
        let is_neutral = neutral.contains(&op);
        if is_neutral {
            tabs -= 1;
        }
        if tabs < 0 {
            tabs = 0;
        }
        if !text.ends_with(b"\n\n")
            && min.contains(&last)
            && !min.contains(&op)
            && !neutral.contains(&op)
        {
            text.push(b'\n');
        }
        let mut next = line.text(e, root);
        if line.error {
            next.extend_from_slice(b" ; there was an error decompiling this line\n");
        }
        let tab_str = vec![b'\t'; tabs as usize];
        let mut replaced = B::new();
        for &b in &next {
            replaced.push(b);
            if b == b'\n' {
                replaced.extend_from_slice(&tab_str);
            }
        }
        text.extend_from_slice(&tab_str);
        if op == code::SCRIPT_NAME {
            header.extend(replaced);
            header.extend_from_slice(b"\n\n");
        } else {
            text.extend(replaced);
            text.push(b'\n');
        }
        if is_neutral || is_add {
            tabs += 1;
        }
        last = op;
    }
    if !is_lambda {
        let mut added: Vec<B> = Vec::new();
        let mut vars = Vec::new();
        let mut node = script + 0x4c;
        while node != 0 {
            vars.push(e.mem.u32(node));
            node = e.mem.u32(node + 4);
        }
        vars.extend(root.strings.clone());
        vars.extend(root.arrays.clone());
        for var in vars {
            if var == 0 {
                continue;
            }
            let name = string_at(e, var + 0x18);
            if added.contains(&name) {
                continue;
            }
            added.push(name);
            let keyword: &[u8] = if root.arrays.contains(&var) {
                b"Array_var "
            } else if root.strings.contains(&var) {
                b"String_var "
            } else if is_reference_type(e, var, script) {
                b"Ref "
            } else if e.mem.u8(var + 0x10) == 0 {
                b"Float "
            } else {
                b"Int "
            };
            header.extend_from_slice(keyword);
            header.extend(var_text(e, root, script, var, 0));
            header.push(b'\n');
        }
    }
    header.extend(text);
    header
}

/// `ScriptAnalyzer(script).DecompileScript()`: the script as text; empty when a line does not
/// read. A lambda (`isLambdaScript`) has no `scn` line and no variable header.
pub fn decompile(e: &mut Engine, script: u32) -> B {
    let mut root = Root::analyzer();
    decompile_with(e, &mut root, script, false)
}

/// Whether the script parses (`!analyzer.error`).
pub fn parses(e: &mut Engine, script: u32) -> bool {
    let mut root = Root::analyzer();
    let (_, error) = parse(e, &mut root, script);
    !error
}

fn tokens_call(e: &mut Engine, root: &mut Root, script: u32, tokens: &[Tok], cmd: u32) -> bool {
    tokens.iter().any(|t| {
        if let Tok::Command { opcode, .. } = t {
            if record_of(e, *opcode) == cmd {
                return true;
            }
            let call = call_token(e, root, script, t);
            if let Some((s, lists)) = &call.expr {
                if lists.iter().any(|l| tokens_call(e, root, *s, l, cmd)) {
                    return true;
                }
            }
        }
        false
    })
}

fn call_calls(e: &mut Engine, root: &mut Root, script: u32, call: &Call, cmd: u32) -> bool {
    if call.record == cmd {
        return true;
    }
    for a in &call.args {
        if let Operand::Inline {
            tokens: Some(t), ..
        } = a
        {
            if tokens_call(e, root, script, t, cmd) {
                return true;
            }
        }
    }
    if let Some((s, lists)) = &call.expr {
        if lists.iter().any(|l| tokens_call(e, root, *s, l, cmd)) {
            return true;
        }
    }
    false
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptAnalyzer.cpp: ScriptContainsCommand,
// ScriptAnalyzer::CallsCommand, DoesTokenCallCmd, DoExpressionEvalTokensCallCmd.
/// Whether the script calls the command of record `cmd` anywhere (a line, an argument's
/// expression, a `set`/`if` expression, a block type); with `event` (an event block record),
/// only from the first `Begin` of that block type on until another block type's `Begin`.
pub fn contains_command(e: &mut Engine, script: u32, cmd: u32, event: u32) -> bool {
    let mut root = Root::analyzer();
    let (lines, error) = parse(e, &mut root, script);
    if error {
        return false;
    }
    let mut last_event = 0u32;
    for line in &lines {
        if event != 0 {
            if let Kind::Begin { call } = &line.kind {
                last_event = call.record;
            }
        }
        if event != 0 && last_event != 0 && last_event != event {
            continue;
        }
        if line.record == cmd {
            return true;
        }
        if let Some(call) = line.call() {
            if call_calls(e, &mut root, line.script, call, cmd) {
                return true;
            }
        }
        if let Kind::Begin { call } = &line.kind {
            if call.record == cmd {
                return true;
            }
        }
        let expr = match &line.kind {
            Kind::SetTo { expr, .. } | Kind::Conditional { expr } => Some(expr),
            _ => None,
        };
        if let Some(x) = expr {
            for t in &x.stack {
                if let ExprTok::Operand(Operand::Call(c)) = t {
                    if call_calls(e, &mut root, line.script, c, cmd) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// The event block record of block type `op` (`GetEventCommandInfo`), 0 past the table.
pub fn event_block(op: u32) -> u32 {
    event_record(op)
}

#[cfg(test)]
#[path = "decompile_tests.rs"]
pub(crate) mod tests;
