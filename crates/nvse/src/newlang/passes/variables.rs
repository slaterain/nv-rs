//! `VariableResolution` (`Compiler/Passes/VariableResolution.cpp` (xNVSE)): each declaration
//! made a variable of its scope, each identifier tied to the variable its scopes find, and the
//! script's variable list made again from them.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). Scopes: the script's (the root), and one
//! per event block, function body, block, `if` (its condition and block; its `else` another),
//! loop and lambda. A name is looked up as written (case counts), inner scope first. A variable of
//! the root scope (a global, a function's parameter, a variable the script had before that is
//! not a `__temp`) keeps its name and takes the next index from 1; one of an inner scope is named
//! `__temp_<name>_<n>` and numbered after every root variable. A variable the script had before
//! keeps its place in the order and takes the type of a declaration of its name.

use crate::newlang::ast::*;
use crate::newlang::{highlight, script_type_of, variable_token_type, Env, RED};
use std::collections::BTreeMap;

struct Scope {
    is_root: bool,
    parent: Option<usize>,
    variables: BTreeMap<Vec<u8>, VarId>,
}

pub struct VariableResolution<'a> {
    pub ast: &'a mut Ast,
    env: &'a mut dyn Env,
    /// Whether a script is compiling (`pScript`): without one the script's variables are not
    /// read or written (`PreProcessNVSEScript` for a user function's parameters).
    has_script: bool,
    globals: Vec<VarId>,
    temps: Vec<VarId>,
    scopes: Vec<Scope>,
    stack: Vec<usize>,
    pub had_error: bool,
}

impl<'a> VariableResolution<'a> {
    pub fn new(ast: &'a mut Ast, env: &'a mut dyn Env, has_script: bool) -> Self {
        VariableResolution {
            ast,
            env,
            has_script,
            globals: Vec::new(),
            temps: Vec::new(),
            scopes: Vec::new(),
            stack: Vec::new(),
            had_error: false,
        }
    }

    fn current(&self) -> usize {
        *self.stack.last().expect("a scope is always open")
    }

    fn enter_scope(&mut self) {
        let parent = self.stack.last().copied();
        self.scopes.push(Scope {
            is_root: false,
            parent,
            variables: BTreeMap::new(),
        });
        self.stack.push(self.scopes.len() - 1);
    }

    fn leave_scope(&mut self) {
        self.stack.pop();
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), VariableResolution.cpp:
    // Scope::AddVariable.
    /// A new variable of the current scope; `None` when the scope has the name already (unless
    /// that one is the script's own, which then takes this type).
    fn add_variable(&mut self, name: &[u8], ty: u8) -> Option<VarId> {
        let scope = self.current();
        if let Some(&existing) = self.scopes[scope].variables.get(name) {
            let v = self.ast.var_mut(existing);
            if v.pre_existing {
                v.ty = ty;
                v.pre_existing = false;
                return Some(existing);
            }
            return None;
        }
        let mut var = VarInfo {
            index: self.globals.len() as u32 + 1,
            original_name: name.to_vec(),
            remapped_name: Vec::new(),
            pre_existing: false,
            ty,
            detailed_type: variable_token_type(ty),
            is_lambda: false,
            lambda_param_types: Vec::new(),
            lambda_return_type: crate::compiler::tt::INVALID,
        };
        let is_root = self.scopes[scope].is_root;
        if is_root {
            var.remapped_name = name.to_vec();
        } else {
            var.remapped_name = [
                b"__temp_".as_slice(),
                name,
                format!("_{}", self.temps.len() + 1).as_bytes(),
            ]
            .concat();
        }
        let id = self.ast.add_var(var);
        if is_root {
            self.globals.push(id);
        } else {
            self.temps.push(id);
        }
        self.scopes[scope].variables.insert(name.to_vec(), id);
        Some(id)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), VariableResolution.cpp: Scope::Lookup.
    fn lookup(&self, name: &[u8]) -> Option<VarId> {
        let mut scope = Some(self.current());
        while let Some(s) = scope {
            if let Some(&v) = self.scopes[s].variables.get(name) {
                return Some(v);
            }
            scope = self.scopes[s].parent;
        }
        None
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), VariableResolution.cpp:
    // VariableResolution::Visit.
    pub fn run(&mut self) -> Result<(), Fail> {
        self.scopes.push(Scope {
            is_root: true,
            parent: None,
            variables: BTreeMap::new(),
        });
        self.stack.push(0);
        if self.has_script {
            for (name, ty) in self.env.script_variables().unwrap_or_default() {
                if name.starts_with(b"__temp") {
                    continue;
                }
                // A second variable of the same name: xNVSE marks the null it gets back.
                let added = self
                    .add_variable(&name, ty)
                    .ok_or(Fail::Crash("VariableResolution: a script variable twice"))?;
                self.ast.var_mut(added).pre_existing = true;
            }
        }
        let globals = self.ast.globals.clone();
        for g in globals {
            self.visit_stmt(g)?;
        }
        let blocks = self.ast.blocks.clone();
        for b in blocks {
            if let StmtKind::Udf { args, body, .. } = self.ast.stmt(b).kind.clone() {
                for a in args {
                    self.visit_stmt(a)?;
                }
                self.enter_scope();
                self.visit_stmt(body)?;
                self.leave_scope();
            } else {
                self.visit_stmt(b)?;
            }
        }
        let first = self.globals.len() as u32 + 1;
        for (index, &t) in (first..).zip(&self.temps) {
            self.ast.var_mut(t).index = index;
        }
        if self.has_script {
            let list: Vec<(Vec<u8>, u8, u32)> = self
                .globals
                .iter()
                .chain(&self.temps)
                .map(|&v| {
                    let v = self.ast.var(v);
                    (v.remapped_name.clone(), v.ty, v.index)
                })
                .collect();
            self.env.set_script_variables(&list);
        }
        Ok(())
    }

    fn scoped(&mut self, f: impl FnOnce(&mut Self) -> Result<(), Fail>) -> Result<(), Fail> {
        self.enter_scope();
        f(self)?;
        self.leave_scope();
        Ok(())
    }
}

impl Visitor for VariableResolution<'_> {
    fn ast(&mut self) -> &mut Ast {
        self.ast
    }

    fn visit_stmt(&mut self, id: StmtId) -> Result<(), Fail> {
        let kind = self.ast.stmt(id).kind.clone();
        match kind {
            // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), VariableResolution.cpp:
            // VariableResolution::VisitVarDeclStmt.
            StmtKind::VarDecl { ty, decls } => {
                for (i, d) in decls.iter().enumerate() {
                    let name = self.ast.ident_name(d.name).to_vec();
                    match self.add_variable(&name, script_type_of(ty)) {
                        Some(var) => {
                            if let StmtKind::VarDecl { decls, .. } = &mut self.ast.stmt_mut(id).kind
                            {
                                decls[i].info = Some(var);
                            }
                            if let Some(value) = d.value {
                                self.visit_expr(value)?;
                            }
                        }
                        None => {
                            let span = self.ast.expr(d.name).span;
                            let msg = highlight(
                                &self.ast.lines,
                                b"Variable with this name already exists in this scope",
                                span,
                                RED,
                            )?;
                            self.env.error(&msg);
                            self.had_error = true;
                        }
                    }
                }
                Ok(())
            }
            StmtKind::Begin { block, .. } => self.scoped(|v| v.visit_stmt(block)),
            StmtKind::Udf { args, body, .. } => self.scoped(|v| {
                for a in args {
                    v.visit_stmt(a)?;
                }
                v.visit_stmt(body)
            }),
            StmtKind::For { .. } | StmtKind::ForEach { .. } => self.scoped(|v| walk_stmt(v, id)),
            StmtKind::If {
                cond,
                block,
                else_block,
            } => {
                self.scoped(|v| {
                    v.visit_expr(need(cond)?)?;
                    v.visit_stmt(block)
                })?;
                if let Some(e) = else_block {
                    self.scoped(|v| v.visit_stmt(e))?;
                }
                Ok(())
            }
            StmtKind::While { cond, block } => self.scoped(|v| {
                v.visit_expr(need(cond)?)?;
                v.visit_stmt(block)
            }),
            StmtKind::Block(stmts) => self.scoped(|v| {
                for s in stmts {
                    v.visit_stmt(s)?;
                }
                Ok(())
            }),
            _ => walk_stmt(self, id),
        }
    }

    fn visit_expr(&mut self, id: ExprId) -> Result<(), Fail> {
        match self.ast.expr(id).kind.clone() {
            ExprKind::Ident { name, .. } => {
                let found = self.lookup(&name);
                if let ExprKind::Ident { var, .. } = &mut self.ast.expr_mut(id).kind {
                    *var = found;
                }
                Ok(())
            }
            ExprKind::Lambda { args, body, .. } => self.scoped(|v| {
                for a in args {
                    v.visit_stmt(a)?;
                }
                v.visit_stmt(body)
            }),
            _ => walk_expr(self, id),
        }
    }
}
