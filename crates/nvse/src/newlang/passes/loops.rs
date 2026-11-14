//! `LoopTransformer` (`Compiler/Passes/LoopTransformer.cpp` (xNVSE)): each `for` loop made a
//! `while` loop. Its initializer goes before the loop, its increment to the end of the loop's
//! block and before each `continue` directly in that block (not in a nested loop or lambda); a
//! `continue` ending the block is dropped first.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). The same increment statement is shared
//! by every place it is put, and a `for` loop's new `while` is walked once more when an
//! initializer was moved before it, as xNVSE's loop does.

use crate::newlang::ast::*;

pub struct LoopTransformer<'a> {
    pub ast: &'a mut Ast,
    /// The increment of each open loop (`loopIncrements`): none for a `while`, `for`-in or
    /// lambda.
    increments: Vec<Option<StmtId>>,
}

impl<'a> LoopTransformer<'a> {
    pub fn new(ast: &'a mut Ast) -> Self {
        LoopTransformer {
            ast,
            increments: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Fail> {
        self.visit_ast()
    }

    /// A block's statements. Only blocks reach here (a loop's body and the blocks holding
    /// statements are always blocks); anything else fails the compile rather than stop the game.
    fn stmts_mut(&mut self, block: StmtId) -> Result<&mut Vec<StmtId>, Fail> {
        match &mut self.ast.stmt_mut(block).kind {
            StmtKind::Block(s) => Ok(s),
            _ => Err(Fail::Crash("LoopTransformer: not a block")),
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), LoopTransformer.cpp:
    // LoopTransformer::VisitBlockStmt.
    fn visit_block(&mut self, id: StmtId) -> Result<(), Fail> {
        let mut i = 0;
        while i < self.ast.block_stmts(id).len() {
            let it = self.ast.block_stmts(id)[i];
            match self.ast.stmt(it).kind.clone() {
                StmtKind::For {
                    init,
                    cond,
                    post,
                    block,
                } => {
                    let span = self.ast.stmt(it).span;
                    let mut at = i;
                    if let Some(init) = init {
                        self.stmts_mut(id)?.insert(i, init);
                        if let StmtKind::For { init, .. } = &mut self.ast.stmt_mut(it).kind {
                            *init = None;
                        }
                        at = i + 1;
                    }
                    let post = need(post)?;
                    let post_span = self.ast.expr(post).span;
                    let incr = self.ast.add_stmt(StmtKind::Expr(Some(post)), post_span);
                    let last = self.ast.block_stmts(block).last().copied();
                    if let Some(l) =
                        last.filter(|&l| matches!(self.ast.stmt(l).kind, StmtKind::Continue))
                    {
                        let s = self.ast.stmt(l).span;
                        note(
                            self.ast,
                            b"Removed redundant 'continue' from end of loop block",
                            s,
                        )?;
                        self.stmts_mut(block)?.pop();
                    }
                    self.increments.push(Some(incr));
                    self.visit_stmt(block)?;
                    self.increments.pop();
                    let s = self.ast.stmt(block).span;
                    note(self.ast, b"Inserted <incr> before end of block", s)?;
                    self.stmts_mut(block)?.push(incr);
                    let w = self.ast.add_stmt(StmtKind::While { cond, block }, span);
                    self.stmts_mut(id)?[at] = w;
                }
                StmtKind::Continue => {
                    if let Some(Some(incr)) = self.increments.last().copied() {
                        let s = self.ast.stmt(it).span;
                        note(self.ast, b"Inserted <incr> before continue stmt", s)?;
                        self.stmts_mut(id)?.insert(i, incr);
                        i += 1;
                    }
                }
                _ => self.visit_stmt(it)?,
            }
            i += 1;
        }
        Ok(())
    }

    fn without_increment(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<(), Fail>,
    ) -> Result<(), Fail> {
        self.increments.push(None);
        f(self)?;
        self.increments.pop();
        Ok(())
    }
}

impl Visitor for LoopTransformer<'_> {
    fn ast(&mut self) -> &mut Ast {
        self.ast
    }

    fn visit_stmt(&mut self, id: StmtId) -> Result<(), Fail> {
        match self.ast.stmt(id).kind {
            StmtKind::Block(_) => self.visit_block(id),
            StmtKind::For { .. } | StmtKind::ForEach { .. } | StmtKind::While { .. } => {
                self.without_increment(|v| walk_stmt(v, id))
            }
            _ => walk_stmt(self, id),
        }
    }

    fn visit_expr(&mut self, id: ExprId) -> Result<(), Fail> {
        if matches!(self.ast.expr(id).kind, ExprKind::Lambda { .. }) {
            return self.without_increment(|v| walk_expr(v, id));
        }
        walk_expr(self, id)
    }
}

/// xNVSE's debug note of a change: printed by debug builds only, but its text is made in every
/// build, and making it can throw (a place past its line).
pub fn note(ast: &Ast, msg: &[u8], span: crate::newlang::lexer::Span) -> Result<(), Fail> {
    crate::newlang::highlight(&ast.lines, msg, span, crate::newlang::CYAN).map(|_| ())
}
