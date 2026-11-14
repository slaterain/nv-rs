//! `LambdaTransformer` (`Compiler/Passes/LambdaTransformer.cpp` (xNVSE)): a variable declared
//! holding a lambda is not a variable in the compiled script: its declaration becomes
//! `SetModLocalData("__lambda_<index>_<name>", lambda)` and each read of it (where a pass may
//! replace an expression) `GetModLocalData("__lambda_<index>_<name>")`.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). A declaration's own value is walked
//! without that replacement (xNVSE visits it directly), so `ref g = f;` keeps reading `f`.

use crate::newlang::ast::*;
use crate::newlang::passes::loops::note;
use crate::newlang::Env;

pub struct LambdaTransformer<'a> {
    pub ast: &'a mut Ast,
    env: &'a mut dyn Env,
}

impl<'a> LambdaTransformer<'a> {
    pub fn new(ast: &'a mut Ast, env: &'a mut dyn Env) -> Self {
        LambdaTransformer { ast, env }
    }

    pub fn run(&mut self) -> Result<(), Fail> {
        self.visit_ast()
    }

    /// `"__lambda_<index>_<name>"` as a string expression at `span`.
    fn key(&mut self, var: VarId, name: &[u8], span: crate::newlang::lexer::Span) -> ExprId {
        let index = self.ast.var(var).index;
        let text = [format!("__lambda_{index}_").as_bytes(), name].concat();
        self.ast.add_expr(ExprKind::Str(text), span)
    }

    /// A call of `command` with `args`, its record set.
    fn call(
        &mut self,
        command: &[u8],
        args: Vec<ExprId>,
        span: crate::newlang::lexer::Span,
    ) -> ExprId {
        let ident = self.ast.add_expr(
            ExprKind::Ident {
                name: command.to_vec(),
                form: 0,
                var: None,
            },
            span,
        );
        let cmd = self.env.command(command);
        self.ast.add_expr(
            ExprKind::Call {
                left: None,
                ident,
                args,
                cmd,
            },
            span,
        )
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), LambdaTransformer.cpp:
    // LambdaTransformer::VisitBlockStmt.
    fn visit_block(&mut self, id: StmtId) -> Result<(), Fail> {
        let mut i = 0usize;
        while i < self.ast.block_stmts(id).len() {
            let it = self.ast.block_stmts(id)[i];
            if !matches!(self.ast.stmt(it).kind, StmtKind::VarDecl { .. }) {
                self.visit_stmt(it)?;
                i += 1;
                continue;
            }
            let mut j = 0usize;
            while j < self.ast.decls(it).len() {
                let d = self.ast.decls(it)[j].clone();
                let lambda = d.info.filter(|&v| self.ast.var(v).is_lambda);
                if let Some(var) = lambda {
                    let span = self.ast.expr(d.name).span;
                    let name = self.ast.ident_name(d.name).to_vec();
                    let key = self.key(var, &name, span);
                    let value = need(d.value)?;
                    let set = self.call(b"SetModLocalData", vec![key, value], span);
                    let stmt = self.ast.add_stmt(StmtKind::Expr(Some(set)), span);
                    if let StmtKind::Block(s) = &mut self.ast.stmt_mut(id).kind {
                        s.insert(i, stmt);
                    }
                    let value_span = self.ast.expr(value).span;
                    note(
                        self.ast,
                        b"Built SetModLocalData call for ident",
                        span.plus(value_span),
                    )?;
                    if let StmtKind::VarDecl { decls, .. } = &mut self.ast.stmt_mut(it).kind {
                        decls.remove(j);
                    }
                    i += 1;
                } else {
                    self.visit_expr(d.name)?;
                    if let Some(value) = d.value {
                        self.visit_expr(value)?;
                    }
                    j += 1;
                }
            }
            if self.ast.decls(it).is_empty() {
                if let StmtKind::Block(s) = &mut self.ast.stmt_mut(id).kind {
                    s.remove(i);
                }
                let s = self.ast.stmt(it).span;
                note(self.ast, b"Erased old variable declaration", s)?;
            } else {
                i += 1;
            }
        }
        Ok(())
    }
}

impl Visitor for LambdaTransformer<'_> {
    fn ast(&mut self) -> &mut Ast {
        self.ast
    }

    fn visit_stmt(&mut self, id: StmtId) -> Result<(), Fail> {
        if matches!(self.ast.stmt(id).kind, StmtKind::Block(_)) {
            return self.visit_block(id);
        }
        walk_stmt(self, id)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), LambdaTransformer.cpp:
    // LambdaTransformer::TransformExpr.
    fn transform(&mut self, id: ExprId) -> Result<ExprId, Fail> {
        let ExprKind::Ident {
            name,
            var: Some(var),
            ..
        } = self.ast.expr(id).kind.clone()
        else {
            return Ok(id);
        };
        if !self.ast.var(var).is_lambda {
            return Ok(id);
        }
        let span = self.ast.expr(id).span;
        let key = self.key(var, &name, span);
        let call = self.call(b"GetModLocalData", vec![key], span);
        note(
            self.ast,
            b"Transformed lambda read into GetModLocalData",
            span,
        )?;
        Ok(call)
    }
}
