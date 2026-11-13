//! xNVSE's hooks in the game's script compiler at run time (`Hooks_Script.cpp`,
//! `PatchRuntimeScriptCompile` and the compile part of `Hook_Script_Init` (xNVSE)), as seams in
//! the translated compiler (`engine-fallout-script`'s `tesscriptcompiler.rs`), and what they run:
//! the preprocessor (`PrecompileScript`), the `#version` tags, the variable types the compile
//! saw (`g_variableDefinitionsMap`), and the script text readers the expression compiler uses
//! (`ScriptTokenizer`, `Tokenizer`, `GetDeclaredVariableType`, the user function parameter
//! lines).
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a): `Hooks_Script.cpp`, `ScriptUtils.cpp`
//! (`Preprocessor`, `ValidateVariable`, `GetSavedVarType`, `SaveVarType`, the user function
//! parameters), `GameScript.cpp` (`GetDeclaredVariableType`,
//! `GetUserFunctionParamTokensFromLine`) and `Utilities.cpp` (`Tokenizer`, `ScriptTokenizer`,
//! `SplitString`, `StripSpace`).
//!
//! The seams (each a row of `engine_core::extend::SITES`):
//! - `005AEBB1` (`ScriptCompiler::Compile`, the call clearing the compiled flag):
//!   `HandleBeginCompile`, answering 1 (go on), 0 (fail) or 2 (compiled already: succeed).
//! - `005AED78` (`Compile`'s check of the referenced objects): passed while a lambda compiles.
//! - `005AEDA0` (`Compile`'s epilogue, every exit): `HookEndScriptCompile`.
//! - `005AFEDB` (`IsValidVariableName`'s form lookup): no form, so a variable may share a form's
//!   editor id.
//! - `005B0A3F` (`CompileLine`'s parentheses check of an `if`/`elseif`): passed over.
//! - `005B0F68` (`CompileLine`'s 0x4000-byte limit of a script): the data grows instead.
//! - `005B19BA` (`GetFunction`'s script command loop): xNVSE's name map.
//! - `005B2FA2` (`StandardCompile`'s byte map read for a value parameter): the double type takes
//!   the integer and float case (xNVSE's `SafeWrite8(0x5B3C30, 1)`), and at install the type
//!   table lets a double parameter be a variable.
//! - `005B3A8D` (`StandardCompile`'s end-of-line check): off while a nested command parses.
//!
//! Not here: xNVSE's compiled script cache on disk (`ScriptDataCache`, a file of compiled data
//! keyed by the text's hash, only a speed-up: plan 3.8's next part), and its two fixes of the
//! game's own expression reader's buffers (`ExpressionParserBufferOverflowHook_1/_2`, the
//! 0x400-byte string buffer at `00593DB6`), which are fixes of the game's `set`/`if` compiler
//! rather than xNVSE's compiler.

use crate::compiler::{self, is_space, ln, opt_text, sb, sc, show_error, state, str_equal, vt};

use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

const BEGIN_COMPILE: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005a_ebb1);
const CHECK_REFERENCES: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005a_ed78);
const END_COMPILE: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005a_eda0);
const VARIABLE_NAME_FORM: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005a_fedb);
const IF_PARENTHESES: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_0a3f);
const SCRIPT_SIZE_LIMIT: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_0f68);
const GET_FUNCTION: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_19ba);
const DOUBLE_PARAMETER: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_2fa2);
const END_OF_LINE: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_3a8d);
/// The script parameter type table's entry for the double type (`0118cdd4 + 0x2C * 8`): whether
/// the parameter may be a variable, whether it names an object.
const DOUBLE_PARAMETER_TYPE: u32 = 0x0118_cf34;

/// `Heap_MemorySize` (`00aa44c0` on the game's heap manager `011f6238`).
const HEAP_MEMORY_SIZE: u32 = 0x00aa_44c0;
const HEAP_MANAGER: u32 = 0x011f_6238;
/// `TESForm` virtual functions: `InitItem`, `MarkForDeletion`, `SetAltered`.
const INIT_ITEM: u32 = 0x88;
const MARK_FOR_DELETION: u32 = 0xc4;
const SET_ALTERED: u32 = 0xc8;
/// The most a script's data grows to here.
const MAX_SCRIPT_DATA: u32 = 0x100_0000;

/// Registers the parse routines and the seams.
pub fn install(e: &mut Engine) {
    e.on_site(BEGIN_COMPILE, begin_compile);
    e.on_site(CHECK_REFERENCES, check_references);
    e.on_site(END_COMPILE, end_compile);
    e.on_site(VARIABLE_NAME_FORM, |_, _| Some(0));
    e.on_site(IF_PARENTHESES, |_, _| Some(1));
    e.on_site(SCRIPT_SIZE_LIMIT, script_size_limit);
    e.on_site(GET_FUNCTION, get_function);
    e.on_site(END_OF_LINE, |e, _| state(e).eol_check_off.then_some(1));
    e.on_site(DOUBLE_PARAMETER, |_, _| Some(1));
    // `Hook_Script_Init`: the double type may be a variable and names no object (`*(UInt16 *)
    // 0x118CF34 = 1`; the exe holds 0x0100 there). Unit tests' stand-in engines map no such page.
    if e.mem.is_mapped(DOUBLE_PARAMETER_TYPE) {
        e.mem.set_u16(DOUBLE_PARAMETER_TYPE, 1);
    }
}

// ---- the seams ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// Runtime::HookParseCommandToken, HandleParseCommandToken.
/// `005B19BA`, words (the word being read): a name xNVSE's name map has is a command (kind
/// `'X'`, its opcode): 1, `GetFunction` answers true; else 0, the console commands are searched.
fn get_function(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let word = w.first().copied().unwrap_or(0);
    let name = crate::table::guest_text(e, word);
    match compiler::command_by_name(e, &name) {
        Some(opcode) => {
            e.mem.set_u8(word + 0x204, b'X');
            e.mem.set_u32(word + 0x208, opcode);
            Some(1)
        }
        None => Some(0),
    }
}

/// `005AED78`, no words: the check answers true while a lambda compiles
/// (`PatchDisable_ScriptBufferValidateRefVars`).
fn check_references(e: &mut Engine, _: &[u32]) -> Option<u32> {
    state(e).skip_ref_check.then_some(1)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// RemoveScriptDataLimit::RemoveScriptDataSizeLimit.
/// `005B0F68`, words (the script buffer, the line): instead of failing at 0x4000 bytes, the
/// script's data doubles (a zeroed block, the data so far copied, the old one freed) when this
/// line and ten bytes more would reach its size; 1, the line is written.
fn script_size_limit(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (buf, line) = (w[0], w[1]);
    let data = e.mem.u32(buf + sb::DATA);
    let size = e.call(HEAP_MEMORY_SIZE, &args![HEAP_MANAGER, data]).u32();
    let offset = e.mem.u32(buf + sb::DATA_OFFSET);
    let need = offset
        .wrapping_add(e.mem.u32(line + ln::DATA_LEN))
        .wrapping_add(10);
    if need < size {
        return Some(1);
    }
    // A size the heap does not know (0) or past any script's (16 MiB) would make xNVSE copy
    // the data into too small a block: the game's own limit check stays then.
    if size == 0 || size > MAX_SCRIPT_DATA / 2 {
        return None;
    }
    let grown = size * 2;
    let block = e.call(compiler::FORM_HEAP_ALLOCATE, &args![grown]).u32();
    e.mem.write(block, &vec![0; grown as usize]);
    let old = e.mem.bytes(data, offset.min(grown));
    e.mem.write(block, &old);
    e.call(compiler::FORM_HEAP_FREE, &args![data]);
    e.mem.set_u32(buf + sb::DATA, block);
    Some(1)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp: HandleBeginCompile,
// Runtime::HandleBeginCompile_SetNotCompiled, Runtime::HookBeginScriptCompile.
/// `005AEBB1`, words (the script, the script buffer): the compile starts. The script goes on the
/// compile stack; its error code and compiled flag are cleared; the `#version` tags become
/// comments (a plugin it names that is not built in fails the compile); the plugins hear
/// `ScriptPrecompile`; the outermost script is preprocessed. Answers 1 to go on, 0 to fail, 2
/// when a plugin compiled it.
fn begin_compile(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (script, buf) = (w[0], w[1]);
    let s = state(e);
    s.scripts.push(script);
    s.pound_restores.push(Vec::new());
    e.mem.set_u32(buf + sb::ERROR, 0);
    e.mem.set_u8(script + sc::COMPILED, 0);
    let text_at = e.mem.u32(buf + sb::TEXT);
    let text = opt_text(e, text_at).unwrap_or_default();
    if text.len() >= 4 && text[..4].eq_ignore_ascii_case(b"name") {
        // The new script language (`Compiler::CompileNVSEScript`): that compiler is not built
        // (plan step 3.8, deferred), so the script does not compile here.
        return Some(0);
    }
    for (at, plugin) in version_tags(&text) {
        e.mem.set_u8(text_at + at as u32, b';');
        if let Some(r) = state(e).pound_restores.last_mut() {
            r.push(at as u32);
        }
        if !plugin.eq_ignore_ascii_case(b"nvse") && crate::plugins::listed(e, &plugin).is_none() {
            let name = plugin.to_ascii_lowercase();
            let message = compiler::format(
                "Script compilation failed: No plugin with name %s could be found.",
                &[compiler::A::S(&name)],
                0x800,
            );
            crate::errors::show(e, 0, &String::from_utf8_lossy(&message));
            return Some(0);
        }
    }
    dispatch_script_message(e, crate::plugins::msg::SCRIPT_PRECOMPILE, script, buf);
    if e.mem.u32(buf + sb::ERROR) >= 1 {
        return Some(0);
    }
    if e.mem.u8(script + sc::COMPILED) != 0 {
        e.vcall(script, INIT_ITEM, &[]);
        e.vcall(script, MARK_FOR_DELETION, &args![0u32]);
        e.vcall(script, SET_ALTERED, &args![1u32]);
        return Some(2);
    }
    if state(e).scripts.len() == 1 && !preprocess(e, buf) {
        e.mem.set_u32(buf + sb::ERROR, 1);
        return Some(0);
    }
    Some(1)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// Runtime::HookEndScriptCompile_Call, PostScriptCompile.
/// `005AEDA0`, words (the result, the script buffer): a compile that succeeded is told to the
/// plugins (`ScriptCompile`); then the script leaves the compile stack, its `#version` tags get
/// their `#` back, and when the stack is empty the variable types the compile saw are
/// forgotten. Answers the result.
fn end_compile(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (result, buf) = (w[0], w[1]);
    if result != 0 {
        if let Some(&script) = state(e).scripts.last() {
            dispatch_script_message(e, crate::plugins::msg::SCRIPT_COMPILE, script, buf);
        }
    }
    let s = state(e);
    if let Some(script) = s.scripts.pop() {
        let restores = s.pound_restores.pop().unwrap_or_default();
        let empty = s.scripts.is_empty();
        if empty {
            s.var_types.clear();
            s.names = None;
        }
        let text = e.mem.u32(script + sc::TEXT);
        if text != 0 {
            for at in restores {
                e.mem.set_u8(text + at, b'#');
            }
        }
    }
    Some(result)
}

/// A plugin message whose data is `ScriptAndScriptBuffer` (the script, the buffer).
fn dispatch_script_message(e: &mut Engine, kind: u32, script: u32, buf: u32) {
    if crate::plugins::built_ins(e)
        .iter()
        .all(|p| p.listener.is_none())
    {
        return;
    }
    e.with_stack(8, |e, at| {
        e.mem.set_u32(at.addr(), script);
        e.mem.set_u32(at.addr() + 4, buf);
        crate::plugins::dispatch(e, kind, at.addr(), 8);
    });
}

/// The `#version("plugin", major[, minor[, beta]])` tags of a text (`#` and `version` without
/// case; at least one blank after the commas of the second and third numbers, any after the
/// first's), each by the position of its `#` and its plugin's name. The numbers are only kept
/// for the new script language, which is not built: they are not read here.
fn version_tags(text: &[u8]) -> Vec<(usize, Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'#' {
            if let Some((end, plugin)) = version_tag_at(text, i) {
                out.push((i, plugin));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// A version tag at `at` (its `#`): where it ends and the plugin's name.
fn version_tag_at(text: &[u8], at: usize) -> Option<(usize, Vec<u8>)> {
    let rest = &text[at + 1..];
    if rest.len() < 9 || !rest[..7].eq_ignore_ascii_case(b"version") || &rest[7..9] != b"(\"" {
        return None;
    }
    let mut i = at + 10;
    let name_start = i;
    while i < text.len() && text[i] != b'"' {
        i += 1;
    }
    if i == name_start || i >= text.len() {
        return None;
    }
    let plugin = text[name_start..i].to_vec();
    i += 1;
    if text.get(i) != Some(&b',') {
        return None;
    }
    i += 1;
    while i < text.len() && is_space(text[i]) {
        i += 1;
    }
    let digits = |i: &mut usize| {
        let s = *i;
        while *i < text.len() && text[*i].is_ascii_digit() {
            *i += 1;
        }
        *i > s
    };
    if !digits(&mut i) {
        return None;
    }
    // The optional numbers (`(?:,\s+(\d+))?` twice): a comma, at least one blank, digits.
    for _ in 0..2 {
        let mut j = i;
        if text.get(j) != Some(&b',') {
            break;
        }
        j += 1;
        let blank_start = j;
        while j < text.len() && is_space(text[j]) {
            j += 1;
        }
        if j == blank_start || !digits(&mut j) {
            break;
        }
        i = j;
    }
    if text.get(i) != Some(&b')') {
        return None;
    }
    Some((i + 1, plugin))
}

// ---- the variable types the compile saw ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: GetSavedVarType,
// SaveVarType, ValidateVariable.
/// `GetSavedVarType`.
pub(crate) fn saved_var_type(e: &mut Engine, script: u32, name: &[u8]) -> u8 {
    if script == 0 {
        return vt::INVALID;
    }
    state(e)
        .var_types
        .get(&(script, name.to_vec()))
        .copied()
        .unwrap_or(vt::INVALID)
}

/// `SaveVarType`: the first type a name got stays.
pub(crate) fn save_var_type(e: &mut Engine, script: u32, name: &[u8], var_type: u8) {
    state(e)
        .var_types
        .entry((script, name.to_vec()))
        .or_insert(var_type);
}

/// `g_variableTypeNames`.
fn type_word(t: u8) -> &'static [u8] {
    match t {
        vt::FLOAT => b"float",
        vt::INTEGER => b"int",
        vt::STRING => b"string_var",
        vt::ARRAY => b"array_var",
        vt::REF => b"ref",
        _ => b"invalid",
    }
}

/// `ValidateVariable`: a name declared again with another type is an error.
pub(crate) fn validate_variable(e: &mut Engine, name: &[u8], var_type: u8, script: u32) -> bool {
    let saved = saved_var_type(e, script, name);
    if saved != var_type && saved != vt::INVALID {
        let text = compiler::format(
            "Variable redefinition with a different type (saw first '%s', then '%s')",
            &[
                compiler::A::S(type_word(saved)),
                compiler::A::S(type_word(var_type)),
            ],
            0x400,
        );
        show_error(e, &text);
        return false;
    }
    save_var_type(e, script, name, var_type);
    true
}

// ---- text readers ----

fn find_first_of(t: &[u8], set: &[u8], from: usize) -> Option<usize> {
    (from..t.len()).find(|&i| set.contains(&t[i]))
}
fn find_first_not_of(t: &[u8], set: &[u8], from: usize) -> Option<usize> {
    (from..t.len()).find(|&i| !set.contains(&t[i]))
}
fn find(t: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    if from > t.len() || pat.len() > t.len() {
        return None;
    }
    (from..=t.len() - pat.len()).find(|&i| &t[i..i + pat.len()] == pat)
}

/// `Tokenizer`: tokens between delimiters.
pub(crate) struct Tokenizer<'a> {
    data: &'a [u8],
    delims: &'a [u8],
    offset: usize,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(data: &'a [u8], delims: &'a [u8]) -> Self {
        Tokenizer {
            data,
            delims,
            offset: 0,
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp: Tokenizer::NextToken.
    pub(crate) fn next_token(&mut self) -> Option<Vec<u8>> {
        if self.offset == self.data.len() {
            return None;
        }
        let start = find_first_not_of(self.data, self.delims, self.offset)?;
        let end = find_first_of(self.data, self.delims, start).unwrap_or(self.data.len());
        self.offset = end;
        Some(self.data[start..end].to_vec())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp: Tokenizer::ToNewLine.
    fn rest_of_line(&mut self) -> Vec<u8> {
        if self.offset == self.data.len() {
            return Vec::new();
        }
        let Some(start) = find_first_not_of(self.data, self.delims, self.offset) else {
            return Vec::new();
        };
        let end = find(self.data, b"\n", start).unwrap_or(self.data.len());
        self.offset = end;
        self.data[start..end].to_vec()
    }
}

const BLANKS: &[u8] = b" \t\n\r";

/// `ScriptTokenizer`: a script's lines as tokens, comments left out.
pub(crate) struct ScriptTokenizer<'a> {
    text: &'a [u8],
    offset: usize,
    in_comment: bool,
    tokens: Vec<(usize, usize)>,
    next: usize,
}

impl<'a> ScriptTokenizer<'a> {
    pub(crate) fn new(text: &'a [u8]) -> Self {
        ScriptTokenizer {
            text,
            offset: 0,
            in_comment: false,
            tokens: Vec::new(),
            next: 0,
        }
    }

    /// `GetNextLineStartPos`: after the line break, past blank lines.
    fn next_line_start(&self, from: usize) -> usize {
        let t = self.text;
        match find_first_of(t, b"\n\r", from) {
            None => t.len(),
            Some(r) => find_first_not_of(t, BLANKS, r + 1).unwrap_or(t.len()),
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp:
    // ScriptTokenizer::TryLoadNextLine.
    /// `TryLoadNextLine`: the next line with tokens (its `;` and `/* */` comments left out).
    pub(crate) fn load_next_line(&mut self) -> bool {
        let t = self.text;
        let len = t.len();
        'line: loop {
            self.tokens.clear();
            self.next = 0;
            if self.offset == len {
                return false;
            }
            let Some(mut pos) = find_first_not_of(t, BLANKS, self.offset) else {
                self.offset = len;
                return false;
            };
            if self.in_comment {
                let Some(end) = find(t, b"*/", pos) else {
                    self.offset = len;
                    return false;
                };
                self.in_comment = false;
                if end + 2 == len {
                    self.offset = len;
                    return false;
                }
                if t[pos..end].contains(&b';') {
                    self.offset = self.next_line_start(end + 2);
                    continue 'line;
                }
                pos = end + 2;
            }
            let Some(p) = find_first_not_of(t, BLANKS, pos) else {
                self.offset = len;
                return false;
            };
            pos = p;
            let line_end = find_first_of(t, b"\n\r", pos).unwrap_or(len);
            loop {
                if t[pos] == b';' {
                    self.offset = self.next_line_start(pos + 1);
                    if self.tokens.is_empty() {
                        continue 'line;
                    }
                    return true;
                }
                // Comments at the token's place (several may follow each other).
                loop {
                    let view_end = if line_end > pos { line_end } else { len };
                    if view_end - pos < 2 || t[pos] != b'/' || t[pos + 1] != b'*' {
                        break;
                    }
                    let spans_lines =
                        pos + 2 == line_end || find(&t[..view_end], b"*/", pos).is_none();
                    if spans_lines {
                        self.in_comment = true;
                        self.offset = find_first_not_of(t, BLANKS, line_end).unwrap_or(len);
                        if !self.tokens.is_empty() {
                            return true;
                        }
                        continue 'line;
                    }
                    let end = find(&t[..view_end], b"*/", pos).unwrap_or(len);
                    if end + 2 == len {
                        self.offset = len;
                        return false;
                    }
                    let inside_end = (end + 2).min(len);
                    if t[(pos + 2).min(inside_end)..inside_end].contains(&b';') {
                        self.offset = self.next_line_start(end + 2);
                        if !self.tokens.is_empty() {
                            return true;
                        }
                        continue 'line;
                    }
                    match find_first_not_of(t, b" \t", end + 2) {
                        None => {
                            self.offset = len;
                            return !self.tokens.is_empty();
                        }
                        Some(o) => {
                            self.offset = o;
                            if o == line_end {
                                if !self.tokens.is_empty() {
                                    return true;
                                }
                                continue 'line;
                            }
                        }
                    }
                    match find_first_not_of(t, BLANKS, end + 2) {
                        None => {
                            self.offset = len;
                            return !self.tokens.is_empty();
                        }
                        Some(p) => pos = p,
                    }
                }
                if t[pos] == b';' {
                    self.offset = self.next_line_start(pos + 1);
                    if self.tokens.is_empty() {
                        continue 'line;
                    }
                    return true;
                }
                let mut end = if t[pos] == b'"' {
                    find_first_of(t, b"\"", pos + 1).map_or(len, |q| q + 1)
                } else {
                    find_first_of(t, BLANKS, pos).unwrap_or(len)
                };
                let token = &t[pos..end];
                // A token can only be empty past a string that ran over lines (xNVSE reads
                // past the text there): the line ends.
                let Some(&last) = token.last() else {
                    break;
                };
                if last == b';' {
                    end -= 1;
                } else if token.len() > 2 && token[token.len() - 2] == b'/' && last == b'*' {
                    end -= 2;
                }
                self.tokens.push((pos, end));
                if end == len {
                    break;
                }
                match find_first_not_of(t, b" \t", end) {
                    None => {
                        pos = len;
                        break;
                    }
                    Some(p) => pos = p,
                }
                if pos == line_end {
                    break;
                }
            }
            self.offset = self.next_line_start(pos);
            return !self.tokens.is_empty();
        }
    }

    /// `GetNextLineToken`.
    pub(crate) fn next_token(&mut self) -> Vec<u8> {
        let Some(&(s, e)) = self.tokens.get(self.next) else {
            return Vec::new();
        };
        self.next += 1;
        self.text[s..e].to_vec()
    }

    /// `GetLineText`: from the first token's start to the last's end.
    pub(crate) fn line_text(&self) -> Vec<u8> {
        match self.tokens.as_slice() {
            [] => Vec::new(),
            [(s, e)] => self.text[*s..*e].to_vec(),
            [(s, _), .., (ls, le)] => {
                if le <= ls {
                    return self.text[*s..*ls].to_vec();
                }
                self.text[*s..*le].to_vec()
            }
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp:
// GetUserFunctionParamTokensFromLine.
/// The words between a line's first `{` and first `}` (blanks and commas between), or `None`.
fn param_tokens_of_line(line: &[u8]) -> Option<Vec<Vec<u8>>> {
    let start = line.iter().position(|&c| c == b'{')?;
    let end = line.iter().position(|&c| c == b'}')?;
    if start > end {
        return None;
    }
    let inside = &line[start + 1..end];
    let mut t = Tokenizer::new(inside, b"\t ,");
    let mut out = Vec::new();
    while let Some(tok) = t.next_token() {
        out.push(tok);
    }
    Some(out)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// GetUserFunctionParamNames.
/// `GetUserFunctionParamNames`: the parameter words of the text's first `begin` line longer
/// than seven characters.
pub(crate) fn user_function_param_names(text: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut t = ScriptTokenizer::new(text);
    while t.load_next_line() {
        let line = t.line_text();
        if line.len() <= 7 {
            continue;
        }
        if t.next_token().eq_ignore_ascii_case(b"begin") {
            return param_tokens_of_line(&line);
        }
    }
    None
}

/// `g_validVariableTypeNames`.
const VALID_TYPE_NAMES: [&[u8]; 8] = [
    b"float",
    b"int",
    b"string_var",
    b"array_var",
    b"ref",
    b"short",
    b"long",
    b"reference",
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionParser::GetUserFunctionParams.
/// `GetUserFunctionParams`: each parameter's variable index and type, a type word before a name
/// declaring it there; `None` for an unknown or repeated variable or a type with no name.
pub(crate) fn user_function_params(
    e: &mut Engine,
    p: &compiler::Parser,
    names: &[Vec<u8>],
    vars_head: u32,
    text: &[u8],
    script: u32,
) -> Option<Vec<(u32, u8)>> {
    let mut out: Vec<(u32, u8)> = Vec::new();
    let mut last_type = vt::INVALID;
    for token in names {
        let mut typed_here = false;
        if last_type != vt::INVALID {
            p.create_variable_named(e, token, last_type);
            typed_here = true;
        } else if let Some(name) = VALID_TYPE_NAMES
            .iter()
            .find(|n| n.eq_ignore_ascii_case(token))
        {
            last_type = compiler::variable_type_by_name(e, name);
            continue;
        }
        let info = compiler::variable_by_name(e, vars_head, token);
        if info == 0 {
            return None;
        }
        let var_type = if typed_here {
            std::mem::replace(&mut last_type, vt::INVALID)
        } else {
            let t = declared_variable_type(e, token, text, script);
            if t == vt::INVALID {
                return None;
            }
            t
        };
        let idx = e.mem.u32(info);
        if out.iter().any(|&(i, _)| i == idx) {
            return None;
        }
        out.push((idx, var_type));
    }
    (last_type == vt::INVALID).then_some(out)
}

/// How many lambda parents `GetDeclaredVariableType` follows (a chain is a lambda inside a
/// lambda: never this long).
const MAX_PARENTS: u32 = 64;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp:
// GetDeclaredVariableType.
/// `GetDeclaredVariableType`: a variable's type from the type the compile saw, else from its
/// declaration in the text (a `type name[, name...]` line, or a typed parameter of a
/// `begin function {...}` line), else from a lambda's parent script; the first found is kept.
pub(crate) fn declared_variable_type(e: &mut Engine, name: &[u8], text: &[u8], script: u32) -> u8 {
    let mut text = text.to_vec();
    let mut script = script;
    for _ in 0..MAX_PARENTS {
        let found = declared_in(e, name, &text, script);
        if found != vt::INVALID {
            return found;
        }
        let parent = compiler::lambda_parent(e, script);
        if parent == 0 {
            return vt::INVALID;
        }
        text = opt_text(e, e.mem.u32(parent + sc::TEXT)).unwrap_or_default();
        script = parent;
    }
    vt::INVALID
}

fn declared_in(e: &mut Engine, name: &[u8], text: &[u8], script: u32) -> u8 {
    let saved = saved_var_type(e, script, name);
    if saved != vt::INVALID {
        return saved;
    }
    let mut t = ScriptTokenizer::new(text);
    while t.load_next_line() {
        let first = t.next_token();
        if first.is_empty() {
            continue;
        }
        if str_equal(e, Some(&first), Some(b"begin")) {
            let params = param_tokens_of_line(&t.line_text()).unwrap_or_default();
            if params.len() <= 1 {
                continue;
            }
            let mut at = None;
            for (i, p) in params.iter().enumerate() {
                if str_equal(e, Some(p), Some(name)) {
                    at = Some(i);
                    break;
                }
            }
            if let Some(i) = at.filter(|&i| i > 0) {
                let var_type = compiler::variable_type_by_name(e, &params[i - 1]);
                if var_type != vt::INVALID {
                    save_var_type(e, script, name, var_type);
                    return var_type;
                }
            }
            continue;
        }
        let second = t.next_token();
        if second.is_empty() {
            continue;
        }
        let var_type = compiler::variable_type_by_name(e, &first);
        if var_type == vt::INVALID {
            continue;
        }
        let mut existing = second;
        loop {
            let comma = existing.last() == Some(&b',');
            if comma {
                existing.pop();
            }
            if str_equal(e, Some(&existing), Some(name)) {
                save_var_type(e, script, name, var_type);
                return var_type;
            }
            existing = t.next_token();
            if existing.is_empty() || !comma {
                break;
            }
        }
    }
    vt::INVALID
}

// ---- the preprocessor ----

/// `BlockType`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Loop,
    If,
}

fn block_name(b: Block) -> &'static [u8] {
    match b {
        Block::Loop => b"Loop",
        Block::If => b"If/EndIf",
    }
}

/// `s_blocks`: the keyword, its block, whether it opens and whether it ends one.
const BLOCKS: [(&[u8], Block, bool, bool); 7] = [
    (b"while", Block::Loop, true, false),
    (b"foreach", Block::Loop, true, false),
    (b"loop", Block::Loop, false, true),
    (b"if", Block::If, true, false),
    (b"elseif", Block::If, true, true),
    (b"else", Block::If, true, true),
    (b"endif", Block::If, false, true),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// Preprocessor::RemoveMultiLineComments.
/// `RemoveMultiLineComments`: `/* ... */` left out outside strings.
pub(crate) fn remove_multi_line_comments(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut in_string = false;
    let mut in_comment = false;
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        if i + 1 == text.len() {
            out.push(c);
            i += 1;
            continue;
        }
        let next = text[i + 1];
        if c == b'"' {
            in_string = !in_string;
        }
        if in_string {
            out.push(c);
            i += 1;
            continue;
        }
        if c == b'/' && next == b'*' {
            in_comment = true;
        } else if c == b'*' && next == b'/' {
            i += 2;
            in_comment = false;
            continue;
        }
        if !in_comment {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// `Preprocessor`'s lines: by `\r\n`, empty ones passed over, `;` comments cut (outside quotes)
/// except on the last line.
struct Lines {
    text: Vec<u8>,
    offset: usize,
    number: u32,
    line: Vec<u8>,
}

impl Lines {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // Preprocessor::AdvanceLine.
    fn advance(&mut self) -> bool {
        loop {
            if self.offset >= self.text.len() {
                return false;
            }
            self.number += 1;
            let Some(end) = find(&self.text, b"\r\n", self.offset) else {
                self.line = self.text[self.offset..].to_vec();
                self.offset = self.text.len();
                return true;
            };
            if self.offset == end {
                self.offset += 2;
                continue;
            }
            let mut line = self.text[self.offset..end].to_vec();
            let mut i = 0;
            while i < line.len() {
                if line[i] == b'"' {
                    if i + 1 == line.len() {
                        break;
                    }
                    match line[i + 1..].iter().position(|&c| c == b'"') {
                        None => break,
                        // The byte after the closing quote is passed over too, as xNVSE's loop
                        // does.
                        Some(q) => i = i + 1 + q + 1,
                    }
                } else if line[i] == b';' {
                    line.truncate(i);
                    break;
                }
                i += 1;
            }
            self.line = line;
            self.offset = end + 2;
            return true;
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: Preprocessor::Process,
// PrecompileScript.
/// `PrecompileScript`: the script's blocks balance (loops and ifs), `continue`/`break` only in
/// loops, `return` never in one, `set` never to an array variable, and declarations keep their
/// first type. Errors go to the console.
fn preprocess(e: &mut Engine, buf: u32) -> bool {
    let text_at = e.mem.u32(buf + sb::TEXT);
    let text = opt_text(e, text_at).unwrap_or_default();
    let script = state(e).scripts.last().copied().unwrap_or(0);
    let mut lines = Lines {
        text: remove_multi_line_comments(&text),
        offset: 0,
        number: 0,
        line: Vec::new(),
    };
    lines.advance();
    let mut blocks: Vec<Block> = Vec::new();
    let mut loop_depth = 0u32;
    let mut more = true;
    while more {
        let line = lines.line.clone();
        let mut tokens = Tokenizer::new(&line, b" \t\r\n(;");
        let Some(token) = tokens.next_token() else {
            more = lines.advance();
            continue;
        };
        let number = i64::from(lines.number);
        let mut keyword = false;
        for &(word, block, opens, ends) in &BLOCKS {
            if !token.eq_ignore_ascii_case(word) {
                continue;
            }
            keyword = true;
            if ends {
                if blocks.last() != Some(&block) {
                    let which = block_name(blocks.last().copied().unwrap_or(block));
                    show_error(
                        e,
                        &compiler::format(
                            "Invalid %s block structure on line %d.",
                            &[compiler::A::S(which), compiler::A::D(number)],
                            0x400,
                        ),
                    );
                    return false;
                }
                blocks.pop();
                if block == Block::Loop {
                    loop_depth = loop_depth.wrapping_sub(1);
                }
            }
            if opens {
                blocks.push(block);
                if block == Block::Loop {
                    loop_depth += 1;
                }
            }
        }
        if !keyword {
            if token.eq_ignore_ascii_case(b"continue") || token.eq_ignore_ascii_case(b"break") {
                if loop_depth == 0 {
                    show_error(
                        e,
                        &compiler::format(
                            "Error line %d:\nFunction %s must be called from within a loop.",
                            &[compiler::A::D(number), compiler::A::S(&token)],
                            0x400,
                        ),
                    );
                    return false;
                }
            } else if token.eq_ignore_ascii_case(b"set") {
                if let Some(var_token) = tokens.next_token() {
                    if set_to_array(e, buf, &var_token) {
                        show_error(
                            e,
                            &compiler::format(
                                "Error line %d:\nSet may not be used to assign to an array variable",
                                &[compiler::A::D(number)],
                                0x400,
                            ),
                        );
                        return false;
                    }
                }
            } else if token.eq_ignore_ascii_case(b"return") && loop_depth != 0 {
                show_error(
                    e,
                    &compiler::format(
                        "Error line %d:\nReturn cannot be called within the body of a loop.",
                        &[compiler::A::D(number)],
                        0x400,
                    ),
                );
                return false;
            } else {
                let var_type = compiler::variable_type_by_name(e, &token);
                if var_type != vt::INVALID {
                    let rest = tokens.rest_of_line();
                    // `int i = 0` and the like are left alone.
                    let plain = rest.iter().all(|&c| {
                        c.is_ascii_alphanumeric() || c == b'_' || c == b',' || is_space(c)
                    });
                    if plain {
                        for part in rest.split(|&c| c == b',') {
                            let name: Vec<u8> =
                                part.iter().copied().filter(|&c| !is_space(c)).collect();
                            if !validate_variable(e, &name, var_type, script) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        if more {
            more = lines.advance();
        }
    }
    if !blocks.is_empty() {
        show_error(e, b"Error: Mismatched block structure.");
        return false;
    }
    true
}

/// The preprocessor's `set` check: whether the variable (`name`, or `form.name` of the script
/// of the form with that editor id) is declared an array variable.
fn set_to_array(e: &mut Engine, buf: u32, var_token: &[u8]) -> bool {
    let mut name = var_token.to_vec();
    let mut text = opt_text(e, e.mem.u32(buf + sb::TEXT));
    if let Some(dot) = var_token.iter().position(|&c| c == b'.') {
        text = None;
        let form = compiler::form_by_editor_id(e, &var_token[..dot]);
        if form != 0 {
            let script = compiler::script_of_form(e, form);
            if script != 0 {
                text = opt_text(e, e.mem.u32(script + sc::TEXT));
            }
        }
        name = var_token[dot + 1..].to_vec();
    }
    let Some(text) = text else {
        return false;
    };
    let current = e.mem.u32(buf + sb::CURRENT_SCRIPT);
    declared_variable_type(e, &name, &text, current) == vt::ARRAY
}

/// The scripts being compiled, the innermost last (`g_currentScriptStack`).
pub fn compiling(e: &mut Engine) -> Vec<u32> {
    state(e).scripts.clone()
}
