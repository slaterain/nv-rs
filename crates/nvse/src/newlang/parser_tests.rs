//! The parser's grammar, places and errors, from xNVSE 6.4.9's `Parser.cpp`: each script's tree
//! written as a short text ([`dump`]) and compared, each error with its exact message.

use super::*;
use crate::newlang::lexer::TokenType;
use crate::newlang::test_env::TestEnv;

/// A tree as text: statements one per line inside braces, expressions in prefix form with the
/// token types' own texts.
pub(crate) fn dump(ast: &Ast) -> String {
    let mut out = format!("name {}\n", String::from_utf8_lossy(&ast.name));
    for &g in &ast.globals {
        out += &stmt(ast, g);
        out += "\n";
    }
    for &b in &ast.blocks {
        out += &stmt(ast, b);
        out += "\n";
    }
    out
}

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

pub(crate) fn stmt(ast: &Ast, id: StmtId) -> String {
    match &ast.stmt(id).kind {
        StmtKind::Begin {
            name, param, block, ..
        } => match param {
            Some(p) => format!("{}::{} {}", s(name), s(&p.lexeme), stmt(ast, *block)),
            None => format!("{} {}", s(name), stmt(ast, *block)),
        },
        StmtKind::Udf { args, body, .. } => {
            format!("fn({}) {}", decl_list(ast, args), stmt(ast, *body))
        }
        StmtKind::VarDecl { ty, decls } => {
            let parts: Vec<String> = decls
                .iter()
                .map(|d| match d.value {
                    Some(v) => format!("{}={}", s(ast.ident_name(d.name)), expr(ast, v)),
                    None => s(ast.ident_name(d.name)),
                })
                .collect();
            format!("{} {}", ty.text(), parts.join(","))
        }
        StmtKind::Expr(Some(e)) => format!("{};", expr(ast, *e)),
        StmtKind::Expr(None) => ";".into(),
        StmtKind::For {
            init,
            cond,
            post,
            block,
        } => format!(
            "for({}; {}; {}) {}",
            init.map(|i| stmt(ast, i)).unwrap_or_default(),
            cond.map(|c| expr(ast, c)).unwrap_or_default(),
            post.map(|p| expr(ast, p)).unwrap_or_default(),
            stmt(ast, *block)
        ),
        StmtKind::ForEach {
            decls,
            rhs,
            block,
            decompose,
        } => {
            let d: Vec<String> = decls
                .iter()
                .map(|d| d.map_or("_".into(), |d| stmt(ast, d)))
                .collect();
            let d = if *decompose {
                format!("[{}]", d.join(","))
            } else {
                d.join(",")
            };
            format!("for({} in {}) {}", d, expr(ast, *rhs), stmt(ast, *block))
        }
        StmtKind::If {
            cond,
            block,
            else_block,
        } => {
            let c = cond.map(|c| expr(ast, c)).unwrap_or("NULL".into());
            match else_block {
                Some(e) => format!("if({}) {} else {}", c, stmt(ast, *block), stmt(ast, *e)),
                None => format!("if({}) {}", c, stmt(ast, *block)),
            }
        }
        StmtKind::Return(Some(e)) => format!("return {};", expr(ast, *e)),
        StmtKind::Return(None) => "return;".into(),
        StmtKind::Continue => "continue;".into(),
        StmtKind::Break => "break;".into(),
        StmtKind::While { cond, block } => format!(
            "while({}) {}",
            cond.map(|c| expr(ast, c)).unwrap_or("NULL".into()),
            stmt(ast, *block)
        ),
        StmtKind::Block(stmts) => {
            let inner: Vec<String> = stmts.iter().map(|&x| stmt(ast, x)).collect();
            format!("{{{}}}", inner.join(" "))
        }
        StmtKind::ShowMessage {
            message,
            args,
            time,
        } => {
            let a: Vec<String> = args.iter().map(|&x| expr(ast, x)).collect();
            format!(
                "ShowMessage({}, [{}], {})",
                expr(ast, *message),
                a.join(","),
                time
            )
        }
        StmtKind::Match {
            expr: e,
            arms,
            default,
        } => {
            let mut parts: Vec<String> = arms
                .iter()
                .map(|a| match a.binding {
                    Some(b) => format!(
                        "{}: {} -> {}",
                        expr(ast, b),
                        expr(ast, a.expr),
                        stmt(ast, a.block)
                    ),
                    None => format!("{} -> {}", expr(ast, a.expr), stmt(ast, a.block)),
                })
                .collect();
            if let Some(d) = default {
                parts.push(format!("_ -> {}", stmt(ast, *d)));
            }
            format!(
                "match({}) {{{}}}",
                e.map(|e| expr(ast, e)).unwrap_or("NULL".into()),
                parts.join(" ")
            )
        }
    }
}

fn decl_list(ast: &Ast, decls: &[StmtId]) -> String {
    decls
        .iter()
        .map(|&d| stmt(ast, d))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn expr(ast: &Ast, id: ExprId) -> String {
    match &ast.expr(id).kind {
        ExprKind::Assign { token, left, value } => {
            format!(
                "({} {} {})",
                token.ty.text(),
                expr(ast, *left),
                expr(ast, *value)
            )
        }
        ExprKind::Ternary {
            cond, left, right, ..
        } => {
            format!(
                "(? {} {} {})",
                expr(ast, *cond),
                expr(ast, *left),
                expr(ast, *right)
            )
        }
        ExprKind::In {
            lhs,
            values,
            expression,
            is_not,
            ..
        } => {
            let word = if *is_not { "not in" } else { "in" };
            match expression {
                Some(e) => format!("({} {} {})", word, expr(ast, *lhs), expr(ast, *e)),
                None => {
                    let v: Vec<String> = values.iter().map(|&x| expr(ast, x)).collect();
                    format!("({} {} [{}])", word, expr(ast, *lhs), v.join(","))
                }
            }
        }
        ExprKind::Binary { op, left, right } => {
            format!(
                "({} {} {})",
                op.ty.text(),
                expr(ast, *left),
                expr(ast, *right)
            )
        }
        ExprKind::Unary {
            op,
            expr: e,
            postfix,
        } => {
            let name = match op.ty {
                TokenType::Negate => "neg".to_string(),
                t => t.text().to_string(),
            };
            if *postfix {
                format!("({} {})", expr(ast, *e), name)
            } else {
                format!("({} {})", name, expr(ast, *e))
            }
        }
        ExprKind::Subscript { left, index, .. } => {
            format!("{}[{}]", expr(ast, *left), expr(ast, *index))
        }
        ExprKind::Call {
            left, ident, args, ..
        } => {
            let a: Vec<String> = args.iter().map(|&x| expr(ast, x)).collect();
            match left {
                Some(l) => format!(
                    "{}.{}({})",
                    expr(ast, *l),
                    s(ast.ident_name(*ident)),
                    a.join(",")
                ),
                None => format!("{}({})", s(ast.ident_name(*ident)), a.join(",")),
            }
        }
        ExprKind::Get { left, ident, .. } => {
            format!("{}.{}", expr(ast, *left), s(ast.ident_name(*ident)))
        }
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Number {
            value,
            is_fp,
            enum_len,
        } => {
            if *enum_len > 0 {
                format!("enum{}:{}", enum_len, value)
            } else if *is_fp {
                format!("{value:?}")
            } else {
                format!("{value}")
            }
        }
        ExprKind::Str(t) => format!("\"{}\"", s(t)),
        ExprKind::Ident { name, .. } => s(name),
        ExprKind::ArrayLit(v) => format!(
            "[{}]",
            v.iter()
                .map(|&x| expr(ast, x))
                .collect::<Vec<_>>()
                .join(",")
        ),
        ExprKind::MapLit(v) => format!(
            "{{{}}}",
            v.iter()
                .map(|&x| expr(ast, x))
                .collect::<Vec<_>>()
                .join(",")
        ),
        ExprKind::Grouping(e) => format!(
            "(group {})",
            e.map(|e| expr(ast, e)).unwrap_or("NULL".into())
        ),
        ExprKind::Lambda { args, body, .. } => {
            format!("fn({}) {}", decl_list(ast, args), stmt(ast, *body))
        }
    }
}

/// The parse of `text` on the stand-in game: the result and the errors printed.
pub(crate) fn run(text: &str) -> (Parsed, Vec<String>) {
    let mut env = TestEnv::default();
    let parsed = parse(&mut env, text.as_bytes()).expect("no crash");
    (parsed, env.take_errors())
}

/// The tree of a script that parses without an error.
pub(crate) fn tree(text: &str) -> String {
    let (p, errors) = run(text);
    assert_eq!(errors, Vec::<String>::new(), "{text}");
    assert!(!p.had_error, "{text}");
    dump(p.ast.as_ref().unwrap())
}

/// The tree of one statement in a `GameMode` block.
fn one(statement: &str) -> String {
    let t = tree(&format!("name t;\nGameMode {{\n{statement}\n}}"));
    let line = t.lines().nth(1).unwrap();
    let inner = line.strip_prefix("GameMode {").unwrap();
    inner.strip_suffix('}').unwrap().to_string()
}

/// The errors of a script.
fn errors(text: &str) -> Vec<String> {
    run(text).1
}

/// "[line 1:C] message", C the column of the `nth` (from 0) `needle` in the one-line `text`.
fn at(text: &str, needle: &str, nth: usize, msg: &str) -> String {
    let col = text.match_indices(needle).nth(nth).expect("needle").0 + 1;
    format!("[line 1:{col}] {msg}")
}

/// The crash a script ends in.
fn crash(text: &str) -> &'static str {
    let mut env = TestEnv::default();
    match parse(&mut env, text.as_bytes()) {
        Err(Fail::Crash(c)) => c,
        other => panic!("{text}: no crash: {:?}", other.map(|p| p.had_error)),
    }
}

#[test]
fn the_header() {
    assert_eq!(tree("name Foo;"), "name Foo\n");
    assert_eq!(tree("NAME foo ;"), "name foo\n");
    assert_eq!(
        errors("scn Foo;"),
        ["[line 1:1] Expected 'name' as first statement of script."]
    );
    assert_eq!(errors("name;"), ["[line 1:5] Expected 'Identifier'"]);
    // At the end of the text the token has no place.
    assert_eq!(errors("name Foo"), ["[line 0:0] Expected ';'."]);
    let (p, _) = run("name;");
    assert!(p.ast.is_none());
}

#[test]
fn globals_and_blocks() {
    assert_eq!(
        tree("name t;\nint a, b; ref r; float f;\nGameMode {}\nMenuMode::1065 {}\nOnActivate::PlayerRef {}"),
        "name t\nint a,b\nref r\ndouble f\nGameMode {}\nMenuMode::1065 {}\nOnActivate::PlayerRef {}\n"
    );
    // A global with an initializer parses (the type checker refuses it).
    assert_eq!(tree("name t; int a = 5;"), "name t\nint a=5\n");
    assert_eq!(
        tree("name t; fn (int a, ref b) { return a; }"),
        "name t\nfn(int a, ref b) {return a;}\n"
    );
}

#[test]
fn block_errors() {
    let t = "name t; GameMode::1 {}";
    // After the error the parse skips to the `}`, which no top-level statement starts with.
    assert_eq!(
        errors(t),
        [
            at(t, "1", 0, "Cannot specify argument for this block type."),
            at(t, "}", 0, TOP)
        ]
    );
    let t = "name t; MenuMode::Foo {}";
    assert_eq!(errors(t)[0], at(t, "{", 0, "Expected number."));
    let t = "name t; OnActivate::5 {}";
    assert_eq!(errors(t)[0], at(t, "{", 0, "Expected identifier."));
    let t = "name t; MenuMode::1.5 {}";
    assert_eq!(errors(t)[0], at(t, "{", 0, "Expected integer."));
    let t = "name t; MenuMode::{}";
    assert_eq!(
        errors(t)[0],
        at(t, "{", 0, "Expected identifier or number.")
    );
    let t = "name t; OnAdd {}";
    assert_eq!(
        errors(t)[0],
        at(
            t,
            "{",
            0,
            "Missing required parameter for block type 'OnAdd'"
        )
    );
    let t = "name t; GameMode ;";
    assert_eq!(errors(t), [at(t, ";", 1, "Expected '{'.")]);
    // After an error the parse goes on with the next statement.
    let (p, e) = run("name t;\nfoo;\nGameMode {}");
    assert_eq!(e, [format!("[line 2:1] {TOP}")]);
    assert!(p.had_error);
    assert_eq!(dump(p.ast.as_ref().unwrap()), "name t\nGameMode {}\n");
}

const TOP: &str = "Expected variable declaration, block type (GameMode, MenuMode, ...), or 'fn'.";

#[test]
fn version_statements() {
    let (p, e) = run("name t;\n#version(\"NVSE\", 6, 4, 9)\n#Version(\"jip ln nvse\", 57)\n");
    assert_eq!(e, Vec::<String>::new());
    let v: Vec<(String, u32)> = p.plugin_versions.iter().map(|(k, v)| (s(k), *v)).collect();
    assert!(v.contains(&("nvse".into(), 0x0604_0090)));
    assert!(v.contains(&("jip ln nvse".into(), 57)));
    // Minor and beta default to 255.
    let (p, _) = run("name t; #version(\"nvse\", 6)");
    assert_eq!(p.plugin_versions.get(b"nvse"), Some(&0x06ff_0ff0));
    assert_eq!(
        errors("name t; #version(\"foo\", 1)"),
        ["[line 1:26] No plugin with name foo could be found.\n"]
    );
    assert_eq!(
        errors("name t; #vers(\"nvse\", 1)"),
        ["[line 1:14] Expected 'version'."]
    );
    assert_eq!(
        errors("name t; #version(nvse, 1)"),
        ["[line 1:18] Expected plugin name."]
    );
}

#[test]
fn statements() {
    assert_eq!(one("int a = 1, b;"), "int a=1,b");
    assert_eq!(
        one("if (a) { b; } else if (c) { d; } else { e; }"),
        "if(a) {b;} else {if(c) {d;} else {e;}}"
    );
    assert_eq!(
        one("while (a) { break; continue; }"),
        "while(a) {break; continue;}"
    );
    assert_eq!(
        one("for (int i = 0; i < 5; i++) { }"),
        "for(int i=0; (< i 5); (i ++)) {}"
    );
    assert_eq!(one("for (array x in arr) {}"), "for(array x in arr) {}");
    assert_eq!(
        one("for ([int k, _] in arr) {}"),
        "for([int k,_] in arr) {}"
    );
    assert_eq!(
        one("for ([_, string v] in arr) {}"),
        "for([_,string v] in arr) {}"
    );
    assert_eq!(one("for ([int v] in arr) {}"), "for([int v] in arr) {}");
    assert_eq!(one("return;"), "return;");
    assert_eq!(one("return a + 1;"), "return (+ a 1);");
    assert_eq!(one("{ a; ; b; }"), "{a; b;}");
    assert_eq!(one(";"), "");
    assert_eq!(
        one("fn f(int x) { return x; };"),
        "ref f=fn(int x) {return x;}"
    );
    assert_eq!(
        one("ShowMessage(MyMessage);"),
        "ShowMessage(MyMessage, [], 0)"
    );
    assert_eq!(
        one("ShowMessage(MyMessage, 5);"),
        "ShowMessage(MyMessage, [], 5)"
    );
    assert_eq!(
        one("match (x) { 1 -> { a; } y: y > 2 -> { b; } z -> {} _ -> { c; } }"),
        "match(x) {1 -> {a;} y: (> y 2) -> {b;} z -> {} _ -> {c;}}"
    );
}

#[test]
fn statement_errors() {
    let t = "name t; GameMode { for (i = 0; i < 5; i++) {} }";
    assert_eq!(
        errors(t)[0],
        at(t, "i", 1, "Expected variable declaration or assignment.")
    );
    let t = "name t; GameMode { break }";
    assert_eq!(errors(t)[0], at(t, "}", 0, "Expected ';'"));
    let t = "name t; GameMode { ShowMessage(M, 1.5); }";
    assert_eq!(
        errors(t)[0],
        at(t, "1.5", 0, "Duration must be an integer.")
    );
    let t = "name t; GameMode { ShowMessage(5); }";
    assert_eq!(errors(t)[0], at(t, "5", 0, "Expected message form"));
    let t = "name t; GameMode { match (x) { _ -> {} _ -> {} } }";
    assert_eq!(
        errors(t)[0],
        at(t, "->", 1, "Only one default case can be specified.")
    );
    let t = "name t; GameMode { match (x) { 1 {} } }";
    assert_eq!(errors(t)[0], at(t, "{", 2, "Expected '->'"));
    let t = "name t; GameMode { a = ; }";
    assert_eq!(errors(t), [at(t, ";", 1, "Expected expression.")]);
    // `for (;` leaves the first `;` unread: the increment's expression fails on it.
    let t = "name t; GameMode { for (; i < 5; i++) {} }";
    assert_eq!(errors(t)[0], at(t, ";", 1, "Expected expression."));
    // An unclosed block ends at the end of the text.
    let (p, e) = run("name t; GameMode { a;");
    assert_eq!(e, ["[line 0:0] Expected expression."]);
    assert_eq!(dump(p.ast.as_ref().unwrap()), "name t\nGameMode {a;}\n");
}

/// A condition that fails is skipped to its `)`: the statement is kept with no condition.
#[test]
fn a_failed_condition_is_skipped() {
    let t = "name t; GameMode { if (a + ; b) { c; } d; }";
    let (p, e) = run(t);
    assert_eq!(e, [at(t, ";", 1, "Expected expression.")]);
    assert!(p.had_error);
    assert_eq!(
        dump(p.ast.as_ref().unwrap()),
        "name t\nGameMode {if(NULL) {c;} d;}\n"
    );
    // The skip starts past the token the error was at: an error at the `)` itself skips past
    // it, and the `)` is missed.
    let t = "name t; GameMode { if (a +) { b; } c; }";
    let e = errors(t);
    assert_eq!(
        e[..2],
        [
            at(t, ")", 0, "Expected expression."),
            "[line 0:0] Expected ')'".to_string()
        ]
    );
}

/// `ShowMessage(message, identifier)`: xNVSE reads the identifier's text from a value it never
/// has.
#[test]
fn show_message_with_an_identifier_crashes() {
    assert_eq!(
        crash("name t; GameMode { ShowMessage(M, x); }"),
        "std::get<std::string>: bad_variant_access"
    );
}

/// A call's arguments at the end of the text: xNVSE's loop never ends.
#[test]
fn an_unclosed_call_at_the_end_crashes() {
    assert_eq!(
        crash("name t; GameMode { f(1, "),
        "call arguments: endless loop at the end of the text"
    );
}

#[test]
fn precedence() {
    assert_eq!(one("a = b ? c : d;"), "(= a (? b c d));");
    assert_eq!(one("a = b ?: d;"), "(= a (? b b d));");
    assert_eq!(one("a += b = c;"), "(+= a (= b c));");
    assert_eq!(one("x = a : b;"), "(= x (: a b));");
    assert_eq!(one("x = a || b && c;"), "(= x (|| a (&& b c)));");
    assert_eq!(one("x = a && b == c;"), "(= x (&& a (== b c)));");
    assert_eq!(one("x = a == b < c;"), "(= x (== a (< b c)));");
    assert_eq!(one("x = a < b | c;"), "(= x (< a (| b c)));");
    assert_eq!(one("x = a | b & c;"), "(= x (| a (& b c)));");
    assert_eq!(one("x = a & b << c;"), "(= x (& a (<< b c)));");
    assert_eq!(one("x = a << b + c;"), "(= x (<< a (+ b c)));");
    assert_eq!(one("x = a + b * c;"), "(= x (+ a (* b c)));");
    assert_eq!(one("x = a * b :: c;"), "(= x (* a (:: b c)));");
    assert_eq!(one("x = a - b - c;"), "(= x (- (- a b) c));");
    assert_eq!(one("x = a ^ b % c / d;"), "(= x (/ (% (^ a b) c) d));");
    assert_eq!(one("x = a < b in c;"), "(= x (< a (in b c)));");
    assert_eq!(one("x = a in [1, 2];"), "(= x (in a [1,2]));");
    assert_eq!(one("x = a not in arr;"), "(= x (not in a arr));");
    assert_eq!(one("x = (a + b) * c;"), "(= x (* (group (+ a b)) c));");
}

#[test]
fn unary_and_postfix() {
    assert_eq!(one("x = -a;"), "(= x (neg a));");
    assert_eq!(one("x = !a;"), "(= x (! a));");
    assert_eq!(one("x = $a;"), "(= x ($ a));");
    assert_eq!(one("x = #a;"), "(= x (# a));");
    assert_eq!(one("x = &a;"), "(= x (Box a));");
    assert_eq!(one("x = *a;"), "(= x (Unbox a));");
    assert_eq!(one("x = ~-a;"), "(= x (~ (neg a)));");
    assert_eq!(one("a--;"), "(a --);");
    assert_eq!(one("q.v++;"), "(q.v ++);");
}

#[test]
fn expression_errors() {
    let t = "name t; GameMode { x = a not b; }";
    assert_eq!(errors(t)[0], at(t, "b", 0, "Expected 'in'."));
    let t = "name t; GameMode { 1 = 2; }";
    assert_eq!(errors(t)[0], at(t, "=", 0, "Invalid assignment target."));
    let t = "name t; GameMode { a[1]++; }";
    assert_eq!(
        errors(t)[0],
        at(t, "++", 0, "Invalid operand for '++', expected identifier.")
    );
    let t = "name t; GameMode { (a)(1); }";
    assert_eq!(errors(t)[0], at(t, "1", 0, "Invalid callee."));
    let t = "name t; GameMode { [1].x; }";
    assert_eq!(errors(t)[0], at(t, "x", 0, "Invalid member access."));
    let t = "name t; GameMode { x = [1 2]; }";
    assert_eq!(errors(t)[0], at(t, "2", 0, "Expected ','."));
    let t = "name t; GameMode { x = {1::2 3}; }";
    assert_eq!(errors(t)[0], at(t, "3", 0, "Expected ','."));
}

/// A call's argument that fails skips to the call's `)` and the call goes on.
#[test]
fn a_failed_argument_skips_to_the_parenthesis() {
    let t = "name t; GameMode { f(1 2, 3); g; }";
    let (p, e) = run(t);
    assert_eq!(e, [at(t, "2", 0, "Expected ',' or ')'.")]);
    assert!(p.had_error);
    assert_eq!(
        dump(p.ast.as_ref().unwrap()),
        "name t\nGameMode {f(1); g;}\n"
    );
}

#[test]
fn calls_and_members() {
    assert_eq!(one("f(1, 2);"), "f(1,2);");
    assert_eq!(one("f();"), "f();");
    assert_eq!(one("x = a.b;"), "(= x a.b);");
    assert_eq!(
        one("player.AddItem(Caps001, 10);"),
        "player.AddItem(Caps001,10);"
    );
    assert_eq!(one("GetSelf().x.y(1);"), "GetSelf().x.y(1);");
}

#[test]
fn literals() {
    assert_eq!(
        one("x = 0x10 + 1.5 + true + null;"),
        "(= x (+ (+ (+ 16 1.5) true) 0));"
    );
    // An integer literal is its value rounded down: `1e3` has no point.
    assert_eq!(one("x = 1e3;"), "(= x 1000);");
    assert_eq!(one("x = [1, 2];"), "(= x [1,2]);");
    assert_eq!(one("x = [];"), "(= x []);");
    assert_eq!(
        one("x = {\"a\"::1, \"b\"::2};"),
        "(= x {(:: \"a\" 1),(:: \"b\" 2)});"
    );
    assert_eq!(
        one("x = \"a${y}b${z}\";"),
        "(= x (+ (+ (+ \"a\" ($ y)) \"b\") ($ z)));"
    );
    // An empty part after an interpolation is left out.
    assert_eq!(one("x = \"${y}\";"), "(= x (+ \"\" ($ y)));");
    assert_eq!(
        one("x = fn (int a) -> a * 2;"),
        "(= x fn(int a) {SetFunctionValue((* a 2));});"
    );
    assert_eq!(one("x = fn () { return 1; };"), "(= x fn() {return 1;});");
}

/// A lexer error is printed and the parser goes on with the token it had, its text gone.
#[test]
fn a_lexer_error_keeps_the_previous_token() {
    assert_eq!(
        errors("name a;\nint x = 1 @;"),
        [
            "[line 2] Unexpected character",
            "[line 2:9] Expected ';' at end of statement."
        ]
    );
    // After a `;` the duplicated token is a `;` the top level cannot start with.
    assert_eq!(
        errors("name a;\nint x = 1;@"),
        [
            "[line 2] Unexpected character",
            "[line 2:10] Expected variable declaration, block type (GameMode, MenuMode, ...), or 'fn'."
        ]
    );
}

/// Places of statements and expressions (the error messages show them).
#[test]
fn places() {
    let (p, _) = run("name t;\nGameMode {\n  x = a + bb;\n}");
    let ast = p.ast.unwrap();
    let StmtKind::Begin { block, .. } = &ast.stmt(ast.blocks[0]).kind else {
        panic!()
    };
    let s0 = ast.block_stmts(*block)[0];
    let span = ast.stmt(s0).span;
    assert_eq!(
        (
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column
        ),
        (3, 3, 3, 13)
    );
    let StmtKind::Expr(Some(e)) = ast.stmt(s0).kind else {
        panic!()
    };
    // An assignment has no place of its own (xNVSE's constructor sets none).
    assert_eq!(ast.expr(e).span, Span::default());
    let ExprKind::Assign { value, .. } = ast.expr(e).kind else {
        panic!()
    };
    let v = ast.expr(value).span;
    assert_eq!((v.start.column, v.end.column), (7, 12));
    // The block: from the `{` to the `}`.
    let b = ast.stmt(*block).span;
    assert_eq!(
        (b.start.line, b.start.column, b.end.line, b.end.column),
        (2, 10, 4, 1)
    );
}
