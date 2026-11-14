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

/// The deepest tree the port compiles: statements and expressions nested along one path (each
/// `(`, `!`, block, operator of a chain like `a + b + ...`, call in a call counts one; an `if`,
/// an `else if`, a match arm after the match pass two; a lambda three, four after the lambda
/// pass). Every pass and the code generation walk the tree recursively, as xNVSE's do, so each
/// level costs native stack; xNVSE has no limit and overflows its stack there, which stops the
/// game. Past this the compile fails silently instead ([`TOO_DEEP`], a deliberate difference
/// like xNVSE's other crashes).
///
/// Measured on the debug test build (the biggest frames), as the smallest thread stack each
/// shape compiles on (2026-11-14): the walks after the parse take about 4.3 KB of stack a level
/// (a chain of 2044 `+` needs 8.5 MB), so this limit uses under 9 MB of the 64 MB stack each
/// game thread has (`engine_core::thread_host`): a seventh of it, the rest left to the game's own
/// frames below the compile. It still compiles any real script (an `else if` chain of 500, a
/// `match` of 900 arms; `codegen_tests` compiles both on a 16 MB thread).
pub const MAX_DEPTH: usize = 2048;

/// The deepest the parser recurses ([`MAX_DEPTH`]'s twin for the parse, and for strings lexed
/// inside interpolations): every statement, `else if`, expression (each parenthesis, call
/// argument, list, subscript, interpolation, an assignment's right side) and operand after a
/// prefix operator one level. An expression goes down the whole precedence ladder (about thirty
/// frames), so a level of parentheses costs about 19 KB in the debug build (2000 of them need
/// 36 MB): this limit keeps the parse under 10 MB, the same share of a game thread's stack.
/// xNVSE's parser has no limit: past what its thread's stack holds, the game stops.
pub const MAX_PARSE_DEPTH: usize = 512;

/// The failure past [`MAX_DEPTH`] or [`MAX_PARSE_DEPTH`].
pub const TOO_DEEP: Fail = Fail::Crash("nesting past the depth limit (xNVSE overflows its stack)");

/// A node of the tree, for [`Ast::deeper_than`].
#[derive(Clone, Copy)]
enum Node {
    S(StmtId),
    E(ExprId),
}

impl Ast {
    /// The children a walk of `n` reaches (every statement and expression slot, as
    /// [`walk_stmt`] and [`walk_expr`] and the code generation visit them).
    fn children(&self, n: Node, out: &mut Vec<Node>) {
        let e = |out: &mut Vec<Node>, x: &Option<ExprId>| {
            if let Some(x) = x {
                out.push(Node::E(*x));
            }
        };
        match n {
            Node::S(id) => match &self.stmt(id).kind {
                StmtKind::Begin { block, .. } => out.push(Node::S(*block)),
                StmtKind::Udf { args, body, .. } => {
                    out.extend(args.iter().map(|a| Node::S(*a)));
                    out.push(Node::S(*body));
                }
                StmtKind::VarDecl { decls, .. } => {
                    for d in decls {
                        out.push(Node::E(d.name));
                        e(out, &d.value);
                    }
                }
                StmtKind::Expr(x) | StmtKind::Return(x) => e(out, x),
                StmtKind::For {
                    init,
                    cond,
                    post,
                    block,
                } => {
                    if let Some(i) = init {
                        out.push(Node::S(*i));
                    }
                    e(out, cond);
                    e(out, post);
                    out.push(Node::S(*block));
                }
                StmtKind::ForEach {
                    decls, rhs, block, ..
                } => {
                    out.extend(decls.iter().flatten().map(|d| Node::S(*d)));
                    out.push(Node::E(*rhs));
                    out.push(Node::S(*block));
                }
                StmtKind::If {
                    cond,
                    block,
                    else_block,
                } => {
                    e(out, cond);
                    out.push(Node::S(*block));
                    if let Some(b) = else_block {
                        out.push(Node::S(*b));
                    }
                }
                StmtKind::While { cond, block } => {
                    e(out, cond);
                    out.push(Node::S(*block));
                }
                StmtKind::Block(stmts) => out.extend(stmts.iter().map(|s| Node::S(*s))),
                StmtKind::ShowMessage { message, args, .. } => {
                    out.push(Node::E(*message));
                    out.extend(args.iter().map(|a| Node::E(*a)));
                }
                StmtKind::Match {
                    expr,
                    arms,
                    default,
                } => {
                    e(out, expr);
                    for arm in arms {
                        e(out, &arm.binding);
                        out.push(Node::E(arm.expr));
                        out.push(Node::S(arm.block));
                    }
                    if let Some(d) = default {
                        out.push(Node::S(*d));
                    }
                }
                StmtKind::Continue | StmtKind::Break => {}
            },
            Node::E(id) => match &self.expr(id).kind {
                ExprKind::Assign { left, value, .. } => {
                    out.extend([Node::E(*left), Node::E(*value)]);
                }
                ExprKind::Ternary {
                    cond, left, right, ..
                } => out.extend([Node::E(*cond), Node::E(*left), Node::E(*right)]),
                ExprKind::In {
                    lhs,
                    values,
                    expression,
                    ..
                } => {
                    out.push(Node::E(*lhs));
                    out.extend(values.iter().map(|v| Node::E(*v)));
                    e(out, expression);
                }
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    left, index: right, ..
                } => out.extend([Node::E(*left), Node::E(*right)]),
                ExprKind::Unary { expr, .. } => out.push(Node::E(*expr)),
                ExprKind::Call {
                    left, ident, args, ..
                } => {
                    e(out, left);
                    out.push(Node::E(*ident));
                    out.extend(args.iter().map(|a| Node::E(*a)));
                }
                ExprKind::Get { left, ident, .. } => {
                    out.extend([Node::E(*left), Node::E(*ident)]);
                }
                ExprKind::ArrayLit(values) | ExprKind::MapLit(values) => {
                    out.extend(values.iter().map(|v| Node::E(*v)));
                }
                ExprKind::Grouping(x) => e(out, x),
                ExprKind::Lambda { args, body, .. } => {
                    out.extend(args.iter().map(|a| Node::S(*a)));
                    out.push(Node::S(*body));
                }
                ExprKind::Bool(_)
                | ExprKind::Number { .. }
                | ExprKind::Str(_)
                | ExprKind::Ident { .. } => {}
            },
        }
    }

    /// Whether some path from a global or block nests more than `limit` nodes (counting both
    /// ends). Walked with a stack of its own, so any tree is measured without recursion; it stops
    /// at the first path past the limit.
    pub fn deeper_than(&self, limit: usize) -> bool {
        let mut todo: Vec<(Node, usize)> = self
            .globals
            .iter()
            .chain(&self.blocks)
            .map(|s| (Node::S(*s), 1))
            .collect();
        let mut kids = Vec::new();
        while let Some((n, depth)) = todo.pop() {
            if depth > limit {
                return true;
            }
            kids.clear();
            self.children(n, &mut kids);
            todo.extend(kids.iter().map(|k| (*k, depth + 1)));
        }
        false
    }
}

/// [`TOO_DEEP`] when the tree nests past [`MAX_DEPTH`]: checked after the parse, after the match
/// pass (which nests without bound) and before the code generation, so no recursive walk goes
/// much deeper than the limit (`compile_with`).
pub fn check_depth(ast: &Ast) -> Result<(), Fail> {
    if ast.deeper_than(MAX_DEPTH) {
        Err(TOO_DEEP)
    } else {
        Ok(())
    }
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
