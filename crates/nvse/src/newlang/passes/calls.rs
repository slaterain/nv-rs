//! `CallTransformer` (`Compiler/Passes/CallTransformer.cpp` (xNVSE)): a call of a reference
//! variable (a lambda, `f(args)`) or of a script form by its editor id (a user function,
//! `MyFunction(args)`) made a call of `Call` with the callee first (`Call(f, args)`).
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a).

use crate::compiler::vt;
use crate::newlang::ast::*;
use crate::newlang::Env;

pub struct CallTransformer<'a> {
    pub ast: &'a mut Ast,
    env: &'a mut dyn Env,
}

impl<'a> CallTransformer<'a> {
    pub fn new(ast: &'a mut Ast, env: &'a mut dyn Env) -> Self {
        CallTransformer { ast, env }
    }

    pub fn run(&mut self) -> Result<(), Fail> {
        self.visit_ast()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CallTransformer.cpp:
    // CallTransformer::TransformCall.
    fn transform_call(&mut self, call: ExprId) -> Result<(), Fail> {
        let ExprKind::Call { ident, .. } = self.ast.expr(call).kind else {
            return Ok(());
        };
        let span = self.ast.expr(ident).span;
        let new_ident = self.ast.add_expr(
            ExprKind::Ident {
                name: b"Call".to_vec(),
                form: 0,
                var: None,
            },
            span,
        );
        if let ExprKind::Call { ident, args, .. } = &mut self.ast.expr_mut(call).kind {
            args.insert(0, *ident);
            *ident = new_ident;
        }
        // xNVSE's debug note: its text is made in every build, and making it can throw.
        let span = self.ast.expr(call).span;
        crate::newlang::highlight(
            &self.ast.lines,
            b"Transformed call expression",
            span,
            crate::newlang::CYAN,
        )?;
        Ok(())
    }
}

impl Visitor for CallTransformer<'_> {
    fn ast(&mut self) -> &mut Ast {
        self.ast
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CallTransformer.cpp:
    // CallTransformer::VisitCallExpr.
    fn visit_expr(&mut self, id: ExprId) -> Result<(), Fail> {
        let ExprKind::Call { left, ident, .. } = self.ast.expr(id).kind.clone() else {
            return walk_expr(self, id);
        };
        if left.is_none() {
            let var = self.ast.ident_var(ident);
            if var.is_some_and(|v| self.ast.var(v).ty == vt::REF) {
                self.transform_call(id)?;
            } else {
                let name = self.ast.ident_name(ident).to_vec();
                let form = self.env.form_by_name(&name);
                if form != 0 && self.env.script_form_type(form).is_some() {
                    self.transform_call(id)?;
                }
            }
        }
        walk_expr(self, id)
    }
}
