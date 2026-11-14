//! The new script language's syntax tree (`Compiler/AST/AST.h`, `ASTForward.h`, `Visitor.cpp`
//! (xNVSE)), held in arenas: a node is an index, so a node xNVSE shares between two places (its
//! `shared_ptr`s: a match's value under every arm, a loop's increment before each `continue`)
//! is the same index in both, and a pass that replaces a child (`TransformExpr`) writes the new
//! index into the parent's slot.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a).

use super::lexer::{Span, Token, TokenType};
use super::msvc::MsvcMap;
use crate::compiler::{tt, Code};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StmtId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

/// A parameter of a command or block record (`ParamInfo`): its name, type and whether it is
/// optional.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamInfo {
    pub name: Vec<u8>,
    pub kind: u32,
    pub optional: bool,
}

/// An event block's record (`g_eventBlockCommandInfos`): its opcode and parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockInfo {
    pub opcode: u16,
    pub params: Vec<ParamInfo>,
}

/// How a command's record parses (its parse word): the game's default parser or xNVSE's
/// `Cmd_Default_Parse` (`IsDefaultParse`), `Cmd_Call_Parse`, or another of xNVSE's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parse {
    Default,
    Call,
    Other,
}

/// A command's record as the passes read it (`CommandInfo`).
#[derive(Clone, Debug, PartialEq)]
pub struct CmdInfo {
    pub opcode: u16,
    pub long_name: Vec<u8>,
    pub needs_parent: bool,
    pub params: Vec<ParamInfo>,
    pub parse: Parse,
    /// `ToTokenType(g_scriptCommands.GetReturnType(cmd))`.
    pub return_type: Code,
    /// `g_scriptCommands.GetParentPlugin(cmd)`: its name and version.
    pub plugin: Option<(Vec<u8>, u32)>,
}

/// `VarInfo`: a variable the variable resolution made or found.
#[derive(Clone, Debug, PartialEq)]
pub struct VarInfo {
    pub index: u32,
    pub original_name: Vec<u8>,
    pub remapped_name: Vec<u8>,
    pub pre_existing: bool,
    /// `Script::VariableType`.
    pub ty: u8,
    pub detailed_type: Code,
    pub is_lambda: bool,
    pub lambda_param_types: Vec<u32>,
    pub lambda_return_type: Code,
}

/// `VarDecl::Declaration`.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// The name (an identifier expression).
    pub name: ExprId,
    pub value: Option<ExprId>,
    pub info: Option<VarId>,
}

/// `Match::MatchArm`.
#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub binding: Option<ExprId>,
    pub expr: ExprId,
    pub block: StmtId,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Begin {
        name: Vec<u8>,
        param: Option<Token>,
        block: StmtId,
        info: BlockInfo,
    },
    /// `UDFDecl` (`fn (args) { ... }` at the top).
    Udf {
        token: Token,
        args: Vec<StmtId>,
        body: StmtId,
    },
    VarDecl {
        ty: TokenType,
        decls: Vec<Declaration>,
    },
    Expr(Option<ExprId>),
    For {
        init: Option<StmtId>,
        cond: Option<ExprId>,
        post: Option<ExprId>,
        block: StmtId,
    },
    ForEach {
        /// Each a `VarDecl`, or none for `_`.
        decls: Vec<Option<StmtId>>,
        rhs: ExprId,
        block: StmtId,
        decompose: bool,
    },
    If {
        /// None only after a condition that failed to parse (the parse failed then).
        cond: Option<ExprId>,
        block: StmtId,
        else_block: Option<StmtId>,
    },
    Return(Option<ExprId>),
    Continue,
    Break,
    While {
        cond: Option<ExprId>,
        block: StmtId,
    },
    Block(Vec<StmtId>),
    ShowMessage {
        message: ExprId,
        args: Vec<ExprId>,
        time: u32,
    },
    Match {
        expr: Option<ExprId>,
        arms: Vec<Arm>,
        default: Option<StmtId>,
    },
}

/// `Statement`: its kind, the type some statements keep (`detailedType`) and its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub detailed_type: Code,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Assign {
        token: Token,
        left: ExprId,
        value: ExprId,
    },
    Ternary {
        token: Token,
        cond: ExprId,
        left: ExprId,
        right: ExprId,
    },
    In {
        lhs: ExprId,
        token: Token,
        values: Vec<ExprId>,
        expression: Option<ExprId>,
        is_not: bool,
    },
    Binary {
        op: Token,
        left: ExprId,
        right: ExprId,
    },
    Unary {
        op: Token,
        expr: ExprId,
        postfix: bool,
    },
    Subscript {
        op: Token,
        left: ExprId,
        index: ExprId,
    },
    Call {
        left: Option<ExprId>,
        /// An identifier expression.
        ident: ExprId,
        args: Vec<ExprId>,
        /// Set by the type checker (`cmdInfo`).
        cmd: Option<CmdInfo>,
    },
    Get {
        left: ExprId,
        ident: ExprId,
        /// Set by the type checker: the variable's type and index (`var_info`), and the form's
        /// editor id (`reference_name`).
        var_info: Option<(u8, u16)>,
        reference_name: Option<Vec<u8>>,
    },
    Bool(bool),
    Number {
        value: f64,
        is_fp: bool,
        enum_len: u32,
    },
    Str(Vec<u8>),
    Ident {
        name: Vec<u8>,
        /// The form the type checker found for it (0: none).
        form: u32,
        var: Option<VarId>,
    },
    ArrayLit(Vec<ExprId>),
    MapLit(Vec<ExprId>),
    Grouping(Option<ExprId>),
    Lambda {
        args: Vec<StmtId>,
        body: StmtId,
        param_types: Vec<u32>,
        return_type: Code,
    },
}

/// `Expr`: its kind, its type (`type`, set by the type checker) and its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Code,
    pub span: Span,
}

/// `AST`.
#[derive(Clone, Debug, Default)]
pub struct Ast {
    pub name: Vec<u8>,
    pub globals: Vec<StmtId>,
    pub blocks: Vec<StmtId>,
    /// The plugins the script needs (`m_mpPluginRequirements`), in MSVC's order.
    pub requirements: MsvcMap<u32>,
    pub lines: Vec<Vec<u8>>,
    pub stmts: Vec<Stmt>,
    pub exprs: Vec<Expr>,
    pub vars: Vec<VarInfo>,
}

impl Ast {
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id.0 as usize]
    }
    pub fn expr_mut(&mut self, id: ExprId) -> &mut Expr {
        &mut self.exprs[id.0 as usize]
    }
    pub fn stmt(&self, id: StmtId) -> &Stmt {
        &self.stmts[id.0 as usize]
    }
    pub fn stmt_mut(&mut self, id: StmtId) -> &mut Stmt {
        &mut self.stmts[id.0 as usize]
    }
    pub fn var(&self, id: VarId) -> &VarInfo {
        &self.vars[id.0 as usize]
    }
    pub fn var_mut(&mut self, id: VarId) -> &mut VarInfo {
        &mut self.vars[id.0 as usize]
    }

    /// A new expression with no type yet (`kTokenType_Invalid`).
    pub fn add_expr(&mut self, kind: ExprKind, span: Span) -> ExprId {
        self.exprs.push(Expr {
            kind,
            ty: tt::INVALID,
            span,
        });
        ExprId(self.exprs.len() as u32 - 1)
    }

    pub fn add_stmt(&mut self, kind: StmtKind, span: Span) -> StmtId {
        self.stmts.push(Stmt {
            kind,
            detailed_type: tt::INVALID,
            span,
        });
        StmtId(self.stmts.len() as u32 - 1)
    }

    pub fn add_var(&mut self, v: VarInfo) -> VarId {
        self.vars.push(v);
        VarId(self.vars.len() as u32 - 1)
    }

    /// An identifier expression's name.
    pub fn ident_name(&self, id: ExprId) -> &[u8] {
        match &self.expr(id).kind {
            ExprKind::Ident { name, .. } => name,
            _ => b"",
        }
    }

    /// An identifier expression's variable.
    pub fn ident_var(&self, id: ExprId) -> Option<VarId> {
        match &self.expr(id).kind {
            ExprKind::Ident { var, .. } => *var,
            _ => None,
        }
    }

    pub fn is_ident(&self, id: ExprId) -> bool {
        matches!(self.expr(id).kind, ExprKind::Ident { .. })
    }

    /// A `VarDecl` statement's declarations (empty for another statement).
    pub fn decls(&self, id: StmtId) -> &[Declaration] {
        match &self.stmt(id).kind {
            StmtKind::VarDecl { decls, .. } => decls,
            _ => &[],
        }
    }

    /// A block statement's statements (empty for another statement).
    pub fn block_stmts(&self, id: StmtId) -> &[StmtId] {
        match &self.stmt(id).kind {
            StmtKind::Block(s) => s,
            _ => &[],
        }
    }
}

/// What a pass's step can end in: an error its pass reports and catches (the parser's
/// `NVSEParseError`, the type checker's `TypeCheckerError`), or a C++ exception nothing in xNVSE
/// catches (a crash there; the port fails the compile instead and prints nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fail {
    Caught,
    Crash(&'static str),
}

/// The children of a node a visitor walks in xNVSE's base `Visitor`, and the slots its
/// `TransformExpr` may replace: each expression child is handed to `transform` (which answers
/// the expression to keep in the slot), then visited.
pub trait Visitor {
    fn ast(&mut self) -> &mut Ast;

    /// `TransformExpr`: the expression to keep in a slot (the same by default).
    fn transform(&mut self, id: ExprId) -> Result<ExprId, Fail> {
        Ok(id)
    }

    fn visit_stmt(&mut self, id: StmtId) -> Result<(), Fail> {
        walk_stmt(self, id)
    }

    fn visit_expr(&mut self, id: ExprId) -> Result<(), Fail> {
        walk_expr(self, id)
    }

    /// `Visitor::Visit(AST*)`: the globals, then the blocks.
    fn visit_ast(&mut self) -> Result<(), Fail> {
        let globals = self.ast().globals.clone();
        for g in globals {
            self.visit_stmt(g)?;
        }
        let blocks = self.ast().blocks.clone();
        for b in blocks {
            self.visit_stmt(b)?;
        }
        Ok(())
    }
}

/// A slot's expression transformed, written back with `set`, then visited.
pub fn transform_visit<V: Visitor + ?Sized>(
    v: &mut V,
    id: ExprId,
    set: impl FnOnce(&mut Ast, ExprId),
) -> Result<ExprId, Fail> {
    let new = v.transform(id)?;
    set(v.ast(), new);
    v.visit_expr(new)?;
    Ok(new)
}

/// A missing expression where xNVSE dereferences one (a null `shared_ptr`): a crash there.
pub fn need<T>(x: Option<T>) -> Result<T, Fail> {
    x.ok_or(Fail::Crash("null expression"))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Visitor.cpp: the Visitor::Visit*Stmt
// methods.
/// The base visitor's walk of a statement.
pub fn walk_stmt<V: Visitor + ?Sized>(v: &mut V, id: StmtId) -> Result<(), Fail> {
    let kind = v.ast().stmt(id).kind.clone();
    match kind {
        StmtKind::Begin { block, .. } => v.visit_stmt(block),
        StmtKind::Udf { args, body, .. } => {
            for a in args {
                v.visit_stmt(a)?;
            }
            v.visit_stmt(body)
        }
        StmtKind::VarDecl { decls, .. } => {
            for (i, d) in decls.iter().enumerate() {
                if let Some(value) = d.value {
                    transform_visit(v, value, |ast, new| set_decl_value(ast, id, i, new))?;
                }
            }
            Ok(())
        }
        StmtKind::Expr(Some(e)) => {
            transform_visit(v, e, |ast, new| {
                ast.stmt_mut(id).kind = StmtKind::Expr(Some(new));
            })?;
            Ok(())
        }
        StmtKind::Expr(None) => Ok(()),
        StmtKind::For {
            init,
            cond,
            post,
            block,
        } => {
            if let Some(init) = init {
                v.visit_stmt(init)?;
            }
            if let Some(cond) = cond {
                transform_visit(v, cond, |ast, new| {
                    if let StmtKind::For { cond, .. } = &mut ast.stmt_mut(id).kind {
                        *cond = Some(new);
                    }
                })?;
            }
            // xNVSE transforms and visits the increment without a check: a loop without one
            // dereferences a null pointer.
            let post = need(post)?;
            transform_visit(v, post, |ast, new| {
                if let StmtKind::For { post, .. } = &mut ast.stmt_mut(id).kind {
                    *post = Some(new);
                }
            })?;
            v.visit_stmt(block)
        }
        StmtKind::ForEach {
            decls, rhs, block, ..
        } => {
            for d in decls.into_iter().flatten() {
                v.visit_stmt(d)?;
            }
            transform_visit(v, rhs, |ast, new| {
                if let StmtKind::ForEach { rhs, .. } = &mut ast.stmt_mut(id).kind {
                    *rhs = new;
                }
            })?;
            v.visit_stmt(block)
        }
        StmtKind::If {
            cond,
            block,
            else_block,
        } => {
            transform_visit(v, need(cond)?, |ast, new| {
                if let StmtKind::If { cond, .. } = &mut ast.stmt_mut(id).kind {
                    *cond = Some(new);
                }
            })?;
            v.visit_stmt(block)?;
            if let Some(e) = else_block {
                v.visit_stmt(e)?;
            }
            Ok(())
        }
        StmtKind::Return(Some(e)) => {
            transform_visit(v, e, |ast, new| {
                ast.stmt_mut(id).kind = StmtKind::Return(Some(new));
            })?;
            Ok(())
        }
        StmtKind::Return(None) | StmtKind::Continue | StmtKind::Break => Ok(()),
        StmtKind::While { cond, block } => {
            transform_visit(v, need(cond)?, |ast, new| {
                if let StmtKind::While { cond, .. } = &mut ast.stmt_mut(id).kind {
                    *cond = Some(new);
                }
            })?;
            v.visit_stmt(block)
        }
        StmtKind::Block(stmts) => {
            for s in stmts {
                v.visit_stmt(s)?;
            }
            Ok(())
        }
        StmtKind::ShowMessage { message, args, .. } => {
            v.visit_expr(message)?;
            for a in args {
                v.visit_expr(a)?;
            }
            Ok(())
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            transform_visit(v, need(expr)?, |ast, new| {
                if let StmtKind::Match { expr, .. } = &mut ast.stmt_mut(id).kind {
                    *expr = Some(new);
                }
            })?;
            for (i, arm) in arms.iter().enumerate() {
                if let Some(b) = arm.binding {
                    v.visit_expr(b)?;
                }
                transform_visit(v, arm.expr, |ast, new| {
                    if let StmtKind::Match { arms, .. } = &mut ast.stmt_mut(id).kind {
                        arms[i].expr = new;
                    }
                })?;
                v.visit_stmt(arm.block)?;
            }
            if let Some(d) = default {
                v.visit_stmt(d)?;
            }
            Ok(())
        }
    }
}

pub fn set_decl_value(ast: &mut Ast, stmt: StmtId, i: usize, new: ExprId) {
    if let StmtKind::VarDecl { decls, .. } = &mut ast.stmt_mut(stmt).kind {
        decls[i].value = Some(new);
    }
}

/// Writes `new` into the expression slot that [`expr_slots`] numbers `slot`.
pub fn set_expr_slot(ast: &mut Ast, id: ExprId, slot: usize, new: ExprId) {
    match &mut ast.expr_mut(id).kind {
        ExprKind::Assign { value, .. } => *value = new,
        ExprKind::Ternary {
            cond, left, right, ..
        } => *[cond, left, right][slot] = new,
        ExprKind::In {
            lhs,
            values,
            expression,
            ..
        } => {
            if slot == 0 {
                *lhs = new;
            } else if values.is_empty() {
                *expression = Some(new);
            } else {
                values[slot - 1] = new;
            }
        }
        ExprKind::Binary { left, right, .. }
        | ExprKind::Subscript {
            left, index: right, ..
        } => *[left, right][slot] = new,
        ExprKind::Unary { expr, .. } => *expr = new,
        ExprKind::Call { left, args, .. } => {
            if slot == usize::MAX {
                *left = Some(new);
            } else {
                args[slot] = new;
            }
        }
        ExprKind::Get { left, .. } => *left = new,
        ExprKind::ArrayLit(values) | ExprKind::MapLit(values) => values[slot] = new,
        ExprKind::Grouping(e) => *e = Some(new),
        _ => {}
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Visitor.cpp: the Visitor::Visit*Expr
// methods.
/// The base visitor's walk of an expression.
pub fn walk_expr<V: Visitor + ?Sized>(v: &mut V, id: ExprId) -> Result<(), Fail> {
    let kind = v.ast().expr(id).kind.clone();
    let tv = |v: &mut V, child: ExprId, slot: usize| {
        transform_visit(v, child, |ast, new| set_expr_slot(ast, id, slot, new))
    };
    match kind {
        ExprKind::Assign { left, value, .. } => {
            v.visit_expr(left)?;
            tv(v, value, 0)?;
        }
        ExprKind::Ternary {
            cond, left, right, ..
        } => {
            tv(v, cond, 0)?;
            tv(v, left, 1)?;
            tv(v, right, 2)?;
        }
        ExprKind::In {
            lhs,
            values,
            expression,
            ..
        } => {
            tv(v, lhs, 0)?;
            if !values.is_empty() {
                for (i, val) in values.into_iter().enumerate() {
                    tv(v, val, i + 1)?;
                }
            } else {
                tv(v, need(expression)?, 1)?;
            }
        }
        ExprKind::Binary { left, right, .. }
        | ExprKind::Subscript {
            left, index: right, ..
        } => {
            tv(v, left, 0)?;
            tv(v, right, 1)?;
        }
        ExprKind::Unary { expr, .. } => {
            tv(v, expr, 0)?;
        }
        ExprKind::Call {
            left, ident, args, ..
        } => {
            if let Some(l) = left {
                tv(v, l, usize::MAX)?;
            }
            v.visit_expr(ident)?;
            for (i, a) in args.into_iter().enumerate() {
                tv(v, a, i)?;
            }
        }
        ExprKind::Get { left, .. } => {
            tv(v, left, 0)?;
        }
        ExprKind::ArrayLit(values) | ExprKind::MapLit(values) => {
            for (i, val) in values.into_iter().enumerate() {
                tv(v, val, i)?;
            }
        }
        ExprKind::Grouping(e) => {
            tv(v, need(e)?, 0)?;
        }
        ExprKind::Lambda { args, body, .. } => {
            for a in args {
                v.visit_stmt(a)?;
            }
            v.visit_stmt(body)?;
        }
        ExprKind::Bool(_) | ExprKind::Number { .. } | ExprKind::Str(_) | ExprKind::Ident { .. } => {
        }
    }
    Ok(())
}
