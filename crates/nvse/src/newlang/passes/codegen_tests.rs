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
