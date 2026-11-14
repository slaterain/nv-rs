//! The code generation's bytes, from xNVSE 6.4.9's `Compiler.cpp` emit rules, worked out by hand
//! for each case on the stand-in game (its opcodes: `Let` 0x1539, `eval` 0x153A,
//! `Jmp_If_False` 0x1680, `Jmp` 0x1681, `PluginVersion` 0x1676, `SetFunctionValue` 0x1546, the
//! vanilla `MoveTo` 0x1003 and `ShowMessage` 0x1008).

use crate::newlang::test_env::TestEnv;
use crate::newlang::{compile_with, Fail};

/// The compiled data of `text` on the stand-in game (no compiling script's variables).
fn compile(text: &str) -> Vec<u8> {
    let mut env = TestEnv::default();
    let c = compile_with(&mut env, text.as_bytes(), false)
        .expect("no crash")
        .unwrap_or_else(|| panic!("{text}: {:?}", env.take_errors()));
    c.data
}

/// `PluginVersion("nvse", 6.4.9)` at statement level: the opcode, the length (the count and
/// two arguments of their own length words), two arguments, the plugin's name (`S`) and the
/// version (`L`).
fn version(name: &[u8], v: u32) -> Vec<u8> {
    let mut b = vec![0x76, 0x16];
    let len = 1 + (2 + 3 + name.len()) + (2 + 5);
    b.extend_from_slice(&(len as u16).to_le_bytes());
    b.push(2);
    b.extend_from_slice(&((2 + 3 + name.len()) as u16).to_le_bytes());
    b.push(b'S');
    b.extend_from_slice(&(name.len() as u16).to_le_bytes());
    b.extend_from_slice(name);
    b.extend_from_slice(&[7, 0, b'L']);
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// The start of every script: the script name statement (32 bits) and xNVSE's requirement.
fn header() -> Vec<u8> {
    let mut b = vec![0x1d, 0, 0, 0];
    b.extend(version(b"nvse", crate::PACKED_VERSION));
    b
}

/// `Begin` of block `opcode` with no parameter, `body`, `End`.
fn block(opcode: u16, body: &[u8]) -> Vec<u8> {
    let mut b = vec![0x10, 0, 6, 0];
    b.extend_from_slice(&opcode.to_le_bytes());
    b.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
    b.extend_from_slice(body);
    b.extend_from_slice(&[0x11, 0, 0, 0]);
    b
}

fn script(parts: &[&[u8]]) -> Vec<u8> {
    let mut b = header();
    for p in parts {
        b.extend_from_slice(p);
    }
    b
}

#[test]
fn an_empty_block() {
    assert_eq!(compile("name t;\nGameMode {\n}"), script(&[&block(0, &[])]));
    assert_eq!(header().len(), 25);
}

/// A declaration with a value: `Let` (statement level: the length counts from after its own
/// word) of the variable token (`V`, type, no reference, index) and the value, then `:=`.
#[test]
fn declarations_compile_to_let() {
    #[rustfmt::skip]
    let let_i = [
        0x39, 0x15, 0x0c, 0x00, 0x01,
        0x0b, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00, b'B', 0x05, 0x00,
    ];
    assert_eq!(
        compile("name t;\nGameMode {\n  int i = 5;\n}"),
        script(&[&block(0, &let_i)])
    );
    // A declaration without a value writes nothing.
    assert_eq!(
        compile("name t;\nGameMode {\n  int i;\n}"),
        script(&[&block(0, &[])])
    );
}

/// Numbers in their smallest form (`B`, `I`, `L`), doubles (`Z`), strings (`S`).
#[test]
fn literals() {
    let got = compile("name t;\nGameMode {\n  float f = 1.5;\n  int a = 300;\n  int b = 70000;\n  string s = \"ab\";\n}");
    let mut f = vec![b'Z'];
    f.extend_from_slice(&1.5f64.to_le_bytes());
    let decl = |ty: u8, index: u8, value: &[u8]| {
        let arg = 2 + 6 + value.len() + 1;
        let mut b = vec![0x39, 0x15];
        b.extend_from_slice(&((1 + arg) as u16).to_le_bytes());
        b.push(1);
        b.extend_from_slice(&(arg as u16).to_le_bytes());
        b.extend_from_slice(&[b'V', ty, 0, 0, index, 0]);
        b.extend_from_slice(value);
        b.push(0);
        b
    };
    let body = [
        decl(0, 1, &f),
        decl(1, 2, &[b'I', 0x2c, 0x01]),
        decl(1, 3, &[b'L', 0x70, 0x11, 0x01, 0x00]),
        decl(2, 4, &[b'S', 2, 0, b'a', b'b']),
    ]
    .concat();
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `if`: `Jmp_If_False` on the condition with a placeholder target (`L`), the block, and the
/// target the offset past the block from the script's start.
#[test]
fn if_compiles_to_a_conditional_jump() {
    let got = compile("name t;\nGameMode {\n  int i;\n  if (i) { i = 1; }\n}");
    // The block starts at 25 + 10: the jump is 20 bytes, the `Let` 16, so the target is 71.
    #[rustfmt::skip]
    let body = [
        0x80, 0x16, 0x10, 0x00, 0x02,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        0x07, 0x00, b'L', 71, 0x00, 0x00, 0x00,
        0x39, 0x15, 0x0c, 0x00, 0x01,
        0x0b, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00, b'B', 0x01, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `i++` as a statement: `eval` of `i i 1 + :=` then `1 -`.
#[test]
fn postfix_increment() {
    let got = compile("name t;\nGameMode {\n  int i;\n  i++;\n}");
    #[rustfmt::skip]
    let body = [
        0x3a, 0x15, 0x16, 0x00, 0x01,
        0x15, 0x00,
        b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'B', 0x01, 0x0e, 0x00, b'B', 0x01, 0x0f,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A method call as a statement: `eval` of the reference (`R`, its index among the script's
/// references), `x` and the command (its length counting its own word inside an expression,
/// a 16-bit count for a default-parsed command, each argument after `0xFFFF`), then `.`.
#[test]
fn a_method_call_goes_through_eval() {
    let got = compile("name t;\nGameMode {\n  player.MoveTo(PlayerRef);\n}");
    #[rustfmt::skip]
    let body = [
        0x3a, 0x15, 0x17, 0x00, 0x01,
        0x16, 0x00,
        b'R', 0x01, 0x00,
        b'x', 0x00, 0x00, 0x03, 0x10, 0x0b, 0x00, 0x01, 0x00,
        0xff, 0xff, 0x05, 0x00, b'R', 0x01, 0x00,
        0x28,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A quest's variable: `V`, its type, the quest's reference index, its index.
#[test]
fn a_member_variable() {
    let got = compile("name t;\nGameMode {\n  int s = MyQuest.stage;\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x10, 0x00, 0x01,
        0x0f, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'V', 0x01, 0x01, 0x00, 0x03, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `ShowMessage(message, duration)`: the game's form of the command (the reference `r`, no
/// variables, the duration in 32 bits).
#[test]
fn show_message() {
    let got = compile("name t;\nGameMode {\n  ShowMessage(MyMessage, 5);\n}");
    #[rustfmt::skip]
    let body = [
        0x08, 0x10, 0x0b, 0x00,
        0x01, 0x00, b'r', 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A block's mode: an integer (`n` and 32 bits), a reference (`r` and its index), or an optional
/// one left out (a zero count).
#[test]
fn block_modes() {
    #[rustfmt::skip]
    let menu = [0x10, 0x00, 0x0d, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x01, 0x00, b'n', 0x29, 0x04, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00];
    assert_eq!(compile("name t;\nMenuMode::1065 {}"), script(&[&menu]));
    #[rustfmt::skip]
    let activate = [0x10, 0x00, 0x0b, 0x00, 0x02, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x01, 0x00, b'r', 0x01, 0x00, 0x11, 0x00, 0x00, 0x00];
    assert_eq!(
        compile("name t;\nOnActivate::PlayerRef {}"),
        script(&[&activate])
    );
    #[rustfmt::skip]
    let menu_any = [0x10, 0x00, 0x08, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x11, 0x00, 0x00, 0x00];
    assert_eq!(compile("name t;\nMenuMode {}"), script(&[&menu_any]));
}

/// A function block: mode 0x0D, its script length, version 1, the parameters (index, type),
/// no array parameters; `return a;` as `SetFunctionValue` then the game's `Return`.
#[test]
fn a_function_block() {
    let got = compile("name t;\nfn (int a, ref b) {\n  return a;\n}");
    #[rustfmt::skip]
    let f = [
        0x10, 0x00, 0x0f, 0x00, 0x0d, 0x00, 0x15, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x01, 0x00, 0x01, 0x02, 0x00, 0x04, 0x00,
        0x46, 0x15, 0x09, 0x00, 0x01, 0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        0x1e, 0x00, 0x00, 0x00,
        0x11, 0x00, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&f]));
}

/// A non-persistent reference named in a script: xNVSE throws past every catch.
#[test]
fn a_reference_that_is_not_persistent_crashes() {
    let mut env = TestEnv::default();
    let r = compile_with(
        &mut env,
        b"name t;\nGameMode {\n  LooseRef.MoveTo(PlayerRef);\n}",
        false,
    );
    assert_eq!(
        r.err(),
        Some(Fail::Crash("Object reference must be persistent"))
    );
}

/// A variable holding a lambda is mod-local data: its declaration `SetModLocalData` of
/// `"__lambda_<index>_<name>"` and the lambda (`F`, its size, then a script of its own: the name
/// statement, `Begin` of mode 0x0D with no parameters, its body, `End`).
#[test]
fn a_lambda_variable() {
    let got = compile("name t;\nGameMode {\n  ref f = fn () -> 5;\n}");
    let mut body = vec![0x49, 0x15, 0x37, 0x00, 0x02, 0x11, 0x00, b'S', 0x0c, 0x00];
    body.extend_from_slice(b"__lambda_1_f");
    #[rustfmt::skip]
    body.extend_from_slice(&[
        0x25, 0x00,
        b'F', 0x1e, 0x00, 0x00, 0x00,
        0x1d, 0x00, 0x00, 0x00,
        0x10, 0x00, 0x09, 0x00, 0x0d, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x46, 0x15, 0x05, 0x00, 0x01, 0x04, 0x00, b'B', 0x05,
        0x11, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A user function called by its editor id: `Call`'s own form (version 1, the callee's
/// expression with its length, the count, each argument with its length).
#[test]
fn a_user_function_call() {
    let got = compile("name t;\nGameMode {\n  MyUDF(1);\n}");
    #[rustfmt::skip]
    let body = [
        0x45, 0x15, 0x0b, 0x00, 0x01, 0x05, 0x00, b'R', 0x01, 0x00, 0x01, 0x04, 0x00, b'B', 0x01,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A ternary inside an expression: `X`, the script variable word, `Ternary`, its length counting
/// its own word, the count, three arguments.
#[test]
fn a_ternary_inside_an_expression() {
    let got = compile("name t;\nGameMode {\n  int x = 1 ? 2 : 3;\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x1e, 0x00, 0x01, 0x1d, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'X', 0x00, 0x00, 0x6e, 0x16, 0x0f, 0x00, 0x03,
        0x04, 0x00, b'B', 0x01, 0x04, 0x00, b'B', 0x02, 0x04, 0x00, b'B', 0x03,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// The plugins a script needs, one `PluginVersion` each, in MSVC's map order.
#[test]
fn requirements_in_map_order() {
    let got = compile("name t;\nGameMode {\n  JipThing();\n}");
    let mut map = crate::newlang::msvc::MsvcMap::default();
    map.set(b"nvse", crate::PACKED_VERSION);
    map.set(b"jip ln nvse", 57);
    let mut want = vec![0x1d, 0, 0, 0];
    for (k, v) in map.iter() {
        want.extend(version(k, *v));
    }
    assert!(got.starts_with(&want), "{got:02x?}");
}

/// The tree after the loop and lambda passes.
fn lowered(text: &str) -> String {
    use crate::newlang::{analyze, passes};
    let mut env = TestEnv::default();
    let mut ast = analyze(&mut env, text.as_bytes(), true)
        .unwrap()
        .ast
        .unwrap_or_else(|| panic!("{:?}", env.take_errors()));
    passes::loops::LoopTransformer::new(&mut ast).run().unwrap();
    passes::lambdas::LambdaTransformer::new(&mut ast, &mut env)
        .run()
        .unwrap();
    crate::newlang::parser::tests::dump(&ast)
}

/// `for`: the initializer before a `while`, the increment at the end of its block and before
/// each `continue` in it (a `continue` ending the block dropped first).
#[test]
fn for_loops_become_while_loops() {
    assert_eq!(
        lowered("name t;\nGameMode {\n  for (int i = 0; i < 3; i++) { if (i) { continue; } continue; }\n}"),
        "name t\nGameMode {int i=0 while((< i 3)) {if(i) {(i ++); continue;} (i ++);}}\n"
    );
    // A `continue` in a nested `while` is that loop's.
    assert_eq!(
        lowered(
            "name t;\nGameMode {\n  for (int i = 0; i < 3; i++) { while (i) { continue; } }\n}"
        ),
        "name t\nGameMode {int i=0 while((< i 3)) {while(i) {continue;} (i ++);}}\n"
    );
}

/// Lambda variables: the declaration made `SetModLocalData`, each read (where a pass replaces
/// expressions) `GetModLocalData`.
#[test]
fn lambda_variables_become_mod_local_data() {
    assert_eq!(
        lowered("name t;\nGameMode {\n  ref f = fn (int a) -> a;\n  f(1);\n}"),
        "name t\nGameMode {SetModLocalData(\"__lambda_1_f\",fn(int a) {SetFunctionValue(a);}); Call(GetModLocalData(\"__lambda_1_f\"),1);}\n"
    );
}

/// The integer forms' edges: 255 is still `B`, 65535 still `I`; a global is `G` and its
/// reference index.
#[test]
fn integer_edges_and_globals() {
    let got = compile("name t;\nGameMode {\n  int a = 255;\n  int b = 256;\n  int c = 65535;\n  int d = 65536;\n  float g = GameHour;\n}");
    let decl = |index: u8, value: &[u8]| {
        let arg = 2 + 6 + value.len() + 1;
        let mut b = vec![0x39, 0x15];
        b.extend_from_slice(&((1 + arg) as u16).to_le_bytes());
        b.push(1);
        b.extend_from_slice(&(arg as u16).to_le_bytes());
        let ty = if index == 5 { 0 } else { 1 };
        b.extend_from_slice(&[b'V', ty, 0, 0, index, 0]);
        b.extend_from_slice(value);
        b.push(0);
        b
    };
    let body = [
        decl(1, &[b'B', 0xff]),
        decl(2, &[b'I', 0x00, 0x01]),
        decl(3, &[b'I', 0xff, 0xff]),
        decl(4, &[b'L', 0x00, 0x00, 0x01, 0x00]),
        decl(5, &[b'G', 0x01, 0x00]),
    ]
    .concat();
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// A vanilla enum argument goes in as its raw bytes (two for an actor value, one for an axis),
/// counted; another argument after `0xFFFF`.
#[test]
fn enum_arguments() {
    #[rustfmt::skip]
    let av = [0x0e, 0x10, 0x0a, 0x00, 0x02, 0x00, 0x10, 0x00, 0xff, 0xff, 0x04, 0x00, b'B', 0x03];
    assert_eq!(
        compile("name t;\nGameMode {\n  GetAV(Health, 3);\n}"),
        script(&[&block(0, &av)])
    );
    let axis = [0x06, 0x10, 0x03, 0x00, 0x01, 0x00, b'X'];
    assert_eq!(
        compile("name t;\nGameMode {\n  GetPos(X);\n}"),
        script(&[&block(0, &axis)])
    );
}

/// `while` with `continue`: the condition's jump to the loop's end (75), the `continue` a jump
/// back to the condition (35), and the jump back at the end of the block.
#[test]
fn while_with_continue() {
    let got = compile("name t;\nGameMode {\n  while (1) { continue; }\n}");
    #[rustfmt::skip]
    let body = [
        0x80, 0x16, 0x0c, 0x00, 0x02, 0x04, 0x00, b'B', 0x01, 0x07, 0x00, b'L', 0x4b, 0x00, 0x00, 0x00,
        0x81, 0x16, 0x08, 0x00, 0x01, 0x07, 0x00, b'L', 0x23, 0x00, 0x00, 0x00,
        0x81, 0x16, 0x08, 0x00, 0x01, 0x07, 0x00, b'L', 0x23, 0x00, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `for ([k, v] in a)`: `ForEachAlt` of the array, then the value's variable, then the key's;
/// its jump word the offset past `loop` (72).
#[test]
fn for_each_alt() {
    let got = compile("name t;\nGameMode {\n  array a;\n  for ([int k, int v] in a) {}\n}");
    #[rustfmt::skip]
    let body = [
        0x70, 0x16, 0x1d, 0x00, 0x48, 0x00, 0x00, 0x00, 0x03,
        0x08, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x03, 0x00,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
    // A key left out (`_`): the optional empty token `K` in its place.
    let got = compile("name t;\nGameMode {\n  array a;\n  for ([_, int v] in a) {}\n}");
    assert!(
        got.windows(3).any(|w| w == [0x03, 0x00, b'K']),
        "{got:02x?}"
    );
}

/// A jump inside a lambda counts from the lambda's own script start (its name statement): the
/// `if`'s target past its block is 33 (4 + 13 of header + the 16-byte jump).
#[test]
fn jumps_in_a_lambda_count_from_its_start() {
    let got = compile("name t;\nGameMode {\n  ref f = fn () { if (1) {} };\n}");
    #[rustfmt::skip]
    let jump = [0x80, 0x16, 0x0c, 0x00, 0x02, 0x04, 0x00, b'B', 0x01, 0x07, 0x00, b'L', 0x21, 0x00, 0x00, 0x00];
    assert!(got.windows(jump.len()).any(|w| w == jump), "{got:02x?}");
}

/// The script's name is looked up among the forms first: a reference of that name that is not
/// persistent throws past every catch.
#[test]
fn a_script_named_after_a_loose_reference_crashes() {
    let mut env = TestEnv::default();
    let r = compile_with(&mut env, b"name LooseRef;\nGameMode {\n}", false);
    assert_eq!(
        r.err(),
        Some(Fail::Crash("Object reference must be persistent"))
    );
}

/// `in` a list: `MatchesAnyOf` of the left side and each value, `!` after it for `not in`;
/// `in` an array: `ar_Exists` of the array, then the left side.
#[test]
fn in_expressions() {
    let got = compile("name t;\nGameMode {\n  int b = 1 not in [1, 2];\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x1f, 0x00, 0x01, 0x1e, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'X', 0x00, 0x00, 0x6f, 0x16, 0x0f, 0x00, 0x03,
        0x04, 0x00, b'B', 0x01, 0x04, 0x00, b'B', 0x01, 0x04, 0x00, b'B', 0x02,
        0x15, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
    let got = compile("name t;\nGameMode {\n  array a;\n  int b = 1 in a;\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x1e, 0x00, 0x01, 0x1d, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        b'X', 0x00, 0x00, 0x71, 0x16, 0x0f, 0x00, 0x02,
        0x08, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, b'B', 0x01,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// The loop and lambda passes make xNVSE's debug notes' text in every build: a place past its
/// line (after a string broke over a line, which the lexer does not count) throws there.
#[test]
fn the_loop_and_lambda_notes_can_throw() {
    for (before, body) in [
        ("", "  for (int k = 0; k < 2; k++) { }"),
        ("", "  ref f = fn () {};"),
        // A declaration that keeps another name: only the note of the built call is made.
        ("", "  ref f = fn () {}, g;"),
        // A lambda variable read on the line past the break.
        ("  ref f = fn () {};\n", "  Call(f);"),
    ] {
        let mut env = TestEnv::default();
        let text = format!("name t;\nGameMode {{\n{before}  int i;\"ab\n{body}\n}}");
        let r = compile_with(&mut env, text.as_bytes(), false);
        assert_eq!(
            r.err(),
            Some(Fail::Crash("string_view::substr: out_of_range")),
            "{body}"
        );
    }
}

// The cases below follow `Compiler.cpp` line by line (xNVSE 6.4.9; the line numbers are its).
// Common to all: the block starts at 35 (25 bytes of name and requirement, 10 of `Begin`), so a
// jump target there is 35 plus the bytes before it; a `Let` at statement level is `39 15`, its
// length (the count, the argument's length word and the argument: `StartCall` 939-971 with
// `startPos` past the word, `FinishCall` 973-990), the count 1, then `StartManualArg` 1018-1027
// (the argument's length counting its own word), the variable token and the value, `00` (`:=`).

/// `for (array v in a) {}`: `VisitForEachStmt`'s plain branch (416-465). `3D 15`, the length of
/// the part after it (from the jump word to the end of the argument: 4 + 1 + 15 = 20), the jump
/// word, the count 1, the argument's length (2 + 6 + 6 + 1 = 15: its word, `v`, `a`, `In` 0x1A),
/// then the block and `loop` (`3C 15 00 00`); the jump word is where `loop` ends, from the
/// script's start (464): 35 + 24 + 4 = 63.
#[test]
fn a_plain_for_each() {
    let got = compile("name t;\nGameMode {\n  array a;\n  for (array v in a) {}\n}");
    #[rustfmt::skip]
    let body = [
        0x3d, 0x15, 0x14, 0x00, 0x3f, 0x00, 0x00, 0x00, 0x01,
        0x0f, 0x00, b'V', 0x03, 0x00, 0x00, 0x02, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00, 0x1a,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// The same loop after a statement and around one: `int i = 5;` (a 16-byte `Let`) puts the loop
/// at 51; its argument ends at 75, the block's `i = 1;` (another 16-byte `Let`) at 91, `loop` at
/// 95, the jump target. `v` is the third variable (`a`, `i`, then the loop's).
#[test]
fn a_plain_for_each_after_and_around_statements() {
    let got = compile(
        "name t;\nGameMode {\n  array a;\n  int i = 5;\n  for (array v in a) { i = 1; }\n}",
    );
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x0c, 0x00, 0x01, 0x0b, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00, b'B', 0x05, 0x00,
        0x3d, 0x15, 0x14, 0x00, 0x5f, 0x00, 0x00, 0x00, 0x01,
        0x0f, 0x00, b'V', 0x03, 0x00, 0x00, 0x03, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00, 0x1a,
        0x39, 0x15, 0x0c, 0x00, 0x01, 0x0b, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00, b'B', 0x01, 0x00,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// Inside a lambda the jump word counts from the lambda's own start (`scriptStart`, pushed at
/// 858): its name statement (4) and function header (13) put the loop at 17, so `loop` ends at
/// 17 + 24 + 4 = 45. `v` is the third variable (`a`, `f`, then the lambda's).
#[test]
fn a_plain_for_each_in_a_lambda() {
    let got =
        compile("name t;\nGameMode {\n  array a;\n  ref f = fn () { for (array v in a) {} };\n}");
    #[rustfmt::skip]
    let lp = [
        0x3d, 0x15, 0x14, 0x00, 0x2d, 0x00, 0x00, 0x00, 0x01,
        0x0f, 0x00, b'V', 0x03, 0x00, 0x00, 0x03, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00, 0x1a,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert!(got.windows(lp.len()).any(|w| w == lp), "{got:02x?}");
}

/// `break;` inside a `ForEach` is xNVSE's own `Break` command (`VisitBreakStmt` 504-511, not an
/// enhanced loop: 454): `3F 15 00 00`, and the jump word moves past it (63 + 4 = 67).
#[test]
fn break_inside_a_for_each() {
    let got = compile("name t;\nGameMode {\n  array a;\n  for (array v in a) { break; }\n}");
    #[rustfmt::skip]
    let body = [
        0x3d, 0x15, 0x14, 0x00, 0x43, 0x00, 0x00, 0x00, 0x01,
        0x0f, 0x00, b'V', 0x03, 0x00, 0x00, 0x02, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00, 0x1a,
        0x3f, 0x15, 0x00, 0x00,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `break;` and `continue;` inside a `ForEachAlt` are xNVSE's `Break` and `Continue` (495-511,
/// not an enhanced loop: 404): `3F 15 00 00`, `3E 15 00 00`; the loop of `for_each_alt` grows by
/// them (72 + 8 = 80).
#[test]
fn break_and_continue_inside_a_for_each_alt() {
    let got = compile(
        "name t;\nGameMode {\n  array a;\n  for ([int k, int v] in a) { break; continue; }\n}",
    );
    #[rustfmt::skip]
    let body = [
        0x70, 0x16, 0x1d, 0x00, 0x50, 0x00, 0x00, 0x00, 0x03,
        0x08, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x03, 0x00,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        0x3f, 0x15, 0x00, 0x00,
        0x3e, 0x15, 0x00, 0x00,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `for ([int k] in m)`: `ForEachAlt` with one name (386-400): the count 1 + 1 = 2 (337), the
/// array (its argument 8 bytes, the part's length 13 then), then the key's variable as the
/// second argument (`V`, its type, `00 00`, its index); the part's length 21, `loop` ends at
/// 35 + 25 + 4 = 64.
#[test]
fn a_for_each_alt_with_one_name() {
    let got = compile("name t;\nGameMode {\n  array m;\n  for ([int k] in m) {}\n}");
    #[rustfmt::skip]
    let body = [
        0x70, 0x16, 0x15, 0x00, 0x40, 0x00, 0x00, 0x00, 0x02,
        0x08, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00,
        0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        0x3c, 0x15, 0x00, 0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `int x = MyUDF(1);`: a `Call` inside an expression (`VisitCallExpr` 649-683): `X` and the
/// script variable word (650-653), `45 15`, a length that counts its own word (656: `+ 0` inside
/// an expression) = 2 + 1 + 5 + 1 + 4 = 13, version 1, the callee's length and expression (`R`,
/// reference 1), the count 1, the argument's length and `B 01`. The `Let`'s argument is 6 + 18 +
/// 1 = 25 bytes (27 with its word).
#[test]
fn a_user_function_call_inside_an_expression() {
    let got = compile("name t;\nGameMode {\n  int x = MyUDF(1);\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x1c, 0x00, 0x01, 0x1b, 0x00, b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'X', 0x00, 0x00, 0x45, 0x15, 0x0d, 0x00, 0x01,
        0x05, 0x00, b'R', 0x01, 0x00, 0x01, 0x04, 0x00, b'B', 0x01,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `array m = {"a"::1};`: a map literal is `ar_Map` (828-834) inside the expression: `X 00 00`,
/// `68 15`, its length counting its own word (2 + 1 + 9 = 12), the count (8 bits: not default
/// parsed), one argument (`AddCallArg` 1001-1016: its length 9, `S 01 00 'a'`, `B 01`, then the
/// pair operator 0x24).
#[test]
fn a_map_literal() {
    let got = compile("name t;\nGameMode {\n  array m = {\"a\"::1};\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x1b, 0x00, 0x01, 0x1a, 0x00, b'V', 0x03, 0x00, 0x00, 0x01, 0x00,
        b'X', 0x00, 0x00, 0x68, 0x15, 0x0c, 0x00, 0x01,
        0x09, 0x00, b'S', 0x01, 0x00, b'a', b'B', 0x01, 0x24,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `int y = a[0];`: a subscript (637-641) is the array, the index, then `[` (0x18).
#[test]
fn a_subscript_read() {
    let got = compile("name t;\nGameMode {\n  array a;\n  int y = a[0];\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x13, 0x00, 0x01, 0x12, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        b'V', 0x03, 0x00, 0x00, 0x01, 0x00, b'B', 0x00, 0x18,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `int a = b = 1;`: an assignment inside an expression (559-567) is its target, its value and
/// its operator (`:=`, 0), with no `Let` of its own; the declaration's `00` follows.
#[test]
fn an_assignment_inside_a_declaration() {
    let got = compile("name t;\nGameMode {\n  int b;\n  int a = b = 1;\n}");
    #[rustfmt::skip]
    let body = [
        0x39, 0x15, 0x13, 0x00, 0x01, 0x12, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        b'V', 0x01, 0x00, 0x00, 0x01, 0x00, b'B', 0x01, 0x00,
        0x00,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `ref f = fn (int p) -> p;`: the lambda's function header with a parameter (`VisitLambdaExpr`
/// 848-913): mode length 12 (the mode, the script length, version 1, the count 1, `p`'s index 2
/// and type 1, no array parameters), its body `SetFunctionValue(p)` (13 bytes) and `End`, so the
/// script length is 17 and the lambda 37 bytes; `SetModLocalData`'s length 1 + 17 + 44 = 62.
#[test]
fn a_lambda_with_a_parameter() {
    let got = compile("name t;\nGameMode {\n  ref f = fn (int p) -> p;\n}");
    let mut body = vec![0x49, 0x15, 0x3e, 0x00, 0x02, 0x11, 0x00, b'S', 0x0c, 0x00];
    body.extend_from_slice(b"__lambda_1_f");
    #[rustfmt::skip]
    body.extend_from_slice(&[
        0x2c, 0x00,
        b'F', 0x25, 0x00, 0x00, 0x00,
        0x1d, 0x00, 0x00, 0x00,
        0x10, 0x00, 0x0c, 0x00, 0x0d, 0x00, 0x11, 0x00, 0x00, 0x00, 0x01, 0x01, 0x02, 0x00, 0x01,
        0x00,
        0x46, 0x15, 0x09, 0x00, 0x01, 0x08, 0x00, b'V', 0x01, 0x00, 0x00, 0x02, 0x00,
        0x11, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `i--` as a statement (607-630): `eval` of `i i 1 - :=` then `1 +` (the old value).
#[test]
fn postfix_decrement() {
    let got = compile("name t;\nGameMode {\n  int i;\n  i--;\n}");
    #[rustfmt::skip]
    let body = [
        0x3a, 0x15, 0x16, 0x00, 0x01,
        0x15, 0x00,
        b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'V', 0x01, 0x00, 0x00, 0x01, 0x00,
        b'B', 0x01, 0x0f, 0x00, b'B', 0x01, 0x0e,
    ];
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// An `if` after a lambda: the lambda's script start is dropped when it ends (909), so the
/// `if`'s target counts from the script's start again: the lambda statement of
/// `a_lambda_variable` is 59 bytes, the `if` (16 bytes) starts at 94, its target is 110.
#[test]
fn an_if_after_a_lambda() {
    let got = compile("name t;\nGameMode {\n  ref f = fn () -> 5;\n  if (1) {}\n}");
    let mut body = vec![0x49, 0x15, 0x37, 0x00, 0x02, 0x11, 0x00, b'S', 0x0c, 0x00];
    body.extend_from_slice(b"__lambda_1_f");
    #[rustfmt::skip]
    body.extend_from_slice(&[
        0x25, 0x00,
        b'F', 0x1e, 0x00, 0x00, 0x00,
        0x1d, 0x00, 0x00, 0x00,
        0x10, 0x00, 0x09, 0x00, 0x0d, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x46, 0x15, 0x05, 0x00, 0x01, 0x04, 0x00, b'B', 0x05,
        0x11, 0x00, 0x00, 0x00,
        0x80, 0x16, 0x0c, 0x00, 0x02, 0x04, 0x00, b'B', 0x01, 0x07, 0x00, b'L', 0x6e, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(body.len(), 59 + 16);
    assert_eq!(got, script(&[&block(0, &body)]));
}

/// `ShowMessage` with variables (714-740): each a type letter (`f` for a float, `s` otherwise)
/// and its index, then the duration. The parser never makes such a statement from text (reading
/// an argument's name is a `std::bad_variant_access` in xNVSE, a crash: the parser's
/// `show_message`), so the tree is made by hand: `ShowMessage(MyMessage, 5)` given the variables
/// `f` (a float, index 1) and `n` (an int, index 2). Its length: the count word, `r` and the
/// reference, the variable count, 3 bytes each, the duration: 2 + 3 + 2 + 6 + 4 = 17.
#[test]
fn show_message_with_variables() {
    use crate::newlang::ast::{Ast, ExprKind, StmtKind, VarId};
    use crate::newlang::{analyze, passes};
    let mut env = TestEnv::default();
    let text = b"name t;\nGameMode {\n  float f;\n  int n;\n  ShowMessage(MyMessage, 5);\n}";
    let mut ast = analyze(&mut env, text, true).unwrap().ast.unwrap();
    let var = |ast: &Ast, name: &[u8]| {
        let i = ast.vars.iter().position(|v| v.original_name == name);
        VarId(i.unwrap() as u32)
    };
    let (f, n) = (var(&ast, b"f"), var(&ast, b"n"));
    let ident = |ast: &mut Ast, name: &[u8], v| {
        let kind = ExprKind::Ident {
            name: name.to_vec(),
            form: 0,
            var: Some(v),
        };
        ast.add_expr(kind, Default::default())
    };
    let (ef, en) = (ident(&mut ast, b"f", f), ident(&mut ast, b"n", n));
    let shown = (0..ast.stmts.len())
        .find(|&i| matches!(ast.stmts[i].kind, StmtKind::ShowMessage { .. }))
        .unwrap();
    if let StmtKind::ShowMessage { args, .. } = &mut ast.stmts[shown].kind {
        *args = vec![ef, en];
    }
    passes::loops::LoopTransformer::new(&mut ast).run().unwrap();
    passes::lambdas::LambdaTransformer::new(&mut ast, &mut env)
        .run()
        .unwrap();
    let mut c = passes::codegen::Compiler::new(&mut ast, &mut env);
    c.run().unwrap();
    #[rustfmt::skip]
    let body = [
        0x08, 0x10, 0x11, 0x00,
        0x01, 0x00, b'r', 0x01, 0x00, 0x02, 0x00, b'f', 0x01, 0x00, b's', 0x02, 0x00,
        0x05, 0x00, 0x00, 0x00,
    ];
    assert_eq!(c.data, script(&[&block(0, &body)]));
}

/// The text of `n` levels: `open` n times, `middle`, `close` n times.
fn nest(open: &str, middle: &str, close: &str, n: usize) -> String {
    let mut s = open.repeat(n);
    s.push_str(middle);
    s.push_str(&close.repeat(n));
    s
}

/// `f` on a thread with a quarter of the stack the game's threads have (16 of 64 MB): the
/// deepest nesting the limit lets through must fit there in a debug build with room to spare.
fn on_quarter_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

/// Whether `text` compiled (`Ok(false)`: an error was reported).
fn compile_result(text: String) -> Result<bool, Fail> {
    let mut env = TestEnv::default();
    compile_with(&mut env, text.as_bytes(), false).map(|c| c.is_some())
}

fn in_block(body: &str) -> String {
    format!("name t;\nGameMode {{\n{body}\n}}")
}

/// Nesting past the limit fails the compile silently (as xNVSE's crashes do) instead of running
/// the native stack out, which would end the game: long chains of `!`, of `-`, parentheses,
/// calls in calls, nested blocks, `if`s, `else if`s, lambdas and interpolated strings, and the
/// chains the parser builds in a loop (`+`, subscripts, member reads, `? :`) that only the
/// tree's measure catches.
#[test]
fn nesting_past_the_limit_fails_the_compile() {
    let n = 100_000;
    let cases: Vec<String> = vec![
        in_block(&format!("  int x = {}1;", "!".repeat(n))),
        in_block(&format!("  int x = {}1;", "- ".repeat(n))),
        in_block(&format!("  int x = {};", nest("(", "1", ")", n))),
        in_block(&format!("  int x = {};", nest("MyUDF(", "1", ")", n))),
        in_block(&nest("{", "", "}", n)),
        in_block(&nest("if (1) {", "", "}", n)),
        in_block(&format!("  if (1) {{}}{}", " else if (1) {}".repeat(n))),
        in_block(&nest("ref f = fn () { ", "", "}; ", n)),
        in_block(&format!(
            "  string s = {};",
            nest("\"${", "\"a\"", "}\"", n)
        )),
        in_block(&format!("  int x = 1{};", "+1".repeat(n))),
        in_block(&format!("  array a;\n  int x = a{};", "[0]".repeat(n))),
        in_block(&format!("  int x = MyQuest{};", ".stage".repeat(n))),
        in_block(&format!("  int x = 1{};", " ? 1 : 1".repeat(n))),
    ];
    for (i, text) in cases.into_iter().enumerate() {
        let r = on_quarter_stack(move || compile_result(text));
        assert_eq!(r, Err(crate::newlang::ast::TOO_DEEP), "case {i}");
    }
}

/// The match pass makes each arm an `else if` one level deeper: a `match` the parse lets through
/// fails once it is a chain, before the passes after it walk it (5000 arms nest 10000 levels,
/// more than a quarter stack holds).
#[test]
fn a_match_too_long_for_its_chain_fails() {
    let arms = |n: usize| {
        in_block(&format!(
            "  match (1) {{ {}_ -> {{}} }}",
            "1 -> {} ".repeat(n)
        ))
    };
    let deep = arms(5000);
    let parsed = on_quarter_stack({
        let deep = deep.clone();
        move || {
            let mut env = TestEnv::default();
            crate::newlang::parser::parse(&mut env, deep.as_bytes()).map(|p| p.ast.is_some())
        }
    });
    assert_eq!(parsed, Ok(true));
    let r = on_quarter_stack(move || compile_result(deep));
    assert_eq!(r, Err(crate::newlang::ast::TOO_DEEP));
    assert_eq!(
        on_quarter_stack(move || compile_result(arms(900))),
        Ok(true)
    );
}

/// Nested lambdas under the parse limit (a statement and an expression a level) compile; the
/// lambda pass makes only the outer one a `SetModLocalData` call (its lambda's own lambda
/// variables stay as they are, as in xNVSE).
#[test]
fn nested_lambdas_under_the_limit_compile() {
    let text = in_block(&nest("ref f = fn () { ", "", "}; ", 250));
    assert_eq!(on_quarter_stack(move || compile_result(text)), Ok(true));
}

/// A user function's parameters read from its text (an old-syntax `Call` of a new-language
/// function) go through the parse and the match pass too: too deep a text gives none, as a
/// failed compile does, and a shallow one its parameters.
#[test]
fn user_function_parameters_of_too_deep_a_text() {
    let arms = "1 -> {} ".repeat(5000);
    let cases = [
        format!("name f;\nfn (int a) {{\n  match (1) {{ {arms}_ -> {{}} }}\n}}"),
        format!(
            "name f;\nfn (int a) {{\n  int x = {};\n}}",
            nest("(", "1", ")", 100_000)
        ),
    ];
    for text in cases {
        let r = on_quarter_stack(move || {
            let mut env = TestEnv::default();
            crate::newlang::user_function_params(&mut env, text.as_bytes())
        });
        assert_eq!(r, None);
    }
    let mut env = TestEnv::default();
    let text = b"name f;\nfn (int a) {\n  match (1) { 1 -> {} _ -> {} }\n}";
    assert_eq!(
        crate::newlang::user_function_params(&mut env, text),
        Some(vec![(1, 1)])
    );
}

/// The passes after the match pass deepen the tree a little without a new measure: the loop
/// pass puts a loop's increment two levels further down (a statement in the `while`'s block).
/// A `for` whose increment is a chain of `+` at the limit (`Begin`, its block, the loop, the
/// assignment, a level per `+`, the first number) is 2050 levels after it, and compiles on the
/// quarter stack.
#[test]
fn a_tree_the_later_passes_deepen_still_compiles() {
    use crate::newlang::ast::MAX_DEPTH;
    let text = |n: usize| {
        let chain = "+1".repeat(n);
        in_block(&format!("  for (int i = 0; i < 1; i = 1{chain}) {{}}"))
    };
    let at_limit = text(MAX_DEPTH - 5);
    assert_eq!(on_quarter_stack(move || compile_result(at_limit)), Ok(true));
    let past = text(MAX_DEPTH - 4);
    let r = on_quarter_stack(move || compile_result(past));
    assert_eq!(r, Err(crate::newlang::ast::TOO_DEEP));
}

/// Nesting just under the limits compiles, on a quarter of a game thread's stack, and one level
/// more fails. The parser counts the statement and the declaration's value, then a level per
/// parenthesis, prefix operator or call argument (so `MAX_PARSE_DEPTH - 2` of them is the
/// most), and a level per block; the lexer a level per string (the outer one and each inside an
/// interpolation, so `MAX_PARSE_DEPTH - 1` interpolations). The tree of a chain of `+` is
/// `Begin`, its block, the declaration, a level per `+` and the first number (`MAX_DEPTH - 4` of
/// them at most). A long `else if` chain compiles too.
#[test]
fn nesting_just_under_the_limit_compiles() {
    use crate::newlang::ast::{MAX_DEPTH, MAX_PARSE_DEPTH, TOO_DEEP};
    let most = MAX_PARSE_DEPTH - 2;
    // The text of n levels, and the most levels that compile.
    type Case = (fn(usize) -> String, usize);
    let cases: [Case; 6] = [
        (
            |n| in_block(&format!("  int x = {};", nest("(", "1", ")", n))),
            most,
        ),
        (
            |n| in_block(&format!("  int x = {}1;", "!".repeat(n))),
            most,
        ),
        (
            |n| in_block(&format!("  int x = {};", nest("MyUDF(", "1", ")", n))),
            most,
        ),
        (
            |n| {
                in_block(&format!(
                    "  string s = {};",
                    nest("\"${", "\"a\"", "}\"", n)
                ))
            },
            MAX_PARSE_DEPTH - 1,
        ),
        (|n| in_block(&nest("{", "", "}", n)), MAX_PARSE_DEPTH),
        (
            |n| in_block(&format!("  int x = 1{};", "+1".repeat(n))),
            MAX_DEPTH - 4,
        ),
    ];
    for (i, (text, most)) in cases.into_iter().enumerate() {
        let r = on_quarter_stack(move || compile_result(text(most)));
        assert_eq!(r, Ok(true), "case {i}");
        let r = on_quarter_stack(move || compile_result(text(most + 1)));
        assert_eq!(r, Err(TOO_DEEP), "case {i}");
    }
    let chain = in_block(&format!("  if (1) {{}}{}", " else if (1) {}".repeat(500)));
    assert_eq!(on_quarter_stack(move || compile_result(chain)), Ok(true));
}

/// Comment lines one after another: xNVSE's lexer calls itself again after each; the port's
/// loops, so many of them compile on an ordinary test thread.
#[test]
fn many_comment_lines_compile() {
    let text = format!("name t;\n{}GameMode {{\n}}", "// c\n".repeat(200_000));
    assert_eq!(compile(&text), script(&[&block(0, &[])]));
}
