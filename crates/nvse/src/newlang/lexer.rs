//! The new script language's lexer (`Compiler/Lexer/Lexer.cpp`, `Lexer.h`, `SourceInfo.h`
//! (xNVSE)): the script's text as tokens with their place (byte index, line, column), the
//! script's lines kept for the error messages.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a). Texts are bytes, as xNVSE's
//! `std::string`s are; the C runtime's character classes are the "C" locale's, where a byte from
//! 0x80 is none of them (MSVC's table answers 0 for a negative `char` below -1).

use super::Fail;

/// `TokenType`, in xNVSE's order (its `TokenTypeStr` is indexed by it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    Invalid,
    // Keywords
    If,
    Else,
    While,
    Fn,
    Return,
    For,
    Name,
    Continue,
    Break,
    Match,
    // Types
    IntType,
    DoubleType,
    RefType,
    StringType,
    ArrayType,
    // Operators
    Plus,
    PlusEq,
    PlusPlus,
    Minus,
    Negate,
    MinusEq,
    MinusMinus,
    Star,
    StarEq,
    Slash,
    SlashEq,
    Mod,
    ModEq,
    Pow,
    PowEq,
    Eq,
    EqEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Bang,
    BangEq,
    LogicOr,
    LogicAnd,
    BitwiseOr,
    BitwiseOrEquals,
    BitwiseAnd,
    BitwiseAndEquals,
    LeftShift,
    RightShift,
    BitwiseNot,
    Dollar,
    Pound,
    Box,
    Unbox,
    // Braces
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    // Literals
    String,
    Number,
    Identifier,
    Bool,
    // Misc
    Comma,
    Semicolon,
    Ternary,
    Slice,
    Not,
    In,
    Eof,
    Dot,
    Interp,
    EndInterp,
    MakePair,
    Arrow,
    Underscore,
}

/// `TokenTypeStr`: each token type's text in the messages.
const TOKEN_TYPE_STR: [&str; 75] = [
    "INVALID",
    "if",
    "else",
    "while",
    "fn",
    "return",
    "for",
    "name",
    "continue",
    "break",
    "match",
    "int",
    "double",
    "ref",
    "string",
    "array",
    "+",
    "+=",
    "++",
    "-",
    "-",
    "-=",
    "--",
    "*",
    "*=",
    "/",
    "/=",
    "%",
    "%=",
    "^",
    "^=",
    "=",
    "==",
    "<",
    ">",
    "<=",
    ">=",
    "!",
    "!=",
    "||",
    "&&",
    "|",
    "|=",
    "&",
    "&=",
    "<<",
    ">>",
    "~",
    "$",
    "#",
    "Box",
    "Unbox",
    "{",
    "}",
    "[",
    "]",
    "(",
    ")",
    "String",
    "Number",
    "Identifier",
    "Bool",
    ",",
    ";",
    "?",
    ":",
    "not",
    "in",
    "end",
    ".",
    "${",
    "}",
    "::",
    "->",
    "_",
];

impl TokenType {
    /// `TokenTypeStr[type]`.
    pub fn text(self) -> &'static str {
        TOKEN_TYPE_STR[self as usize]
    }
}

/// `SourcePos`: a byte index into the lexer's text (tabs made four blanks, `\r\n` made `\n`),
/// a line from 1 and a column from 1 (0: no place).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourcePos {
    pub index: usize,
    pub line: usize,
    pub column: usize,
}

impl SourcePos {
    /// `SourcePos::operator-` (the index and the column, `size_t` arithmetic).
    pub fn minus(self, amount: usize) -> SourcePos {
        SourcePos {
            index: self.index.wrapping_sub(amount),
            line: self.line,
            column: self.column.wrapping_sub(amount),
        }
    }
}

/// `SourceSpan`: where a token, expression or statement starts and ends (both inclusive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: SourcePos,
    pub end: SourcePos,
}

impl Span {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), SourceInfo.h: SourceSpan::operator+.
    /// `SourceSpan::operator+`, as xNVSE has it: the start is this span's start when it lies
    /// before the other's start, else the other's END; the end is the later end.
    pub fn plus(self, other: Span) -> Span {
        let start = if self.start.index < other.start.index {
            self.start
        } else {
            other.end
        };
        let end = if other.end.index > self.end.index {
            other.end
        } else {
            self.end
        };
        Span { start, end }
    }
}

/// A token's value (`std::variant<std::monostate, double, std::string>`).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    None,
    Number(f64),
    Text(Vec<u8>),
}

/// `Token`.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub ty: TokenType,
    pub value: Value,
    pub lexeme: Vec<u8>,
    pub span: Span,
}

impl Default for Token {
    /// `Token()`: an end of file with no place.
    fn default() -> Token {
        Token::bare(TokenType::Eof, b"")
    }
}

impl Token {
    /// `Token(type)` and `Token(type, lexeme)`: no value, no place.
    pub fn bare(ty: TokenType, lexeme: &[u8]) -> Token {
        Token {
            ty,
            value: Value::None,
            lexeme: lexeme.to_vec(),
            span: Span::default(),
        }
    }

    fn at(ty: TokenType, lexeme: Vec<u8>, start: SourcePos, end: SourcePos, value: Value) -> Token {
        Token {
            ty,
            value,
            lexeme,
            span: Span { start, end },
        }
    }

    /// The number a token holds (`std::get<double>`), `None` for another alternative.
    pub fn number(&self) -> Option<f64> {
        match self.value {
            Value::Number(v) => Some(v),
            _ => None,
        }
    }

    /// What is left of this token after it was moved from (`previousToken = std::move
    /// (currentToken)`): its type and place, its lexeme emptied, a text value emptied (MSVC's
    /// `std::string` move leaves the source empty), a number kept.
    pub fn moved_from(&self) -> Token {
        Token {
            ty: self.ty,
            value: match self.value {
                Value::Text(_) => Value::Text(Vec::new()),
                ref v => v.clone(),
            },
            lexeme: Vec::new(),
            span: self.span,
        }
    }
}

/// A lexer error (`std::runtime_error`): its text.
#[derive(Debug, PartialEq, Eq)]
pub struct LexError(pub &'static str);

/// What a lexing step can end in: a token, a lexer error the parser reports, or a C++ exception
/// nothing catches (xNVSE stops the game).
#[derive(Debug)]
pub enum LexFail {
    Error(LexError),
    Crash(Fail),
}

impl From<LexError> for LexFail {
    fn from(e: LexError) -> LexFail {
        LexFail::Error(e)
    }
}

// The C runtime's character classes in the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, 0x09..=0x0d | b' ')
}
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}
fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

/// `Lexer`.
pub struct Lexer {
    input: Vec<u8>,
    pos: usize,
    /// For string interpolation: tokens lexed ahead (`tokenStack`).
    stack: std::collections::VecDeque<Token>,
    pub column: usize,
    pub line: usize,
    pub lines: Vec<Vec<u8>>,
}

fn replace_all(text: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

impl Lexer {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Lexer.cpp: Lexer::Lexer.
    /// The text with every tab made four blanks and every `\r\n` made `\n`, and its lines.
    pub fn new(text: &[u8]) -> Lexer {
        let tabs = replace_all(text, b"\t", b"    ");
        let input = replace_all(&tabs, b"\r\n", b"\n");
        let lines = input.split(|&c| c == b'\n').map(<[u8]>::to_vec).collect();
        Lexer {
            input,
            pos: 0,
            stack: std::collections::VecDeque::new(),
            column: 1,
            line: 1,
            lines,
        }
    }

    /// `input[i]`: a `std::string` reads its NUL at `size()`; past it nothing here reads.
    fn at(&self, i: usize) -> u8 {
        self.input.get(i).copied().unwrap_or(0)
    }

    fn advance_char(&mut self) -> usize {
        self.column += 1;
        self.pos += 1;
        self.pos - 1
    }

    pub fn cur_pos(&self) -> SourcePos {
        SourcePos {
            index: self.pos,
            line: self.line,
            column: self.column,
        }
    }

    pub fn prev_pos(&self) -> SourcePos {
        SourcePos {
            index: self.pos,
            line: self.line,
            column: self.column.wrapping_sub(1),
        }
    }

    /// `GetSourceText(startPos)`: from `start` to the current place.
    fn text_from(&self, start: SourcePos) -> Vec<u8> {
        self.input[start.index..self.pos].to_vec()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Lexer.cpp: Lexer::Match.
    /// `Match(std::string_view)`: the text at the place, then past it (the column too).
    fn matches(&mut self, s: &[u8]) -> bool {
        if self.pos >= self.input.len() {
            return false;
        }
        if self.pos + s.len() > self.input.len() {
            return false;
        }
        if &self.input[self.pos..self.pos + s.len()] != s {
            return false;
        }
        self.column += s.len();
        self.pos += s.len();
        true
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Lexer.cpp: Lexer::lexString.
    /// `lexString`, after the opening quote: the string's tokens, with an interpolation's
    /// (`${ ... }`) tokens between its parts.
    fn lex_string(&mut self) -> Result<Vec<Token>, LexFail> {
        let mut text: Vec<u8> = Vec::new();
        let mut start_pos = self.prev_pos();
        let start = self.pos - 1;
        let start_line = self.line;
        let mut results = Vec::new();
        while self.pos < self.input.len() {
            let at = self.advance_char();
            let c = self.input[at];
            if c == b'\\' {
                if self.pos >= self.input.len() {
                    return Err(LexError("Unexpected end of input after escape character.").into());
                }
                let at = self.advance_char();
                let next = self.input[at];
                match next {
                    b'n' => text.push(b'\n'),
                    b'\\' | b'"' => text.push(next),
                    _ => return Err(LexError("Invalid escape sequence.").into()),
                }
            } else if c == b'\n' {
                return Err(LexError("Unexpected end of line.").into());
            } else if c == b'"' {
                if self.line != start_line {
                    return Err(LexError("Multiline strings are not allowed.").into());
                }
                let lexeme = quoted(&text);
                results.push(Token::at(
                    TokenType::String,
                    lexeme,
                    start_pos,
                    self.prev_pos(),
                    Value::Text(text),
                ));
                return Ok(results);
            } else if c == b'$'
                && self.pos < self.input.len()
                && self.input[self.pos] == b'{'
                && (self.pos == start + 1 || self.input[self.pos - 2] != b'\\')
            {
                let lexeme = quoted(&text);
                results.push(Token::at(
                    TokenType::String,
                    lexeme,
                    start_pos,
                    self.cur_pos().minus(2),
                    Value::Text(std::mem::take(&mut text)),
                ));
                results.push(Token::at(
                    TokenType::Interp,
                    b"${".to_vec(),
                    self.prev_pos(),
                    self.cur_pos(),
                    Value::None,
                ));
                self.advance_char();
                loop {
                    let tok = self.next_token(false)?;
                    if tok.ty == TokenType::RightBrace {
                        break;
                    }
                    if tok.ty == TokenType::Eof {
                        return Err(LexError("Unexpected end of file.").into());
                    }
                    results.push(tok);
                }
                results.push(Token::at(
                    TokenType::EndInterp,
                    b"}".to_vec(),
                    self.prev_pos(),
                    self.prev_pos(),
                    Value::None,
                ));
                start_pos = self.cur_pos();
            } else {
                text.push(c);
            }
        }
        Err(LexError("Unexpected end of input in string.").into())
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Lexer.cpp: Lexer::GetNextToken.
    /// `GetNextToken(useStack)`: the next token (one lexed ahead first when `use_stack`).
    pub fn next_token(&mut self, use_stack: bool) -> Result<Token, LexFail> {
        if use_stack {
            if let Some(tok) = self.stack.pop_front() {
                return Ok(tok);
            }
        }
        // Blanks.
        while is_space(self.at(self.pos)) {
            if self.input[self.pos] == b'\n' {
                self.column = 1;
                self.line += 1;
            } else {
                self.column += 1;
            }
            self.pos += 1;
            if self.pos >= self.input.len() {
                break;
            }
        }
        // A `//` comment, to the end of its line.
        if self.matches(b"//") {
            while self.pos < self.input.len() {
                if self.input[self.pos] == b'\n' {
                    self.column = 1;
                    self.line += 1;
                    self.pos += 1;
                    return self.next_token(use_stack);
                }
                self.pos += 1;
                if self.pos >= self.input.len() {
                    break;
                }
            }
        }
        if self.pos >= self.input.len() {
            return Ok(Token::bare(TokenType::Eof, b""));
        }
        let current = self.input[self.pos];
        if is_digit(current) {
            if let Some(tok) = self.number()? {
                return Ok(tok);
            }
        }
        let current = self.input[self.pos];
        // An identifier has a letter or digit first, or underscores before a letter.
        let mut valid_identifier = is_alnum(current);
        if !valid_identifier && current == b'_' {
            let mut look = self.pos + 1;
            while look < self.input.len() {
                if is_alpha(self.input[look]) {
                    valid_identifier = true;
                    break;
                } else if self.input[look] != b'_' {
                    break;
                }
                look += 1;
            }
        }
        if valid_identifier {
            return Ok(self.identifier());
        }
        if current == b'"' {
            self.advance_char();
            let mut results = self.lex_string()?;
            let tok = results.remove(0);
            self.stack.extend(results);
            return Ok(tok);
        }
        let start = self.cur_pos();
        use TokenType as T;
        #[rustfmt::skip]
        const OPERATORS: [(&[u8], TokenType); 46] = [
            (b"+=", T::PlusEq), (b"++", T::PlusPlus), (b"+", T::Plus),
            (b"-=", T::MinusEq), (b"--", T::MinusMinus), (b"->", T::Arrow), (b"-", T::Minus),
            (b"*=", T::StarEq), (b"*", T::Star),
            (b"/=", T::SlashEq), (b"/", T::Slash),
            (b"%=", T::ModEq), (b"%", T::Mod),
            (b"^=", T::PowEq), (b"^", T::Pow),
            (b"==", T::EqEq), (b"=", T::Eq),
            (b"!=", T::BangEq), (b"!", T::Bang),
            (b"<=", T::LessEq), (b"<<", T::LeftShift), (b"<", T::Less),
            (b">=", T::GreaterEq), (b">>", T::RightShift), (b">", T::Greater),
            (b"||", T::LogicOr), (b"|=", T::BitwiseOrEquals), (b"|", T::BitwiseOr),
            (b"~", T::BitwiseNot),
            (b"&&", T::LogicAnd), (b"&=", T::BitwiseAndEquals), (b"&", T::BitwiseAnd),
            (b"::", T::MakePair), (b":", T::Slice),
            (b"$", T::Dollar), (b"#", T::Pound),
            (b"{", T::LeftBrace), (b"}", T::RightBrace), (b"[", T::LeftBracket),
            (b"]", T::RightBracket), (b"(", T::LeftParen), (b")", T::RightParen),
            (b",", T::Comma), (b";", T::Semicolon), (b"?", T::Ternary), (b".", T::Dot),
        ];
        let mut ty = T::Invalid;
        for (text, t) in OPERATORS {
            if self.matches(text) {
                ty = t;
                break;
            }
        }
        if ty == T::Invalid && self.matches(b"_") {
            ty = T::Underscore;
        }
        if ty == T::Invalid {
            self.pos += 1;
            return Err(LexError("Unexpected character").into());
        }
        let lexeme = self.text_from(start);
        Ok(Token::at(ty, lexeme, start, self.prev_pos(), Value::None))
    }

    /// The identifier at the place: `true`, `false` and `null` as values, the keywords and type
    /// names (without case), else an identifier.
    fn identifier(&mut self) -> Token {
        let start_pos = self.cur_pos();
        let start = self.pos;
        while self.pos < self.input.len()
            && (is_alnum(self.input[self.pos]) || self.input[self.pos] == b'_')
        {
            self.advance_char();
        }
        let word = self.input[start..self.pos].to_vec();
        let is = |w: &[u8]| word.eq_ignore_ascii_case(w);
        if is(b"true") {
            return Token::at(
                TokenType::Bool,
                word,
                start_pos,
                self.cur_pos(),
                Value::Number(1.0),
            );
        }
        if is(b"false") {
            return Token::at(
                TokenType::Bool,
                word,
                start_pos,
                self.cur_pos(),
                Value::Number(0.0),
            );
        }
        if is(b"null") {
            return Token::at(
                TokenType::Number,
                word,
                start_pos,
                self.cur_pos(),
                Value::Number(0.0),
            );
        }
        use TokenType as T;
        const KEYWORDS: [(&[u8], TokenType); 19] = [
            (b"if", T::If),
            (b"else", T::Else),
            (b"while", T::While),
            (b"fn", T::Fn),
            (b"return", T::Return),
            (b"for", T::For),
            (b"name", T::Name),
            (b"continue", T::Continue),
            (b"break", T::Break),
            (b"in", T::In),
            (b"not", T::Not),
            (b"match", T::Match),
            (b"int", T::IntType),
            (b"bool", T::IntType),
            (b"double", T::DoubleType),
            (b"float", T::DoubleType),
            (b"ref", T::RefType),
            (b"string", T::StringType),
            (b"array", T::ArrayType),
        ];
        let ty = KEYWORDS
            .iter()
            .find(|(w, _)| is(w))
            .map_or(T::Identifier, |&(_, t)| t);
        Token::at(ty, word, start_pos, self.prev_pos(), Value::None)
    }

    /// A number at the place (a digit first): decimal with at most one point, `0x` hexadecimal
    /// or `0b` binary (any first digit before the `x` or `b`). `None` when the text there is not
    /// a number that ends before a blank or a punctuation byte; the identifier reader then reads
    /// it (`5abc`, `1.`, a number at the very end of the text).
    fn number(&mut self) -> Result<Option<Token>, LexFail> {
        let start = self.cur_pos();
        let mut base = 10u32;
        let mut cur = self.pos + 1;
        if self.at(cur) == b'b' {
            base = 2;
            cur += 1;
        } else if self.at(cur) == b'x' {
            base = 16;
            cur += 1;
        }
        let mut decimals = 0;
        let mut view: Option<usize> = None;
        while cur < self.input.len() {
            let c = self.input[cur];
            let len = cur - start.index;
            if is_digit(c) {
                if base == 2 && c != b'0' && c != b'1' {
                    break;
                }
                cur += 1;
                continue;
            }
            if c == b'.' {
                decimals += 1;
                if base != 10 || cur + 1 >= self.input.len() {
                    break;
                }
                if is_digit(self.input[cur + 1]) {
                    cur += 2;
                    continue;
                }
                break;
            }
            if c == b' ' {
                view = Some(len);
                self.pos += len;
                self.column += len;
                break;
            }
            if is_alpha(c) {
                if (b'a'..=b'f').contains(&c.to_ascii_lowercase()) {
                    cur += 1;
                    continue;
                }
                break;
            }
            view = Some(len);
            self.pos += len;
            self.column += len;
            break;
        }
        if decimals > 1 {
            view = None;
        }
        let Some(len) = view.filter(|&l| l > 0) else {
            return Ok(None);
        };
        let text = self.input[start.index..start.index + len].to_vec();
        let end = SourcePos {
            index: start.index + len - 1,
            line: start.line,
            column: start.column + len - 1,
        };
        let value = match base {
            16 => stoll(&text, 16)? as f64,
            2 => stoll(&text[2..], 2)? as f64,
            _ => stod(&text)?,
        };
        Ok(Some(Token::at(
            TokenType::Number,
            text,
            start,
            end,
            Value::Number(value),
        )))
    }
}

/// `'"' + text + '"'`.
fn quoted(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 2);
    out.push(b'"');
    out.extend_from_slice(text);
    out.push(b'"');
    out
}

/// `std::stoll(text, nullptr, base)`: the longest number at the start (an `0x` before
/// hexadecimal digits allowed); no digits (`std::invalid_argument`) or a value past 64 bits
/// (`std::out_of_range`) is an exception nothing catches.
fn stoll(text: &[u8], base: u32) -> Result<i64, LexFail> {
    let mut digits = text;
    if base == 16
        && digits.len() > 2
        && digits[0] == b'0'
        && (digits[1] | 0x20) == b'x'
        && (digits[2] as char).is_ascii_hexdigit()
    {
        digits = &digits[2..];
    }
    let n = digits
        .iter()
        .take_while(|&&c| (c as char).is_digit(base))
        .count();
    if n == 0 {
        return Err(LexFail::Crash(Fail::Crash("std::stoll: invalid_argument")));
    }
    let s = std::str::from_utf8(&digits[..n]).unwrap_or("0");
    i64::from_str_radix(s, base)
        .map_err(|_| LexFail::Crash(Fail::Crash("std::stoll: out_of_range")))
}

/// `std::stod(text)`: the longest decimal number at the start (digits, a point and digits, an
/// exponent); a value past a double's range (`std::out_of_range`) is an exception nothing
/// catches.
fn stod(text: &[u8]) -> Result<f64, LexFail> {
    let mut i = text.iter().take_while(|c| c.is_ascii_digit()).count();
    if text.get(i) == Some(&b'.') {
        i += 1;
        i += text[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    }
    if matches!(text.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(text.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let digits = text[j.min(text.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digits > 0 {
            i = j + digits;
        }
    }
    let s = std::str::from_utf8(&text[..i]).unwrap_or("0");
    let v: f64 = s.parse().unwrap_or(0.0);
    if v.is_infinite() {
        return Err(LexFail::Crash(Fail::Crash("std::stod: out_of_range")));
    }
    Ok(v)
}

#[cfg(test)]
#[path = "lexer_tests.rs"]
mod tests;
