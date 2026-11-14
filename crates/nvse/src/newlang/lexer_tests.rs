//! The lexer's tokens, places and errors, from xNVSE 6.4.9's `Lexer.cpp`.

use super::*;

/// Every token of `text` (`GetNextToken(true)`) to the end of file or the first error.
fn lex(text: &str) -> (Vec<Token>, Option<&'static str>) {
    let mut l = Lexer::new(text.as_bytes());
    let mut out = Vec::new();
    loop {
        match l.next_token(true) {
            Ok(t) if t.ty == TokenType::Eof => return (out, None),
            Ok(t) => out.push(t),
            Err(LexFail::Error(e)) => return (out, Some(e.0)),
            Err(LexFail::Crash(Fail::Crash(c))) => return (out, Some(c)),
            Err(LexFail::Crash(_)) => unreachable!(),
        }
    }
}

fn types(text: &str) -> Vec<TokenType> {
    lex(text).0.iter().map(|t| t.ty).collect()
}

fn lexemes(text: &str) -> Vec<String> {
    lex(text)
        .0
        .iter()
        .map(|t| String::from_utf8_lossy(&t.lexeme).into_owned())
        .collect()
}

fn pos(index: usize, line: usize, column: usize) -> SourcePos {
    SourcePos {
        index,
        line,
        column,
    }
}

/// Keywords and type names without case, `bool` an int and `float` a double; `true`, `false`
/// and `null` as values; everything else an identifier.
#[test]
fn keywords_types_and_values() {
    use TokenType::*;
    assert_eq!(
        types("if ELSE While fn return for Name continue break in not match"),
        [If, Else, While, Fn, Return, For, Name, Continue, Break, In, Not, Match]
    );
    assert_eq!(
        types("int bool double float ref string array"),
        [IntType, IntType, DoubleType, DoubleType, RefType, StringType, ArrayType]
    );
    let (toks, _) = lex("true FALSE null foo _bar __x1");
    assert_eq!(toks[0].ty, Bool);
    assert_eq!(toks[0].value, Value::Number(1.0));
    assert_eq!(toks[1].value, Value::Number(0.0));
    assert_eq!(toks[2].ty, Number);
    assert_eq!(toks[2].value, Value::Number(0.0));
    assert_eq!(toks[2].lexeme, b"null");
    assert!(toks[3..].iter().all(|t| t.ty == Identifier));
}

/// Places: an identifier ends at its last column, `true` one past it; operators end at their
/// last column; lines count from 1, tabs are four columns.
#[test]
fn token_places() {
    let (toks, _) = lex("ab\n\tcd += true");
    assert_eq!(
        toks[0].span,
        Span {
            start: pos(0, 1, 1),
            end: pos(2, 1, 2)
        }
    );
    assert_eq!(
        toks[1].span,
        Span {
            start: pos(7, 2, 5),
            end: pos(9, 2, 6)
        }
    );
    assert_eq!(
        toks[2].span,
        Span {
            start: pos(10, 2, 8),
            end: pos(12, 2, 9)
        }
    );
    assert_eq!(
        toks[3].span,
        Span {
            start: pos(13, 2, 11),
            end: pos(17, 2, 15)
        }
    );
    let l = Lexer::new(b"a\r\nb\tc");
    assert_eq!(l.lines, vec![b"a".to_vec(), b"b    c".to_vec()]);
}

/// Every operator, the longest first.
#[test]
fn operators() {
    use TokenType::*;
    assert_eq!(
        types("+= ++ + -= -- -> - *= * /= / %= % ^= ^ == = != ! <= << < >= >> > || |= | ~ && &= & :: : $ # { } [ ] ( ) , ; ? . _"),
        [
            PlusEq, PlusPlus, Plus, MinusEq, MinusMinus, Arrow, Minus, StarEq, Star, SlashEq,
            Slash, ModEq, Mod, PowEq, Pow, EqEq, Eq, BangEq, Bang, LessEq, LeftShift, Less,
            GreaterEq, RightShift, Greater, LogicOr, BitwiseOrEquals, BitwiseOr, BitwiseNot,
            LogicAnd, BitwiseAndEquals, BitwiseAnd, MakePair, Slice, Dollar, Pound, LeftBrace,
            RightBrace, LeftBracket, RightBracket, LeftParen, RightParen, Comma, Semicolon,
            Ternary, Dot, Underscore,
        ]
    );
    assert_eq!(lexemes("a<<=b"), ["a", "<<", "=", "b"]);
    // `_` before a digit is not an identifier.
    assert_eq!(types("_1;"), [Underscore, Number, Semicolon]);
    // A number at the very end of the text is read as an identifier.
    assert_eq!(types("_1"), [Underscore, Identifier]);
}

/// Numbers: decimal with one point, `0x` and `0b` (after any first digit); a number ends at a
/// blank or a punctuation byte. Anything else there is read as an identifier.
#[test]
fn numbers() {
    let n = |t: &str| {
        let (toks, err) = lex(t);
        assert_eq!(err, None, "{t}");
        (
            toks[0].ty,
            toks[0].value.clone(),
            String::from_utf8_lossy(&toks[0].lexeme).into_owned(),
        )
    };
    assert_eq!(
        n("12;"),
        (TokenType::Number, Value::Number(12.0), "12".into())
    );
    assert_eq!(
        n("1.5 "),
        (TokenType::Number, Value::Number(1.5), "1.5".into())
    );
    assert_eq!(
        n("0x1F)"),
        (TokenType::Number, Value::Number(31.0), "0x1F".into())
    );
    assert_eq!(
        n("0b101,"),
        (TokenType::Number, Value::Number(5.0), "0b101".into())
    );
    // `5x10`: base 16 from the `x`, and `stoll` stops at it.
    assert_eq!(
        n("5x10;"),
        (TokenType::Number, Value::Number(5.0), "5x10".into())
    );
    // Hexadecimal letters are taken in a decimal number too: `stod` reads the exponent.
    assert_eq!(
        n("1e3;"),
        (TokenType::Number, Value::Number(1000.0), "1e3".into())
    );
    assert_eq!(
        n("12ab;"),
        (TokenType::Number, Value::Number(12.0), "12ab".into())
    );
    // Not ended by a blank or punctuation: the identifier reader takes it.
    assert_eq!(n("5abg;").0, TokenType::Identifier);
    assert_eq!(lexemes("5abg;"), ["5abg", ";"]);
    assert_eq!(n("7").0, TokenType::Identifier);
    assert_eq!(lexemes("1.;"), ["1", ".", ";"]);
    assert_eq!(lexemes("0b12 "), ["0b12"]);
    assert_eq!(n("0b12 ").0, TokenType::Identifier);
    // The place of a number: its last column.
    let (toks, _) = lex("  42;");
    assert_eq!(
        toks[0].span,
        Span {
            start: pos(2, 1, 3),
            end: pos(3, 1, 4)
        }
    );
    assert_eq!(toks[1].span.start, pos(4, 1, 5));
}

/// Two points: the number is dropped after the reader moved past it, and the byte after it is
/// read next (a blank there is an unexpected character).
#[test]
fn two_points_skip_the_number() {
    assert_eq!(lexemes("1.5.3;x"), [";", "x"]);
    let (toks, err) = lex("1.5.3 x");
    assert!(toks.is_empty());
    assert_eq!(err, Some("Unexpected character"));
}

/// `stoll`/`stod` exceptions nothing catches: no binary digits, past 64 bits, past a double.
#[test]
fn number_conversions_that_throw() {
    assert_eq!(lex("0b ").1, Some("std::stoll: invalid_argument"));
    assert_eq!(
        lex("0x1FFFFFFFFFFFFFFFF ").1,
        Some("std::stoll: out_of_range")
    );
    assert_eq!(lex("1e999 ").1, Some("std::stod: out_of_range"));
}

/// Strings: escapes `\n`, `\\`, `\"`; the lexeme keeps the quotes, the value not; errors.
#[test]
fn strings() {
    let (toks, err) = lex(r#""a\"b\\c\nd" x"#);
    assert_eq!(err, None);
    assert_eq!(toks[0].ty, TokenType::String);
    assert_eq!(toks[0].value, Value::Text(b"a\"b\\c\nd".to_vec()));
    assert_eq!(toks[0].lexeme, b"\"a\"b\\c\nd\"");
    // A string starts at its quote's column with the index after it (`PrevSourcePos`).
    assert_eq!(
        toks[0].span,
        Span {
            start: pos(1, 1, 1),
            end: pos(12, 1, 12)
        }
    );
    assert_eq!(lex(r#""a\q""#).1, Some("Invalid escape sequence."));
    assert_eq!(lex("\"ab\ncd\"").1, Some("Unexpected end of line."));
    assert_eq!(lex("\"abc").1, Some("Unexpected end of input in string."));
    assert_eq!(
        lex("\"abc\\").1,
        Some("Unexpected end of input after escape character.")
    );
}

/// A newline inside a string is an error that does not count the line: later lines are one
/// short.
#[test]
fn a_newline_in_a_string_is_not_counted() {
    let mut l = Lexer::new(b"\"ab\nc\n x");
    assert!(matches!(l.next_token(true), Err(LexFail::Error(_))));
    let t = l.next_token(true).unwrap();
    assert_eq!(t.lexeme, b"c");
    assert_eq!(t.span.start.line, 1);
}

/// Interpolation: the parts and the inner tokens, `${` and the closing `}` as their own tokens.
#[test]
fn interpolation() {
    use TokenType::*;
    let (toks, err) = lex(r#""a${x + 1}b${y}" ;"#);
    assert_eq!(err, None);
    assert_eq!(
        toks.iter().map(|t| t.ty).collect::<Vec<_>>(),
        [
            String, Interp, Identifier, Plus, Number, EndInterp, String, Interp, Identifier,
            EndInterp, String, Semicolon
        ]
    );
    assert_eq!(toks[0].value, Value::Text(b"a".to_vec()));
    assert_eq!(toks[6].value, Value::Text(b"b".to_vec()));
    assert_eq!(toks[10].lexeme, b"\"\"");
    assert_eq!(lex(r#""a${x"#).1, Some("Unexpected end of file."));
    // An escaped backslash before `$` stops the interpolation.
    let (toks, _) = lex(r#""a\\${x}""#);
    assert_eq!(toks[0].value, Value::Text(b"a\\${x}".to_vec()));
}

/// A string inside an interpolation: its own later tokens go on the lexer's stack before the
/// outer string's, so they come out first (xNVSE's order).
#[test]
fn a_string_inside_an_interpolation() {
    use TokenType::*;
    let (toks, _) = lex(r#""a${"b${c}d"}e""#);
    assert_eq!(
        toks.iter().map(|t| t.ty).collect::<Vec<_>>(),
        [String, Interp, Identifier, EndInterp, String, Interp, String, EndInterp, String]
    );
}

/// `//` comments to the end of the line; an unknown byte is an error past which lexing goes on.
#[test]
fn comments_and_unknown_bytes() {
    assert_eq!(lexemes("a // b c\nd"), ["a", "d"]);
    assert_eq!(lexemes("a // b"), ["a"]);
    let mut l = Lexer::new(b"a @ b");
    assert_eq!(l.next_token(true).unwrap().lexeme, b"a");
    assert!(matches!(
        l.next_token(true),
        Err(LexFail::Error(LexError("Unexpected character")))
    ));
    assert_eq!(l.next_token(true).unwrap().lexeme, b"b");
    // Bytes from 0x80 are none of the character classes.
    let mut l = Lexer::new(&[0xe9, b'a']);
    assert!(matches!(l.next_token(true), Err(LexFail::Error(_))));
}

/// `SourceSpan::operator+` as xNVSE has it: the other's END when this span does not start
/// first.
#[test]
fn span_plus_takes_the_others_end() {
    let a = Span {
        start: pos(5, 1, 6),
        end: pos(7, 1, 8),
    };
    let b = Span {
        start: pos(2, 1, 3),
        end: pos(3, 1, 4),
    };
    assert_eq!(
        a.plus(b),
        Span {
            start: pos(3, 1, 4),
            end: pos(7, 1, 8)
        }
    );
    assert_eq!(
        b.plus(a),
        Span {
            start: pos(2, 1, 3),
            end: pos(7, 1, 8)
        }
    );
    assert_eq!(pos(5, 2, 9).minus(2), pos(3, 2, 7));
}

/// A moved-from token keeps its type, place and number, and loses its text.
#[test]
fn moved_from_tokens() {
    let t = Token {
        ty: TokenType::String,
        value: Value::Text(b"x".to_vec()),
        lexeme: b"\"x\"".to_vec(),
        span: Span {
            start: pos(1, 1, 2),
            end: pos(3, 1, 4),
        },
    };
    let m = t.moved_from();
    assert_eq!(m.ty, TokenType::String);
    assert_eq!(m.value, Value::Text(Vec::new()));
    assert!(m.lexeme.is_empty());
    assert_eq!(m.span, t.span);
    let n = Token {
        value: Value::Number(4.0),
        ..t
    };
    assert_eq!(n.moved_from().value, Value::Number(4.0));
}

/// `TokenTypeStr`.
#[test]
fn token_texts() {
    assert_eq!(TokenType::Invalid.text(), "INVALID");
    assert_eq!(TokenType::Negate.text(), "-");
    assert_eq!(TokenType::Eof.text(), "end");
    assert_eq!(TokenType::EndInterp.text(), "}");
    assert_eq!(TokenType::Underscore.text(), "_");
    assert_eq!(TokenType::Box.text(), "Box");
}

/// A bare `0x`: the UCRT passes over the prefix whatever follows, then reads no digit, and
/// `std::stoll` throws.
#[test]
fn a_bare_hex_prefix_throws() {
    assert_eq!(lex("0x;").1, Some("std::stoll: invalid_argument"));
    assert_eq!(lex("0x ").1, Some("std::stoll: invalid_argument"));
    assert_eq!(lex("0x0 ").0[0].value, Value::Number(0.0));
}

/// A decimal that rounds to zero from non-zero digits is the UCRT's underflow (`std::stod`
/// throws); one that rounds to a denormal is kept; a literal zero is zero.
#[test]
fn decimal_underflow() {
    let tiny = format!("0.{}1 ", "0".repeat(400));
    assert_eq!(lex(&tiny).1, Some("std::stod: out_of_range"));
    let denormal = format!("0.{}5 ", "0".repeat(320));
    let (toks, err) = lex(&denormal);
    assert_eq!(err, None);
    let Value::Number(v) = toks[0].value else {
        panic!()
    };
    assert!(v > 0.0 && v < f64::MIN_POSITIVE);
    let zero = format!("0.{} ", "0".repeat(400));
    assert_eq!(lex(&zero).0[0].value, Value::Number(0.0));
}
