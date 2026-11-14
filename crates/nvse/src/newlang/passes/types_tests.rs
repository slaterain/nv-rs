//! The passes up to the type checker (match statements, variables, calls, types), from xNVSE
//! 6.4.9's `MatchTransformer.cpp`, `VariableResolution.cpp`, `CallTransformer.cpp` and
//! `TypeChecker.cpp`: the trees they leave, the types they give and their exact messages, on the
//! stand-in game.

use super::*;
use crate::newlang::parser::tests::{dump, expr as dump_expr};
use crate::newlang::test_env::TestEnv;
use crate::newlang::{analyze, parser, passes};

const R: &str = "\x1B[31m";
const X: &str = "\x1B[0m";

/// The highlight of columns `from..=to` of one line (`HighlightSourceSpan`).
fn hl(msg: &str, n: usize, line: &str, from: usize, to: usize) -> String {
    let rest = if to < line.len() { &line[to..] } else { "" };
    format!(
        "\r    | {msg}\n{n:>3} | {}{R}{}{X}{rest}\n",
        &line[..from - 1],
        &line[from - 1..to.min(line.len())]
    )
}

/// The message of a span with no place.
fn bug(msg: &str) -> String {
    format!("\r    | {msg}\n    | {R}BUG: Failed to print source information{X}\n\n")
}

/// The analysis of a script (no compiling script): the checked tree and the messages.
fn check_with(env: &mut TestEnv, text: &str, has_script: bool) -> (Option<Ast>, Vec<String>) {
    let a = analyze(env, text.as_bytes(), has_script).expect("no crash");
    (a.ast, env.take_errors())
}

fn check(text: &str) -> (Option<Ast>, Vec<String>) {
    check_with(&mut TestEnv::default(), text, false)
}

/// The messages of a script in a `GameMode` block, the block's statement on line 3.
fn block_errors(statements: &str) -> Vec<String> {
    let (ast, e) = check(&format!("name t;\nGameMode {{\n{statements}\n}}"));
    assert!(ast.is_none() || e.is_empty());
    e
}

fn crash(text: &str) -> &'static str {
    match analyze(&mut TestEnv::default(), text.as_bytes(), false) {
        Err(Fail::Crash(c)) => c,
        Ok(a) => panic!("{text}: no crash ({})", a.ast.is_some()),
        Err(f) => panic!("{text}: {f:?}"),
    }
}

/// The tree after the parse and the match pass only.
fn matched(text: &str) -> String {
    let mut env = TestEnv::default();
    let p = parser::parse(&mut env, text.as_bytes()).unwrap();
    let mut ast = p.ast.unwrap();
    passes::match_transform::MatchTransformer::new(&mut ast)
        .run()
        .unwrap();
    dump(&ast)
}

#[test]
fn a_clean_script_checks() {
    let (ast, e) = check(
        "name t;\nint g;\nGameMode {\n  int i = 1 + 2;\n  string s = \"a\" + $i;\n  array a = [1, 2];\n  g = i * 2;\n  if (i > 1) { i += 1; }\n  while (i < 10) { i++; }\n  for (int k = 0; k < 3; k++) { continue; }\n  for (array v in a) { break; }\n  a = {\"x\"::1, \"y\"::2};\n}",
    );
    assert_eq!(e, Vec::<String>::new());
    let ast = ast.unwrap();
    // The types the checker gave: `g = i * 2` is a number assignment.
    let StmtKind::Begin { block, .. } = &ast.stmt(ast.blocks[0]).kind else {
        panic!()
    };
    let s = ast.block_stmts(*block)[3];
    let StmtKind::Expr(Some(assign)) = ast.stmt(s).kind else {
        panic!()
    };
    assert_eq!(ast.expr(assign).ty, tt::NUMBER);
    let ExprKind::Assign { left, .. } = ast.expr(assign).kind else {
        panic!()
    };
    assert_eq!(ast.expr(left).ty, tt::NUMBER);
    // The declarations' variables.
    let names: Vec<(String, u32)> = ast
        .vars
        .iter()
        .map(|v| {
            (
                String::from_utf8_lossy(&v.remapped_name).into_owned(),
                v.index,
            )
        })
        .collect();
    assert_eq!(names[0], ("g".into(), 1));
    assert_eq!(names[1], ("__temp_i_1".into(), 2));
    assert!(ast.vars.iter().all(|v| v.detailed_type != tt::INVALID));
}

#[test]
fn global_initializers() {
    let line = "int a = 5;";
    let (_, e) = check(&format!("name t;\n{line}"));
    assert_eq!(
        e,
        [
            hl(
                "Variable initializers are not allowed in global scope.",
                2,
                line,
                1,
                9
            ),
            String::new()
        ]
    );
    // An initializer that fails to check is outside any catch: a crash.
    assert_eq!(
        crash("name t;\nint a = foo;"),
        "TypeCheckerError outside any catch"
    );
}

#[test]
fn blocks() {
    let (_, e) = check("name t;\nGameMode {}\nGameMode {}");
    let first = format!("\r    | Duplicate block declaration.\n  2 | GameMode {{{R}}}{X}\n  3 | {R}GameMode {{}}{X}\n");
    assert_eq!(e, [first, String::new()]);
    // Another mode is another block.
    assert_eq!(
        check("name t;\nMenuMode::1 {}\nMenuMode::2 {}\nMenuMode {}").1,
        Vec::<String>::new()
    );
    let (_, e) = check("name t;\nGameMode {}\nfn () {}");
    assert_eq!(e.len(), 2);
    assert!(e[0].contains("Cannot have a function block and an event block in the same script."));
    let (_, e) = check("name t;\nfn () {}\nfn () {}");
    assert!(e[0].contains("Duplicate block declaration."));
}

#[test]
fn identifiers_and_commands() {
    let line = "  x = foo;";
    assert_eq!(
        block_errors(line),
        [
            hl("Unable to resolve identifier 'x'.", 3, line, 3, 3),
            String::new()
        ]
    );
    let line = "  f(1);";
    assert_eq!(
        block_errors(line),
        [hl("Not a valid command", 3, line, 3, 3), String::new()]
    );
    let line = "  int GetPos;";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Variable name 'GetPos' conflicts with a command with the same name.",
                3,
                line,
                3,
                12
            ),
            String::new()
        ]
    );
    // `player` is the player's reference; a global is a global.
    assert_eq!(
        block_errors("  ref r = player;\n  float f = GameHour;"),
        Vec::<String>::new()
    );
}

#[test]
fn operators() {
    let line = "  int i = \"a\" - \"b\";";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid types 'String' and 'String' for operator - (-).",
                3,
                line,
                11,
                19
            ),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  int i = -\"a\";"),
        [
            bug("Invalid operand type 'String' for operator - (-)."),
            String::new()
        ]
    );
    // A span starting where a unary operand left no place (column 0) and ending in a token's:
    // either column 0 prints the BUG line.
    assert_eq!(
        block_errors("  int i = -1 + \"a\";"),
        [
            bug("Invalid types 'Number' and 'String' for operator + (+)."),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  string s;\n  s++;"),
        [
            bug("Invalid operand type 'String Variable' for operator ++ (++)."),
            String::new()
        ]
    );
    let line = "  string s = \"a\"; int x = s[\"b\"];";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Expression type 'String' not valid for operator [].",
                3,
                line,
                28,
                32
            ),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  int x = 1 ? 2 : \"a\";"),
        [bug("Incompatible value types ('Number' and 'String') specified for ternary expression."), String::new()]
    );
    assert_eq!(
        block_errors("  int b = 1 in [1, \"a\"];"),
        [bug("Value 2 (type 'String') cannot compare against the Number specified on lhs of 'in' expression."), String::new()]
    );
    assert_eq!(
        block_errors("  int b = 1 in 5;"),
        [
            bug("Expected array for 'in' expression (Got 'Number')."),
            String::new()
        ]
    );
}

#[test]
fn conditions_and_loops() {
    let line = "  if (1::2) {}";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid expression type 'Pair' for if statement.",
                3,
                line,
                3,
                14
            ),
            String::new()
        ]
    );
    let line = "  while (\"a\") {}";
    assert_eq!(
        block_errors(line),
        [
            hl("Invalid expression type for while loop.", 3, line, 10, 12),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  for (int x in 5) {}"),
        [
            bug("Invalid types 'Numeric Variable' and 'Number' passed to for-in expression."),
            String::new()
        ]
    );
    let line = "  continue;";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Keyword 'continue' not valid outside of loops.",
                3,
                line,
                3,
                11
            ),
            String::new()
        ]
    );
    let line = "  break;";
    assert_eq!(
        block_errors(line),
        [
            hl("Keyword 'break' not valid outside of loops.", 3, line, 3, 8),
            String::new()
        ]
    );
    // A lambda's body is not inside the loop around it.
    let e = block_errors("  while (1) { ref f = fn () { break; }; }");
    assert!(e[0].contains("Keyword 'break' not valid outside of loops."));
}

#[test]
fn returns() {
    let line = "  return 1; return \"a\";";
    let (_, e) = check(&format!("name t;\nfn () {{\n{line}\n}}"));
    assert_eq!(
        e,
        [
            hl("Return type 'String' not compatible with return type defined previously. ('Number' at line 0)", 3, line, 20, 22),
            String::new()
        ]
    );
    // A value returned outside a function: xNVSE reads an empty stack.
    assert_eq!(
        crash("name t;\nGameMode { return 1; }"),
        "return value outside a function: empty stack"
    );
    assert_eq!(
        check("name t;\nGameMode { return; }").1,
        Vec::<String>::new()
    );
}

#[test]
fn literals() {
    assert_eq!(
        block_errors("  int x = 4294967296;"),
        [
            hl(
                "Maximum value for integer literal exceeded. (Max: 4294967295)",
                3,
                "  int x = 4294967296;",
                11,
                20
            ),
            String::new()
        ]
    );
    let line = "  array m = {};";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "A map literal cannot be empty, as the key type cannot be deduced.",
                3,
                line,
                13,
                14
            ),
            String::new(),
            hl(
                "Cannot convert from Invalid to Array Variable",
                3,
                line,
                3,
                14
            ),
            String::new()
        ]
    );
    let line = "  array a = [1, \"b\"];";
    assert_eq!(
        block_errors(line),
        [
            hl("Value 1 (type 'String') specified for array literal conflicts with the type already specified in first element ('Number').", 3, line, 13, 20),
            String::new()
        ]
    );
    let line = "  array a = [1::2];";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid type inside of array literal. Expected array, string, ref, or number.",
                3,
                line,
                14,
                17
            ),
            String::new()
        ]
    );
    let line = "  array m = {1::2, \"a\"::3};";
    assert_eq!(
        block_errors(line),
        [
            hl("Key for value 1 (type 'String') specified for map literal conflicts with key type of previous value ('Number').", 3, line, 13, 26),
            String::new()
        ]
    );
    let line = "  array m = {1, 2};";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Value 1 is not a pair and is not valid for a map literal.",
                3,
                line,
                13,
                18
            ),
            String::new(),
            hl(
                "Cannot convert from Invalid to Array Variable",
                3,
                line,
                3,
                18
            ),
            String::new()
        ]
    );
}

#[test]
fn command_parameters() {
    // Vanilla enum names become numbers of their length (an axis one byte).
    let (ast, e) = check("name t;\nGameMode {\n  int x = GetAV(Health);\n  x = GetPos(X);\n}");
    assert_eq!(e, Vec::<String>::new());
    let ast = ast.unwrap();
    let text = dump(&ast);
    assert!(text.contains("GetAV(enum2:16)"), "{text}");
    assert!(text.contains("GetPos(enum1:88)"), "{text}");
    let line = "  MoveTo(Caps001);";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid expression for parameter 1. Expected object reference.",
                3,
                line,
                10,
                16
            ),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  MoveTo(PlayerRef);\n  AddItem(Caps001, 10);"),
        Vec::<String>::new()
    );
    let line = "  Print(5);";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid expression for parameter 1. (Expected string, got Number).",
                3,
                line,
                9,
                9
            ),
            String::new()
        ]
    );
    let line = "  Print();";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid number of parameters specified to Print (Expected 1, got 0).",
                3,
                line,
                3,
                9
            ),
            String::new()
        ]
    );
}

/// A command that needs a calling reference, called without one from a script that is not an
/// object or magic effect script.
#[test]
fn calling_references() {
    let text = "name t;\nGameMode {\n  GetDisabled();\n}";
    let mut env = TestEnv {
        script: Some((script_type::QUEST, Vec::new())),
        ..TestEnv::default()
    };
    let (_, e) = check_with(&mut env, text, true);
    assert_eq!(
        e,
        [
            hl(
                "Command 'GetDisabled' requires a calling reference.",
                3,
                "  GetDisabled();",
                3,
                15
            ),
            String::new()
        ]
    );
    let mut env = TestEnv {
        script: Some((script_type::OBJECT, Vec::new())),
        ..TestEnv::default()
    };
    assert_eq!(check_with(&mut env, text, true).1, Vec::<String>::new());
    // Without a compiling script the check is not made.
    assert_eq!(check(text).1, Vec::<String>::new());
}

#[test]
fn lambdas() {
    let (_, e) = check("name t;\nGameMode {\n  ref f = fn (int a) -> a;\n  f(1, 2);\n}");
    assert_eq!(
        e,
        [
            hl(
                "Invalid number of parameters specified for lambda 'f' (Expected 1, got 2).",
                4,
                "  f(1, 2);",
                3,
                9
            ),
            String::new()
        ]
    );
    let (_, e) = check("name t;\nGameMode {\n  ref f = fn (string a) -> a;\n  f(1);\n}");
    assert_eq!(
        e,
        [
            hl(
                "Invalid expression for lambda parameter 1. (Expected String, got Number)",
                4,
                "  f(1);",
                3,
                6
            ),
            String::new()
        ]
    );
    // For the delayed calls (time, function, flags, then their own arguments) xNVSE reads the
    // parameter type at the argument's place less one: the first argument against the third
    // parameter.
    let line = "  CallAfterSeconds(1, f, 0, 5);";
    let (_, e) = check(&format!(
        "name t;\nGameMode {{\n  ref f = fn (int a, int b, string c) {{}};\n{line}\n}}"
    ));
    let first = "Invalid expression for lambda parameter 1. (Expected String, got Number)";
    let count = "Invalid number of parameters specified for lambda 'f' (Expected 3, got 1).";
    assert_eq!(
        e,
        [
            hl(first, 4, line, 3, 30),
            String::new(),
            hl(count, 4, line, 3, 30),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  ref f = fn () {};\n  f = 5;"),
        [
            bug("Cannot assign to a variable that is holding a lambda."),
            String::new()
        ]
    );
    assert_eq!(
        block_errors("  ref f;\n  f = fn () {};"),
        [
            bug("Cannot assign a lambda to a variable after it has been declared."),
            String::new()
        ]
    );
    // A user function by its editor id: `Call` with it first; a script that is not an object
    // script is no user function.
    let (ast, e) = check("name t;\nGameMode {\n  MyUDF(1);\n}");
    assert_eq!(e, Vec::<String>::new());
    assert!(dump(&ast.unwrap()).contains("Call(MyUDF,1)"));
    let line = "  CallAfterSeconds(1, QuestScript);";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Target script is not an object script (Invalid UDF)",
                3,
                line,
                23,
                33
            ),
            String::new()
        ]
    );
    // A lambda parameter named after a command has no type: xNVSE throws past every catch.
    assert_eq!(
        crash("name t;\nGameMode {\n  ref f = fn (int GetPos) {};\n}"),
        "TokenType_To_ParamType: not a basic type"
    );
}

#[test]
fn member_access() {
    let (ast, e) = check("name t;\nGameMode {\n  int s = MyQuest.stage;\n}");
    assert_eq!(e, Vec::<String>::new());
    let ast = ast.unwrap();
    let get = ast
        .exprs
        .iter()
        .find(|x| matches!(x.kind, ExprKind::Get { .. }))
        .unwrap();
    assert_eq!(get.ty, tt::AMBIGUOUS);
    let ExprKind::Get {
        var_info,
        reference_name,
        ..
    } = &get.kind
    else {
        panic!()
    };
    assert_eq!(*var_info, Some((crate::compiler::vt::INTEGER, 3)));
    assert_eq!(reference_name.as_deref(), Some(&b"MyQuest"[..]));
    let line = "  int s = MyQuest.nothing;";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Variable nothing not found on form MyQuest.",
                3,
                line,
                11,
                17
            ),
            String::new()
        ]
    );
    let line = "  int s = NoScriptQuest.x;";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Unable to resolve type for 'NoScriptQuest.x'.",
                3,
                line,
                11,
                23
            ),
            String::new()
        ]
    );
    let line = "  int s = SomeWeapon.x;";
    assert_eq!(
        block_errors(line),
        [
            hl("Unexpected form type found.", 3, line, 11, 20),
            String::new()
        ]
    );
    let line = "  int v; int s = v.x;";
    assert_eq!(
        block_errors(line),
        [hl("Member access not valid here. Left side of '.' must be a form or persistent reference.", 3, line, 18, 18), String::new()]
    );
}

/// The plugins a script needs: xNVSE's own version, a `#version`'s, and each plugin a command
/// of it belongs to (the greater version kept).
#[test]
fn requirements() {
    let (ast, _) = check("name t;\nGameMode {\n  JipThing();\n  JipOther();\n  Print(\"a\");\n}");
    let ast = ast.unwrap();
    let r: Vec<(String, u32)> = ast
        .requirements
        .iter()
        .map(|(k, v)| (String::from_utf8_lossy(k).into_owned(), *v))
        .collect();
    assert_eq!(r.len(), 2);
    assert!(r.contains(&("nvse".into(), crate::PACKED_VERSION)));
    assert!(r.contains(&("jip ln nvse".into(), 57)));
    let (ast, _) = check("name t;\n#version(\"nvse\", 5)\nGameMode {\n  Print(\"a\");\n}");
    assert_eq!(ast.unwrap().requirements.get(b"nvse"), Some(&0x05ff_0ff0));
    assert_eq!(crate::PACKED_VERSION, crate::newlang::PACKED_NVSE_VERSION);
}

#[test]
fn variables_and_scopes() {
    let text = "name t;\nint g;\nGameMode {\n  int a;\n  { int a; }\n}";
    let mut env = TestEnv {
        script: Some((0, vec![(b"old".to_vec(), 0), (b"__temp_x_1".to_vec(), 1)])),
        ..TestEnv::default()
    };
    let (_, e) = check_with(&mut env, text, true);
    assert_eq!(e, Vec::<String>::new());
    let w: Vec<(String, u8, u32)> = env
        .written_vars
        .unwrap()
        .into_iter()
        .map(|(n, t, i)| (String::from_utf8_lossy(&n).into_owned(), t, i))
        .collect();
    assert_eq!(
        w,
        [
            ("old".into(), 0, 1),
            ("g".into(), 1, 2),
            ("__temp_a_1".into(), 1, 3),
            ("__temp_a_2".into(), 1, 4)
        ]
    );
    // A script's own variable declared again takes the declaration's type and keeps its place.
    let mut env = TestEnv {
        script: Some((0, vec![(b"old".to_vec(), 0)])),
        ..TestEnv::default()
    };
    check_with(&mut env, "name t;\nint g;\nstring old;", true);
    let w = env.written_vars.unwrap();
    assert_eq!(w[0], (b"old".to_vec(), 2, 1));
    // A function's parameters are root variables.
    let mut env = TestEnv {
        script: Some((0, Vec::new())),
        ..TestEnv::default()
    };
    check_with(&mut env, "name t;\nfn (int a) { int b; }", true);
    let w = env.written_vars.unwrap();
    assert_eq!(w, [(b"a".to_vec(), 1, 1), (b"__temp_b_1".to_vec(), 1, 2)]);
    // The same name twice in one scope.
    let line = "  int a; int a;";
    assert_eq!(
        check(&format!("name t;\nGameMode {{\n{line}\n}}")).1,
        [hl(
            "Variable with this name already exists in this scope",
            3,
            line,
            14,
            14
        )]
    );
    // Names are looked up as written.
    assert!(block_errors("  int a;\n  A = 1;")[0].contains("Unable to resolve identifier 'A'."));
}

#[test]
fn match_statements() {
    assert_eq!(
        matched("name t;\nGameMode {\n  match (x) { 1 -> { a; } y: y > 2 -> { b; } _ -> { c; } }\n}"),
        "name t\nGameMode {array _mr1=[x] if((== (Unbox _mr1) 1)) {a;} else {if((> (Unbox _mr1) 2)) {b;} else {c;}}}\n"
    );
    // An arm's condition that is the binding itself is not renamed (xNVSE walks the arm's
    // expression without replacing it); a declaration of the binding's name ends it.
    assert_eq!(
        matched("name t;\nGameMode {\n  match (x) { y: y -> { int y; y; } }\n}"),
        "name t\nGameMode {array _mr1=[x] if(y) {int y y;}}\n"
    );
    // Nested matches number their variables by depth.
    let t = matched("name t;\nGameMode {\n  match (x) { y: 1 -> { match (z) { w: w -> {} } } }\n}");
    assert!(t.contains("array _mr2=[z]"), "{t}");
    // Only a default arm: xNVSE leaves a null statement and calls through it.
    let mut env = TestEnv::default();
    let p = parser::parse(&mut env, b"name t;\nGameMode { match (x) { _ -> {} } }").unwrap();
    let mut ast = p.ast.unwrap();
    assert_eq!(
        passes::match_transform::MatchTransformer::new(&mut ast).run(),
        Err(Fail::Crash("match with no arms: null statement"))
    );
    let _ = dump_expr;
}

/// A `for` loop without an increment: xNVSE's walk dereferences it.
#[test]
fn a_for_loop_without_an_increment_crashes() {
    assert_eq!(
        crash("name t;\nGameMode { for (int i = 0; i < 3;) {} }"),
        "null expression"
    );
}

/// `ValidateArgType` for vanilla parameters.
#[test]
fn vanilla_argument_types() {
    use crate::compiler::tt::*;
    let sv = cmd_named("sv_Thing", 1);
    let many = cmd_named("Many", 20);
    let plain = cmd_named("Plain", 1);
    // Ambiguous fits anything.
    assert!(validate(0x01, AMBIGUOUS, false, None));
    assert!(validate(0x00, STRING, false, None));
    assert!(!validate(0x00, NUMBER, false, None));
    // A string for an integer only for the old string commands and long parameter lists.
    assert!(validate(0x01, STRING, false, Some(&sv)));
    assert!(validate(0x01, STRING, false, Some(&many)));
    assert!(!validate(0x01, STRING, false, Some(&plain)));
    assert!(validate(0x01, STRING_VAR, false, Some(&plain)));
    assert!(validate(0x05, STRING, false, None));
    assert!(!validate(0x16, NUMBER, false, None));
    assert!(validate(0x1f, FORM, false, None));
    assert!(validate(0x100, ARRAY, false, None));
    assert!(validate(0x2d, NUMERIC_VAR, false, None));
    assert!(validate(0x0e, REF_VAR, false, None));
    assert!(!validate(0x0e, NUMBER, false, None));
    // xNVSE's types.
    assert!(validate(
        crate::compiler::NVSE_STRING,
        AMBIGUOUS,
        true,
        None
    ));
    assert!(!validate(crate::compiler::NVSE_STRING, NUMBER, true, None));
}

fn cmd_named(name: &str, params: usize) -> CmdInfo {
    crate::newlang::test_env::cmd(
        name,
        0x1000,
        (0..params)
            .map(|_| crate::newlang::ast::ParamInfo {
                name: b"int".to_vec(),
                kind: 1,
                optional: true,
            })
            .collect(),
        Parse::Default,
        tt::INVALID,
    )
}

#[test]
fn token_operators() {
    use crate::newlang::lexer::TokenType as T;
    assert_eq!(token_op(T::PlusPlus), op::ASSIGNMENT);
    assert_eq!(token_op(T::Negate), op::NEGATION);
    assert_eq!(token_op(T::Unbox), op::DEREFERENCE);
    assert_eq!(token_op(T::Box), op::BOX);
    assert_eq!(token_op(T::Dollar), op::TO_STRING);
    assert_eq!(token_op(T::Pound), op::TO_NUMBER);
    assert_eq!(token_op(T::ModEq), op::MODULO_EQUALS);
    assert_eq!(token_op(T::BitwiseOrEquals), op::BITWISE_OR_EQUALS);
    assert_eq!(token_op(T::Slice), op::SLICE);
    assert_eq!(token_op(T::MakePair), op::MAKE_PAIR);
    assert_eq!(token_op(T::Dot), op::DOT);
}

/// `TokenTypeToString` for every type, from `ScriptTokens.cpp`.
#[test]
fn token_type_names() {
    let want = [
        "Number",
        "Boolean",
        "String",
        "Form",
        "Ref",
        "Global",
        "Array",
        "Array Element",
        "Slice",
        "Command",
        "Variable",
        "Numeric Variable",
        "Reference Variable",
        "String Variable",
        "Array Variable",
        "Ambiguous",
        "Operator",
        "ForEach Context",
        "Byte",
        "Short",
        "Int",
        "Pair",
        "Assignable String",
        "Anonymous Function",
        "Unknown",
        "Unknown",
        "Unknown",
        "OptionalEmpty",
        "Invalid",
        "Empty",
        "Unknown",
    ];
    for (t, name) in want.iter().enumerate() {
        assert_eq!(crate::compiler::type_name(t as u8), *name, "{t}");
    }
    assert_eq!(crate::compiler::type_name(255), "Unknown");
    let line = "  int x = fn () {};";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Cannot convert from Anonymous Function to Numeric Variable",
                3,
                line,
                3,
                18
            ),
            String::new()
        ]
    );
}

/// `tokenOpToNVSEOpType` (`Utils.cpp`), all 38 rows, with `kOpType_*`'s values from
/// `ScriptTokens.h`; `operator[]` gives 0 for a token it lacks.
#[test]
fn every_token_operator() {
    use crate::newlang::lexer::TokenType as T;
    let table: [(T, u8); 38] = [
        (T::EqEq, 4),
        (T::LogicOr, 1),
        (T::LogicAnd, 2),
        (T::Greater, 6),
        (T::GreaterEq, 8),
        (T::Less, 7),
        (T::LessEq, 9),
        (T::Bang, 21),
        (T::BangEq, 5),
        (T::Plus, 14),
        (T::Minus, 15),
        (T::Star, 16),
        (T::Slash, 17),
        (T::Mod, 18),
        (T::Pow, 19),
        (T::Eq, 0),
        (T::PlusEq, 28),
        (T::MinusEq, 32),
        (T::StarEq, 29),
        (T::SlashEq, 30),
        (T::ModEq, 43),
        (T::PowEq, 31),
        (T::MakePair, 36),
        (T::Slice, 3),
        (T::BitwiseAnd, 11),
        (T::BitwiseOr, 10),
        (T::BitwiseAndEquals, 42),
        (T::BitwiseOrEquals, 41),
        (T::LeftShift, 12),
        (T::RightShift, 13),
        (T::Negate, 20),
        (T::Dollar, 27),
        (T::Pound, 33),
        (T::Box, 37),
        (T::Unbox, 34),
        (T::LeftBracket, 24),
        (T::Dot, 40),
        (T::BitwiseNot, 44),
    ];
    for (t, o) in table {
        assert_eq!(token_op(t), o, "{t:?}");
    }
    assert_eq!(token_op(T::PlusPlus), 0);
    assert_eq!(token_op(T::MinusMinus), 0);
}

/// A message without a place is a `vsprintf` format in xNVSE: an operator's `%` there is a
/// conversion it refuses, which stops the game.
#[test]
fn a_percent_in_a_placeless_message_crashes() {
    assert_eq!(
        crash("name t;\nGameMode {\n  string s;\n  s %= 2;\n}"),
        "vsprintf_s: invalid format in a message without a place"
    );
    assert_eq!(
        crash("name t;\nGameMode {\n  int x = -1 % \"a\";\n}"),
        "vsprintf_s: invalid format in a message without a place"
    );
}

/// The type checker's other branches.
#[test]
fn more_type_checker_branches() {
    // A calling reference given; a magic effect script needs none.
    let mut env = TestEnv {
        script: Some((script_type::QUEST, Vec::new())),
        ..TestEnv::default()
    };
    assert_eq!(
        check_with(
            &mut env,
            "name t;\nGameMode {\n  player.GetDisabled();\n}",
            true
        )
        .1,
        Vec::<String>::new()
    );
    let mut env = TestEnv {
        script: Some((script_type::MAGIC, Vec::new())),
        ..TestEnv::default()
    };
    assert_eq!(
        check_with(&mut env, "name t;\nGameMode {\n  GetDisabled();\n}", true).1,
        Vec::<String>::new()
    );
    // The ternary's condition (the ternary has no place).
    assert_eq!(
        block_errors("  int x = (1::2) ? 1 : 2;"),
        [
            bug("Invalid expression type 'Pair' for if statement."),
            String::new()
        ]
    );
    // A grouping catches its own error: the outer operator then sees an invalid type.
    let line = "  int x = (1 - \"a\") + 1;";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid types 'Number' and 'String' for operator - (-).",
                3,
                line,
                12,
                18
            ),
            String::new(),
            hl(
                "Invalid types 'Invalid' and 'Number' for operator + (+).",
                3,
                line,
                11,
                23
            ),
            String::new()
        ]
    );
    // An argument that failed to check is passed over: no parameter error after it.
    let line = "  Print(nope);";
    assert_eq!(
        block_errors(line),
        [
            hl("Unable to resolve identifier 'nope'.", 3, line, 9, 12),
            String::new()
        ]
    );
    // A duplicate block over several lines: its middle lines red whole, the text after its
    // closing brace not.
    let (_, e) = check("name t;\nGameMode {}\nGameMode {\n  int a;\n} int b;");
    let want = format!(
        "\r    | Duplicate block declaration.\n  2 | GameMode {{{R}}}{X}\n  3 | {R}GameMode {{{X}\n  4 | {R}  int a;{X}\n  5 | {R}}}{X} int b;\n"
    );
    assert_eq!(e, [want, String::new()]);
    // The for loop's condition.
    let line = "  for (int i = 0; \"a\"::1; i++) {}";
    assert_eq!(
        block_errors(line),
        [
            hl(
                "Invalid expression type ('Invalid') for loop condition.",
                3,
                line,
                3,
                33
            ),
            String::new()
        ]
    );
}

/// A script variable listed twice: one variable, of the later type.
#[test]
fn a_script_variable_listed_twice() {
    let mut env = TestEnv {
        script: Some((0, vec![(b"v".to_vec(), 0), (b"v".to_vec(), 1)])),
        ..TestEnv::default()
    };
    check_with(&mut env, "name t;\nGameMode {}", true);
    assert_eq!(env.written_vars.unwrap(), [(b"v".to_vec(), 1, 1)]);
}

/// The match and call passes make xNVSE's debug note's text in every build: a place past its
/// line (after a string broke over a line, which the lexer does not count) throws there.
#[test]
fn the_passes_debug_notes_can_throw() {
    assert_eq!(
        crash("name t;\nGameMode {\n  int i;\"ab\n  MyUDF(1);\n}"),
        "string_view::substr: out_of_range"
    );
    assert_eq!(
        crash("name t;\nGameMode {\n  int i;\"ab\n  match (i) { 1 -> {} }\n}"),
        "string_view::substr: out_of_range"
    );
}
