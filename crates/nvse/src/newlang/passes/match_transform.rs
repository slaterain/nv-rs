//! `MatchTransformer` (`Compiler/Passes/MatchTransformer.cpp` (xNVSE)): each `match` statement
//! made an `if`/`else if` chain. Its value is kept first in an array variable `_mrN` (one element,
//! `[value]`), which every arm reads unboxed (`*_mrN`): an arm with a binding (`x: condition`)
//! tests its condition with the binding's uses made `*_mrN`; an arm without one tests
//! `*_mrN == expression`; the default is the last `else`.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), with its quirks: the variable is made for
//! every `match`, bound or not; arms without a binding are not walked before the chain replaces
//! the statement (the walk of the chain reaches them); a binding's use is renamed to the first
//! binding of that name on the stack. A `match` with only a default arm leaves a null statement in
//! its block, which xNVSE then dereferences: a crash.

use crate::newlang::ast::*;
use crate::newlang::lexer::{Token, TokenType};

pub struct MatchTransformer<'a> {
    pub ast: &'a mut Ast,
    bindings: Vec<Vec<u8>>,
}

impl<'a> MatchTransformer<'a> {
    pub fn new(ast: &'a mut Ast) -> Self {
        MatchTransformer {
            ast,
            bindings: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Fail> {
        self.visit_ast()
    }

    fn unbox(&mut self, ident: ExprId) -> ExprId {
        self.ast.add_expr(
            ExprKind::Unary {
                op: Token::bare(TokenType::Unbox, b""),
                expr: ident,
                postfix: false,
            },
            Default::default(),
        )
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), MatchTransformer.cpp:
    // MatchTransformer::VisitBlockStmt.
    fn visit_block(&mut self, id: StmtId) -> Result<(), Fail> {
        let mut i = 0;
        while i < self.ast.block_stmts(id).len() {
            let line = self.ast.block_stmts(id)[i];
            let StmtKind::Match {
                expr,
                arms,
                default,
            } = self.ast.stmt(line).kind.clone()
            else {
                self.visit_stmt(line)?;
                i += 1;
                continue;
            };
            let value = need(expr)?;
            let binding_name = format!("_mr{}", self.bindings.len() + 1).into_bytes();
            for arm in &arms {
                if let Some(binding) = arm.binding {
                    let name = self.ast.ident_name(binding).to_vec();
                    self.bindings.push(name);
                    self.visit_expr(arm.expr)?;
                    self.visit_stmt(arm.block)?;
                    self.bindings.pop();
                    if let ExprKind::Ident { name, .. } = &mut self.ast.expr_mut(binding).kind {
                        *name = binding_name.clone();
                    }
                }
            }
            let value_span = self.ast.expr(value).span;
            let list = self
                .ast
                .add_expr(ExprKind::ArrayLit(vec![value]), value_span);
            let ident = self.ast.add_expr(
                ExprKind::Ident {
                    name: binding_name.clone(),
                    form: 0,
                    var: None,
                },
                value_span,
            );
            let decl = self.ast.add_stmt(
                StmtKind::VarDecl {
                    ty: TokenType::ArrayType,
                    decls: vec![Declaration {
                        name: ident,
                        value: Some(list),
                        info: None,
                    }],
                },
                value_span,
            );
            if let StmtKind::Block(stmts) = &mut self.ast.stmt_mut(id).kind {
                stmts.insert(i, decl);
            }
            let matched = self.unbox(ident);
            if let StmtKind::Match { expr, .. } = &mut self.ast.stmt_mut(line).kind {
                *expr = Some(matched);
            }
            // The chain.
            let mut first: Option<StmtId> = None;
            let mut current: Option<StmtId> = None;
            for arm in &arms {
                let cond = if arm.binding.is_some() {
                    arm.expr
                } else {
                    let span = self.ast.expr(arm.expr).span;
                    self.ast.add_expr(
                        ExprKind::Binary {
                            op: Token::bare(TokenType::EqEq, b""),
                            left: matched,
                            right: arm.expr,
                        },
                        span,
                    )
                };
                let block_span = self.ast.stmt(arm.block).span;
                let if_stmt = self.ast.add_stmt(
                    StmtKind::If {
                        cond: Some(cond),
                        block: arm.block,
                        else_block: None,
                    },
                    block_span,
                );
                match current {
                    None => first = Some(if_stmt),
                    Some(cur) => {
                        let wrapper = self
                            .ast
                            .add_stmt(StmtKind::Block(vec![if_stmt]), block_span);
                        set_else(self.ast, cur, wrapper);
                    }
                }
                current = Some(if_stmt);
            }
            if let (Some(d), Some(cur)) = (default, current) {
                set_else(self.ast, cur, d);
            }
            // The statement replaced by the chain (xNVSE writes a null one for no arms, which
            // the walk then calls through).
            let Some(first) = first else {
                return Err(Fail::Crash("match with no arms: null statement"));
            };
            if let StmtKind::Block(stmts) = &mut self.ast.stmt_mut(id).kind {
                stmts[i + 1] = first;
            }
            i += 1;
        }
        Ok(())
    }
}

fn set_else(ast: &mut Ast, if_stmt: StmtId, else_stmt: StmtId) {
    if let StmtKind::If { else_block, .. } = &mut ast.stmt_mut(if_stmt).kind {
        *else_block = Some(else_stmt);
    }
}

impl Visitor for MatchTransformer<'_> {
    fn ast(&mut self) -> &mut Ast {
        self.ast
    }

    fn visit_stmt(&mut self, id: StmtId) -> Result<(), Fail> {
        match self.ast.stmt(id).kind {
            StmtKind::Block(_) => self.visit_block(id),
            StmtKind::VarDecl { .. } => {
                // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), MatchTransformer.cpp:
                // MatchTransformer::VisitVarDeclStmt.
                let decls = self.ast.decls(id).to_vec();
                for (i, d) in decls.iter().enumerate() {
                    if let Some(value) = d.value {
                        transform_visit(self, value, |ast, new| set_decl_value(ast, id, i, new))?;
                    }
                    // A declaration of a binding's name ends the binding.
                    let name = self.ast.ident_name(d.name).to_vec();
                    for b in &mut self.bindings {
                        if *b == name {
                            b.clear();
                        }
                    }
                }
                Ok(())
            }
            _ => walk_stmt(self, id),
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), MatchTransformer.cpp:
    // MatchTransformer::TransformExpr.
    /// A binding's use, made `*_mrN` (N the binding's place on the stack, from 1).
    fn transform(&mut self, id: ExprId) -> Result<ExprId, Fail> {
        if self.bindings.is_empty() || !self.ast.is_ident(id) {
            return Ok(id);
        }
        let mut out = id;
        for i in 0..self.bindings.len() {
            if self.bindings[i].as_slice() == self.ast.ident_name(id) {
                if let ExprKind::Ident { name, .. } = &mut self.ast.expr_mut(id).kind {
                    *name = format!("_mr{}", i + 1).into_bytes();
                }
                out = self.unbox(id);
            }
        }
        Ok(out)
    }
}
