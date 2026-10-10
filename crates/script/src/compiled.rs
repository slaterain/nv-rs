//! The compiled form of a script (`SCDA`): what the game actually runs.
//! It's a list of statements, each a 2-byte code and a 2-byte length
//! followed by that many bytes, except that a call on a reference is
//! preceded by a 4-byte marker (code `0x1C` and the reference's number in
//! the record's `SCRO` list, counting from 1). Codes from `0x1000` are
//! function calls (`0x1000` + the function's number); the ones below are
//! the language's keywords.
//!
//! The source is what [`crate::parse`] reads; this is for checking what
//! the game's own compiler made of source that's oddly written.

/// One compiled statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement<'a> {
    /// Where it starts in the compiled bytes.
    pub offset: usize,
    /// For a call on a reference: its number in the `SCRO` list (from 1).
    pub on: Option<u16>,
    pub code: u16,
    pub data: &'a [u8],
}

const REFERENCE_PREFIX: u16 = 0x1C;

/// Splits compiled bytes into statements. Stops at the end or at bytes
/// that don't fit (a length past the end).
pub fn statements(bytes: &[u8]) -> Vec<Statement<'_>> {
    let mut out = Vec::new();
    let mut at = 0;
    let u16_at = |i: usize| -> Option<u16> {
        bytes
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    while let Some(mut code) = u16_at(at) {
        let offset = at;
        let mut on = None;
        if code == REFERENCE_PREFIX {
            on = u16_at(at + 2);
            at += 4;
            match u16_at(at) {
                Some(c) => code = c,
                None => break,
            }
        }
        let Some(len) = u16_at(at + 2) else { break };
        let start = at + 4;
        let end = start + usize::from(len);
        if end > bytes.len() {
            break;
        }
        out.push(Statement {
            offset,
            on,
            code,
            data: &bytes[start..end],
        });
        at = end;
    }
    out
}

/// A token of a compiled expression. Expressions are stored in evaluation
/// order (operands before their operator), each token after a space.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// A number or an operator, as text (`110`, `0.5`, `==`, `&&`).
    Text(&'a str),
    /// A local variable: `s` (whole number) or `f` (float), and its index.
    Local(u8, u16),
    /// A global variable, by `SCRO` number.
    Global(u16),
    /// A reference as a value, by `SCRO` number (stored `Z`, or `r` with
    /// nothing after it).
    Ref(u16),
    /// A reference owning the variable (`Local`) or receiving the call
    /// (`Call`) that follows.
    Owner(u16),
    /// A function call: its code and its parameter bytes (which start with
    /// the parameter count).
    Call(u16, &'a [u8]),
}

/// The tokens of a compiled expression, or `None` if the bytes don't read.
pub fn expression(bytes: &[u8]) -> Option<Vec<Token<'_>>> {
    let mut out = Vec::new();
    let mut at = 0;
    let u16_at = |i: usize| -> Option<u16> {
        bytes
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    // Whether the previous token joins the next one (a reference before
    // its variable or call).
    let mut joined = false;
    while at < bytes.len() {
        if bytes[at] == b' ' {
            at += 1;
        } else if !joined {
            return None;
        }
        joined = false;
        match *bytes.get(at)? {
            b'X' => {
                let code = u16_at(at + 1)?;
                let len = usize::from(u16_at(at + 3)?);
                let params = bytes.get(at + 5..at + 5 + len)?;
                out.push(Token::Call(code, params));
                at += 5 + len;
            }
            b'r' => {
                let r = u16_at(at + 1)?;
                at += 3;
                joined = bytes.get(at).is_some_and(|&b| b != b' ');
                out.push(if joined {
                    Token::Owner(r)
                } else {
                    Token::Ref(r)
                });
            }
            c @ (b's' | b'f') => {
                out.push(Token::Local(c, u16_at(at + 1)?));
                at += 3;
            }
            b'G' => {
                out.push(Token::Global(u16_at(at + 1)?));
                at += 3;
            }
            b'Z' => {
                out.push(Token::Ref(u16_at(at + 1)?));
                at += 3;
            }
            _ => {
                let end = bytes[at..]
                    .iter()
                    .position(|&b| b == b' ')
                    .map_or(bytes.len(), |p| at + p);
                out.push(Token::Text(std::str::from_utf8(&bytes[at..end]).ok()?));
                at = end;
            }
        }
    }
    Some(out)
}

/// An expression's tokens as text: `r1.GetDead() 1 ==`.
pub fn expression_text(tokens: &[Token]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut owner: Option<u16> = None;
    for t in tokens {
        let prefix = owner.take().map(|r| format!("r{r}.")).unwrap_or_default();
        parts.push(match t {
            Token::Text(s) => s.to_string(),
            Token::Local(k, i) => format!("{prefix}{}{i}", *k as char),
            Token::Global(i) => format!("g{i}"),
            Token::Ref(r) => format!("r{r}"),
            Token::Owner(r) => {
                owner = Some(*r);
                continue;
            }
            Token::Call(code, params) => {
                // The parameters after their count, as bytes.
                let bytes: Vec<String> =
                    params.iter().skip(2).map(|b| format!("{b:02X}")).collect();
                format!("{prefix}{}({})", code_name(*code), bytes.join(" "))
            }
        });
    }
    parts.join(" ")
}

/// An expression's shape: `o` for each value, operators as stored
/// (`o o == o &&`). For comparing with a parsed expression.
pub fn expression_shape(tokens: &[Token]) -> String {
    tokens
        .iter()
        .filter_map(|t| match t {
            Token::Owner(_) => None,
            Token::Text(s) if s.parse::<f64>().is_err() => Some(s.to_string()),
            _ => Some("o".to_string()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Where a `set`'s value starts: after its target, a local (3 bytes) or a
/// reference's variable (6).
fn set_target_len(data: &[u8]) -> usize {
    if data.first() == Some(&b'r') {
        6
    } else {
        3
    }
}

/// The expression an `if` or `elseif` tests, or the value a `set` stores
/// (each kept after a 2-byte length; an `if`'s starts with its 2-byte jump).
pub fn statement_expression<'a>(s: &Statement<'a>) -> Option<Vec<Token<'a>>> {
    let start = match s.code {
        0x16 | 0x18 => 2,
        0x15 => set_target_len(s.data),
        _ => return None,
    };
    let len = usize::from(u16::from_le_bytes([
        *s.data.get(start)?,
        *s.data.get(start + 1)?,
    ]));
    expression(s.data.get(start + 2..start + 2 + len)?)
}

/// A function call in compiled code: a statement of its own or part of an
/// `if`'s, `elseif`'s or `set`'s expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call<'a> {
    /// The function's number (its code minus 0x1000).
    pub function: u16,
    /// Called on a reference: its number in the `SCRO`/`SCRV` list (from 1).
    pub on: Option<u16>,
    /// The parameter bytes: the parameter count, then each parameter
    /// (empty for a statement without parameters).
    pub params: &'a [u8],
}

/// Every function call in compiled bytes, in the order they're stored.
pub fn calls(bytes: &[u8]) -> Vec<Call<'_>> {
    let mut out = Vec::new();
    for s in statements(bytes) {
        if s.code >= 0x1000 {
            out.push(Call {
                function: s.code - 0x1000,
                on: s.on,
                params: s.data,
            });
            continue;
        }
        let mut owner = None;
        for t in statement_expression(&s).unwrap_or_default() {
            match t {
                Token::Owner(r) => owner = Some(r),
                Token::Call(code, params) if code >= 0x1000 => out.push(Call {
                    function: code - 0x1000,
                    on: owner.take(),
                    params,
                }),
                _ => owner = None,
            }
        }
    }
    out
}

/// One compiled parameter of a call.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// A record, by `SCRO`/`SCRV` number (from 1): stored `r` and the number.
    Ref(u16),
    /// A whole number (`n`).
    Int(i32),
    /// A float (`z`).
    Float(f64),
    /// A local variable (`s` or `f` and its index).
    Local(u16),
    /// A global variable, by `SCRO` number (`G`).
    Global(u16),
    /// A reference's variable: the reference's number and the variable's
    /// index (`r`, then `s` or `f`).
    RefVariable(u16, u16),
    /// A string: its bytes.
    Text(&'a [u8]),
    /// A number from a fixed list (actor value, animation group, sex, ...),
    /// stored as 2 bytes.
    Code(u16),
    /// An axis letter (`X`, `Y` or `Z`).
    Axis(u8),
}

/// A call's parameters, read by the kinds its signature gives (as the
/// game's compiled scripts store them: numbers and stages with a type
/// letter, records as `r` and their number, strings after their length,
/// axes as one letter, other lists as 2 bytes). Stops at the first one
/// that doesn't read (and at the count stored), so it may return fewer
/// than the call has. Calls that keep more after their parameters
/// (`ShowMessage`'s variables) keep it out of the list.
pub fn parameters<'a>(function: u16, params: &'a [u8]) -> Vec<Value<'a>> {
    let mut out = Vec::new();
    let Some(sig) = crate::functions::FUNCTIONS.get(usize::from(function)) else {
        return out;
    };
    let u16_at = |i: usize| -> Option<u16> {
        params
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let Some(count) = u16_at(0) else { return out };
    let mut at = 2;
    for p in sig.params.iter().take(usize::from(count)) {
        let numeric = matches!(p.kind, 1 | 2 | 23);
        let read = || -> Option<(Value<'a>, usize)> {
            if p.kind == 0 {
                let len = usize::from(u16_at(at)?);
                return Some((Value::Text(params.get(at + 2..at + 2 + len)?), 2 + len));
            }
            if p.kind == 8 {
                return Some((Value::Axis(*params.get(at)?), 1));
            }
            if !numeric && !crate::param_is_form(p.kind) {
                return Some((Value::Code(u16_at(at)?), 2));
            }
            match *params.get(at)? {
                b'n' => {
                    let b = params.get(at + 1..at + 5)?;
                    Some((Value::Int(i32::from_le_bytes([b[0], b[1], b[2], b[3]])), 5))
                }
                b'z' => {
                    let b: [u8; 8] = params.get(at + 1..at + 9)?.try_into().ok()?;
                    Some((Value::Float(f64::from_le_bytes(b)), 9))
                }
                b's' | b'f' => Some((Value::Local(u16_at(at + 1)?), 3)),
                b'G' => Some((Value::Global(u16_at(at + 1)?), 3)),
                b'r' => {
                    let r = u16_at(at + 1)?;
                    match params.get(at + 3) {
                        Some(b's' | b'f') if numeric => {
                            Some((Value::RefVariable(r, u16_at(at + 4)?), 6))
                        }
                        _ => Some((Value::Ref(r), 3)),
                    }
                }
                _ => None,
            }
        };
        let Some((value, len)) = read() else { break };
        out.push(value);
        at += len;
    }
    out
}

/// The keyword or function a statement code stands for.
pub fn code_name(code: u16) -> String {
    match code {
        0x10 => "Begin".into(),
        0x11 => "End".into(),
        0x15 => "Set".into(),
        0x16 => "If".into(),
        0x17 => "Else".into(),
        0x18 => "ElseIf".into(),
        0x19 => "EndIf".into(),
        0x1D => "ScriptName".into(),
        0x1E => "Return".into(),
        c if c >= 0x1000 => crate::function_name(c - 0x1000),
        c => format!("code {c:#x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_statements_and_reference_calls() {
        // ScriptName; Begin GameMode (block of 12 bytes); Player(ref 1)
        // .Disable with no parameters; End.
        let bytes = [
            0x1D, 0x00, 0x00, 0x00, // ScriptName
            0x10, 0x00, 0x06, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, // Begin
            0x1C, 0x00, 0x01, 0x00, 0x22, 0x10, 0x00, 0x00, // 1.Disable
            0x11, 0x00, 0x00, 0x00, // End
        ];
        let s = statements(&bytes);
        assert_eq!(s.len(), 4);
        assert_eq!(code_name(s[0].code), "ScriptName");
        assert_eq!(s[1].data.len(), 6);
        assert_eq!(s[2].on, Some(1));
        assert_eq!(s[2].offset, 14);
        assert_eq!(code_name(s[2].code), crate::function_name(0x22));
        assert_eq!(code_name(s[3].code), "End");
    }

    #[test]
    fn reads_expressions() {
        // `GetStage VMQYesMan01a 110 != 1` as the game stores it, then a
        // reference's variable and a bare reference.
        let mut bytes = vec![
            b' ', b'X', 0x3A, 0x10, 0x05, 0x00, 0x01, 0x00, b'r', 0x01, 0x00,
        ];
        bytes.extend_from_slice(b" 110 1 != ");
        bytes.extend_from_slice(&[b'r', 0x02, 0x00, b's', 0x04, 0x00, b' ', b'r', 0x03, 0x00]);
        let t = expression(&bytes).unwrap();
        assert_eq!(t.len(), 7);
        assert_eq!(t[0], Token::Call(0x103A, &[0x01, 0x00, b'r', 0x01, 0x00]));
        assert_eq!(t[3], Token::Text("!="));
        assert_eq!(t[4], Token::Owner(2));
        assert_eq!(t[6], Token::Ref(3));
        assert_eq!(
            expression_text(&t),
            format!("{}(72 01 00) 110 1 != r2.s4 r3", crate::function_name(0x3A))
        );
        assert_eq!(expression_shape(&t), "o o o != o o");
    }

    fn number(name: &str) -> u16 {
        crate::function(name).unwrap().0
    }

    #[test]
    fn finds_calls_in_statements_and_expressions() {
        let code = |name: &str| (0x1000 + number(name)).to_le_bytes();
        let mut bytes = Vec::new();
        // SetStage (quest = reference 1) 110
        bytes.extend(code("SetStage"));
        bytes.extend([10, 0, 2, 0, b'r', 1, 0, b'n', 110, 0, 0, 0]);
        // 2.MoveTo (reference 3)
        bytes.extend([0x1C, 0x00, 0x02, 0x00]);
        bytes.extend(code("MoveTo"));
        bytes.extend([5, 0, 1, 0, b'r', 3, 0]);
        // if GetStage (reference 4) >= 10
        let mut e = vec![b' ', b'X'];
        e.extend(code("GetStage"));
        e.extend([5, 0, 1, 0, b'r', 4, 0]);
        e.extend(b" 10 >=");
        bytes.extend([0x16, 0x00]);
        bytes.extend(((e.len() + 4) as u16).to_le_bytes());
        bytes.extend([0, 0]); // the jump
        bytes.extend((e.len() as u16).to_le_bytes());
        bytes.extend(&e);
        // set local 1 to 5.GetLocked
        let mut e = vec![b' ', b'r', 5, 0, b'X'];
        e.extend(code("GetLocked"));
        e.extend([2, 0, 0, 0]);
        bytes.extend([0x15, 0x00]);
        bytes.extend(((e.len() + 5) as u16).to_le_bytes());
        bytes.extend([b's', 1, 0]);
        bytes.extend((e.len() as u16).to_le_bytes());
        bytes.extend(&e);

        let c = calls(&bytes);
        let names: Vec<(String, Option<u16>)> = c
            .iter()
            .map(|c| (crate::function_name(c.function), c.on))
            .collect();
        assert_eq!(
            names,
            vec![
                ("SetStage".to_string(), None),
                ("MoveToMarker".to_string(), Some(2)),
                ("GetStage".to_string(), None),
                ("GetLocked".to_string(), Some(5)),
            ]
        );
        assert_eq!(
            parameters(c[0].function, c[0].params),
            vec![Value::Ref(1), Value::Int(110)]
        );
        assert_eq!(parameters(c[1].function, c[1].params), vec![Value::Ref(3)]);
        assert_eq!(parameters(c[2].function, c[2].params), vec![Value::Ref(4)]);
        assert_eq!(parameters(c[3].function, c[3].params), vec![]);
    }

    #[test]
    fn reads_parameters_by_kind() {
        // A stage held in a reference's variable.
        let p = [2, 0, b'r', 1, 0, b'r', 2, 0, b's', 4, 0];
        assert_eq!(
            parameters(number("SetStage"), &p),
            vec![Value::Ref(1), Value::RefVariable(2, 4)]
        );
        // An actor value is a 2-byte number; an axis one letter.
        assert_eq!(
            parameters(number("GetActorValue"), &[1, 0, 16, 0]),
            vec![Value::Code(16)]
        );
        assert_eq!(
            parameters(number("GetPos"), &[1, 0, b'Z']),
            vec![Value::Axis(b'Z')]
        );
        // ShowMessage keeps its variable count and duration after the
        // message: they aren't parameters.
        let p = [1, 0, b'r', 3, 0, 1, 0, b's', 1, 0, 0, 0, 0, 0];
        assert_eq!(parameters(number("ShowMessage"), &p), vec![Value::Ref(3)]);
        // A string after its length, then a whole number.
        let mut p = vec![2, 0, 3, 0];
        p.extend(b"abc");
        p.extend([b'n', 7, 0, 0, 0]);
        assert_eq!(
            parameters(number("PlayBink"), &p),
            vec![Value::Text(b"abc"), Value::Int(7)]
        );
        // A float.
        let mut p = vec![2, 0, b'r', 1, 0, b'z'];
        p.extend(0.5f64.to_le_bytes());
        assert_eq!(
            parameters(number("SetQuestDelay"), &p),
            vec![Value::Ref(1), Value::Float(0.5)]
        );
        // Bytes that don't read stop the list; no count, no parameters.
        assert_eq!(parameters(number("SetStage"), &[2, 0, b'?']), vec![]);
        assert_eq!(parameters(number("SetStage"), &[]), vec![]);
    }
}
