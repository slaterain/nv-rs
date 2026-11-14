//! The new script language's parser (`Compiler/Parser/Parser.cpp`, `Parser.h` (xNVSE)): a
//! recursive descent over the lexer's tokens into the syntax tree, with xNVSE's error messages
//! ("[line L:C] message") and its recovery (to the next `;`, statement keyword or `}`).
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). A lexer error is printed ("[line L]
//! message") and the parser goes on with the token it had (what xNVSE's moved-from token holds:
//! its type and place, no text), without marking the parse failed, as xNVSE does.

use super::ast::*;
use super::lexer::{LexFail, Lexer, Span, Token, TokenType as T, Value};
use super::msvc::MsvcMap;
use super::{make_version, Env};

/// What the parse gives: the tree (`None` when the script's start was wrong), whether an error
/// was reported (`HadError`), and the `#version` statements' plugin versions
/// (`g_currentCompilerPluginVersions.top()`).
pub struct Parsed {
    pub ast: Option<Ast>,
    pub had_error: bool,
    pub plugin_versions: MsvcMap<u32>,
}

pub struct Parser<'a> {
    env: &'a mut dyn Env,
    lexer: Lexer,
    current: Token,
    previous: Token,
    panic_mode: bool,
    had_error: bool,
    ast: Ast,
    plugin_versions: MsvcMap<u32>,
}

type R<T> = Result<T, Fail>;

impl<'a> Parser<'a> {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Parser.
    pub fn new(env: &'a mut dyn Env, text: &[u8]) -> R<Parser<'a>> {
        let mut p = Parser {
            env,
            lexer: Lexer::new(text),
            current: Token::default(),
            previous: Token::default(),
            panic_mode: false,
            had_error: false,
            ast: Ast::default(),
            plugin_versions: MsvcMap::default(),
        };
        p.advance()?;
        Ok(p)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Parse.
    /// `Parse`: `name <identifier>;`, then version statements, event blocks, a function block
    /// and global variable declarations to the end.
    pub fn parse(mut self) -> R<Parsed> {
        match self.parse_script() {
            Ok(()) => {}
            Err(Fail::Caught) => {
                self.had_error = true;
                return Ok(Parsed {
                    ast: None,
                    had_error: true,
                    plugin_versions: self.plugin_versions,
                });
            }
            Err(crash) => return Err(crash),
        }
        self.ast.lines = std::mem::take(&mut self.lexer.lines);
        Ok(Parsed {
            ast: Some(self.ast),
            had_error: self.had_error,
            plugin_versions: self.plugin_versions,
        })
    }

    fn parse_script(&mut self) -> R<()> {
        self.expect_msg(T::Name, "Expected 'name' as first statement of script.")?;
        let name_expr = self.ident_expr()?;
        self.expect_msg(T::Semicolon, "Expected ';'.")?;
        self.ast.name = self.ast.ident_name(name_expr).to_vec();
        while self.current.ty != T::Eof {
            match self.top_level() {
                Ok(()) => {}
                Err(Fail::Caught) => {
                    self.had_error = true;
                    self.synchronize()?;
                }
                Err(crash) => return Err(crash),
            }
        }
        Ok(())
    }

    fn top_level(&mut self) -> R<()> {
        if self.matches(T::Pound)? {
            if !self.matches(T::Identifier)? {
                self.error_at_current("Expected 'version'.")?;
            }
            if !self.previous.lexeme.eq_ignore_ascii_case(b"version") {
                self.error_at_current("Expected 'version'.")?;
            }
            self.expect_msg(T::LeftParen, "Expected '('.")?;
            let plugin = self.expect_msg(T::String, "Expected plugin name.")?;
            self.expect_msg(T::Comma, "Expected ','.")?;
            let Value::Text(name) = &plugin.value else {
                return Err(Fail::Crash("std::get<std::string>: bad_variant_access"));
            };
            let name = name.to_ascii_lowercase();
            // `StrEqual` (the game's upper case table; the plugin name is ASCII here).
            if name.eq_ignore_ascii_case(b"nvse") {
                let major = self.version_number("Expected major version")?;
                let mut minor = 255;
                let mut beta = 255;
                if self.matches(T::Comma)? {
                    minor = self.version_number("Expected minor version")?;
                }
                if self.matches(T::Comma)? {
                    beta = self.version_number("Expected beta version")?;
                }
                self.plugin_versions
                    .set(&name, make_version(major, minor, beta));
            } else {
                let version = self.version_number("Expected plugin version")?;
                if !self.env.plugin_listed(&name) {
                    let msg = format!(
                        "No plugin with name {} could be found.\n",
                        String::from_utf8_lossy(&name)
                    );
                    self.error_at_current(&msg)?;
                }
                self.plugin_versions.set(&name, version as u32);
            }
            self.expect_msg(T::RightParen, "Expected ')'.")?;
        } else if self.peek_block_type() {
            let b = self.begin()?;
            self.ast.blocks.push(b);
        } else if self.matches(T::Fn)? {
            let fn_token = self.previous.clone();
            let args = self.parse_args()?;
            let body = self.block_statement()?;
            let span = fn_token.span.plus(self.previous.span);
            let s = self.ast.add_stmt(
                StmtKind::Udf {
                    token: fn_token,
                    args,
                    body,
                },
                span,
            );
            self.ast.blocks.push(s);
        } else if self.peek_type() {
            let decl = self.var_decl(true, false)?;
            self.ast.globals.push(decl);
            self.expect_msg(T::Semicolon, "Expected ';' at end of statement.")?;
        } else {
            self.error_at_current(
                "Expected variable declaration, block type (GameMode, MenuMode, ...), or 'fn'.",
            )?;
        }
        Ok(())
    }

    /// A version number (`static_cast<int>(std::get<double>(...))`).
    fn version_number(&mut self, msg: &str) -> R<i32> {
        let tok = self.expect_msg(T::Number, msg)?;
        match tok.number() {
            Some(v) => Ok(crate::args::f64_to_i32(v)),
            None => Err(Fail::Crash("std::get<double>: bad_variant_access")),
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Begin.
    /// `Begin`: an event block, `Name[::mode] { ... }`.
    fn begin(&mut self) -> R<StmtId> {
        let start = self.previous.clone();
        self.expect_block_type("Expected block type")?;
        let block_name = self.previous.clone();
        let info = self
            .env
            .block(&block_name.lexeme)
            .ok_or(Fail::Crash("Begin: no block record"))?;
        let mut mode = None;
        if self.matches(T::MakePair)? {
            if info.params.is_empty() {
                self.error_at_current("Cannot specify argument for this block type.")?;
            }
            let first_kind = info.params.first().map(|p| p.kind);
            if self.matches(T::Number)? {
                if first_kind != Some(PARAM_INTEGER) {
                    self.error_at_current("Expected identifier.")?;
                }
                let mode_token = self.previous.clone();
                if mode_token.lexeme.contains(&b'.') {
                    self.error_at_current("Expected integer.")?;
                }
                mode = Some(mode_token);
            } else if self.matches(T::Identifier)? {
                if first_kind == Some(PARAM_INTEGER) {
                    self.error_at_current("Expected number.")?;
                }
                mode = Some(self.previous.clone());
            } else {
                self.error_at_current("Expected identifier or number.")?;
            }
        } else if info.params.first().is_some_and(|p| !p.optional) {
            let msg = format!(
                "Missing required parameter for block type '{}'",
                String::from_utf8_lossy(&block_name.lexeme)
            );
            self.error_at_current(&msg)?;
        }
        if !self.peek(T::LeftBrace) {
            self.error_at_current("Expected '{'.")?;
        }
        let block = self.block_statement()?;
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(
            StmtKind::Begin {
                name: block_name.lexeme,
                param: mode,
                block,
                info,
            },
            span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Statement.
    fn statement(&mut self) -> R<StmtId> {
        if self.peek(T::Identifier) && self.current.lexeme.eq_ignore_ascii_case(b"ShowMessage") {
            return self.show_message();
        }
        if self.peek(T::For) {
            return self.for_statement();
        }
        if self.peek(T::If) {
            return self.if_statement();
        }
        if self.peek(T::Match) {
            return self.match_statement();
        }
        if self.peek(T::Return) {
            return self.return_statement();
        }
        if self.matches(T::Break)? {
            let token = self.previous.clone();
            self.expect(T::Semicolon)?;
            let span = token.span.plus(self.previous.span);
            return Ok(self.ast.add_stmt(StmtKind::Break, span));
        }
        if self.matches(T::Continue)? {
            let token = self.previous.clone();
            self.expect(T::Semicolon)?;
            let span = token.span.plus(self.previous.span);
            return Ok(self.ast.add_stmt(StmtKind::Continue, span));
        }
        if self.peek(T::While) {
            return self.while_statement();
        }
        if self.peek(T::LeftBrace) {
            return self.block_statement();
        }
        if self.peek(T::Fn) {
            let start = self.expect(T::Fn)?;
            let ident = self.ident_expr()?;
            let args = self.parse_args()?;
            let block = self.block_statement()?;
            self.expect(T::Semicolon)?;
            let span = start.span.plus(self.previous.span);
            let lambda = self.ast.add_expr(
                ExprKind::Lambda {
                    args,
                    body: block,
                    param_types: Vec::new(),
                    return_type: crate::compiler::tt::INVALID,
                },
                span,
            );
            return Ok(self.ast.add_stmt(
                StmtKind::VarDecl {
                    ty: T::RefType,
                    decls: vec![Declaration {
                        name: ident,
                        value: Some(lambda),
                        info: None,
                    }],
                },
                span,
            ));
        }
        if self.peek_type() {
            let decl = self.var_decl(true, false)?;
            self.expect(T::Semicolon)?;
            return Ok(decl);
        }
        self.expression_statement()
    }

    /// `ShowMessage(message[, duration])`: the message box statement.
    fn show_message(&mut self) -> R<StmtId> {
        let _show_message_span = self.current.span;
        self.advance()?;
        self.expect_msg(T::LeftParen, "Expected '('")?;
        let ident = self.expect_msg(T::Identifier, "Expected message form")?;
        let message = self.ast.add_expr(
            ExprKind::Ident {
                name: ident.lexeme,
                form: 0,
                var: None,
            },
            self.previous.span,
        );
        let mut args = Vec::new();
        let mut time = 0u32;
        while self.matches(T::Comma)? {
            if self.peek(T::Number) {
                let lit = self.numeric_literal()?;
                let (value, is_fp) = match self.ast.expr(lit).kind {
                    ExprKind::Number { value, is_fp, .. } => (value, is_fp),
                    _ => (0.0, false),
                };
                if is_fp {
                    self.error_at_previous("Duration must be an integer.")?;
                }
                time = crate::commands::string::dtoui3(value);
                break;
            }
            let ident_token = self.expect_msg(T::Identifier, "Expected identifier.")?;
            // xNVSE reads the identifier's text with `std::get<2>` of its value, which an
            // identifier never has: `std::bad_variant_access`, which nothing catches.
            let Value::Text(text) = &ident_token.value else {
                return Err(Fail::Crash("std::get<std::string>: bad_variant_access"));
            };
            let a = self.ast.add_expr(
                ExprKind::Ident {
                    name: text.clone(),
                    form: 0,
                    var: None,
                },
                ident_token.span,
            );
            args.push(a);
        }
        self.expect_msg(T::RightParen, "Expected ')'")?;
        self.expect_msg(T::Semicolon, "Expected ';'")?;
        Ok(self.ast.add_stmt(
            StmtKind::ShowMessage {
                message,
                args,
                time,
            },
            Span::default(),
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::VarDecl.
    fn var_decl(&mut self, allow_value: bool, only_one: bool) -> R<StmtId> {
        if !self.matches_type()? {
            self.error_at_current("Expected type.")?;
        }
        let var_type = self.previous.clone();
        let mut decls = Vec::new();
        loop {
            let name = self.ident_expr()?;
            let mut value = None;
            if self.matches(T::Eq)? {
                if !allow_value {
                    self.error_at_previous("Variable definition is not allowed here.")?;
                }
                value = Some(self.expression()?);
            }
            decls.push(Declaration {
                name,
                value,
                info: None,
            });
            if only_one || !self.matches(T::Comma)? {
                break;
            }
        }
        let span = var_type.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(
            StmtKind::VarDecl {
                ty: var_type.ty,
                decls,
            },
            span,
        ))
    }

    /// A one-name declaration (`std::make_shared<VarDecl>(type, ident, value, span)`).
    fn single_decl(&mut self, ty: T, name: ExprId, value: Option<ExprId>, span: Span) -> StmtId {
        self.ast.add_stmt(
            StmtKind::VarDecl {
                ty,
                decls: vec![Declaration {
                    name,
                    value,
                    info: None,
                }],
            },
            span,
        )
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::ForStatement.
    fn for_statement(&mut self) -> R<StmtId> {
        let start = self.expect(T::For)?;
        let lparen = self.expect(T::LeftParen)?;
        let mut init = None;
        if !self.peek(T::Semicolon) {
            if self.matches_type()? {
                let type_token = self.previous.clone();
                let ident = self.ident_expr()?;
                if self.matches(T::In)? {
                    let span = type_token.span.plus(self.previous.span);
                    let decl = self.single_decl(type_token.ty, ident, None, span);
                    let rhs = self.expression()?;
                    self.expect_msg(T::RightParen, "Expected ')'.")?;
                    let block = self.block_statement()?;
                    return Ok(self.ast.add_stmt(
                        StmtKind::ForEach {
                            decls: vec![Some(decl)],
                            rhs,
                            block,
                            decompose: false,
                        },
                        Span::default(),
                    ));
                }
                let mut value = None;
                if self.matches(T::Eq)? {
                    value = Some(self.expression()?);
                }
                self.expect_msg(T::Semicolon, "Expected ';' after loop initializer.")?;
                let span = type_token.span.plus(self.previous.span);
                init = Some(self.single_decl(type_token.ty, ident, value, span));
            } else if self.matches(T::LeftBracket)? {
                let mut decls = Vec::new();
                if self.matches(T::Underscore)? {
                    decls.push(None);
                } else {
                    decls.push(Some(self.var_decl(false, true)?));
                }
                if self.matches(T::Comma)? {
                    if self.matches(T::Underscore)? {
                        decls.push(None);
                    } else {
                        decls.push(Some(self.var_decl(false, true)?));
                    }
                }
                self.expect_msg(T::RightBracket, "Expected ']'.")?;
                self.expect_msg(T::In, "Expected 'in'.")?;
                let rhs = self.expression()?;
                self.expect_msg(T::RightParen, "Expected ')'.")?;
                let block = self.block_statement()?;
                return Ok(self.ast.add_stmt(
                    StmtKind::ForEach {
                        decls,
                        rhs,
                        block,
                        decompose: true,
                    },
                    Span::default(),
                ));
            } else {
                // xNVSE asks whether the statement is an assignment expression, which a
                // statement never is: any other initializer is this error.
                let _ = self.statement()?;
                self.error_at_current("Expected variable declaration or assignment.")?;
            }
        }
        let cond_span = lparen.span.plus(self.previous.span);
        let mut cond = self.ast.add_expr(ExprKind::Bool(true), cond_span);
        if !self.peek(T::Semicolon) {
            cond = self.expression()?;
            self.expect(T::Semicolon)?;
        }
        let mut post = None;
        if !self.peek(T::RightParen) {
            post = Some(self.expression()?);
        }
        self.expect_msg(T::RightParen, "Expected ')'.")?;
        let block = self.block_statement()?;
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(
            StmtKind::For {
                init,
                cond: Some(cond),
                post,
                block,
            },
            span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp:
    // Parser::AdvanceToClosingCharOf.
    fn advance_to_closing_char_of(&mut self, left: T) -> R<()> {
        let right = match left {
            T::LeftParen => T::RightParen,
            T::LeftBrace => T::RightBrace,
            T::LeftBracket => T::RightBracket,
            _ => return Ok(()),
        };
        let mut depth = 0;
        loop {
            self.advance()?;
            let cur = self.current.ty;
            if cur == T::Eof {
                break;
            }
            if cur == left {
                depth += 1;
            } else if cur == right {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
        }
        Ok(())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp:
    // Parser::ParenthesizedExpression.
    /// `( expression )`; an expression that fails is skipped to its closing parenthesis (none
    /// then).
    fn parenthesized_expression(&mut self) -> R<Option<ExprId>> {
        self.expect(T::LeftParen)?;
        let mut cond = None;
        match self.expression() {
            Ok(e) => cond = Some(e),
            Err(Fail::Caught) => {
                self.had_error = true;
                self.advance_to_closing_char_of(T::LeftParen)?;
            }
            Err(crash) => return Err(crash),
        }
        self.expect(T::RightParen)?;
        Ok(cond)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::IfStatement.
    fn if_statement(&mut self) -> R<StmtId> {
        let start = self.expect(T::If)?;
        let cond = self.parenthesized_expression()?;
        let block = self.block_statement()?;
        let mut else_block = None;
        if self.matches(T::Else)? {
            if self.peek(T::If) {
                let inner = self.if_statement()?;
                let span = self.ast.stmt(inner).span;
                else_block = Some(self.ast.add_stmt(StmtKind::Block(vec![inner]), span));
            } else {
                else_block = Some(self.block_statement()?);
            }
        }
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(
            StmtKind::If {
                cond,
                block,
                else_block,
            },
            span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::MatchStatement.
    fn match_statement(&mut self) -> R<StmtId> {
        let start = self.expect(T::Match)?;
        let cond = self.parenthesized_expression()?;
        self.expect(T::LeftBrace)?;
        let mut arms = Vec::new();
        let mut default: Option<StmtId> = None;
        loop {
            if self.matches(T::Underscore)? {
                if default.is_some() {
                    self.error_at_current("Only one default case can be specified.")?;
                }
                self.expect(T::Arrow)?;
                default = Some(self.block_statement()?);
            } else {
                let mut binding = None;
                let expr;
                if self.matches(T::Identifier)? {
                    let ident = self.ast.add_expr(
                        ExprKind::Ident {
                            name: self.previous.lexeme.clone(),
                            form: 0,
                            var: None,
                        },
                        self.previous.span,
                    );
                    if self.matches(T::Slice)? {
                        binding = Some(ident);
                        expr = self.expression()?;
                    } else {
                        expr = ident;
                    }
                } else {
                    expr = self.expression()?;
                }
                self.expect(T::Arrow)?;
                let block = self.block_statement()?;
                arms.push(Arm {
                    binding,
                    expr,
                    block,
                });
            }
            if self.peek(T::RightBrace) {
                break;
            }
        }
        self.expect(T::RightBrace)?;
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(
            StmtKind::Match {
                expr: cond,
                arms,
                default,
            },
            span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::ReturnStatement.
    fn return_statement(&mut self) -> R<StmtId> {
        let token = self.expect(T::Return)?;
        if self.matches(T::Semicolon)? {
            return Ok(self.ast.add_stmt(StmtKind::Return(None), token.span));
        }
        let expr = self.expression()?;
        let span = token.span.plus(self.ast.expr(expr).span);
        let s = self.ast.add_stmt(StmtKind::Return(Some(expr)), span);
        self.expect(T::Semicolon)?;
        Ok(s)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::WhileStatement.
    fn while_statement(&mut self) -> R<StmtId> {
        let start = self.expect(T::While)?;
        let cond = self.parenthesized_expression()?;
        let block = self.block_statement()?;
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(StmtKind::While { cond, block }, span))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::BlockStatement.
    /// `{ statements }`; a statement that fails is skipped (`Synchronize`), and the block ends at
    /// the end of the text.
    fn block_statement(&mut self) -> R<StmtId> {
        let start = self.expect(T::LeftBrace)?;
        let mut stmts = Vec::new();
        while !self.matches(T::RightBrace)? {
            if self.matches(T::Semicolon)? {
                continue;
            }
            match self.statement() {
                Ok(s) => stmts.push(s),
                Err(Fail::Caught) => {
                    self.had_error = true;
                    self.synchronize()?;
                    if self.current.ty == T::Eof {
                        break;
                    }
                }
                Err(crash) => return Err(crash),
            }
        }
        let span = start.span.plus(self.previous.span);
        Ok(self.ast.add_stmt(StmtKind::Block(stmts), span))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp:
    // Parser::ExpressionStatement.
    fn expression_statement(&mut self) -> R<StmtId> {
        let start = self.current.span;
        if self.matches(T::Semicolon)? {
            return Ok(self.ast.add_stmt(StmtKind::Expr(None), self.previous.span));
        }
        let expr = self.expression()?;
        self.expect(T::Semicolon)?;
        let span = start.plus(self.previous.span);
        Ok(self.ast.add_stmt(StmtKind::Expr(Some(expr)), span))
    }

    fn expression(&mut self) -> R<ExprId> {
        self.assignment()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Assignment.
    fn assignment(&mut self) -> R<ExprId> {
        let left = self.slice()?;
        for t in [
            T::Eq,
            T::PlusEq,
            T::MinusEq,
            T::StarEq,
            T::SlashEq,
            T::ModEq,
            T::PowEq,
            T::BitwiseOrEquals,
            T::BitwiseAndEquals,
        ] {
            if self.matches(t)? {
                let token = self.previous.clone();
                let value = self.assignment()?;
                if matches!(
                    self.ast.expr(left).kind,
                    ExprKind::Ident { .. } | ExprKind::Get { .. } | ExprKind::Subscript { .. }
                ) {
                    return Ok(self
                        .ast
                        .add_expr(ExprKind::Assign { token, left, value }, Span::default()));
                }
                self.error_at(&token, "Invalid assignment target.")?;
            }
        }
        Ok(left)
    }

    /// A binary expression of `left` and `right` (`BinaryExpr`, its place the two places'
    /// `operator+`).
    fn binary(&mut self, op: Token, left: ExprId, right: ExprId) -> ExprId {
        let span = self.ast.expr(left).span.plus(self.ast.expr(right).span);
        self.ast
            .add_expr(ExprKind::Binary { op, left, right }, span)
    }

    /// One left-associative level: `next` operands joined by any of `ops`.
    fn left_assoc(&mut self, ops: &[T], next: fn(&mut Self) -> R<ExprId>) -> R<ExprId> {
        let mut left = next(self)?;
        'outer: loop {
            for &t in ops {
                if self.matches(t)? {
                    let op = self.previous.clone();
                    let right = next(self)?;
                    left = self.binary(op, left, right);
                    continue 'outer;
                }
            }
            return Ok(left);
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Slice,
    // LogicalOr, LogicalAnd, Equality, Comparison, BitwiseOr, BitwiseAnd, Shift, Term, Factor,
    // Pair.
    fn slice(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::Slice], Self::ternary)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Ternary.
    fn ternary(&mut self) -> R<ExprId> {
        let mut cond = self.logical_or()?;
        while self.matches(T::Ternary)? {
            let token = self.previous.clone();
            let left = if self.matches(T::Slice)? {
                cond
            } else {
                let l = self.logical_or()?;
                self.expect(T::Slice)?;
                l
            };
            let right = self.logical_or()?;
            cond = self.ast.add_expr(
                ExprKind::Ternary {
                    token,
                    cond,
                    left,
                    right,
                },
                Span::default(),
            );
        }
        Ok(cond)
    }

    fn logical_or(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::LogicOr], Self::logical_and)
    }
    fn logical_and(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::LogicAnd], Self::equality)
    }
    fn equality(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::EqEq, T::BangEq], Self::comparison)
    }
    fn comparison(&mut self) -> R<ExprId> {
        self.left_assoc(
            &[T::Less, T::LessEq, T::Greater, T::GreaterEq],
            Self::in_expr,
        )
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::In.
    fn in_expr(&mut self) -> R<ExprId> {
        let left = self.bitwise_or()?;
        let is_not = self.matches(T::Not)?;
        if self.matches(T::In)? {
            let op = self.previous.clone();
            if self.matches(T::LeftBracket)? {
                let mut values = Vec::new();
                while !self.matches(T::RightBracket)? {
                    if !values.is_empty() {
                        self.expect(T::Comma)?;
                    }
                    values.push(self.expression()?);
                }
                return Ok(self.ast.add_expr(
                    ExprKind::In {
                        lhs: left,
                        token: op,
                        values,
                        expression: None,
                        is_not,
                    },
                    Span::default(),
                ));
            }
            let expr = self.expression()?;
            return Ok(self.ast.add_expr(
                ExprKind::In {
                    lhs: left,
                    token: op,
                    values: Vec::new(),
                    expression: Some(expr),
                    is_not,
                },
                Span::default(),
            ));
        }
        if is_not {
            self.error_at_current("Expected 'in'.")?;
        }
        Ok(left)
    }

    fn bitwise_or(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::BitwiseOr], Self::bitwise_and)
    }
    fn bitwise_and(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::BitwiseAnd], Self::shift)
    }
    fn shift(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::LeftShift, T::RightShift], Self::term)
    }
    fn term(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::Plus, T::Minus], Self::factor)
    }
    fn factor(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::Star, T::Slash, T::Mod, T::Pow], Self::pair)
    }
    fn pair(&mut self) -> R<ExprId> {
        self.left_assoc(&[T::MakePair], Self::unary)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Unary.
    /// `!`, `-` (negation), `$`, `#`, `&` (box), `*` (unbox) and `~` before an operand.
    fn unary(&mut self) -> R<ExprId> {
        for t in [
            T::Bang,
            T::Minus,
            T::Dollar,
            T::Pound,
            T::BitwiseAnd,
            T::Star,
            T::BitwiseNot,
        ] {
            if self.matches(t)? {
                let mut op = self.previous.clone();
                let right = self.unary()?;
                op.ty = match op.ty {
                    T::BitwiseAnd => T::Box,
                    T::Star => T::Unbox,
                    T::Minus => T::Negate,
                    other => other,
                };
                return Ok(self.ast.add_expr(
                    ExprKind::Unary {
                        op,
                        expr: right,
                        postfix: false,
                    },
                    Span::default(),
                ));
            }
        }
        self.postfix()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Postfix.
    /// Subscripts, then `++` or `--`.
    fn postfix(&mut self) -> R<ExprId> {
        let mut expr = self.call()?;
        while self.matches(T::LeftBracket)? {
            let token = self.previous.clone();
            let inner = self.expression()?;
            let closing = self.expect(T::RightBracket)?;
            let span = token.span.plus(closing.span);
            expr = self.ast.add_expr(
                ExprKind::Subscript {
                    op: token,
                    left: expr,
                    index: inner,
                },
                span,
            );
        }
        if self.matches(T::PlusPlus)? || self.matches(T::MinusMinus)? {
            if !matches!(
                self.ast.expr(expr).kind,
                ExprKind::Ident { .. } | ExprKind::Get { .. }
            ) {
                self.error_at_previous("Invalid operand for '++', expected identifier.")?;
            }
            let op = self.previous.clone();
            return Ok(self.ast.add_expr(
                ExprKind::Unary {
                    op,
                    expr,
                    postfix: true,
                },
                Span::default(),
            ));
        }
        Ok(expr)
    }

    /// A call's arguments to its `)`; an argument that fails is skipped to the closing
    /// parenthesis. At the end of the text xNVSE's loop never ends (it asks for the next
    /// argument at the end again and again, printing an error each time): a crash here.
    fn call_args(&mut self) -> R<Vec<ExprId>> {
        let mut args = Vec::new();
        while !self.matches(T::RightParen)? {
            let step = (|| -> R<()> {
                args.push(self.expression()?);
                if !self.peek(T::RightParen) && !self.matches(T::Comma)? {
                    self.error_at_current("Expected ',' or ')'.")?;
                }
                Ok(())
            })();
            match step {
                Ok(()) => {}
                Err(Fail::Caught) => {
                    self.advance_to_closing_char_of(T::LeftParen)?;
                    if self.current.ty == T::Eof {
                        return Err(Fail::Crash(
                            "call arguments: endless loop at the end of the text",
                        ));
                    }
                }
                Err(crash) => return Err(crash),
            }
        }
        Ok(args)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Call.
    /// Member access (`a.b`), method calls (`a.b(...)`) and calls (`f(...)`).
    fn call(&mut self) -> R<ExprId> {
        let start = self.current.clone();
        let mut expr = self.primary()?;
        loop {
            let dot = self.matches(T::Dot)?;
            if !dot && !self.matches(T::LeftParen)? {
                break;
            }
            if dot {
                if !matches!(
                    self.ast.expr(expr).kind,
                    ExprKind::Grouping(_)
                        | ExprKind::Ident { .. }
                        | ExprKind::Call { .. }
                        | ExprKind::Get { .. }
                ) {
                    self.error_at_current("Invalid member access.")?;
                }
                let ident = self.ident_expr()?;
                if self.matches(T::LeftParen)? {
                    let args = self.call_args()?;
                    let span = start.span.plus(self.previous.span);
                    expr = self.ast.add_expr(
                        ExprKind::Call {
                            left: Some(expr),
                            ident,
                            args,
                            cmd: None,
                        },
                        span,
                    );
                } else {
                    let span = self.ast.expr(expr).span;
                    expr = self.ast.add_expr(
                        ExprKind::Get {
                            left: expr,
                            ident,
                            var_info: None,
                            reference_name: None,
                        },
                        span,
                    );
                }
            } else {
                if !self.ast.is_ident(expr) {
                    self.error_at_current("Invalid callee.")?;
                }
                let args = self.call_args()?;
                let span = start.span.plus(self.previous.span);
                expr = self.ast.add_expr(
                    ExprKind::Call {
                        left: None,
                        ident: expr,
                        args,
                        cmd: None,
                    },
                    span,
                );
            }
        }
        Ok(expr)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Primary.
    fn primary(&mut self) -> R<ExprId> {
        if self.matches(T::Bool)? {
            let v = self.previous.number().unwrap_or(0.0) != 0.0;
            return Ok(self.ast.add_expr(ExprKind::Bool(v), self.previous.span));
        }
        if self.peek(T::Number) {
            return self.numeric_literal();
        }
        if self.matches(T::String)? {
            let text = match &self.previous.value {
                Value::Text(t) => t.clone(),
                _ => return Err(Fail::Crash("std::get<std::string>: bad_variant_access")),
            };
            let mut expr = self.ast.add_expr(ExprKind::Str(text), self.previous.span);
            while self.matches(T::Interp)? {
                let inside = self.expression()?;
                let inner = self.ast.add_expr(
                    ExprKind::Unary {
                        op: Token::bare(T::Dollar, b"$"),
                        expr: inside,
                        postfix: false,
                    },
                    Span::default(),
                );
                self.expect_msg(T::EndInterp, "Expected '}'")?;
                expr = self.ast.add_expr(
                    ExprKind::Binary {
                        op: Token::bare(T::Plus, b""),
                        left: expr,
                        right: inner,
                    },
                    Span::default(),
                );
                if self.matches(T::String)? && self.previous.lexeme.len() > 2 {
                    let text = match &self.previous.value {
                        Value::Text(t) => t.clone(),
                        _ => return Err(Fail::Crash("std::get<std::string>: bad_variant_access")),
                    };
                    let end = self.ast.add_expr(ExprKind::Str(text), self.previous.span);
                    let span = self.previous.span;
                    expr = self.ast.add_expr(
                        ExprKind::Binary {
                            op: Token::bare(T::Plus, b"+"),
                            left: expr,
                            right: end,
                        },
                        span,
                    );
                }
            }
            return Ok(expr);
        }
        if self.peek(T::Identifier) {
            return self.ident_expr();
        }
        if self.peek(T::LeftParen) {
            let start = self.current.clone();
            let inner = self.parenthesized_expression()?;
            let span = start.span.plus(self.previous.span);
            return Ok(self.ast.add_expr(ExprKind::Grouping(inner), span));
        }
        if self.peek(T::LeftBracket) {
            return self.list_literal(T::LeftBracket, T::RightBracket);
        }
        if self.peek(T::LeftBrace) {
            return self.list_literal(T::LeftBrace, T::RightBrace);
        }
        if self.peek(T::Fn) {
            return self.fn_expr();
        }
        self.error_at_current("Expected expression.")?;
        unreachable!("an error always fails")
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::IdentExpr.
    fn ident_expr(&mut self) -> R<ExprId> {
        let tok = self.expect(T::Identifier)?;
        Ok(self.ast.add_expr(
            ExprKind::Ident {
                name: tok.lexeme,
                form: 0,
                var: None,
            },
            tok.span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::NumericLiteral.
    /// A number: a double when its text has a point, else its value rounded down.
    fn numeric_literal(&mut self) -> R<ExprId> {
        self.matches(T::Number)?;
        let value = self
            .previous
            .number()
            .ok_or(Fail::Crash("std::get<double>: bad_variant_access"))?;
        let is_fp = self.previous.lexeme.contains(&b'.');
        Ok(self.ast.add_expr(
            ExprKind::Number {
                value: if is_fp { value } else { value.floor() },
                is_fp,
                enum_len: 0,
            },
            self.previous.span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::FnExpr.
    /// A lambda: `fn (args) { ... }`, or `fn (args) -> expression` (a `SetFunctionValue` call).
    fn fn_expr(&mut self) -> R<ExprId> {
        let token = self.expect(T::Fn)?;
        let args = self.parse_args()?;
        if self.matches(T::Arrow)? {
            let expr = self.expression()?;
            let expr_span = self.ast.expr(expr).span;
            let ident = self.ast.add_expr(
                ExprKind::Ident {
                    name: b"SetFunctionValue".to_vec(),
                    form: 0,
                    var: None,
                },
                expr_span,
            );
            let call_span = self.previous.span.plus(expr_span);
            let call = self.ast.add_expr(
                ExprKind::Call {
                    left: None,
                    ident,
                    args: vec![expr],
                    cmd: None,
                },
                call_span,
            );
            let stmt = self.ast.add_stmt(StmtKind::Expr(Some(call)), call_span);
            let block = self.ast.add_stmt(StmtKind::Block(vec![stmt]), call_span);
            let span = token.span.plus(self.previous.span);
            return Ok(self.ast.add_expr(
                ExprKind::Lambda {
                    args,
                    body: block,
                    param_types: Vec::new(),
                    return_type: crate::compiler::tt::INVALID,
                },
                span,
            ));
        }
        let block = self.block_statement()?;
        let span = token.span.plus(self.previous.span);
        Ok(self.ast.add_expr(
            ExprKind::Lambda {
                args,
                body: block,
                param_types: Vec::new(),
                return_type: crate::compiler::tt::INVALID,
            },
            span,
        ))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::ArrayLiteral,
    // Parser::MapLiteral.
    /// `[a, b, ...]` or `{a, b, ...}`; a value that fails skips to the closing bracket and ends
    /// the literal.
    fn list_literal(&mut self, open: T, close: T) -> R<ExprId> {
        let start = self.expect(open)?;
        let mut values = Vec::new();
        while !self.matches(close)? {
            let step = (|| -> R<ExprId> {
                if !values.is_empty() {
                    self.expect_msg(T::Comma, "Expected ','.")?;
                }
                self.expression()
            })();
            match step {
                Ok(v) => values.push(v),
                Err(Fail::Caught) => {
                    self.had_error = true;
                    self.advance_to_closing_char_of(open)?;
                    break;
                }
                Err(crash) => return Err(crash),
            }
        }
        let span = start.span.plus(self.previous.span);
        let kind = if open == T::LeftBracket {
            ExprKind::ArrayLit(values)
        } else {
            ExprKind::MapLit(values)
        };
        Ok(self.ast.add_expr(kind, span))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::ParseArgs.
    /// A function's or lambda's parameters: `(type name, ...)`.
    fn parse_args(&mut self) -> R<Vec<StmtId>> {
        self.expect(T::LeftParen)?;
        let mut args = Vec::new();
        while !self.matches(T::RightParen)? {
            if !self.matches_type()? {
                self.error_at_current("Expected type.")?;
            }
            let ty = self.previous.clone();
            let ident = self.ident_expr()?;
            let span = ty.span.plus(self.previous.span);
            let decl = self.single_decl(ty.ty, ident, None, span);
            if !self.peek(T::RightParen) && !self.matches(T::Comma)? {
                self.error_at_current("Expected ',' or ')'.")?;
            }
            args.push(decl);
        }
        Ok(args)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Advance.
    /// The next token; a lexer error is printed and the token stays the moved-from one.
    fn advance(&mut self) -> R<()> {
        self.previous = std::mem::take(&mut self.current);
        match self.lexer.next_token(true) {
            Ok(tok) => self.current = tok,
            Err(LexFail::Error(e)) => {
                self.current = self.previous.moved_from();
                let msg = format!("[line {}] {}", self.lexer.line, e.0);
                self.env.error(msg.as_bytes());
            }
            Err(LexFail::Crash(f)) => return Err(f),
        }
        Ok(())
    }

    fn matches(&mut self, ty: T) -> R<bool> {
        if self.current.ty == ty {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn matches_type(&mut self) -> R<bool> {
        for t in [
            T::IntType,
            T::DoubleType,
            T::RefType,
            T::ArrayType,
            T::StringType,
        ] {
            if self.matches(t)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn peek(&self, ty: T) -> bool {
        self.current.ty == ty
    }

    fn peek_type(&self) -> bool {
        [
            T::IntType,
            T::DoubleType,
            T::RefType,
            T::ArrayType,
            T::StringType,
        ]
        .contains(&self.current.ty)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::PeekBlockType,
    // Parser::ExpectBlockType.
    fn peek_block_type(&mut self) -> bool {
        if !self.peek(T::Identifier) {
            return false;
        }
        let name = self.current.lexeme.clone();
        self.env.block(&name).is_some()
    }

    fn expect_block_type(&mut self, msg: &str) -> R<Token> {
        if !self.peek_block_type() {
            self.error_at_current(msg)?;
        }
        self.advance()?;
        Ok(self.previous.clone())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Error.
    /// `Error(token, message)`: "[line L:C] message" printed; the parse step fails.
    fn error_at(&mut self, token: &Token, msg: &str) -> R<()> {
        self.panic_mode = true;
        self.had_error = true;
        let text = format!(
            "[line {}:{}] {}",
            token.span.start.line, token.span.start.column, msg
        );
        self.env.error(text.as_bytes());
        Err(Fail::Caught)
    }

    fn error_at_current(&mut self, msg: &str) -> R<()> {
        let tok = self.current.clone();
        self.error_at(&tok, msg)
    }

    fn error_at_previous(&mut self, msg: &str) -> R<()> {
        let tok = self.previous.clone();
        self.error_at(&tok, msg)
    }

    fn expect_msg(&mut self, ty: T, msg: &str) -> R<Token> {
        if !self.matches(ty)? {
            self.error_at_current(msg)?;
        }
        Ok(self.previous.clone())
    }

    /// `Expect(type)`: "Expected '<type's text>'".
    fn expect(&mut self, ty: T) -> R<Token> {
        if !self.matches(ty)? {
            let msg = format!("Expected '{}'", ty.text());
            self.error_at_current(&msg)?;
        }
        Ok(self.previous.clone())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Parser.cpp: Parser::Synchronize.
    /// Past the failed statement: after a `;`, or before `if`, `while`, `for`, `return` or `}`.
    fn synchronize(&mut self) -> R<()> {
        self.advance()?;
        while self.current.ty != T::Eof {
            if self.previous.ty == T::Semicolon {
                self.panic_mode = false;
                return Ok(());
            }
            if matches!(
                self.current.ty,
                T::If | T::While | T::For | T::Return | T::RightBrace
            ) {
                self.panic_mode = false;
                return Ok(());
            }
            self.advance()?;
        }
        Ok(())
    }
}

/// `kParamType_Integer`.
pub const PARAM_INTEGER: u32 = 1;

/// Parses `text` (`Parser{text}.Parse()`).
pub fn parse(env: &mut dyn Env, text: &[u8]) -> Result<Parsed, Fail> {
    Parser::new(env, text)?.parse()
}

#[cfg(test)]
#[path = "parser_tests.rs"]
pub(crate) mod tests;
