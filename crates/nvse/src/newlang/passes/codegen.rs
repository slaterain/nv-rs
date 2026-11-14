//! `Compiler` (`Compiler/Passes/Compiler.cpp`, `Compiler.h` (xNVSE)): the checked tree written as
//! the script's compiled data, in the bytecode the old syntax compiles to: event blocks and a
//! function block as the game's `Begin`/`End`, statements as calls of xNVSE's commands (`Let`,
//! `eval`, `ForEach`/`ForEachAlt` and `loop`, `Jmp`/`Jmp_If_False` for `if` and `while`,
//! `SetFunctionValue` and the game's `Return`, `Call`, `Ternary`, `MatchesAnyOf`, `ar_Exists`,
//! `ar_List`/`ar_Map`, `PluginVersion` for each plugin the script needs), expressions as xNVSE's
//! expression tokens, lambdas as nested scripts.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). Jump targets are offsets from the start of
//! the script (or of the lambda's script). The statement counter xNVSE keeps is never read for
//! the output and is left out. A `std::runtime_error` the pass throws (an unknown or not
//! persistent reference, a call with no record) is caught by nothing in xNVSE: a crash.

use crate::compiler::vt;
use crate::newlang::ast::*;
use crate::newlang::lexer::TokenType;
use crate::newlang::passes::token_op;
use crate::newlang::{script_type_of, Env};
use std::collections::BTreeSet;

/// xNVSE's opcodes the pass writes (`OPCodes`).
pub mod opcode {
    pub const LET: u16 = 0x1539;
    pub const EVAL: u16 = 0x153a;
    pub const LOOP: u16 = 0x153c;
    pub const FOREACH: u16 = 0x153d;
    pub const FOREACH_ALT: u16 = 0x1670;
    pub const AR_EXISTS: u16 = 0x1671;
    pub const TERNARY: u16 = 0x166e;
    pub const CALL: u16 = 0x1545;
    pub const SET_FUNCTION_VALUE: u16 = 0x1546;
    pub const MATCHES_ANY: u16 = 0x166f;
    pub const AR_LIST: u16 = 0x1567;
    pub const AR_MAP: u16 = 0x1568;
    pub const CONTINUE: u16 = 0x153e;
    pub const BREAK: u16 = 0x153f;
    pub const VERSION: u16 = 0x1676;
}

/// `ScriptStatementCode`: `Begin`, `End`, `ScriptName`, `Return`.
const BEGIN: u16 = 0x10;
const END: u32 = 0x11;
const SCRIPT_NAME: u32 = 0x1d;
const RETURN: u32 = 0x1e;
/// The function block's mode (`OP_MODE` 0x0D) and its bytecode version.
const FUNCTION_MODE: u16 = 0x0d;
/// `kParamType_Integer`.
const PARAM_INTEGER: u32 = 1;
/// `kOpType_In`.
const OP_IN: u8 = crate::expr::op::IN;

/// `CallBuffer`: a call being written.
#[derive(Clone, Copy)]
struct CallBuffer {
    start_pos: usize,
    start_patch: usize,
    arg_pos: usize,
    num_args: u32,
    arg_start: usize,
    arg_patch: usize,
    default_parse: bool,
    stack_ref: bool,
}

pub struct Compiler<'a> {
    pub ast: &'a mut Ast,
    env: &'a mut dyn Env,
    pub data: Vec<u8>,
    calls: Vec<CallBuffer>,
    inside_enhanced_loop: Vec<bool>,
    continue_stack: Vec<u32>,
    loop_end_stack: Vec<Vec<usize>>,
    inside_nvse_expr: Vec<bool>,
    /// The names read (`usedVars`: a variable's name, or an identifier's own for a form).
    pub used_vars: BTreeSet<Vec<u8>>,
    script_start: Vec<usize>,
}

type R = Result<(), Fail>;

impl<'a> Compiler<'a> {
    pub fn new(ast: &'a mut Ast, env: &'a mut dyn Env) -> Self {
        Compiler {
            ast,
            env,
            data: Vec::new(),
            calls: Vec::new(),
            inside_enhanced_loop: Vec::new(),
            continue_stack: Vec::new(),
            loop_end_stack: Vec::new(),
            inside_nvse_expr: Vec::new(),
            used_vars: BTreeSet::new(),
            script_start: Vec::new(),
        }
    }

    fn add_u8(&mut self, b: u8) -> usize {
        self.data.push(b);
        self.data.len() - 1
    }
    fn add_u16(&mut self, v: u16) -> usize {
        self.data.extend_from_slice(&v.to_le_bytes());
        self.data.len() - 2
    }
    fn add_u32(&mut self, v: u32) -> usize {
        self.data.extend_from_slice(&v.to_le_bytes());
        self.data.len() - 4
    }
    fn add_f64(&mut self, v: f64) {
        self.add_u8(b'Z');
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    /// `AddString`: `S`, the length (16 bits), the bytes.
    fn add_string(&mut self, s: &[u8]) {
        self.add_u8(b'S');
        self.add_u16(s.len() as u16);
        self.data.extend_from_slice(s);
    }
    fn set_u8(&mut self, at: usize, v: u8) {
        self.data[at] = v;
    }
    fn set_u16(&mut self, at: usize, v: u16) {
        self.data[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn set_u32(&mut self, at: usize, v: u32) {
        self.data[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn inside(&self) -> bool {
        self.inside_nvse_expr.last().copied().unwrap_or(false)
    }
    fn start(&self) -> usize {
        self.script_start.last().copied().unwrap_or(0)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::Visit.
    /// The pass: the script name statement, a `PluginVersion` call per required plugin, the
    /// globals and the blocks.
    pub fn run(&mut self) -> R {
        self.inside_nvse_expr.push(false);
        self.script_start.push(0);
        // The name looked up among the forms (xNVSE's error for a name another form has is
        // commented out): only a reference that is not persistent ends the compile there.
        let name = self.ast.name.clone();
        self.resolve_obj_reference(&name, false)?;
        self.add_u32(SCRIPT_NAME);
        let requirements: Vec<(Vec<u8>, u32)> = self
            .ast
            .requirements
            .iter()
            .map(|(k, v)| (k.to_vec(), *v))
            .collect();
        for (plugin, version) in requirements {
            self.start_call_opcode(opcode::VERSION, None)?;
            self.start_manual_arg()?;
            self.add_string(&plugin);
            self.finish_manual_arg()?;
            self.start_manual_arg()?;
            self.add_u8(b'L');
            self.add_u32(version);
            self.finish_manual_arg()?;
            self.finish_call()?;
        }
        let globals = self.ast.globals.clone();
        for g in globals {
            self.visit_stmt(g)?;
        }
        let blocks = self.ast.blocks.clone();
        for b in blocks {
            self.visit_stmt(b)?;
        }
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::StartCall.
    /// A command's opcode (`X` or, with a calling reference on the stack, `x` and the script
    /// variable word first inside an expression), its length word and its argument count (16
    /// bits for a default-parsed command, 8 otherwise).
    fn start_call(&mut self, cmd: &CmdInfo, stack_ref: Option<ExprId>) -> R {
        if let Some(r) = stack_ref {
            self.visit_expr(r)?;
        }
        let inside = self.inside();
        if inside {
            self.add_u8(if stack_ref.is_some() { b'x' } else { b'X' });
            self.add_u16(0);
        }
        self.add_u16(cmd.opcode);
        let start_pos = self.data.len() + if inside { 0 } else { 2 };
        let start_patch = self.add_u16(0);
        let default_parse = cmd.parse == Parse::Default;
        let arg_pos = if default_parse {
            self.add_u16(0)
        } else {
            self.add_u8(0)
        };
        self.calls.push(CallBuffer {
            start_pos,
            start_patch,
            arg_pos,
            num_args: 0,
            arg_start: 0,
            arg_patch: 0,
            default_parse,
            stack_ref: stack_ref.is_some(),
        });
        Ok(())
    }

    /// `StartCall(opcode)`: the record `GetByOpcode` finds.
    fn start_call_opcode(&mut self, op: u16, stack_ref: Option<ExprId>) -> R {
        let cmd = self
            .env
            .command_by_opcode(op)
            .ok_or(Fail::Crash("StartCall: no command record"))?;
        self.start_call(&cmd, stack_ref)
    }

    fn top_call(&mut self) -> Result<&mut CallBuffer, Fail> {
        self.calls.last_mut().ok_or(Fail::Crash("no open call"))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::FinishCall.
    fn finish_call(&mut self) -> R {
        let buf = self.calls.pop().ok_or(Fail::Crash("no open call"))?;
        let len = (self.data.len() - buf.start_pos) as u16;
        self.set_u16(buf.start_patch, len);
        if buf.default_parse {
            self.set_u16(buf.arg_pos, buf.num_args as u16);
        } else {
            self.set_u8(buf.arg_pos, buf.num_args as u8);
        }
        if buf.stack_ref {
            self.add_u8(token_op(TokenType::Dot));
        }
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::AddCallArg.
    /// An argument: (for a default-parsed command `0xFFFF` first) its length word and its
    /// expression.
    fn add_call_arg(&mut self, arg: ExprId) -> R {
        if self.top_call()?.default_parse {
            self.add_u16(0xffff);
        }
        let start = self.data.len();
        let patch = self.add_u16(0);
        self.inside_nvse_expr.push(true);
        self.visit_expr(arg)?;
        self.inside_nvse_expr.pop();
        let len = (self.data.len() - start) as u16;
        self.set_u16(patch, len);
        self.top_call()?.num_args += 1;
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
    // Compiler::StartManualArg, Compiler::FinishManualArg.
    fn start_manual_arg(&mut self) -> R {
        if self.top_call()?.default_parse {
            self.add_u16(0xffff);
        }
        let start = self.data.len();
        let patch = self.add_u16(0);
        let top = self.top_call()?;
        top.arg_start = start;
        top.arg_patch = patch;
        self.inside_nvse_expr.push(true);
        Ok(())
    }

    fn finish_manual_arg(&mut self) -> R {
        self.inside_nvse_expr.pop();
        let top = *self.top_call()?;
        let len = (self.data.len() - top.arg_start) as u16;
        self.set_u16(top.arg_patch, len);
        self.top_call()?.num_args += 1;
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::CreateJMP,
    // Compiler::CreateJNE.
    /// A `Jmp` call with a placeholder target (`L` and 32 bits); the target's place.
    fn create_jmp(&mut self) -> Result<usize, Fail> {
        let cmd = self
            .env
            .command(b"jmp")
            .ok_or(Fail::Crash("GetByName(\"jmp\"): no record"))?;
        self.start_call(&cmd, None)?;
        self.start_manual_arg()?;
        self.add_u8(b'L');
        let off = self.add_u32(0);
        self.finish_manual_arg()?;
        self.finish_call()?;
        Ok(off)
    }

    /// A `Jmp_If_False` call on `cond` with a placeholder target; the target's place.
    fn create_jne(&mut self, cond: ExprId) -> Result<usize, Fail> {
        let cmd = self
            .env
            .command(b"jmp_if_false")
            .ok_or(Fail::Crash("GetByName(\"jmp_if_false\"): no record"))?;
        self.start_call(&cmd, None)?;
        self.add_call_arg(cond)?;
        self.start_manual_arg()?;
        self.add_u8(b'L');
        let off = self.add_u32(0);
        self.finish_manual_arg()?;
        self.finish_call()?;
        Ok(off)
    }

    /// The offset of the data's end from the start of the script being written.
    fn here(&self) -> u32 {
        (self.data.len() - self.start()) as u32
    }

    /// A function or lambda's header: `Begin` with mode 0x0D, the length word of the mode's
    /// part, the script length's place, version 1, the parameters (index and type each) and no
    /// array parameters. The places of the script length and of where it counts from.
    fn function_header(&mut self, args: &[StmtId]) -> Result<(usize, usize), Fail> {
        self.add_u16(BEGIN);
        let mode_len_patch = self.add_u16(0);
        let mode_start = self.data.len();
        self.add_u16(FUNCTION_MODE);
        let script_len_patch = self.add_u32(0);
        self.add_u8(1);
        self.add_u8(args.len() as u8);
        for &a in args {
            for d in self.ast.decls(a).to_vec() {
                let var = need(d.info)?;
                let (index, ty) = (self.ast.var(var).index, self.ast.var(var).ty);
                self.add_u16(index as u16);
                self.add_u8(ty);
            }
        }
        self.add_u8(0);
        let script_len_start = self.data.len();
        let len = (self.data.len() - mode_start) as u16;
        self.set_u16(mode_len_patch, len);
        Ok((script_len_patch, script_len_start))
    }

    fn visit_stmt(&mut self, id: StmtId) -> R {
        let kind = self.ast.stmt(id).kind.clone();
        match kind {
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitBeginStmt.
            StmtKind::Begin {
                param, block, info, ..
            } => {
                self.add_u16(BEGIN);
                let begin_patch = self.add_u16(0);
                let begin_start = self.data.len();
                self.add_u16(info.opcode);
                let block_patch = self.add_u32(0);
                if let Some(param) = param {
                    self.add_u16(1);
                    if info.params.first().map(|p| p.kind) == Some(PARAM_INTEGER) {
                        let v = param
                            .number()
                            .ok_or(Fail::Crash("std::get<double>: bad_variant_access"))?;
                        self.add_u8(b'n');
                        self.add_u32(crate::commands::string::dtoui3(v));
                    } else {
                        let r = self.resolve_obj_reference(&param.lexeme, true)?;
                        if r == 0 {
                            return Err(Fail::Crash("Unable to resolve form"));
                        }
                        self.add_u8(b'r');
                        self.add_u16(r);
                    }
                } else if !info.params.is_empty() {
                    self.add_u16(0);
                }
                let len = (self.data.len() - begin_start) as u16;
                self.set_u16(begin_patch, len);
                let block_start = self.data.len();
                self.visit_stmt(block)?;
                self.add_u32(END);
                let len = (self.data.len() - block_start) as u32;
                self.set_u32(block_patch, len);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitFnStmt.
            StmtKind::Udf { args, body, .. } => {
                let (patch, start) = self.function_header(&args)?;
                self.visit_stmt(body)?;
                self.add_u32(END);
                let len = (self.data.len() - start) as u32;
                self.set_u32(patch, len);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitVarDeclStmt.
            // Each declaration with a value: `Let` of the variable token and the value with
            // the assignment operator.
            StmtKind::VarDecl { ty, decls } => {
                let var_type = script_type_of(ty);
                for d in decls {
                    let Some(value) = d.value else {
                        continue;
                    };
                    let index = self.ast.var(need(d.info)?).index;
                    self.start_call_opcode(opcode::LET, None)?;
                    self.start_manual_arg()?;
                    self.add_u8(b'V');
                    self.add_u8(var_type);
                    self.add_u16(0);
                    self.add_u16(index as u16);
                    self.visit_expr(value)?;
                    self.add_u8(0);
                    self.finish_manual_arg()?;
                    self.finish_call()?;
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitExprStmt.
            StmtKind::Expr(e) => {
                let e = e.ok_or(Fail::Crash(
                    "VisitExprStmt on an empty expression statement",
                ))?;
                if matches!(
                    self.ast.expr(e).kind,
                    ExprKind::Call { .. } | ExprKind::Assign { .. }
                ) {
                    return self.visit_expr(e);
                }
                self.start_call_opcode(opcode::EVAL, None)?;
                self.add_call_arg(e)?;
                self.finish_call()
            }
            StmtKind::For { .. } => Err(Fail::Crash("Compiler::VisitForStmt was called")),
            StmtKind::ForEach {
                decls,
                rhs,
                block,
                decompose,
            } => self.visit_for_each(&decls, rhs, block, decompose),
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitIfStmt.
            StmtKind::If {
                cond,
                block,
                else_block,
            } => {
                let to_else = self.create_jne(need(cond)?)?;
                self.visit_stmt(block)?;
                let to_end = match else_block {
                    Some(_) => self.create_jmp()?,
                    None => 0,
                };
                let here = self.here();
                self.set_u32(to_else, here);
                if let Some(e) = else_block {
                    self.visit_stmt(e)?;
                    let here = self.here();
                    self.set_u32(to_end, here);
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitReturnStmt.
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    self.start_call_opcode(opcode::SET_FUNCTION_VALUE, None)?;
                    self.add_call_arg(e)?;
                    self.finish_call()?;
                }
                self.add_u32(RETURN);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitContinueStmt, Compiler::VisitBreakStmt.
            StmtKind::Continue => {
                if self.enhanced_loop()? {
                    let target = *self
                        .continue_stack
                        .last()
                        .ok_or(Fail::Crash("empty stack"))?;
                    let off = self.create_jmp()?;
                    self.set_u32(off, target);
                } else {
                    self.add_u32(u32::from(opcode::CONTINUE));
                }
                Ok(())
            }
            StmtKind::Break => {
                if self.enhanced_loop()? {
                    let off = self.create_jmp()?;
                    self.loop_end_stack
                        .last_mut()
                        .ok_or(Fail::Crash("empty stack"))?
                        .push(off);
                } else {
                    self.add_u32(u32::from(opcode::BREAK));
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitWhileStmt.
            // `Jmp_If_False` to the end, the block, `Jmp` back to the condition; a `break`
            // jumps to the end, a `continue` to the condition.
            StmtKind::While { cond, block } => {
                let loop_start = self.here();
                let end_patch = self.create_jne(need(cond)?)?;
                self.inside_enhanced_loop.push(true);
                self.continue_stack.push(loop_start);
                self.loop_end_stack.push(Vec::new());
                self.visit_stmt(block)?;
                let back = self.create_jmp()?;
                self.set_u32(back, loop_start);
                let end = self.here();
                self.set_u32(end_patch, end);
                for off in self.loop_end_stack.pop().unwrap_or_default() {
                    self.set_u32(off, end);
                }
                self.continue_stack.pop();
                self.inside_enhanced_loop.pop();
                Ok(())
            }
            StmtKind::Block(stmts) => {
                for s in stmts {
                    self.visit_stmt(s)?;
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitShowMessageStmt.
            StmtKind::ShowMessage {
                message,
                args,
                time,
            } => {
                let op = self
                    .env
                    .command(b"ShowMessage")
                    .ok_or(Fail::Crash("GetByName(\"ShowMessage\"): no record"))?
                    .opcode;
                self.add_u16(op);
                let patch = self.add_u16(0);
                let start = self.data.len();
                self.add_u16(1);
                self.add_u8(b'r');
                let name = self.ast.ident_name(message).to_vec();
                let r = self.resolve_obj_reference(&name, true)?;
                self.add_u16(r);
                self.add_u16(args.len() as u16);
                for a in args {
                    let var = need(self.ast.ident_var(a))?;
                    let v = self.ast.var(var);
                    let (ty, index) = (v.ty, v.index);
                    self.add_u8(if ty == vt::FLOAT { b'f' } else { b's' });
                    self.add_u16(index as u16);
                }
                self.add_u32(time);
                let len = (self.data.len() - start) as u16;
                self.set_u16(patch, len);
                Ok(())
            }
            StmtKind::Match { .. } => Ok(()),
        }
    }

    fn enhanced_loop(&self) -> Result<bool, Fail> {
        self.inside_enhanced_loop
            .last()
            .copied()
            .ok_or(Fail::Crash("empty stack"))
    }

    /// A variable token (`V`, the type, no reference, the index) for a declaration.
    fn var_token(&mut self, var: Option<VarId>) {
        match var {
            Some(v) => {
                let (ty, index) = (self.ast.var(v).ty, self.ast.var(v).index);
                self.add_u8(b'V');
                self.add_u8(ty);
                self.add_u16(0);
                self.add_u16(index as u16);
            }
            None => {
                self.add_u8(b'K');
            }
        }
    }

    /// An argument of its own length word (`ForEachAlt`'s).
    fn length_arg(
        &mut self,
        exprs_start: usize,
        expr_patch: usize,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let start = self.data.len();
        let patch = self.add_u16(0);
        f(self)?;
        let len = (self.data.len() - start) as u16;
        self.set_u16(patch, len);
        let len = (self.data.len() - exprs_start) as u16;
        self.set_u16(expr_patch, len);
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
    // Compiler::VisitForEachStmt.
    /// `for (type x in a)`: `ForEach` of `x <- a` (the `In` operator); `for ([k, v] in a)`:
    /// `ForEachAlt` of the array, the value's variable, then the key's (`K` for one left out).
    /// Then the block and `loop`; the jump word is the offset past the loop.
    fn visit_for_each(
        &mut self,
        decls: &[Option<StmtId>],
        rhs: ExprId,
        block: StmtId,
        decompose: bool,
    ) -> R {
        let var_of = |ast: &Ast, d: Option<StmtId>| {
            d.and_then(|d| ast.decls(d).first().and_then(|x| x.info))
        };
        if decompose {
            let var1 = var_of(self.ast, decls.first().copied().flatten());
            let var2 = if decls.len() == 2 {
                var_of(self.ast, decls[1])
            } else {
                None
            };
            self.add_u16(opcode::FOREACH_ALT);
            let expr_patch = self.add_u16(0);
            let expr_start = self.data.len();
            let jmp_patch = self.add_u32(0);
            self.add_u8(1 + decls.len() as u8);
            self.inside_nvse_expr.push(true);
            self.length_arg(expr_start, expr_patch, |c| c.visit_expr(rhs))?;
            if decls.len() == 2 {
                self.length_arg(expr_start, expr_patch, |c| {
                    c.var_token(var2);
                    Ok(())
                })?;
                self.length_arg(expr_start, expr_patch, |c| {
                    c.var_token(var1);
                    Ok(())
                })?;
            } else {
                self.length_arg(expr_start, expr_patch, |c| {
                    let (ty, index) = match var1 {
                        Some(v) => (c.ast.var(v).ty, c.ast.var(v).index as u16),
                        None => (0, 0),
                    };
                    c.add_u8(b'V');
                    c.add_u8(ty);
                    c.add_u16(0);
                    c.add_u16(index);
                    Ok(())
                })?;
            }
            self.inside_nvse_expr.pop();
            self.loop_block(block)?;
            let at = self.here();
            self.set_u32(jmp_patch, at);
        } else {
            let decl = decls
                .first()
                .copied()
                .flatten()
                .ok_or(Fail::Crash("Unexpected compiler error."))?;
            let var = need(self.ast.decls(decl).first().and_then(|d| d.info))?;
            self.add_u16(opcode::FOREACH);
            let expr_patch = self.add_u16(0);
            let expr_start = self.data.len();
            let jmp_patch = self.add_u32(0);
            self.add_u8(1);
            let arg_start = self.data.len();
            let arg_patch = self.add_u16(0);
            self.inside_nvse_expr.push(true);
            self.var_token(Some(var));
            self.visit_expr(rhs)?;
            self.add_u8(OP_IN);
            self.inside_nvse_expr.pop();
            let len = (self.data.len() - arg_start) as u16;
            self.set_u16(arg_patch, len);
            let len = (self.data.len() - expr_start) as u16;
            self.set_u16(expr_patch, len);
            self.loop_block(block)?;
            let at = self.here();
            self.set_u32(jmp_patch, at);
        }
        Ok(())
    }

    /// A `ForEach`'s block (not an enhanced loop: `continue` and `break` are xNVSE's commands
    /// there), then `loop`.
    fn loop_block(&mut self, block: StmtId) -> R {
        self.inside_enhanced_loop.push(false);
        self.visit_stmt(block)?;
        self.inside_enhanced_loop.pop();
        self.add_u16(opcode::LOOP);
        self.add_u16(0);
        Ok(())
    }

    fn visit_expr(&mut self, id: ExprId) -> R {
        let kind = self.ast.expr(id).kind.clone();
        match kind {
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitAssignmentExpr.
            ExprKind::Assign { token, left, value } => {
                let op = token_op(token.ty);
                if !self.inside() {
                    self.start_call_opcode(opcode::LET, None)?;
                    self.start_manual_arg()?;
                    self.visit_expr(left)?;
                    self.visit_expr(value)?;
                    self.add_u8(op);
                    self.finish_manual_arg()?;
                    self.finish_call()
                } else {
                    self.visit_expr(left)?;
                    self.visit_expr(value)?;
                    self.add_u8(op);
                    Ok(())
                }
            }
            ExprKind::Ternary {
                cond, left, right, ..
            } => {
                self.start_call_opcode(opcode::TERNARY, None)?;
                self.add_call_arg(cond)?;
                self.add_call_arg(left)?;
                self.add_call_arg(right)?;
                self.finish_call()
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitInExpr.
            ExprKind::In {
                lhs,
                values,
                expression,
                is_not,
                ..
            } => {
                if !values.is_empty() {
                    self.start_call_opcode(opcode::MATCHES_ANY, None)?;
                    self.add_call_arg(lhs)?;
                    for v in values {
                        self.add_call_arg(v)?;
                    }
                    self.finish_call()?;
                } else {
                    self.start_call_opcode(opcode::AR_EXISTS, None)?;
                    self.add_call_arg(need(expression)?)?;
                    self.add_call_arg(lhs)?;
                    self.finish_call()?;
                }
                if is_not {
                    self.add_u8(token_op(TokenType::Bang));
                }
                Ok(())
            }
            ExprKind::Binary { op, left, right } => {
                self.visit_expr(left)?;
                self.visit_expr(right)?;
                self.add_u8(token_op(op.ty));
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitUnaryExpr.
            // A prefix operator after its operand; `x++` as `x x 1 + :=` then `1 -` (the old
            // value), `x--` the other way round.
            ExprKind::Unary { op, expr, postfix } => {
                if postfix {
                    self.visit_expr(expr)?;
                    self.visit_expr(expr)?;
                    self.add_u8(b'B');
                    self.add_u8(1);
                    let plus = op.ty == TokenType::PlusPlus;
                    self.add_u8(token_op(if plus {
                        TokenType::Plus
                    } else {
                        TokenType::Minus
                    }));
                    self.add_u8(token_op(TokenType::Eq));
                    self.add_u8(b'B');
                    self.add_u8(1);
                    self.add_u8(token_op(if plus {
                        TokenType::Minus
                    } else {
                        TokenType::Plus
                    }));
                } else {
                    self.visit_expr(expr)?;
                    self.add_u8(token_op(op.ty));
                }
                Ok(())
            }
            ExprKind::Subscript { left, index, .. } => {
                self.visit_expr(left)?;
                self.visit_expr(index)?;
                self.add_u8(token_op(TokenType::LeftBracket));
                Ok(())
            }
            ExprKind::Call { .. } => self.visit_call(id),
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitGetExpr.
            ExprKind::Get {
                var_info,
                reference_name,
                ..
            } => {
                let name = reference_name.ok_or(Fail::Crash("GetExpr without a reference"))?;
                let r = self.resolve_obj_reference(&name, true)?;
                if r == 0 {
                    return Err(Fail::Crash("Unable to resolve reference"));
                }
                let (ty, index) = var_info.ok_or(Fail::Crash("GetExpr without a variable"))?;
                self.add_u8(b'V');
                self.add_u8(ty);
                self.add_u16(r);
                self.add_u16(index);
                Ok(())
            }
            ExprKind::Bool(b) => {
                self.add_u8(b'B');
                self.add_u8(u8::from(b));
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitNumberExpr.
            // A double (`Z`), an enum's raw bytes (one or two), or the smallest integer form:
            // `B` to 255, `I` to 65535, else `L`.
            ExprKind::Number {
                value,
                is_fp,
                enum_len,
            } => {
                if is_fp {
                    self.add_f64(value);
                } else if enum_len != 0 {
                    if enum_len == 1 {
                        self.add_u8(crate::args::f64_to_i32(value) as u8);
                    } else {
                        self.add_u16(crate::args::f64_to_i32(value) as u16);
                    }
                } else if value <= f64::from(u8::MAX) {
                    self.add_u8(b'B');
                    self.add_u8(crate::args::f64_to_i32(value) as u8);
                } else if value <= f64::from(u16::MAX) {
                    self.add_u8(b'I');
                    self.add_u16(crate::args::f64_to_i32(value) as u16);
                } else {
                    self.add_u8(b'L');
                    self.add_u32(crate::commands::string::dtoui3(value));
                }
                Ok(())
            }
            ExprKind::Str(s) => {
                self.add_string(&s);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitIdentExpr.
            // A variable (`V`), else a form of the script's references (`G` for a global, `R`
            // otherwise, the reference's index); nothing for a name that resolves to neither.
            ExprKind::Ident { name, var, .. } => {
                let name = match var {
                    Some(v) => self.ast.var(v).remapped_name.clone(),
                    None => name,
                };
                self.used_vars.insert(name.clone());
                if let Some(v) = var {
                    let (ty, index) = (self.ast.var(v).ty, self.ast.var(v).index);
                    self.add_u8(b'V');
                    self.add_u8(ty);
                    self.add_u16(0);
                    self.add_u16(index as u16);
                } else {
                    let r = self.resolve_obj_reference(&name, true)?;
                    if r != 0 {
                        let global = self.env.reference_is_global(r);
                        self.add_u8(if global { b'G' } else { b'R' });
                        self.add_u16(r);
                    }
                }
                Ok(())
            }
            ExprKind::MapLit(values) | ExprKind::ArrayLit(values) => {
                let op = if matches!(self.ast.expr(id).kind, ExprKind::MapLit(_)) {
                    opcode::AR_MAP
                } else {
                    opcode::AR_LIST
                };
                self.start_call_opcode(op, None)?;
                for v in values {
                    self.add_call_arg(v)?;
                }
                self.finish_call()
            }
            ExprKind::Grouping(inner) => self.visit_expr(need(inner)?),
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp:
            // Compiler::VisitLambdaExpr.
            // `F`, the length (32 bits), then a script of its own: the script name statement,
            // the function header, the body, `End`; jumps inside count from its start.
            ExprKind::Lambda { args, body, .. } => {
                self.add_u8(b'F');
                let size_patch = self.add_u32(0);
                let arg_start = self.data.len();
                self.script_start.push(self.data.len());
                self.add_u32(SCRIPT_NAME);
                let (patch, start) = self.function_header(&args)?;
                self.inside_nvse_expr.push(false);
                self.visit_stmt(body)?;
                self.inside_nvse_expr.pop();
                self.add_u32(END);
                let len = (self.data.len() - start) as u32;
                self.set_u32(patch, len);
                self.script_start.pop();
                let len = (self.data.len() - arg_start) as u32;
                self.set_u32(size_patch, len);
                Ok(())
            }
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Compiler.cpp: Compiler::VisitCallExpr.
    /// `Call` (its own form: version 1, the callee's expression, the count, each argument with
    /// its length word); a method call outside an expression wrapped in `eval`; any other call
    /// with its calling reference on the stack and each argument (an enum's raw bytes as they
    /// are).
    fn visit_call(&mut self, id: ExprId) -> R {
        let ExprKind::Call {
            left, args, cmd, ..
        } = self.ast.expr(id).kind.clone()
        else {
            return Ok(());
        };
        let cmd = cmd.ok_or(Fail::Crash("expr->cmdInfo was null"))?;
        if cmd.parse == Parse::Call {
            let inside = self.inside();
            if inside {
                self.add_u8(b'X');
                self.add_u16(0);
            }
            self.add_u16(opcode::CALL);
            let len_start = self.data.len() + if inside { 0 } else { 2 };
            let len_patch = self.add_u16(0);
            self.inside_nvse_expr.push(true);
            self.add_u8(1);
            let expr_start = self.data.len();
            let expr_patch = self.add_u16(0);
            let callee = *args.first().ok_or(Fail::Crash("Call without a callee"))?;
            self.visit_expr(callee)?;
            let len = (self.data.len() - expr_start) as u16;
            self.set_u16(expr_patch, len);
            self.add_u8((args.len() - 1) as u8);
            for &a in &args[1..] {
                let start = self.data.len();
                let patch = self.add_u16(0);
                self.visit_expr(a)?;
                let len = (self.data.len() - start) as u16;
                self.set_u16(patch, len);
            }
            let len = (self.data.len() - len_start) as u16;
            self.set_u16(len_patch, len);
            self.inside_nvse_expr.pop();
            return Ok(());
        }
        if left.is_some() && !self.inside() {
            self.start_call_opcode(opcode::EVAL, None)?;
            self.start_manual_arg()?;
            self.visit_expr(id)?;
            self.finish_manual_arg()?;
            return self.finish_call();
        }
        self.start_call(&cmd, left)?;
        for a in args {
            if matches!(self.ast.expr(a).kind, ExprKind::Number { enum_len, .. } if enum_len > 0) {
                self.visit_expr(a)?;
                self.top_call()?.num_args += 1;
            } else {
                self.add_call_arg(a)?;
            }
        }
        self.finish_call()
    }

    /// `ResolveObjReference`: the reference index of the form `name` names (`player` the
    /// player's reference), added to the script's references when new and `add`; 0 for no form.
    fn resolve_obj_reference(&mut self, name: &[u8], add: bool) -> Result<u16, Fail> {
        self.env.resolve_reference(name, add)
    }
}

#[cfg(test)]
#[path = "codegen_tests.rs"]
mod tests;
