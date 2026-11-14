//! `TypeChecker` (`Compiler/Passes/TypeChecker.cpp` (xNVSE)): every expression's type from the
//! operator table's rules (`s_operators`, the old syntax's [`crate::compiler`] has them), the
//! commands' parameter and return types, the variables' types; vanilla enum names given as
//! identifiers (an actor value, an axis, ...) made numbers; the plugins a script needs gathered.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). An error prints the message with the
//! source lines it is about and fails the step it is in (`TypeCheckerError`); the step that
//! catches it (`WRAP_ERROR`: each statement of a block, each block, some sub-steps) prints a
//! second, empty message, the exception's text, and goes on. An error outside any catch (in a
//! global's initializer) is an exception nothing catches in xNVSE: a crash.

use crate::compiler::{
    convert, result_type, tt, type_name, validate_arg_type, Code, NVSE_ARRAY, NVSE_FORM,
    NVSE_NUMBER, NVSE_STRING,
};
use crate::expr::op;
use crate::newlang::ast::*;
use crate::newlang::lexer::{Span, TokenType};
use crate::newlang::msvc::MsvcMap;
use crate::newlang::passes::{cat, token_op};
use crate::newlang::{error_text, script_type, Env, PACKED_NVSE_VERSION};
use std::collections::{HashMap, HashSet};

type R = Result<(), Fail>;

/// `kParamType_*` the checks name.
mod param {
    pub const STRING: u32 = 0x00;
    pub const INTEGER: u32 = 0x01;
    pub const FLOAT: u32 = 0x02;
    pub const OBJECT_REF: u32 = 0x04;
    pub const ACTOR_VALUE: u32 = 0x05;
    pub const AXIS: u32 = 0x08;
    pub const ANIMATION_GROUP: u32 = 0x0a;
    pub const SEX: u32 = 0x12;
    pub const VARIABLE_NAME: u32 = 0x16;
    pub const QUEST_STAGE: u32 = 0x17;
    pub const CRIME_TYPE: u32 = 0x1c;
    pub const MAGIC_EFFECT: u32 = 0x1f;
    pub const FORM_TYPE: u32 = 0x20;
    pub const DOUBLE: u32 = 0x2c;
    pub const SCRIPT_VARIABLE: u32 = 0x2d;
    pub const REGION: u32 = 0x45;
    pub const ARRAY: u32 = 0x100;
}

/// `GetBasicTokenType`.
pub fn basic_type(t: Code) -> Code {
    match t {
        tt::ARRAY | tt::ARRAY_VAR => tt::ARRAY,
        tt::NUMBER | tt::NUMERIC_VAR => tt::NUMBER,
        tt::STRING | tt::STRING_VAR => tt::STRING,
        tt::REF | tt::REF_VAR => tt::REF,
        other => other,
    }
}

/// `NVSETokenType_To_TokenType`: the type a declaration of `t` gives its variable.
pub fn declared_type(t: TokenType) -> Code {
    match t {
        TokenType::StringType => tt::STRING_VAR,
        TokenType::ArrayType => tt::ARRAY_VAR,
        TokenType::RefType => tt::REF_VAR,
        TokenType::DoubleType | TokenType::IntType => tt::NUMERIC_VAR,
        _ => tt::AMBIGUOUS,
    }
}

/// `CanUseDotOperator`.
fn can_use_dot(t: Code) -> bool {
    t == tt::FORM || basic_type(t) == tt::REF || t == tt::AMBIGUOUS || t == tt::ARRAY_ELEMENT
}

/// `GetBasicParamTypeString`.
fn basic_param_type_string(pt: u32) -> &'static str {
    match pt {
        NVSE_NUMBER => "Number",
        NVSE_FORM => "Form/Ref",
        NVSE_ARRAY => "Array",
        NVSE_STRING => "String",
        _ => "<Not implemented>",
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionParser::ValidateArgType.
/// `ValidateArgType`: an xNVSE parameter type's check ([`validate_arg_type`]), or a vanilla
/// parameter's: any type for an ambiguous argument, else the conversions each kind takes.
pub fn validate(kind: u32, arg: Code, nvse_param: bool, cmd: Option<&CmdInfo>) -> bool {
    if nvse_param {
        return validate_arg_type(kind, arg);
    }
    if arg == tt::AMBIGUOUS {
        return true;
    }
    match kind {
        param::STRING | param::AXIS | param::SEX => convert(arg, tt::STRING),
        param::FLOAT | param::DOUBLE | param::INTEGER | param::QUEST_STAGE | param::CRIME_TYPE => {
            // xNVSE also takes the parameters of `kParams_FormatString`, all 21 of them, which
            // the count covers.
            if arg != tt::STRING_VAR
                && convert(arg, tt::STRING)
                && cmd.is_some_and(|c| c.long_name.starts_with(b"sv_") || c.params.len() >= 20)
            {
                return true;
            }
            convert(arg, tt::NUMBER) || convert(arg, tt::STRING_VAR) || convert(arg, VARIABLE)
        }
        param::ANIMATION_GROUP | param::ACTOR_VALUE => {
            convert(arg, tt::STRING) || convert(arg, tt::NUMBER)
        }
        param::VARIABLE_NAME | param::FORM_TYPE => false,
        param::MAGIC_EFFECT => {
            convert(arg, tt::STRING) || convert(arg, tt::NUMBER) || convert(arg, tt::FORM)
        }
        param::ARRAY => convert(arg, tt::ARRAY),
        param::SCRIPT_VARIABLE => convert(arg, VARIABLE),
        _ => convert(arg, tt::FORM),
    }
}

/// `kTokenType_Variable`.
const VARIABLE: Code = 10;

/// `TokenType_To_ParamType`; another type is a `std::runtime_error` nothing catches.
fn param_type_of(t: Code) -> Result<u32, Fail> {
    match t {
        tt::NUMBER => Ok(NVSE_NUMBER),
        tt::FORM | tt::REF => Ok(NVSE_FORM),
        tt::ARRAY => Ok(NVSE_ARRAY),
        tt::STRING => Ok(NVSE_STRING),
        _ => Err(Fail::Crash("TokenType_To_ParamType: not a basic type")),
    }
}

/// The call commands (`callCmds`): the callee's argument and where its own arguments start.
fn call_command(name: &[u8]) -> Option<(usize, usize)> {
    [
        (&b"Call"[..], (0, 1)),
        (b"CallAfterFrames", (1, 3)),
        (b"CallAfterSeconds", (1, 3)),
        (b"CallForSeconds", (1, 3)),
    ]
    .into_iter()
    .find(|(n, _)| n.eq_ignore_ascii_case(name))
    .map(|(_, v)| v)
}

pub struct TypeChecker<'a> {
    pub ast: &'a mut Ast,
    env: &'a mut dyn Env,
    /// The `#version` statements' plugins (`g_currentCompilerPluginVersions.top()`).
    plugin_versions: &'a MsvcMap<u32>,
    /// Whether a script is compiling (`engineScript`).
    has_script: bool,
    inside_loop: Vec<bool>,
    /// The return type of each open function or lambda (`ReturnInfo`; its line is never set).
    return_types: Vec<Code>,
    pub had_error: bool,
}

impl<'a> TypeChecker<'a> {
    pub fn new(
        ast: &'a mut Ast,
        env: &'a mut dyn Env,
        plugin_versions: &'a MsvcMap<u32>,
        has_script: bool,
    ) -> Self {
        TypeChecker {
            ast,
            env,
            plugin_versions,
            has_script,
            inside_loop: Vec::new(),
            return_types: Vec::new(),
            had_error: false,
        }
    }

    fn fail(&mut self, span: Span, msg: &[u8]) -> R {
        self.had_error = true;
        let text = error_text(&self.ast.lines, msg, span)?;
        self.env.error(&text);
        Err(Fail::Caught)
    }

    /// `error(expr, message)`.
    fn error_expr(&mut self, id: ExprId, msg: &[u8]) -> R {
        let span = self.ast.expr(id).span;
        self.fail(span, msg)
    }

    /// `error(stmt, message)`.
    fn error_stmt(&mut self, id: StmtId, msg: &[u8]) -> R {
        let span = self.ast.stmt(id).span;
        self.fail(span, msg)
    }

    /// `WRAP_ERROR`: a caught error's empty message printed, and on.
    fn wrap(&mut self, r: R) -> R {
        match r {
            Err(Fail::Caught) => {
                self.env.error(b"");
                Ok(())
            }
            other => other,
        }
    }

    fn ty(&self, id: ExprId) -> Code {
        self.ast.expr(id).ty
    }

    fn set_ty(&mut self, id: ExprId, t: Code) {
        self.ast.expr_mut(id).ty = t;
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp: TypeChecker::Visit.
    /// The pass. An error no step caught comes back as a crash (xNVSE's exception is not caught
    /// above the pass).
    pub fn run(&mut self) -> R {
        match self.visit_script() {
            Err(Fail::Caught) => Err(Fail::Crash("TypeCheckerError outside any catch")),
            r => r,
        }
    }

    fn visit_script(&mut self) -> R {
        self.ast.requirements.set(b"nvse", PACKED_NVSE_VERSION);
        let versions: Vec<(Vec<u8>, u32)> = self
            .plugin_versions
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), *v))
            .collect();
        for (k, v) in versions {
            self.ast.requirements.set(&k, v);
        }
        let globals = self.ast.globals.clone();
        for g in globals {
            self.visit_stmt(g)?;
            let decls = self.ast.decls(g).to_vec();
            for d in decls {
                if d.value.is_some() {
                    let r = self
                        .error_stmt(g, b"Variable initializers are not allowed in global scope.");
                    self.wrap(r)?;
                }
            }
        }
        let mut modes: HashMap<Vec<u8>, HashSet<Vec<u8>>> = HashMap::new();
        let (mut found_fn, mut found_event) = (false, false);
        let blocks = self.ast.blocks.clone();
        for &b in &blocks {
            match &self.ast.stmt(b).kind {
                StmtKind::Begin { name, param, .. } => {
                    let (name, param) = (
                        name.clone(),
                        param.as_ref().map(|p| p.lexeme.clone()).unwrap_or_default(),
                    );
                    if found_fn {
                        let r = self.error_stmt(
                            b,
                            b"Cannot have a function block and an event block in the same script.",
                        );
                        self.wrap(r)?;
                    }
                    found_event = true;
                    if modes.get(&name).is_some_and(|m| m.contains(&param)) {
                        let r = self.error_stmt(b, b"Duplicate block declaration.");
                        self.wrap(r)?;
                        continue;
                    }
                    modes.entry(name).or_default().insert(param);
                }
                StmtKind::Udf { .. } => {
                    if found_event {
                        let r = self.error_stmt(
                            b,
                            b"Cannot have a function block and an event block in the same script.",
                        );
                        self.wrap(r)?;
                    }
                    if found_fn {
                        let r = self.error_stmt(b, b"Duplicate block declaration.");
                        self.wrap(r)?;
                    }
                    found_fn = true;
                }
                _ => {}
            }
        }
        for b in blocks {
            let r = self.visit_stmt(b);
            self.wrap(r)?;
        }
        Ok(())
    }

    fn visit_stmt(&mut self, id: StmtId) -> R {
        let kind = self.ast.stmt(id).kind.clone();
        match kind {
            StmtKind::Begin { block, .. } => self.visit_stmt(block),
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitFnStmt.
            StmtKind::Udf { args, body, .. } => {
                for a in args {
                    let r = self.visit_stmt(a);
                    self.wrap(r)?;
                }
                self.return_types.push(tt::INVALID);
                self.visit_stmt(body)?;
                self.return_types.pop();
                Ok(())
            }
            StmtKind::VarDecl { ty, decls } => self.visit_var_decl(id, ty, decls),
            StmtKind::Expr(e) => match e {
                Some(e) => self.visit_expr(e),
                None => Ok(()),
            },
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitForStmt.
            StmtKind::For {
                init,
                cond,
                post,
                block,
            } => {
                if let Some(init) = init {
                    let r = self.visit_stmt(init);
                    self.wrap(r)?;
                }
                if let Some(cond) = cond {
                    let r = (|| {
                        self.visit_expr(cond)?;
                        let o = result_type(op::EQUALS, self.ty(cond), tt::BOOLEAN);
                        if o != tt::BOOLEAN {
                            let m = format!(
                                "Invalid expression type ('{}') for loop condition.",
                                type_name(o)
                            );
                            self.error_stmt(id, m.as_bytes())?;
                        }
                        self.set_ty(cond, o);
                        Ok(())
                    })();
                    self.wrap(r)?;
                }
                if let Some(post) = post {
                    let r = self.visit_expr(post);
                    self.wrap(r)?;
                }
                self.loop_body(true, block)
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitForEachStmt.
            StmtKind::ForEach {
                decls,
                rhs,
                block,
                decompose,
            } => {
                for d in decls.iter().flatten() {
                    let r = self.visit_stmt(*d);
                    self.wrap(r)?;
                }
                self.visit_expr(rhs)?;
                if !decompose {
                    let first = need(decls.first().copied().flatten())?;
                    let l = self.ast.stmt(first).detailed_type;
                    let r = self.ty(rhs);
                    if result_type(op::IN, l, r) == tt::INVALID {
                        let m = format!(
                            "Invalid types '{}' and '{}' passed to for-in expression.",
                            type_name(l),
                            type_name(r)
                        );
                        let res = self.error_stmt(id, m.as_bytes());
                        self.wrap(res)?;
                    }
                }
                self.loop_body(true, block)
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitIfStmt.
            StmtKind::If {
                cond,
                block,
                else_block,
            } => {
                let cond = need(cond)?;
                let r = (|| {
                    self.visit_expr(cond)?;
                    let l = self.ty(cond);
                    if l != tt::AMBIGUOUS && !convert(l, tt::BOOLEAN) {
                        let m = format!(
                            "Invalid expression type '{}' for if statement.",
                            type_name(l)
                        );
                        self.error_stmt(id, m.as_bytes())?;
                    } else {
                        self.set_ty(cond, tt::BOOLEAN);
                    }
                    Ok(())
                })();
                self.wrap(r)?;
                self.visit_stmt(block)?;
                if let Some(e) = else_block {
                    self.visit_stmt(e)?;
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitReturnStmt.
            StmtKind::Return(e) => {
                let Some(e) = e else {
                    self.ast.stmt_mut(id).detailed_type = tt::EMPTY;
                    return Ok(());
                };
                self.visit_expr(e)?;
                let basic = basic_type(self.ty(e));
                // Outside a function or lambda xNVSE reads the top of an empty stack.
                let top = *self
                    .return_types
                    .last()
                    .ok_or(Fail::Crash("return value outside a function: empty stack"))?;
                if top != tt::INVALID {
                    if !convert(basic, top) {
                        let m = format!(
                            "Return type '{}' not compatible with return type defined previously. ('{}' at line {})",
                            type_name(basic),
                            type_name(top),
                            0
                        );
                        self.error_expr(e, m.as_bytes())?;
                    }
                } else if let Some(t) = self.return_types.last_mut() {
                    *t = basic;
                }
                self.ast.stmt_mut(id).detailed_type = self.ty(e);
                Ok(())
            }
            StmtKind::Continue => {
                if !self.inside_loop.last().copied().unwrap_or(false) {
                    self.error_stmt(id, b"Keyword 'continue' not valid outside of loops.")?;
                }
                Ok(())
            }
            StmtKind::Break => {
                if !self.inside_loop.last().copied().unwrap_or(false) {
                    self.error_stmt(id, b"Keyword 'break' not valid outside of loops.")?;
                }
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitWhileStmt.
            StmtKind::While { cond, block } => {
                let cond = need(cond)?;
                let r = (|| {
                    self.visit_expr(cond)?;
                    let o = result_type(op::EQUALS, self.ty(cond), tt::BOOLEAN);
                    if o != tt::BOOLEAN {
                        self.error_expr(cond, b"Invalid expression type for while loop.")?;
                    }
                    self.set_ty(cond, o);
                    Ok(())
                })();
                self.wrap(r)?;
                self.loop_body(true, block)
            }
            StmtKind::Block(stmts) => {
                for s in stmts {
                    let r = self.visit_stmt(s);
                    self.wrap(r)?;
                }
                Ok(())
            }
            StmtKind::ShowMessage { message, args, .. } => {
                self.visit_expr(message)?;
                for a in args {
                    self.visit_expr(a)?;
                }
                Ok(())
            }
            StmtKind::Match { .. } => Ok(()),
        }
    }

    fn loop_body(&mut self, inside: bool, block: StmtId) -> R {
        self.inside_loop.push(inside);
        self.visit_stmt(block)?;
        self.inside_loop.pop();
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitVarDeclStmt.
    fn visit_var_decl(&mut self, id: StmtId, ty: TokenType, decls: Vec<Declaration>) -> R {
        let detailed = declared_type(ty);
        for d in decls {
            let name = self.ast.ident_name(d.name).to_vec();
            if self.env.command(&name).is_some() {
                let m = cat(&[
                    b"Variable name '",
                    &name,
                    b"' conflicts with a command with the same name.",
                ]);
                let r = self.error_stmt(id, &m);
                self.wrap(r)?;
                continue;
            }
            // xNVSE's note of a variable that shadows a form goes to nvse.log only.
            if let Some(value) = d.value {
                self.visit_expr(value)?;
                let rhs = self.ty(value);
                if result_type(op::ASSIGNMENT, detailed, rhs) == tt::INVALID {
                    let m = format!(
                        "Cannot convert from {} to {}",
                        type_name(rhs),
                        type_name(detailed)
                    );
                    let r = self.error_stmt(id, m.as_bytes());
                    self.wrap(r)?;
                    return Ok(());
                }
            }
            let Some(var) = d.info else {
                let value = need(d.value)?;
                return self.error_expr(
                    value,
                    b"Unexpected compiler failure. Please report this as a bug.",
                );
            };
            self.ast.var_mut(var).detailed_type = detailed;
            if let Some(value) = d.value {
                if let ExprKind::Lambda {
                    param_types,
                    return_type,
                    ..
                } = &self.ast.expr(value).kind
                {
                    let (p, r) = (param_types.clone(), *return_type);
                    let v = self.ast.var_mut(var);
                    v.is_lambda = true;
                    v.lambda_return_type = r;
                    v.lambda_param_types = p;
                }
            }
            self.ast.stmt_mut(id).detailed_type = detailed;
        }
        Ok(())
    }

    fn visit_expr(&mut self, id: ExprId) -> R {
        let kind = self.ast.expr(id).kind.clone();
        match kind {
            ExprKind::Assign { token, left, value } => {
                self.visit_assignment(id, token.ty, &token.lexeme, left, value)
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitTernaryExpr.
            ExprKind::Ternary {
                cond, left, right, ..
            } => {
                let r = (|| {
                    self.visit_expr(cond)?;
                    let l = self.ty(cond);
                    let o = result_type(op::EQUALS, l, tt::BOOLEAN);
                    if o == tt::INVALID {
                        let m = format!(
                            "Invalid expression type '{}' for if statement.",
                            type_name(l)
                        );
                        self.error_expr(id, m.as_bytes())?;
                    } else {
                        self.set_ty(cond, o);
                    }
                    Ok(())
                })();
                self.wrap(r)?;
                let r = self.visit_expr(left);
                self.wrap(r)?;
                let r = self.visit_expr(right);
                self.wrap(r)?;
                if !convert(self.ty(right), basic_type(self.ty(left))) {
                    let m = format!(
                        "Incompatible value types ('{}' and '{}') specified for ternary expression.",
                        type_name(self.ty(left)),
                        type_name(self.ty(right))
                    );
                    self.error_expr(id, m.as_bytes())?;
                }
                let t = self.ty(left);
                self.set_ty(id, t);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitInExpr.
            ExprKind::In {
                lhs,
                values,
                expression,
                ..
            } => {
                self.visit_expr(lhs)?;
                if !values.is_empty() {
                    for (i, val) in values.into_iter().enumerate() {
                        self.visit_expr(val)?;
                        let (l, r) = (self.ty(lhs), self.ty(val));
                        if result_type(op::EQUALS, l, r) == tt::INVALID {
                            let m = format!(
                                "Value {} (type '{}') cannot compare against the {} specified on lhs of 'in' expression.",
                                i + 1,
                                type_name(r),
                                type_name(l)
                            );
                            let res = self.error_expr(id, m.as_bytes());
                            self.wrap(res)?;
                        }
                    }
                } else {
                    let e = need(expression)?;
                    self.visit_expr(e)?;
                    let t = self.ty(e);
                    if t != tt::AMBIGUOUS && t != tt::ARRAY && t != tt::ARRAY_VAR {
                        let m = format!(
                            "Expected array for 'in' expression (Got '{}').",
                            type_name(t)
                        );
                        let res = self.error_expr(id, m.as_bytes());
                        self.wrap(res)?;
                    }
                }
                self.set_ty(id, tt::BOOLEAN);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitBinaryExpr.
            ExprKind::Binary { op, left, right } => {
                self.visit_expr(left)?;
                self.visit_expr(right)?;
                let (l, r) = (self.ty(left), self.ty(right));
                let o = result_type(token_op(op.ty), l, r);
                if o == tt::INVALID {
                    let m = operator_message(
                        &format!("Invalid types '{}' and '{}'", type_name(l), type_name(r)),
                        &op.lexeme,
                        op.ty,
                    );
                    return self.error_expr(id, &m);
                }
                self.set_ty(id, o);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitUnaryExpr.
            ExprKind::Unary { op, expr, postfix } => {
                self.visit_expr(expr)?;
                let l = self.ty(expr);
                let rhs = if postfix { tt::NUMBER } else { tt::INVALID };
                let o = result_type(token_op(op.ty), l, rhs);
                if o == tt::INVALID {
                    let m = operator_message(
                        &format!("Invalid operand type '{}'", type_name(l)),
                        &op.lexeme,
                        op.ty,
                    );
                    self.error_expr(id, &m)?;
                }
                self.set_ty(id, o);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitSubscriptExpr.
            ExprKind::Subscript { left, index, .. } => {
                self.visit_expr(left)?;
                self.visit_expr(index)?;
                let o = result_type(op::LEFT_BRACKET, self.ty(left), self.ty(index));
                if o == tt::INVALID {
                    let m = format!(
                        "Expression type '{}' not valid for operator [].",
                        type_name(self.ty(index))
                    );
                    return self.error_expr(id, m.as_bytes());
                }
                self.set_ty(id, o);
                Ok(())
            }
            ExprKind::Call { .. } => self.visit_call(id),
            ExprKind::Get { .. } => self.visit_get(id),
            ExprKind::Bool(_) => {
                self.set_ty(id, tt::BOOLEAN);
                Ok(())
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitNumberExpr.
            ExprKind::Number { value, is_fp, .. } => {
                if !is_fp && value > f64::from(u32::MAX) {
                    let r = self.error_expr(
                        id,
                        b"Maximum value for integer literal exceeded. (Max: 4294967295)",
                    );
                    self.wrap(r)?;
                }
                self.set_ty(id, tt::NUMBER);
                Ok(())
            }
            ExprKind::MapLit(values) => self.visit_map(id, values),
            ExprKind::ArrayLit(values) => self.visit_array(id, values),
            ExprKind::Str(_) => {
                self.set_ty(id, tt::STRING);
                Ok(())
            }
            ExprKind::Ident { .. } => self.visit_ident(id),
            ExprKind::Grouping(inner) => {
                let inner = need(inner)?;
                let r = (|| {
                    self.visit_expr(inner)?;
                    let t = self.ty(inner);
                    self.set_ty(id, t);
                    Ok(())
                })();
                self.wrap(r)
            }
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
            // TypeChecker::VisitLambdaExpr.
            ExprKind::Lambda { args, body, .. } => {
                self.return_types.push(tt::INVALID);
                for a in args {
                    let r = self.visit_stmt(a);
                    self.wrap(r)?;
                    let pt = param_type_of(basic_type(self.ast.stmt(a).detailed_type))?;
                    if let ExprKind::Lambda { param_types, .. } = &mut self.ast.expr_mut(id).kind {
                        param_types.push(pt);
                    }
                }
                self.loop_body(false, body)?;
                self.set_ty(id, tt::LAMBDA);
                let ret = self.return_types.last().copied().unwrap_or(tt::INVALID);
                if let ExprKind::Lambda { return_type, .. } = &mut self.ast.expr_mut(id).kind {
                    *return_type = ret;
                }
                self.return_types.pop();
                Ok(())
            }
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitAssignmentExpr.
    fn visit_assignment(
        &mut self,
        id: ExprId,
        token: TokenType,
        lexeme: &[u8],
        left: ExprId,
        value: ExprId,
    ) -> R {
        self.visit_expr(left)?;
        self.visit_expr(value)?;
        let (l, r) = (self.ty(left), self.ty(value));
        let o = result_type(token_op(token), l, r);
        if o == tt::INVALID {
            let m = operator_message(
                &format!("Invalid types '{}' and '{}'", type_name(l), type_name(r)),
                lexeme,
                token,
            );
            return self.error_expr(id, &m);
        }
        if self.ast.is_ident(left) {
            if let Some(v) = self.ast.ident_var(left) {
                if self.ast.var(v).is_lambda {
                    return self
                        .error_expr(id, b"Cannot assign to a variable that is holding a lambda.");
                }
            }
            if self.ty(value) == tt::LAMBDA {
                return self.error_expr(
                    id,
                    b"Cannot assign a lambda to a variable after it has been declared.",
                );
            }
        }
        self.set_ty(id, o);
        self.set_ty(left, o);
        Ok(())
    }

    fn call_parts(&self, id: ExprId) -> (Option<ExprId>, ExprId, Vec<ExprId>) {
        match &self.ast.expr(id).kind {
            ExprKind::Call {
                left, ident, args, ..
            } => (*left, *ident, args.clone()),
            _ => unreachable!("a call"),
        }
    }

    fn set_arg(&mut self, id: ExprId, i: usize, arg: ExprId) {
        if let ExprKind::Call { args, .. } = &mut self.ast.expr_mut(id).kind {
            args[i] = arg;
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitCallExpr.
    fn visit_call(&mut self, id: ExprId) -> R {
        let (left, ident, _) = self.call_parts(id);
        let name = self.ast.ident_name(ident).to_vec();
        let Some(cmd) = self.env.command(&name) else {
            return self.error_expr(ident, b"Not a valid command");
        };
        if let ExprKind::Call { cmd: c, .. } = &mut self.ast.expr_mut(id).kind {
            *c = Some(cmd.clone());
        }
        if let Some((plugin, version)) = &cmd.plugin {
            let lower = plugin.to_ascii_lowercase();
            let v = match self.ast.requirements.get(&lower) {
                None => *version,
                Some(&old) => old.max(*version),
            };
            self.ast.requirements.set(&lower, v);
        }
        if let Some(l) = left {
            self.visit_expr(l)?;
            if !can_use_dot(self.ty(l)) {
                let r = self.error_expr(id, b"Left side of '.' must be a form or reference.");
                self.wrap(r)?;
            }
        }
        if self.has_script {
            let t = self.env.script_type();
            if cmd.needs_parent
                && left.is_none()
                && t != Some(script_type::OBJECT)
                && t != Some(script_type::MAGIC)
            {
                let m = cat(&[b"Command '", &name, b"' requires a calling reference."]);
                let r = self.error_expr(id, &m);
                self.wrap(r)?;
            }
        }
        let default_parse = cmd.parse == Parse::Default;
        let mut arg_idx = 0usize;
        let mut param_idx = 0usize;
        loop {
            let args = self.call_parts(id).2;
            if param_idx >= cmd.params.len() || arg_idx >= args.len() {
                break;
            }
            let p = cmd.params[param_idx].clone();
            let mut arg = args[arg_idx];
            let mut converted = false;
            if default_parse || cmd.long_name.eq_ignore_ascii_case(b"ShowMessage") {
                let ident_arg = self.ast.is_ident(arg).then_some(arg);
                let (mut index, mut len) = (u32::MAX, 0);
                if let Some(i) = ident_arg {
                    let text = self.ast.ident_name(i).to_vec();
                    (index, len) = self.env.vanilla_enum(p.kind, &text);
                }
                if index != u32::MAX {
                    let span = self.ast.expr(arg).span;
                    arg = self.ast.add_expr(
                        ExprKind::Number {
                            value: f64::from(index),
                            is_fp: false,
                            enum_len: len,
                        },
                        span,
                    );
                    self.set_ty(arg, tt::NUMBER);
                    converted = true;
                } else {
                    let r = self.visit_expr(arg);
                    self.wrap(r)?;
                }
                if let Some(i) = ident_arg {
                    if self.ty(arg) == tt::FORM
                        && (param::OBJECT_REF..=param::REGION).contains(&p.kind)
                    {
                        let form = match self.ast.expr(i).kind {
                            ExprKind::Ident { form, .. } => form,
                            _ => 0,
                        };
                        if form == 0 || !self.env.form_matches_param(form, p.kind) {
                            if !p.optional {
                                let m = cat(&[
                                    format!(
                                        "Invalid expression for parameter {}. Expected ",
                                        arg_idx + 1
                                    )
                                    .as_bytes(),
                                    &p.name,
                                    b".",
                                ]);
                                let r = self.error_expr(arg, &m);
                                self.wrap(r)?;
                            }
                        } else {
                            arg_idx += 1;
                            param_idx += 1;
                            continue;
                        }
                    }
                }
            } else {
                let r = self.visit_expr(arg);
                self.wrap(r)?;
                if self.ty(arg) == tt::INVALID {
                    arg_idx += 1;
                    param_idx += 1;
                    continue;
                }
            }
            if !converted && !validate(p.kind, self.ty(arg), !default_parse, Some(&cmd)) {
                if !p.optional {
                    let m = cat(&[
                        format!(
                            "Invalid expression for parameter {}. (Expected ",
                            arg_idx + 1
                        )
                        .as_bytes(),
                        &p.name,
                        format!(", got {}).", type_name(self.ty(arg))).as_bytes(),
                    ]);
                    let r = self.error_expr(arg, &m);
                    self.wrap(r)?;
                }
            } else {
                self.set_arg(id, arg_idx, arg);
                arg_idx += 1;
            }
            param_idx += 1;
        }
        let required = cmd.params.iter().filter(|p| !p.optional).count();
        let args = self.call_parts(id).2;
        let enough = args.len() >= required;
        if !enough {
            let m = cat(&[
                b"Invalid number of parameters specified to ",
                &name,
                format!(" (Expected {}, got {}).", required, args.len()).as_bytes(),
            ]);
            let r = self.error_expr(id, &m);
            self.wrap(r)?;
        }
        if let Some((func_index, arg_start)) = call_command(&cmd.long_name) {
            if !enough {
                return Ok(());
            }
            let callee = *args
                .get(func_index)
                .ok_or(Fail::Crash("call command: callee past the arguments"))?;
            let callee_ident = self.ast.is_ident(callee).then_some(callee);
            if let Some(ci) = callee_ident {
                let callee_name = self.ast.ident_name(ci).to_vec();
                let form = self.env.form_by_name(&callee_name);
                if form != 0 {
                    if let Some(t) = self.env.script_form_type(form) {
                        if t != script_type::OBJECT {
                            let r = self.error_expr(
                                ci,
                                b"Target script is not an object script (Invalid UDF)",
                            );
                            self.wrap(r)?;
                        }
                    }
                }
            }
            let lambda_var = callee_ident
                .and_then(|ci| self.ast.ident_var(ci))
                .filter(|&v| self.ast.var(v).is_lambda);
            while arg_idx < args.len() {
                self.visit_expr(args[arg_idx])?;
                arg_idx += 1;
            }
            match (lambda_var, callee_ident) {
                (Some(var), Some(ci)) => {
                    self.check_lambda_args(id, ci, var, arg_start)?;
                    let t = self.ast.var(var).lambda_return_type;
                    self.set_ty(id, t);
                }
                _ => self.set_ty(id, tt::AMBIGUOUS),
            }
            return Ok(());
        }
        let mut t = cmd.return_type;
        if t == tt::INVALID {
            t = tt::AMBIGUOUS;
        }
        self.set_ty(id, t);
        Ok(())
    }

    /// `checkLambdaArgs`: each argument after `start` against the lambda's parameter types
    /// (xNVSE reads the type at the argument's place less one, whatever `start` is), then the
    /// count.
    fn check_lambda_args(&mut self, id: ExprId, ident: ExprId, var: VarId, start: usize) -> R {
        let params = self.ast.var(var).lambda_param_types.clone();
        let args = self.call_parts(id).2;
        for (i, &arg) in args.iter().enumerate().skip(start) {
            let n = i - start + 1;
            if i > params.len() {
                break;
            }
            let expected = params[i - 1];
            if !validate(expected, self.ty(arg), true, None) {
                let m = format!(
                    "Invalid expression for lambda parameter {}. (Expected {}, got {})",
                    n,
                    basic_param_type_string(expected),
                    type_name(self.ty(arg))
                );
                let r = self.error_expr(id, m.as_bytes());
                self.wrap(r)?;
            }
        }
        let passed = args.len().saturating_sub(start);
        if passed != params.len() {
            let m = cat(&[
                b"Invalid number of parameters specified for lambda '",
                self.ast.ident_name(ident),
                format!("' (Expected {}, got {}).", params.len(), passed).as_bytes(),
            ]);
            let r = self.error_expr(id, &m);
            self.wrap(r)?;
        }
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitGetExpr.
    /// `form.variable`: the left side must be an identifier naming a form (a reference or a
    /// quest) whose script has the variable. Its type is left ambiguous, as xNVSE leaves it.
    fn visit_get(&mut self, id: ExprId) -> R {
        let ExprKind::Get { left, ident, .. } = self.ast.expr(id).kind.clone() else {
            return Ok(());
        };
        self.visit_expr(left)?;
        let form = match self.ast.expr(left).kind {
            ExprKind::Ident { form, .. } if self.ty(left) == tt::FORM => form,
            _ => {
                return self.error_expr(
                    id,
                    b"Member access not valid here. Left side of '.' must be a form or persistent reference.",
                )
            }
        };
        let lhs_name = self.ast.ident_name(left).to_vec();
        let rhs_name = self.ast.ident_name(ident).to_vec();
        let form_type = self.env.form_type(form);
        if form_type != FORM_REFR && form_type != FORM_QUEST {
            return self.error_expr(id, b"Unexpected form type found.");
        }
        if let Some(found) = self.env.form_script_variable(form, &rhs_name) {
            if let Some(info) = found {
                let reference = self.env.editor_id(form);
                if let ExprKind::Get {
                    var_info,
                    reference_name,
                    ..
                } = &mut self.ast.expr_mut(id).kind
                {
                    *var_info = Some(info);
                    *reference_name = Some(reference);
                }
                self.set_ty(id, tt::AMBIGUOUS);
                return Ok(());
            }
            let m = cat(&[
                b"Variable ",
                &rhs_name,
                b" not found on form ",
                &lhs_name,
                b".",
            ]);
            return self.error_expr(id, &m);
        }
        let m = cat(&[
            b"Unable to resolve type for '",
            &lhs_name,
            b".",
            &rhs_name,
            b"'.",
        ]);
        self.error_expr(id, &m)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitMapLiteralExpr.
    fn visit_map(&mut self, id: ExprId, values: Vec<ExprId>) -> R {
        if values.is_empty() {
            let r = self.error_expr(
                id,
                b"A map literal cannot be empty, as the key type cannot be deduced.",
            );
            self.wrap(r)?;
            return Ok(());
        }
        for (i, &val) in values.iter().enumerate() {
            self.visit_expr(val)?;
            let t = self.ty(val);
            if t != tt::PAIR && t != tt::AMBIGUOUS {
                let m = format!(
                    "Value {} is not a pair and is not valid for a map literal.",
                    i + 1
                );
                let r = self.error_expr(id, m.as_bytes());
                self.wrap(r)?;
                return Ok(());
            }
        }
        let mut key_type = tt::INVALID;
        for (i, &val) in values.iter().enumerate() {
            if self.ty(val) != tt::PAIR {
                continue;
            }
            let ExprKind::Binary { op, left, .. } = &self.ast.expr(val).kind else {
                continue;
            };
            let (op_ty, left) = (op.ty, *left);
            if op_ty != TokenType::MakePair && self.ty(left) != tt::PAIR {
                continue;
            }
            if key_type == tt::INVALID {
                key_type = basic_type(self.ty(left));
            } else if key_type != self.ty(left) {
                let m = format!(
                    "Key for value {} (type '{}') specified for map literal conflicts with key type of previous value ('{}').",
                    i,
                    type_name(self.ty(left)),
                    type_name(key_type)
                );
                let r = self.error_expr(id, m.as_bytes());
                self.wrap(r)?;
            }
        }
        self.set_ty(id, tt::ARRAY);
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitArrayLiteralExpr.
    fn visit_array(&mut self, id: ExprId, values: Vec<ExprId>) -> R {
        if values.is_empty() {
            self.set_ty(id, tt::ARRAY);
            return Ok(());
        }
        for &val in &values {
            self.visit_expr(val)?;
            if self.ty(val) == tt::PAIR {
                let r = self.error_expr(
                    val,
                    b"Invalid type inside of array literal. Expected array, string, ref, or number.",
                );
                self.wrap(r)?;
            }
        }
        let first = self.ty(values[0]);
        for (i, &val) in values.iter().enumerate().skip(1) {
            if !convert(self.ty(val), first) {
                let m = format!(
                    "Value {} (type '{}') specified for array literal conflicts with the type already specified in first element ('{}').",
                    i,
                    type_name(self.ty(val)),
                    type_name(first)
                );
                let r = self.error_expr(id, m.as_bytes());
                self.wrap(r)?;
            }
        }
        self.set_ty(id, tt::ARRAY);
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitIdentExpr.
    /// A variable's type, else the form of that editor id (`player`: the player's reference):
    /// a global's type, or a form's (kept on the expression).
    fn visit_ident(&mut self, id: ExprId) -> R {
        let ExprKind::Ident { name, var, .. } = self.ast.expr(id).kind.clone() else {
            return Ok(());
        };
        if let Some(v) = var {
            let t = self.ast.var(v).detailed_type;
            self.set_ty(id, t);
            return Ok(());
        }
        let form = if name.eq_ignore_ascii_case(b"player") {
            self.env.form_by_id(0x14)
        } else {
            self.env.form_by_name(&name)
        };
        if form == 0 {
            self.set_ty(id, tt::INVALID);
            let m = cat(&[b"Unable to resolve identifier '", &name, b"'."]);
            return self.error_expr(id, &m);
        }
        if self.env.form_type(form) == FORM_GLOBAL {
            self.set_ty(id, tt::GLOBAL);
        } else {
            self.set_ty(id, tt::FORM);
            if let ExprKind::Ident { form: f, .. } = &mut self.ast.expr_mut(id).kind {
                *f = form;
            }
        }
        Ok(())
    }
}

/// Form types: `TESGlobal`, `TESObjectREFR`, `TESQuest`.
pub const FORM_GLOBAL: u8 = 6;
pub const FORM_REFR: u8 = 0x3a;
pub const FORM_QUEST: u8 = 0x47;

/// "<start> for operator <lexeme> (<token's text>)."
fn operator_message(start: &str, lexeme: &[u8], t: TokenType) -> Vec<u8> {
    cat(&[
        start.as_bytes(),
        b" for operator ",
        lexeme,
        format!(" ({}).", t.text()).as_bytes(),
    ])
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
