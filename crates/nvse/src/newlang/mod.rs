//! xNVSE's new script language (ADR-0016 §I-14; plan step 3.8, its last part): scripts whose
//! text starts with `name`, compiled by xNVSE's second compiler (`Compiler/` (xNVSE): a lexer, a
//! parser, a syntax tree, passes over it and a code generator) into the same bytecode the old
//! syntax compiles to, which the evaluator ([`crate::expr`]) runs and the decompiler
//! ([`crate::decompile`]) reads back.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a): `Compiler/Lexer`, `Compiler/Parser`,
//! `Compiler/AST`, `Compiler/Passes` (`MatchTransformer`, `VariableResolution`,
//! `CallTransformer`, `TypeChecker`), `Compiler/Utils.cpp` (`HighlightSourceSpan`, the token to
//! operator map, `ResolveVanillaEum`, `DoesFormTypeMatchParamType`) and `SourceInfo.h`.
//!
//! The passes run in xNVSE's order (`CompileNVSEScript`): parse, match statements made `if`
//! chains, variables resolved, calls of lambdas and user functions made `Call`s, types checked.
//! What they need of the game (command records, forms by editor id, the script's variables, the
//! event block records, the plugin list) comes through [`Env`]; [`engine_env`] reads it from the
//! running game as xNVSE does.
//!
//! What a player or mod sees of the compiler is its error messages: at run time xNVSE prints them
//! with `ShowRuntimeError` (the header "Error in script 00000000 in mod Unknown or deleted
//! script", then the message) to the console. The type checker's and the variable resolution's
//! messages carry the source lines with the faulty part between ANSI colour escapes, as xNVSE
//! prints them; every error the type checker catches is followed by a second, empty message (its
//! exception's empty text, printed where it is caught). The rest of xNVSE's output here goes to
//! its debug console or `nvse.log` only (`DbgPrint`, `InfoPrint`, the tree printer, the
//! "variable shadows a form" note): a release build shows none of it and the port writes no
//! `nvse.log`, so none of it is built.
//!
//! Deliberate differences, each where xNVSE stops the game (an exception nothing catches, a read
//! through a null pointer, a format past its buffer): the port fails the compile instead, with no
//! further message ([`Fail::Crash`] names the place). Messages are held to xNVSE's 0x800-byte
//! buffer (its `vsprintf_s` would stop the game past it). A `%` in a message printed without its
//! source lines is printed as it is (xNVSE hands that text to `vsprintf` as a format).

pub mod ast;
pub mod lexer;
pub mod msvc;
pub mod parser;
pub mod passes;

pub use ast::Fail;

use crate::compiler::Code;
use ast::{BlockInfo, CmdInfo};

/// What the compiler needs of the game. [`engine_env`]'s answers come from the running game.
pub trait Env {
    /// An error message, as xNVSE's `ErrPrint`/`ErrPrintln` print it at run time
    /// (`ShowRuntimeError` with no script).
    fn error(&mut self, message: &[u8]);
    /// `g_eventBlockCommandInfos`' record whose long name is `name` (without case).
    fn block(&mut self, name: &[u8]) -> Option<BlockInfo>;
    /// `g_pluginManager.GetInfoByName(name)`.
    fn plugin_listed(&mut self, name: &[u8]) -> bool;
    /// `g_scriptCommands.GetByName(name)`.
    fn command(&mut self, name: &[u8]) -> Option<CmdInfo>;
    /// `GetFormByID(editorID)`: the form (0 for none).
    fn form_by_name(&mut self, name: &[u8]) -> u32;
    /// `LookupFormByID(id)`.
    fn form_by_id(&mut self, id: u32) -> u32;
    /// A form's type (`typeID`).
    fn form_type(&mut self, form: u32) -> u8;
    /// `DYNAMIC_CAST(form, TESForm, Script)`: the script's type (`Script::Type()`), `None` when
    /// the form is not a script.
    fn script_form_type(&mut self, form: u32) -> Option<u16>;
    /// The variable `name` of the script of the scriptable form `form` (a reference's base
    /// form's, or a quest's): `Some(None)` when it has a script without that variable, `None`
    /// when it has no script. The variable's type and index.
    fn form_script_variable(&mut self, form: u32, name: &[u8]) -> Option<Option<(u8, u16)>>;
    /// A form's editor id (`GetEditorID`).
    fn editor_id(&mut self, form: u32) -> Vec<u8>;
    /// `ResolveVanillaEum`: a vanilla parameter's enum value for an identifier and its length
    /// in bytes (`(-1, 2)` for none).
    fn vanilla_enum(&mut self, kind: u32, text: &[u8]) -> (u32, u32);
    /// `DoesFormTypeMatchParamType`.
    fn form_matches_param(&mut self, form: u32, kind: u32) -> bool;
    /// The compiling script's variables (`pScript->varList`): name and type, `None` without a
    /// script.
    fn script_variables(&mut self) -> Option<Vec<(Vec<u8>, u8)>>;
    /// The compiling script's variable and reference lists made again from the resolved
    /// variables (name, type, index), as `VariableResolution::Visit` does.
    fn set_script_variables(&mut self, vars: &[(Vec<u8>, u8, u32)]);
    /// The compiling script's type (`engineScript->Type()`), `None` without a script.
    fn script_type(&mut self) -> Option<u16>;
}

pub mod engine_env;

/// `Script::eType_Object`, `eType_Quest`, `eType_Magic`.
pub mod script_type {
    pub const OBJECT: u16 = 0;
    pub const QUEST: u16 = 1;
    pub const MAGIC: u16 = 0x100;
}

/// `ESCAPE_RED`, `ESCAPE_CYAN`, `ESCAPE_RESET`.
pub const RED: &[u8] = b"\x1B[31m";
pub const CYAN: &[u8] = b"\x1B[36m";
pub const RESET: &[u8] = b"\x1B[0m";

/// `string_view::substr(pos, n)`: past the end is `std::out_of_range`, nothing catches it.
fn substr(s: &[u8], pos: usize, n: usize) -> Result<&[u8], Fail> {
    if pos > s.len() {
        return Err(Fail::Crash("string_view::substr: out_of_range"));
    }
    let end = pos.saturating_add(n).min(s.len());
    Ok(&s[pos..end])
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utils.cpp: HighlightSourceSpan.
/// `HighlightSourceSpan`: the message, then each line of the span with its number (three wide)
/// and the span's part between `color` and the reset escape. A span with no place (a column 0)
/// gives the message and a "BUG" line. xNVSE doubles every `%` of the result for the `printf`
/// it is handed to, which undoes it: the text here is what is printed.
pub fn highlight(
    lines: &[Vec<u8>],
    msg: &[u8],
    span: lexer::Span,
    color: &[u8],
) -> Result<Vec<u8>, Fail> {
    let mut out = b"\r    | ".to_vec();
    out.extend_from_slice(msg);
    out.push(b'\n');
    let (start, end) = (span.start, span.end);
    if start.column == 0 || end.column == 0 {
        out.extend_from_slice(b"    | ");
        out.extend_from_slice(RED);
        out.extend_from_slice(b"BUG: Failed to print source information");
        out.extend_from_slice(RESET);
        out.extend_from_slice(b"\n\n");
        return Ok(out);
    }
    let mut i = start.line;
    while i <= end.line {
        let line = i
            .checked_sub(1)
            .and_then(|n| lines.get(n))
            .ok_or(Fail::Crash("HighlightSourceSpan: line out of range"))?;
        out.extend_from_slice(format!("{i:>3} | ").as_bytes());
        if i == start.line {
            if start.line == end.line {
                let preceding = substr(line, 0, start.column - 1)?;
                let shown = substr(
                    line,
                    start.column - 1,
                    end.column.wrapping_sub(start.column).wrapping_add(1),
                )?;
                let remaining: &[u8] = if end.column < line.len() {
                    substr(line, end.column, usize::MAX)?
                } else {
                    b""
                };
                out.extend_from_slice(preceding);
                out.extend_from_slice(color);
                out.extend_from_slice(shown);
                out.extend_from_slice(RESET);
                out.extend_from_slice(remaining);
            } else {
                let preceding = substr(line, 0, start.column - 1)?;
                let shown = substr(line, start.column - 1, usize::MAX)?;
                out.extend_from_slice(preceding);
                out.extend_from_slice(color);
                out.extend_from_slice(shown);
                out.extend_from_slice(RESET);
            }
        } else if i != end.line {
            out.extend_from_slice(color);
            out.extend_from_slice(line);
            out.extend_from_slice(RESET);
        } else {
            let shown = substr(line, 0, end.column)?;
            let remaining = substr(line, end.column, usize::MAX)?;
            out.extend_from_slice(color);
            out.extend_from_slice(shown);
            out.extend_from_slice(RESET);
            out.extend_from_slice(remaining);
        }
        out.push(b'\n');
        i += 1;
    }
    Ok(out)
}

/// xNVSE's `PACKED_NVSE_VERSION` (6.4.9: `MAKE_NEW_VEGAS_VERSION(6, 4, 9)`).
pub const PACKED_NVSE_VERSION: u32 = (6 << 24) | (4 << 16) | (9 << 4);

/// `MAKE_NEW_VEGAS_VERSION(major, minor, build)`.
pub fn make_version(major: i32, minor: i32, build: i32) -> u32 {
    (((major & 0xff) as u32) << 24)
        | (((minor & 0xff) as u32) << 16)
        | (((build & 0xfff) as u32) << 4)
}

/// The type a token type's variable has (`GetScriptTypeFromTokenType`).
pub fn script_type_of(t: lexer::TokenType) -> u8 {
    use crate::compiler::vt;
    use lexer::TokenType as T;
    match t {
        T::DoubleType => vt::FLOAT,
        T::IntType => vt::INTEGER,
        T::RefType => vt::REF,
        T::ArrayType => vt::ARRAY,
        T::StringType => vt::STRING,
        _ => vt::INVALID,
    }
}

/// `VariableType_To_TokenType`.
pub fn variable_token_type(t: u8) -> Code {
    use crate::compiler::{tt, vt};
    match t {
        vt::FLOAT | vt::INTEGER => tt::NUMBER,
        vt::STRING => tt::STRING,
        vt::ARRAY => tt::ARRAY,
        vt::REF => tt::REF,
        _ => tt::INVALID,
    }
}

/// What [`analyze`] gives: the checked tree, or `None` when a step reported an error (the
/// compile fails then; the messages were printed).
pub struct Analyzed {
    pub ast: Option<ast::Ast>,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utils.cpp: CompileNVSEScript (its passes
// up to the type checker).
/// The parse and the passes up to the type checker, in xNVSE's order, each run only when the
/// one before reported no error. `has_script`: a script is compiling (its variables are read and
/// made again, its type is checked against commands that need a calling reference).
pub fn analyze(env: &mut dyn Env, text: &[u8], has_script: bool) -> Result<Analyzed, Fail> {
    let parsed = parser::parse(env, text)?;
    let (Some(mut tree), false) = (parsed.ast, parsed.had_error) else {
        return Ok(Analyzed { ast: None });
    };
    let mut m = passes::match_transform::MatchTransformer::new(&mut tree);
    m.run()?;
    let mut v = passes::variables::VariableResolution::new(&mut tree, env, has_script);
    v.run()?;
    if v.had_error {
        return Ok(Analyzed { ast: None });
    }
    passes::calls::CallTransformer::new(&mut tree, env).run()?;
    let mut t =
        passes::types::TypeChecker::new(&mut tree, env, &parsed.plugin_versions, has_script);
    t.run()?;
    if t.had_error {
        return Ok(Analyzed { ast: None });
    }
    Ok(Analyzed { ast: Some(tree) })
}

#[cfg(test)]
pub(crate) mod test_env;
