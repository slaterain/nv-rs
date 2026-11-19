//! The regular expressions of JIP LN NVSE's `sv_RegexMatch`, `sv_RegexSearch` and
//! `sv_RegexReplace`: MSVC's `std::regex` (ECMAScript grammar, `char`, the "C" locale), which
//! JIP's DLL carries compiled from the Visual Studio 2022 17.7 headers. Rust's `regex` crate has
//! no back-references or look-ahead and picks other matches, so this module is MSVC's own engine
//! translated: the same parser, the same graph of nodes and the same backtracking matcher, so a
//! pattern answers what it answers in the game with JIP, quirks included.
//!
//! Translated from Microsoft's C++ Standard Library, `<regex>` (MSVC 14.44.35207, the
//! `_Parser`, `_Builder`, `_Matcher`, `_Regex_search2`, `_Regex_replace1` and `_Format_default`
//! templates), which is licensed under the Apache License v2.0 with LLVM Exception
//! (Copyright (c) Microsoft Corporation; SPDX-License-Identifier: Apache-2.0 WITH
//! LLVM-exception). Modified: translated to Rust, only the `char` and ECMAScript paths kept, made
//! to answer as JIP's DLL does (the two differences below), with a limit of its own of 10,000
//! nested groups. Each part was compared with JIP 57.30's `jip_nvse.dll` (its `<regex>`
//! instantiations, linker 14.37) and is the same code there: the match loop, the repeat and
//! simple-loop steps, the alternation, the word boundary and its table, the bracket test, the
//! collating lookup, the better-match rule, the skip ahead of the search, the search driver, the
//! replace loop and the `$` format; the parser's disjunction, alternative, group, quantifier,
//! escape, bracket atom and bracket name (`[: :]`, `[= =]`, `[. .]`) steps, the repeat builder
//! (with the rewrite of `x?` as an alternation) and the loop-simplicity walk. Two differences
//! from the 14.44 source, both taken from the DLL: the class escapes `\D \S \W` are built the
//! older way (a complemented set instead of a negated node, the same bytes for 8-bit text), and
//! there is no limit on the number of groups (14.44 refuses the 1000th). Not in the DLL: the
//! "C" locale's class table, which `msvcp140.dll` takes from the UCRT (measured on this
//! machine's `ucrtbase.dll` under `setlocale(LC_ALL, "C")`).
//!
//! What MSVC does where ECMAScript leaves room or where it differs (settled for the commands):
//! - `.` matches any byte but `\n` and `\r`.
//! - `^` matches at the start of the text and after every `\n`; `$` at the end and before every
//!   `\n` (no multiline switch: always so), so `$` also matches before a final `\n`.
//! - `\w`, `\d`, `\s` and the bracket classes are the "C" locale's: ASCII only. Bytes 0x80-0xFF
//!   are in no class (not word, digit, space or letter), and `\b` sees them as non-word.
//!   `[[:blank:]]` is `[[:space:]]` (MSVC's masks), `[[:print:]]` is 0x20-0x7E.
//! - An empty alternative is allowed (`a|`, `|a`, `()`) and matches the empty text.
//! - Quantifiers: `{n}`, `{n,}`, `{n,m}` (`m < n`, `{,m}` and `a{` are errors), `?` after one
//!   makes it lazy; a quantifier after another (`a**`), after `^`, `$`, `\b`, `\B` or a look-ahead
//!   is an error. A repeated part that matched nothing ends its loop. A greedy loop of a single
//!   simple part keeps trying more repeats after a match and keeps the longest overall match
//!   (MSVC's `_Better_match`); loops with groups or alternatives take the first match.
//! - Groups: entering a group again (in a loop) forgets every group numbered after it; a
//!   back-reference to a group that took no part, or matched nothing, matches the empty text; a
//!   back-reference must name a group already closed before it in the pattern (`(a\1)` and
//!   `\1(a)` are errors). `\0` is the byte 0; `\01` is a back-reference to group 1 (all digits
//!   are read); there are no octal escapes. Any number of groups; nesting deeper than
//!   [`MAX_NESTING`] is refused (the port's own limit: MSVC's parser runs out of stack there).
//! - Escapes: `\f \n \r \t \v`, `\cX` (a letter, its value % 32), `\xHH`, `\uHHHH` (above 0xFF an
//!   error); any other byte but `c d D s S w W` stands for itself, so `[\b]` is `b`, not a
//!   backspace. `[[:name:]]`, `[[=x=]]` and `[[.x.]]` work in brackets as MSVC has them
//!   (`[[.x.]]`'s lookup is MSVC's, which only matches at the end of the text).
//! - `regex_replace`'s format: `$$`, `` $` `` (the text from where that search started), `$'`,
//!   `$&`, `$n` and `$nn` (two digits are taken when there), `$0` and `$00` kept as written, a
//!   group number at or past the group count gives nothing, any other `$x` and a final `$` kept.
//! - Limits: MSVC's own (32-bit build): a match attempt fails with `error_stack` past 1000 nested
//!   steps and with `error_complexity` past 10,000,000 steps, counted again for each start
//!   position. The original then throws out of JIP's command and the game stops; the commands
//!   here treat it as no match (docs/FIXES.md).
//!
//! Positions are byte offsets in the text. Threads: a value of this module is plain data; the
//! commands build and use one on the script's thread.

#[cfg(test)]
#[path = "ecma_regex_tests.rs"]
mod tests;

/// `regex_constants::error_type`: why a pattern was refused or a match given up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Collate = 0,
    Ctype = 1,
    Escape = 2,
    Backref = 3,
    Brack = 4,
    Paren = 5,
    Brace = 6,
    Badbrace = 7,
    Range = 8,
    Badrepeat = 10,
    Complexity = 11,
    Stack = 12,
    Syntax = 14,
    /// The port's own: groups nested deeper than [`MAX_NESTING`].
    TooDeep = 100,
}

/// The deepest nesting of groups the parser takes (the port's own limit, docs/FIXES.md). MSVC's
/// parser has none and recurses once a group, about 68 to 84 bytes of stack a level in JIP's
/// DLL, so on the game's 1 MB main thread stack it runs out at about 12,000 to 15,000 and the
/// game crashes; and any way through more than about 1,000 nested groups passes the matcher's
/// 1000 nested steps. Only a branch that matches before a deep part could answer past that.
pub const MAX_NESTING: i32 = 10_000;

/// `regex_constants::match_flag_type` bits the commands' paths use.
pub const MATCH_NOT_BOL: u32 = 0x0001;
pub const MATCH_NOT_EOL: u32 = 0x0002;
pub const MATCH_NOT_BOW: u32 = 0x0004;
pub const MATCH_NOT_EOW: u32 = 0x0008;
pub const MATCH_ANY: u32 = 0x0010;
pub const MATCH_NOT_NULL: u32 = 0x0020;
pub const MATCH_CONTINUOUS: u32 = 0x0040;
pub const MATCH_PREV_AVAIL: u32 = 0x0100;
/// `regex_constants::_Match_not_null` (the replace loop's "not empty at the start").
pub const MATCH_NOT_NULL_AT_START: u32 = 0x2000;

/// `_REGEX_MAX_STACK_COUNT` (a 32-bit build) and `_REGEX_MAX_COMPLEXITY_COUNT`.
pub const MAX_STACK: i64 = 1000;
pub const MAX_COMPLEXITY: i64 = 10_000_000;

// ---- the nodes (`_Node_base` and its kinds) ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Nop,
    Bol,
    Eol,
    Wbound,
    Dot,
    Str,
    Class,
    Group,
    EndGroup,
    Assert,
    NegAssert,
    EndAssert,
    Capture,
    EndCapture,
    Back,
    If,
    Endif,
    Rep,
    EndRep,
    Begin,
    End,
}

/// `_Node_flags`.
const NEGATE: u8 = 1;
const GREEDY: u8 = 2;
const FINAL: u8 = 4;

/// A bracket expression (`_Node_class` for `char`: the 256-bit table; `_Coll`, the collating
/// elements, grouped by length, longest first).
#[derive(Clone, Debug, Default)]
struct ClassSet {
    bits: [u64; 4],
    coll: Vec<(usize, Vec<u8>)>,
}

impl ClassSet {
    fn mark(&mut self, c: u8) {
        self.bits[usize::from(c >> 6)] |= 1u64 << (c & 63);
    }
    fn find(&self, c: u8) -> bool {
        self.bits[usize::from(c >> 6)] & (1u64 << (c & 63)) != 0
    }
}

/// One node. `link` is what the kind points at: a group's number (Capture, Back), the node a
/// closing node closes (EndGroup, EndCapture, EndAssert), an assertion's body (Assert,
/// NegAssert), an alternative's end (If), a repeat's end (Rep) and an end's repeat (EndRep).
#[derive(Clone, Debug)]
struct Node {
    kind: Kind,
    flags: u8,
    next: Option<usize>,
    prev: Option<usize>,
    link: usize,
    /// If: the next branch.
    child: Option<usize>,
    text: Vec<u8>,
    class: Option<Box<ClassSet>>,
    min: i32,
    max: i32,
    loop_number: usize,
    /// `_Simple_loop`: -1 undetermined, 0 holds an alternative or another repeat, 1 simple.
    simple: i8,
}

impl Node {
    fn new(kind: Kind) -> Node {
        Node {
            kind,
            flags: 0,
            next: None,
            prev: None,
            link: 0,
            child: None,
            text: Vec::new(),
            class: None,
            min: 0,
            max: 0,
            loop_number: 0,
            simple: -1,
        }
    }
}

/// A compiled pattern (`basic_regex<char>`).
#[derive(Clone, Debug)]
pub struct Regex {
    nodes: Vec<Node>,
    loops: usize,
    /// `_Marks`: the groups with group 0.
    marks: usize,
}

impl Regex {
    // Translated from MSVC's <regex>: basic_regex's constructor (`_Parser::_Compile`, ECMAScript).
    /// The pattern compiled, or the error `std::regex`'s constructor throws.
    pub fn new(pattern: &[u8]) -> Result<Regex, Error> {
        Parser::new(pattern).compile()
    }

    /// `mark_count()`: the capture groups, group 0 not counted.
    pub fn mark_count(&self) -> usize {
        self.marks - 1
    }
}

// ---- the C locale's classes (`ctype<char>` of `locale::classic()`, UCRT's "C" table) ----

const UP: u16 = 0x1;
const LO: u16 = 0x2;
const DI: u16 = 0x4;
const SPACE: u16 = 0x8;
const PU: u16 = 0x10;
const CONTROL: u16 = 0x20;
const BLANK: u16 = 0x40;
const HEX: u16 = 0x80;
const XA: u16 = 0x100;
/// `ctype_base::mask` values (MSVC's <xlocale>); `w` is -1.
const ALNUM: u16 = DI | LO | UP | XA;
const ALPHA: u16 = LO | UP | XA;
const WORD: u16 = 0xffff;

/// The "C" locale's classification of a byte (UCRT's table: nothing for 0x80-0xFF).
fn c_table(c: u8) -> u16 {
    match c {
        0x09..=0x0d => CONTROL | SPACE,
        0x00..=0x1f | 0x7f => CONTROL,
        0x20 => SPACE | BLANK,
        b'0'..=b'9' => DI | HEX,
        b'A'..=b'F' => UP | HEX,
        b'G'..=b'Z' => UP,
        b'a'..=b'f' => LO | HEX,
        b'g'..=b'z' => LO,
        0x21..=0x7e => PU,
        _ => 0,
    }
}

/// `regex_traits<char>::isctype`.
fn isctype(c: u8, mask: u16) -> bool {
    if mask == WORD {
        c == b'_' || c_table(c) & ALNUM != 0
    } else {
        c_table(c) & mask != 0
    }
}

/// `regex_traits<char>::lookup_classname` (case-blind, no `icase`): 0 for an unknown name.
fn lookup_classname(name: &[u8]) -> u16 {
    const NAMES: [(&[u8], u16); 15] = [
        (b"alnum", ALNUM),
        (b"alpha", ALPHA),
        (b"blank", SPACE | BLANK),
        (b"cntrl", CONTROL),
        (b"d", DI),
        (b"digit", DI),
        (b"graph", DI | LO | PU | UP | XA),
        (b"lower", LO),
        (b"print", DI | LO | PU | BLANK | UP | XA | HEX),
        (b"punct", PU),
        (b"space", SPACE | BLANK),
        (b"s", SPACE | BLANK),
        (b"upper", UP),
        (b"w", WORD),
        (b"xdigit", HEX),
    ];
    NAMES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map_or(0, |&(_, mask)| mask)
}

/// `_Is_word`: the ASCII letters, digits and `_`.
fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

// ---- the builder (`_Builder`) ----

struct Builder {
    nodes: Vec<Node>,
    current: usize,
    loops: usize,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            nodes: vec![Node::new(Kind::Begin)],
            current: 0,
            loops: 0,
        }
    }

    fn make(&mut self, kind: Kind) -> usize {
        self.nodes.push(Node::new(kind));
        self.nodes.len() - 1
    }

    fn kind(&self, n: usize) -> Kind {
        self.nodes[n].kind
    }

    fn negate(&mut self) {
        self.nodes[self.current].flags ^= NEGATE;
    }

    fn mark_final(&mut self) {
        self.nodes[self.current].flags |= FINAL;
    }

    fn link_node(&mut self, nx: usize) -> usize {
        let cur = self.current;
        self.nodes[nx].prev = Some(cur);
        if let Some(next) = self.nodes[cur].next {
            self.nodes[nx].next = Some(next);
            self.nodes[next].prev = Some(nx);
        }
        self.nodes[cur].next = Some(nx);
        self.current = nx;
        nx
    }

    fn insert_node(&mut self, before: usize, to_insert: usize) {
        let prev = self.nodes[before].prev;
        if let Some(p) = prev {
            self.nodes[p].next = Some(to_insert);
        }
        self.nodes[to_insert].prev = prev;
        self.nodes[before].prev = Some(to_insert);
        self.nodes[to_insert].next = Some(before);
    }

    fn new_node(&mut self, kind: Kind) -> usize {
        let n = self.make(kind);
        self.link_node(n)
    }

    fn add_char(&mut self, c: u8) {
        let cur = self.current;
        if self.kind(cur) != Kind::Str || self.nodes[cur].flags & FINAL != 0 {
            self.new_node(Kind::Str);
        }
        let cur = self.current;
        self.nodes[cur].text.push(c);
    }

    fn add_class(&mut self) {
        let n = self.make(Kind::Class);
        self.nodes[n].class = Some(Box::default());
        self.link_node(n);
    }

    fn class(&mut self) -> &mut ClassSet {
        let cur = self.current;
        self.nodes[cur]
            .class
            .as_mut()
            .expect("the bracket being built")
    }

    fn add_char_to_class(&mut self, c: u8) {
        self.class().mark(c);
    }

    fn add_range2(&mut self, a: u8, b: u8) {
        let class = self.class();
        for c in a..=b {
            class.mark(c);
        }
    }

    fn add_named_class(&mut self, mask: u16, negate: bool) {
        let class = self.class();
        for c in 0..=255u8 {
            if isctype(c, mask) != negate {
                class.mark(c);
            }
        }
    }

    /// `_Add_equiv`: the bytes whose primary key (the "C" locale: the byte made lower case) is
    /// the name's.
    fn add_equiv(&mut self, name: &[u8]) {
        let key = name.to_ascii_lowercase();
        let class = self.class();
        for c in 0..=255u8 {
            if key == [c.to_ascii_lowercase()] {
                class.mark(c);
            }
        }
    }

    /// `_Add_coll` (`_Char_to_elts`): the element joins the group of its length (longest first).
    fn add_coll(&mut self, name: &[u8]) {
        let len = name.len();
        let coll = &mut self.class().coll;
        let at = coll
            .iter()
            .position(|(size, _)| len >= *size)
            .unwrap_or(coll.len());
        if at == coll.len() || coll[at].0 != len {
            coll.insert(at, (len, Vec::new()));
        }
        coll[at].1.extend_from_slice(name);
    }

    fn begin_group(&mut self) -> usize {
        self.new_node(Kind::Group)
    }

    fn end_group(&mut self, back: usize) {
        let kind = match self.kind(back) {
            Kind::Group => Kind::EndGroup,
            Kind::Assert | Kind::NegAssert => Kind::EndAssert,
            _ => Kind::EndCapture,
        };
        let n = self.make(kind);
        self.nodes[n].link = back;
        self.link_node(n);
    }

    fn begin_assert_group(&mut self, neg: bool) -> usize {
        let node1 = self.make(if neg { Kind::NegAssert } else { Kind::Assert });
        let node2 = self.make(Kind::Nop);
        self.link_node(node1);
        self.nodes[node1].link = node2;
        self.nodes[node2].prev = Some(node1);
        self.current = node2;
        node1
    }

    fn end_assert_group(&mut self, nx: usize) {
        self.end_group(nx);
        self.current = nx;
    }

    fn begin_capture_group(&mut self, idx: usize) -> usize {
        let n = self.make(Kind::Capture);
        self.nodes[n].link = idx;
        self.link_node(n)
    }

    fn add_backreference(&mut self, idx: usize) {
        let n = self.make(Kind::Back);
        self.nodes[n].link = idx;
        self.link_node(n);
    }

    fn begin_if(&mut self, start: usize) -> usize {
        let res = self.make(Kind::Endif);
        self.link_node(res);
        let node1 = self.make(Kind::If);
        self.nodes[node1].link = res;
        if let Some(pos) = self.nodes[start].next {
            self.insert_node(pos, node1);
        }
        res
    }

    fn else_if(&mut self, start: usize, end: usize) {
        let Some(mut parent) = self.nodes[start].next else {
            return;
        };
        let first = self.nodes[end].next;
        self.nodes[end].next = None;
        let last = self.current;
        self.current = end;
        self.nodes[last].next = Some(end);
        while let Some(c) = self.nodes[parent].child {
            parent = c;
        }
        let branch = self.make(Kind::If);
        self.nodes[branch].link = end;
        self.nodes[parent].child = Some(branch);
        self.nodes[branch].next = first;
        if let Some(f) = first {
            self.nodes[f].prev = Some(branch);
        }
    }

    fn add_rep(&mut self, min: i32, max: i32, greedy: bool) {
        let cur = self.current;
        if self.kind(cur) == Kind::Str && self.nodes[cur].text.len() != 1 {
            // The final byte moves to a string node of its own (the current one is final).
            if let Some(c) = self.nodes[cur].text.pop() {
                self.add_char(c);
            }
        }
        let mut pos = self.current;
        if matches!(self.kind(pos), Kind::EndGroup | Kind::EndCapture) {
            pos = self.nodes[pos].link;
        }
        if min == 0 && max == 1 {
            // `x?` as an alternation of `x` and an empty group (the empty one first when lazy).
            let end = self.make(Kind::Endif);
            let if_expr = self.make(Kind::If);
            let if_empty = self.make(Kind::If);
            let gbegin = self.make(Kind::Group);
            let gend = self.make(Kind::EndGroup);
            self.nodes[if_expr].link = end;
            self.nodes[if_empty].link = end;
            self.nodes[gend].link = gbegin;
            self.nodes[if_empty].next = Some(gbegin);
            self.nodes[gbegin].prev = Some(if_empty);
            self.nodes[gbegin].next = Some(gend);
            self.nodes[gend].prev = Some(gbegin);
            self.nodes[gend].next = Some(end);
            self.nodes[if_expr].child = Some(if_empty);
            self.link_node(end);
            self.insert_node(pos, if_expr);
            if !greedy {
                let (Some(a), Some(b)) = (self.nodes[if_expr].next, self.nodes[if_empty].next)
                else {
                    return;
                };
                let (pa, pb) = (self.nodes[a].prev, self.nodes[b].prev);
                self.nodes[a].prev = pb;
                self.nodes[b].prev = pa;
                self.nodes[if_expr].next = Some(b);
                self.nodes[if_empty].next = Some(a);
            }
        } else {
            let node0 = self.make(Kind::EndRep);
            let nx = self.make(Kind::Rep);
            {
                let rep = &mut self.nodes[nx];
                rep.flags = if greedy { GREEDY } else { 0 };
                rep.min = min;
                rep.max = max;
                rep.link = node0;
                rep.loop_number = self.loops;
            }
            self.loops += 1;
            self.nodes[node0].link = nx;
            self.link_node(node0);
            self.insert_node(pos, nx);
        }
    }
}

// ---- the parser (`_Parser`, ECMAScript's grammar flags) ----

/// `_Meta_type`: `-1` the end of the pattern, `0` an ordinary byte, else the meta byte itself.
const EOS: i32 = -1;
const CHR: i32 = 0;
/// `_Meta_map`: the bytes the parser sees as meta bytes.
const META_MAP: &[u8] = b"()$^.*+?[]|\\-{},:=!\n\r\x08";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Prs {
    None,
    Chr,
    Set,
}

fn meta(c: u8) -> i32 {
    i32::from(c)
}

struct Parser<'a> {
    pat: &'a [u8],
    pos: usize,
    mchar: i32,
    /// `_Char`: the byte (0xFF, `(char)_Meta_eos`, at the end).
    ch: u8,
    val: i32,
    grp_idx: usize,
    disj_count: i32,
    finished: Vec<bool>,
    nfa: Builder,
}

type Parsed<T> = Result<T, Error>;

impl<'a> Parser<'a> {
    fn new(pat: &'a [u8]) -> Parser<'a> {
        let mut p = Parser {
            pat,
            pos: 0,
            mchar: EOS,
            ch: 0xff,
            val: 0,
            grp_idx: 0,
            disj_count: 0,
            finished: Vec::new(),
            nfa: Builder::new(),
        };
        p.trans();
        p
    }

    fn trans(&mut self) {
        match self.pat.get(self.pos) {
            None => {
                self.mchar = EOS;
                self.ch = 0xff;
            }
            Some(&c) => {
                self.ch = c;
                self.mchar = if META_MAP.contains(&c) { meta(c) } else { CHR };
            }
        }
    }

    fn next(&mut self) {
        if self.pos < self.pat.len() {
            self.pos += 1;
        }
        self.trans();
    }

    fn expect(&mut self, st: u8, code: Error) -> Parsed<()> {
        if self.mchar != meta(st) {
            return Err(code);
        }
        self.next();
        Ok(())
    }

    /// `regex_traits<char>::value`.
    fn value(c: u8, base: i32) -> i32 {
        if (base != 8 && c.is_ascii_digit()) || (base == 8 && (b'0'..=b'7').contains(&c)) {
            return i32::from(c - b'0');
        }
        if base != 16 {
            return -1;
        }
        match c {
            b'a'..=b'f' => i32::from(c - b'a') + 10,
            b'A'..=b'F' => i32::from(c - b'A') + 10,
            _ => -1,
        }
    }

    fn do_digits(&mut self, base: i32, count: i32, err: Error) -> Parsed<i32> {
        let mut count = count;
        self.val = 0;
        while count != 0 {
            let chv = Self::value(self.ch, base);
            if chv == -1 {
                break;
            }
            if self.val > (i32::MAX - chv) / base {
                return Err(err);
            }
            count -= 1;
            self.val = self.val * base + chv;
            self.next();
        }
        Ok(count)
    }

    fn decimal_digits2(&mut self, err: Error, count: i32) -> Parsed<bool> {
        Ok(self.do_digits(10, count, err)? != count)
    }

    fn hex_digits(&mut self, count: i32) -> Parsed<()> {
        if self.do_digits(16, count, Error::Escape)? != 0 {
            return Err(Error::Escape);
        }
        Ok(())
    }

    fn do_ex_class(&mut self, end_arg: i32) -> Parsed<()> {
        let errtype = if end_arg == meta(b':') {
            Error::Ctype
        } else if end_arg == meta(b'=') || end_arg == meta(b'.') {
            Error::Collate
        } else {
            Error::Syntax
        };
        let beg = self.pos;
        while self.mchar != meta(b':')
            && self.mchar != meta(b'=')
            && self.mchar != meta(b'.')
            && self.mchar != EOS
        {
            self.next();
        }
        if self.mchar != end_arg {
            return Err(errtype);
        }
        let pat: &'a [u8] = self.pat;
        let name = &pat[beg..self.pos];
        if end_arg == meta(b':') {
            let mask = lookup_classname(name);
            if mask == 0 {
                return Err(Error::Ctype);
            }
            self.nfa.add_named_class(mask, false);
        } else if name.is_empty() {
            return Err(Error::Collate);
        } else if end_arg == meta(b'=') {
            self.nfa.add_equiv(name);
        } else {
            self.nfa.add_coll(name);
        }
        self.next();
        self.expect(b']', errtype)
    }

    /// `_CharacterClassEscape` as JIP's DLL has it: `\D \S \W` (an upper-case letter) add the
    /// complement of the class's bytes.
    fn character_class_escape(&mut self, addit: bool) -> bool {
        if self.pos >= self.pat.len() {
            return false;
        }
        let mask = lookup_classname(&self.pat[self.pos..self.pos + 1]);
        if mask == 0 {
            return false;
        }
        if addit {
            self.nfa.add_class();
        }
        let negated = isctype(self.ch, UP);
        self.nfa.add_named_class(mask, negated);
        self.next();
        true
    }

    fn class_escape2(&mut self) -> Parsed<Prs> {
        if self.ch == b'\\' {
            self.val = i32::from(b'\\');
            self.next();
            return Ok(Prs::Chr);
        }
        if self.character_class_escape(false) {
            return Ok(Prs::Set);
        }
        if self.decimal_digits2(Error::Escape, i32::MAX)? {
            if self.val != 0 {
                return Err(Error::Escape);
            }
            return Ok(Prs::Chr);
        }
        Ok(if self.character_escape()? {
            Prs::Chr
        } else {
            Prs::None
        })
    }

    fn class_atom(&mut self) -> Parsed<Prs> {
        if self.mchar == meta(b'\\') {
            self.next();
            self.class_escape2()
        } else if self.mchar == meta(b'[') {
            self.next();
            if self.mchar == meta(b':') || self.mchar == meta(b'=') || self.mchar == meta(b'.') {
                let st = self.mchar;
                self.next();
                self.do_ex_class(st)?;
                Ok(Prs::Set)
            } else {
                self.val = i32::from(b'[');
                Ok(Prs::Chr)
            }
        } else if self.mchar == meta(b']') || self.mchar == EOS {
            Ok(Prs::None)
        } else {
            self.val = i32::from(self.ch);
            self.next();
            Ok(Prs::Chr)
        }
    }

    fn class_ranges(&mut self) -> Parsed<()> {
        loop {
            let ret = self.class_atom()?;
            if ret == Prs::None {
                return Ok(());
            }
            if self.mchar == meta(b'-') {
                self.next();
                let chr1 = self.val as u8;
                let set_preceding = ret == Prs::Set;
                let ret2 = self.class_atom()?;
                if ret2 == Prs::None {
                    // The dash is an ordinary byte.
                    if !set_preceding {
                        self.nfa.add_char_to_class(chr1);
                    }
                    self.nfa.add_char_to_class(b'-');
                    return Ok(());
                }
                if set_preceding || ret2 == Prs::Set {
                    return Err(Error::Range);
                }
                let chr2 = self.val as u8;
                if chr2 < chr1 {
                    return Err(Error::Range);
                }
                self.nfa.add_range2(chr1, chr2);
            } else if ret == Prs::Chr {
                self.nfa.add_char_to_class(self.val as u8);
            }
        }
    }

    fn character_class(&mut self) -> Parsed<()> {
        self.nfa.add_class();
        if self.mchar == meta(b'^') {
            self.nfa.negate();
            self.next();
        }
        self.class_ranges()
    }

    fn do_capture_group(&mut self) -> Parsed<()> {
        // No limit on the groups: JIP's DLL (17.7) has none (later MSVC refuses the 1000th).
        self.grp_idx += 1;
        let pos1 = self.nfa.begin_capture_group(self.grp_idx);
        self.disjunction()?;
        self.nfa.end_group(pos1);
        self.finished.resize(self.grp_idx + 1, false);
        let idx = self.nfa.nodes[pos1].link;
        self.finished[idx] = true;
        Ok(())
    }

    fn do_noncapture_group(&mut self) -> Parsed<()> {
        let pos1 = self.nfa.begin_group();
        self.disjunction()?;
        self.nfa.end_group(pos1);
        Ok(())
    }

    fn do_assert_group(&mut self, neg: bool) -> Parsed<()> {
        let pos1 = self.nfa.begin_assert_group(neg);
        self.disjunction()?;
        self.nfa.end_assert_group(pos1);
        Ok(())
    }

    fn wrapped_disjunction(&mut self) -> Parsed<bool> {
        self.disj_count += 1;
        if self.disj_count > MAX_NESTING {
            return Err(Error::TooDeep);
        }
        if self.mchar == meta(b'?') {
            self.next();
            let c = self.mchar;
            self.next();
            if c == meta(b':') {
                self.do_noncapture_group()?;
            } else if c == meta(b'!') {
                self.do_assert_group(true)?;
                self.disj_count -= 1;
                return Ok(false);
            } else if c == meta(b'=') {
                self.do_assert_group(false)?;
                self.disj_count -= 1;
                return Ok(false);
            } else {
                return Err(Error::Syntax);
            }
        } else {
            self.do_capture_group()?;
        }
        self.disj_count -= 1;
        Ok(true)
    }

    fn identity_escape(&mut self) -> bool {
        if matches!(self.ch, b'c' | b'd' | b'D' | b's' | b'S' | b'w' | b'W') {
            return false;
        }
        self.val = i32::from(self.ch);
        self.next();
        true
    }

    fn character_escape(&mut self) -> Parsed<bool> {
        if self.mchar == EOS {
            return Err(Error::Escape);
        }
        let ffn = match self.ch {
            b'f' => Some(0x0c),
            b'n' => Some(b'\n'),
            b'r' => Some(b'\r'),
            b't' => Some(b'\t'),
            b'v' => Some(0x0b),
            _ => None,
        };
        if let Some(v) = ffn {
            self.val = i32::from(v);
            self.next();
        } else if self.ch == b'c' {
            self.next();
            if !isctype(self.ch, ALPHA) {
                return Err(Error::Escape);
            }
            self.val = i32::from(self.ch % 32);
            self.next();
        } else if self.ch == b'x' {
            self.next();
            self.hex_digits(2)?;
        } else if self.ch == b'u' {
            self.next();
            self.hex_digits(4)?;
        } else {
            return Ok(self.identity_escape());
        }
        if self.val > 0xff {
            return Err(Error::Escape);
        }
        Ok(true)
    }

    fn atom_escape(&mut self) -> Parsed<()> {
        if self.decimal_digits2(Error::Backref, i32::MAX)? {
            let n = self.val as usize;
            if n == 0 {
                self.nfa.add_char(0);
            } else if self.grp_idx < n || self.finished.len() <= n || !self.finished[n] {
                return Err(Error::Backref);
            } else {
                self.nfa.add_backreference(n);
            }
        } else if self.character_escape()? {
            self.nfa.add_char(self.val as u8);
        } else if !self.character_class_escape(true) {
            return Err(Error::Escape);
        }
        Ok(())
    }

    fn quantifier(&mut self) -> Parsed<()> {
        let mut min = 0;
        let mut max = -1;
        if self.mchar != meta(b'*') {
            if self.mchar == meta(b'+') {
                min = 1;
            } else if self.mchar == meta(b'?') {
                max = 1;
            } else if self.mchar == meta(b'{') {
                self.next();
                if !self.decimal_digits2(Error::Badbrace, i32::MAX)? {
                    return Err(Error::Badbrace);
                }
                min = self.val;
                if self.mchar != meta(b',') {
                    max = min;
                } else {
                    self.next();
                    if self.mchar != meta(b'}') {
                        if !self.decimal_digits2(Error::Badbrace, i32::MAX)? {
                            return Err(Error::Badbrace);
                        }
                        max = self.val;
                    }
                }
                if self.mchar != meta(b'}') || (max != -1 && max < min) {
                    return Err(Error::Badbrace);
                }
            } else {
                return Ok(());
            }
        }
        self.nfa.mark_final();
        self.next();
        let greedy = self.mchar != meta(b'?');
        if !greedy {
            self.next();
        }
        self.nfa.add_rep(min, max, greedy);
        Ok(())
    }

    fn alternative(&mut self) -> Parsed<bool> {
        let mut found = false;
        loop {
            let mut quant = true;
            let m = self.mchar;
            if m == EOS || m == meta(b'|') || (m == meta(b')') && self.disj_count != 0) {
                return Ok(found);
            } else if m == meta(b')') {
                return Err(Error::Paren);
            } else if m == meta(b'.') {
                self.nfa.new_node(Kind::Dot);
                self.next();
            } else if m == meta(b'\\') {
                self.next();
                if self.ch == b'b' {
                    self.nfa.new_node(Kind::Wbound);
                    self.next();
                    quant = false;
                } else if self.ch == b'B' {
                    self.nfa.new_node(Kind::Wbound);
                    self.nfa.negate();
                    self.next();
                    quant = false;
                } else {
                    self.atom_escape()?;
                }
            } else if m == meta(b'[') {
                self.next();
                self.character_class()?;
                self.expect(b']', Error::Brack)?;
            } else if m == meta(b'(') {
                self.next();
                quant = self.wrapped_disjunction()?;
                self.expect(b')', Error::Paren)?;
            } else if m == meta(b'^') {
                self.nfa.new_node(Kind::Bol);
                self.next();
                quant = false;
            } else if m == meta(b'$') {
                self.nfa.new_node(Kind::Eol);
                self.next();
                quant = false;
            } else if m == meta(b'*') || m == meta(b'+') || m == meta(b'?') || m == meta(b'{') {
                return Err(Error::Badrepeat);
            } else if m == meta(b'}') {
                return Err(Error::Brace);
            } else if m == meta(b']') {
                return Err(Error::Brack);
            } else {
                self.nfa.add_char(self.ch);
                self.next();
            }
            if quant {
                self.quantifier()?;
            }
            found = true;
        }
    }

    fn disjunction(&mut self) -> Parsed<()> {
        let pos1 = self.nfa.current;
        if !self.alternative()? {
            if self.mchar != meta(b'|') {
                return Ok(());
            }
            let pos3 = self.nfa.begin_group();
            self.nfa.end_group(pos3);
        }
        let pos2 = self.nfa.begin_if(pos1);
        while self.mchar == meta(b'|') {
            self.next();
            if !self.alternative()? {
                let pos3 = self.nfa.begin_group();
                self.nfa.end_group(pos3);
            }
            self.nfa.else_if(pos1, pos2);
        }
        Ok(())
    }

    fn compile(mut self) -> Parsed<Regex> {
        let pos1 = self.nfa.begin_capture_group(0);
        self.disjunction()?;
        if self.pos != self.pat.len() {
            return Err(Error::Syntax);
        }
        self.nfa.end_group(pos1);
        self.nfa.new_node(Kind::End);
        let mut nodes = self.nfa.nodes;
        calculate_loop_simplicity(&mut nodes, Some(0), None, None);
        Ok(Regex {
            nodes,
            loops: self.nfa.loops,
            marks: self.grp_idx + 1,
        })
    }
}

/// `_Calculate_loop_simplicity`: a repeat is simple (1) unless it holds an alternative or another
/// repeat (0; both repeats then).
fn calculate_loop_simplicity(
    nodes: &mut [Node],
    from: Option<usize>,
    to: Option<usize>,
    outer: Option<usize>,
) {
    let mut outer = outer;
    let mut nx = from;
    while let Some(n) = nx {
        if Some(n) == to {
            break;
        }
        match nodes[n].kind {
            Kind::If => {
                if let Some(o) = outer {
                    nodes[o].simple = 0;
                }
                let mut branch = nodes[n].child;
                while let Some(b) = branch {
                    let (next, endif) = (nodes[b].next, nodes[b].link);
                    calculate_loop_simplicity(nodes, next, Some(endif), outer);
                    branch = nodes[b].child;
                }
            }
            Kind::Assert | Kind::NegAssert => {
                let child = nodes[n].link;
                calculate_loop_simplicity(nodes, Some(child), None, None);
            }
            Kind::Rep => {
                if let Some(o) = outer {
                    nodes[o].simple = 0;
                    nodes[n].simple = 0;
                } else {
                    outer = Some(n);
                }
            }
            Kind::EndRep if outer == Some(nodes[n].link) => {
                if let Some(o) = outer {
                    if nodes[o].simple == -1 {
                        nodes[o].simple = 1;
                    }
                }
                outer = None;
            }
            _ => {}
        }
        nx = nodes[n].next;
    }
}

// ---- the matcher (`_Matcher`) ----

/// `_Tgt_state_t`: the place in the text, which groups are valid and their places.
#[derive(Clone, Debug, Default)]
struct State {
    cur: usize,
    grp_valid: Vec<bool>,
    grps: Vec<(usize, usize)>,
}

/// The way a match attempt was given up (MSVC throws `regex_error` with it).
type Step<T> = Result<T, Error>;

struct Matcher<'r, 't> {
    re: &'r Regex,
    text: &'t [u8],
    tgt: State,
    res: State,
    /// `_Loop_vals`: each repeat's count so far and the place its current repeat started at
    /// (MSVC keeps a pointer to that place; the place itself never changes while it is used).
    loop_vals: Vec<(i32, usize)>,
    begin: usize,
    end: usize,
    first: usize,
    mflags: u32,
    matched: bool,
    cap: bool,
    ncap: usize,
    full: bool,
    complexity: i64,
    stack: i64,
    max_complexity: i64,
    max_stack: i64,
}

/// A match's groups (`None`: no part in the match) and its prefix and suffix, as byte ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    pub groups: Vec<Option<(usize, usize)>>,
    pub prefix: (usize, usize),
    pub suffix: (usize, usize),
}

impl Captures {
    /// Group 0, the whole match.
    pub fn whole(&self) -> (usize, usize) {
        self.groups[0].unwrap_or((self.suffix.1, self.suffix.1))
    }
}

impl<'r, 't> Matcher<'r, 't> {
    fn new(re: &'r Regex, text: &'t [u8], first: usize, mflags: u32, limits: (i64, i64)) -> Self {
        Matcher {
            re,
            text,
            tgt: State::default(),
            res: State::default(),
            loop_vals: vec![(0, 0); re.loops],
            begin: first,
            end: text.len(),
            first,
            mflags,
            matched: false,
            cap: false,
            ncap: re.marks,
            full: false,
            complexity: 0,
            stack: 0,
            max_stack: limits.0,
            max_complexity: limits.1,
        }
    }

    fn node(&self, n: usize) -> &'r Node {
        &self.re.nodes[n]
    }

    /// `_Match`: one attempt at `self.first`; the groups when it matched.
    fn try_match(&mut self, cap: bool, full: bool) -> Step<Option<Captures>> {
        self.begin = self.first;
        self.tgt.cur = self.first;
        self.tgt.grp_valid.resize(self.ncap, false);
        self.tgt.grps.resize(self.ncap, (0, 0));
        self.cap = cap;
        self.full = full;
        self.complexity = self.max_complexity;
        self.stack = self.max_stack;
        self.matched = false;
        // A true answer always comes through the end node, which records the match.
        if !self.match_pat(Some(0))? || !self.matched {
            return Ok(None);
        }
        let groups: Vec<Option<(usize, usize)>> = (0..self.ncap)
            .map(|i| self.res.grp_valid[i].then_some(self.res.grps[i]))
            .collect();
        let start = groups[0].map_or(self.end, |g| g.0);
        let stop = groups[0].map_or(self.end, |g| g.1);
        Ok(Some(Captures {
            groups,
            prefix: (self.begin, start),
            suffix: (stop, self.end),
        }))
    }

    /// A byte of the text; past its end the 0 after it (a C string's).
    fn byte_at(&self, at: usize) -> u8 {
        self.text.get(at).copied().unwrap_or(0)
    }

    fn compare(&self, cur: usize, s: &[u8]) -> usize {
        if self.text.len() >= cur + s.len() && &self.text[cur..cur + s.len()] == s {
            cur + s.len()
        } else {
            cur
        }
    }

    fn is_wbound(&self) -> bool {
        let cur = self.tgt.cur;
        if self.mflags & MATCH_PREV_AVAIL != 0 || cur != self.begin {
            let before = is_word(self.byte_at(cur.wrapping_sub(1)));
            if cur == self.end {
                self.mflags & MATCH_NOT_EOW == 0 && before
            } else {
                before != is_word(self.text[cur])
            }
        } else if cur == self.end {
            self.mflags & (MATCH_NOT_BOW | MATCH_NOT_EOW) == 0
        } else {
            self.mflags & MATCH_NOT_BOW == 0 && is_word(self.text[cur])
        }
    }

    /// `_Lookup_coll`, MSVC's: an element counts when its compare, which steps past the first
    /// byte that differs, ends at `last`.
    fn lookup_coll(&self, first: usize, last: usize, coll: &[(usize, Vec<u8>)]) -> usize {
        for (size, data) in coll {
            for element in data.chunks(*size) {
                let mut res = first;
                for &b in element {
                    let c = self.byte_at(res);
                    res += 1;
                    if c != b {
                        break;
                    }
                }
                if res == last {
                    return last;
                }
            }
        }
        first
    }

    fn do_class(&mut self, n: usize) -> bool {
        let node = self.node(n);
        let class = node.class.as_deref().expect("a bracket node");
        let cur = self.tgt.cur;
        let mut res0 = cur + 1;
        let mut found = false;
        if !class.coll.is_empty() {
            let resx = self.lookup_coll(cur, self.end, &class.coll);
            if resx != cur {
                res0 = resx;
                found = true;
            }
        }
        if !found {
            found = class.find(self.text[cur]);
        }
        if found == (node.flags & NEGATE != 0) {
            false
        } else {
            self.tgt.cur = res0;
            true
        }
    }

    fn better_match(&self) -> bool {
        for i in 0..self.ncap {
            if self.res.grp_valid[i] && self.tgt.grp_valid[i] {
                let (r, t) = (self.res.grps[i], self.tgt.grps[i]);
                let from = |at: usize| at as i64 - self.begin as i64;
                if r.0 != t.0 {
                    return from(r.0) < from(t.0);
                }
                if r.1 != t.1 {
                    return from(r.1) < from(t.1);
                }
            }
        }
        false
    }

    fn do_if(&mut self, n: usize) -> Step<bool> {
        let st = self.tgt.clone();
        let mut branch = Some(n);
        while let Some(b) = branch {
            self.tgt = st.clone();
            if self.match_pat(self.node(b).next)? {
                break;
            }
            branch = self.node(b).child;
        }
        // `_Longest` (POSIX grammars only) is never set for ECMAScript: the first branch wins.
        Ok(branch.is_some())
    }

    fn do_rep0(&mut self, n: usize, greedy: bool) -> Step<bool> {
        let node = self.node(n);
        let (min, max) = (node.min, node.max);
        let body = node.next;
        let tail = self.node(node.link).next;
        let mut ix = 0;
        let st = self.tgt.clone();
        while ix < min {
            let cur = self.tgt.cur;
            if !self.match_pat(body)? {
                self.tgt = st;
                return Ok(false);
            } else if cur == self.tgt.cur {
                ix = min - 1;
            }
            ix += 1;
        }
        let mut last = self.tgt.clone();
        let mut matched0 = false;
        let mut saved_pos = self.tgt.cur;
        if self.match_pat(tail)? {
            if !greedy {
                return Ok(true);
            }
            last = self.tgt.clone();
            matched0 = true;
        }
        loop {
            // MSVC's `_Ix++ < _Max`, tested before the count goes up (the count is not read after
            // the loop, and `{2147483647}` would take it past `i32::MAX`).
            if max != -1 {
                if ix >= max {
                    break;
                }
                ix += 1;
            }
            self.tgt.cur = saved_pos;
            self.tgt.grp_valid.clone_from(&st.grp_valid);
            if !self.match_pat(body)? {
                break;
            }
            let mid = self.tgt.cur;
            if self.match_pat(tail)? {
                if !greedy {
                    return Ok(true);
                }
                last = self.tgt.clone();
                matched0 = true;
            }
            if saved_pos == mid {
                break;
            }
            saved_pos = mid;
        }
        self.tgt = if matched0 { last } else { st };
        Ok(matched0)
    }

    fn do_rep(&mut self, n: usize, greedy: bool, init_idx: i32) -> Step<bool> {
        let node = self.node(n);
        if node.simple == 1 {
            return self.do_rep0(n, greedy);
        }
        let (min, max) = (node.min, node.max);
        let body = node.next;
        let tail = self.node(node.link).next;
        let slot = node.loop_number;
        let mut matched0 = false;
        let st = self.tgt.clone();
        let (idx_sav, iter_sav) = self.loop_vals[slot];
        let cur_iter = self.tgt.cur;
        let progress = init_idx == 0 || iter_sav != cur_iter;
        if 0 <= max && max <= init_idx {
            matched0 = self.match_pat(tail)?;
        } else if init_idx < min {
            if !progress {
                matched0 = self.match_pat(tail)?;
            } else {
                self.loop_vals[slot] = (init_idx + 1, cur_iter);
                matched0 = self.match_pat(body)?;
            }
        } else if !greedy {
            matched0 = self.match_pat(tail)?;
            if !matched0 && progress {
                self.tgt = st.clone();
                self.loop_vals[slot] = (init_idx + 1, cur_iter);
                matched0 = self.match_pat(body)?;
            }
        } else {
            if progress {
                self.loop_vals[slot] = (init_idx + 1, cur_iter);
                matched0 = self.match_pat(body)?;
            }
            if (progress || 1 >= init_idx) && !matched0 {
                self.loop_vals[slot] = (idx_sav, iter_sav);
                self.tgt = st.clone();
                matched0 = self.match_pat(tail)?;
            }
        }
        if !matched0 {
            self.tgt = st;
        }
        self.loop_vals[slot] = (idx_sav, iter_sav);
        Ok(matched0)
    }

    // Translated from MSVC's <regex>: _Matcher::_Match_pat.
    fn match_pat(&mut self, start: Option<usize>) -> Step<bool> {
        if self.stack > 0 {
            self.stack -= 1;
            if self.stack <= 0 {
                return Err(Error::Stack);
            }
        }
        if self.complexity > 0 {
            self.complexity -= 1;
            if self.complexity <= 0 {
                return Err(Error::Complexity);
            }
        }
        let mut failed = false;
        let mut nx = start;
        while let Some(n) = nx {
            let node = self.node(n);
            let mut stop = false;
            match node.kind {
                Kind::Nop | Kind::Group | Kind::EndGroup | Kind::Endif | Kind::Begin => {}
                Kind::Bol => {
                    let cur = self.tgt.cur;
                    failed = if self.mflags & MATCH_PREV_AVAIL != 0 || cur != self.begin {
                        self.byte_at(cur.wrapping_sub(1)) != b'\n'
                    } else {
                        self.mflags & MATCH_NOT_BOL != 0
                    };
                }
                Kind::Eol => {
                    let cur = self.tgt.cur;
                    failed = if cur == self.end {
                        self.mflags & MATCH_NOT_EOL != 0
                    } else {
                        self.text[cur] != b'\n'
                    };
                }
                Kind::Wbound => failed = self.is_wbound() == (node.flags & NEGATE != 0),
                Kind::Dot => {
                    let cur = self.tgt.cur;
                    if cur == self.end || matches!(self.text[cur], b'\n' | b'\r') {
                        failed = true;
                    } else {
                        self.tgt.cur += 1;
                    }
                }
                Kind::Str => {
                    let res0 = self.compare(self.tgt.cur, &node.text);
                    if res0 != self.tgt.cur {
                        self.tgt.cur = res0;
                    } else {
                        failed = true;
                    }
                }
                Kind::Class => failed = self.tgt.cur == self.end || !self.do_class(n),
                Kind::Assert | Kind::NegAssert => {
                    let ch = self.tgt.cur;
                    let neg = node.kind == Kind::NegAssert;
                    let (cur, valid) = (self.tgt.cur, self.tgt.grp_valid.clone());
                    if self.match_pat(Some(node.link))? == neg {
                        self.tgt.cur = cur;
                        self.tgt.grp_valid = valid;
                        failed = true;
                    } else {
                        self.tgt.cur = ch;
                    }
                }
                Kind::EndAssert => stop = true,
                Kind::Capture => {
                    let idx = node.link;
                    self.tgt.grps[idx].0 = self.tgt.cur;
                    for v in &mut self.tgt.grp_valid[idx + 1..] {
                        *v = false;
                    }
                }
                Kind::EndCapture => {
                    let idx = self.node(node.link).link;
                    if self.cap || idx != 0 {
                        self.tgt.grp_valid[idx] = true;
                        self.tgt.grps[idx].1 = self.tgt.cur;
                    }
                }
                Kind::Back => {
                    let idx = node.link;
                    if self.tgt.grp_valid[idx] {
                        let (b, e) = self.tgt.grps[idx];
                        if b != e {
                            let cur = self.tgt.cur;
                            // A valid group always ends after it starts (no path re-enters a
                            // group and reads it back before it ends); MSVC would read on.
                            let res0 = self.text.get(b..e).map_or(cur, |s| self.compare(cur, s));
                            if res0 == cur {
                                failed = true;
                            } else {
                                self.tgt.cur = res0;
                            }
                        }
                    }
                }
                Kind::If => {
                    if !self.do_if(n)? {
                        failed = true;
                    }
                    stop = true;
                }
                Kind::Rep => {
                    if !self.do_rep(n, node.flags & GREEDY != 0, 0)? {
                        failed = true;
                    }
                    stop = true;
                }
                Kind::EndRep => {
                    let rep = self.node(node.link);
                    if rep.simple == 0 {
                        let count = self.loop_vals[rep.loop_number].0;
                        if !self.do_rep(node.link, rep.flags & GREEDY != 0, count)? {
                            failed = true;
                        }
                    }
                    stop = true;
                }
                Kind::End => {
                    let cur = self.tgt.cur;
                    if (self.mflags & (MATCH_NOT_NULL | MATCH_NOT_NULL_AT_START) != 0
                        && self.begin == cur)
                        || (self.full && cur != self.end)
                    {
                        failed = true;
                    } else if !self.matched || self.better_match() {
                        self.res = self.tgt.clone();
                        self.matched = true;
                    }
                    stop = true;
                }
            }
            nx = if failed || stop { None } else { node.next };
        }
        if self.stack > 0 {
            self.stack += 1;
        }
        Ok(!failed)
    }

    // Translated from MSVC's <regex>: _Matcher::_Skip.
    /// The first place at or after `first` where a match can start (`last` for none).
    fn skip(&self, first: usize, last: usize, from: Option<usize>) -> usize {
        let mut first = first;
        let mut last = last;
        let mut nx = from.or(Some(0));
        while first != last {
            let Some(n) = nx else {
                break;
            };
            let node = self.node(n);
            match node.kind {
                Kind::Nop
                | Kind::Group
                | Kind::EndGroup
                | Kind::Capture
                | Kind::EndCapture
                | Kind::Begin => {}
                Kind::Bol => {
                    if self.byte_at(first - 1) != b'\n' {
                        first = self.text[first..last]
                            .iter()
                            .position(|&b| b == b'\n')
                            .map_or(last, |p| first + p + 1);
                    }
                    return first;
                }
                Kind::Eol => {
                    return self.text[first..last]
                        .iter()
                        .position(|&b| b == b'\n')
                        .map_or(last, |p| first + p);
                }
                Kind::Str => {
                    let c = node.text[0];
                    return self.text[first..last]
                        .iter()
                        .position(|&b| b == c)
                        .map_or(last, |p| first + p);
                }
                Kind::Class => {
                    let class = node.class.as_deref().expect("a bracket node");
                    let negated = node.flags & NEGATE != 0;
                    while first != last {
                        let found = (!class.coll.is_empty()
                            && self.lookup_coll(first, first + 1, &class.coll) != first)
                            || class.find(self.text[first]);
                        if found != negated {
                            return first;
                        }
                        first += 1;
                    }
                    return first;
                }
                Kind::If => {
                    let mut branch = Some(n);
                    while let Some(b) = branch {
                        if first == last {
                            break;
                        }
                        last = self.skip(first, last, self.node(b).next);
                        branch = self.node(b).child;
                    }
                    return last;
                }
                Kind::EndAssert | Kind::End => nx = None,
                Kind::Wbound
                | Kind::Dot
                | Kind::Assert
                | Kind::NegAssert
                | Kind::Back
                | Kind::Endif
                | Kind::Rep
                | Kind::EndRep => return first,
            }
            if let Some(n) = nx {
                nx = self.node(n).next;
            }
        }
        first
    }
}

/// MSVC's limits (`MAX_STACK`, `MAX_COMPLEXITY`).
const LIMITS: (i64, i64) = (MAX_STACK, MAX_COMPLEXITY);

// Translated from MSVC's <regex>: regex_match (no match results: `match_any`, no captures).
/// Whether the whole text matches.
pub fn full_match(re: &Regex, text: &[u8]) -> Result<bool, Error> {
    full_match_with(re, text, LIMITS)
}

fn full_match_with(re: &Regex, text: &[u8], limits: (i64, i64)) -> Result<bool, Error> {
    let mut mx = Matcher::new(re, text, 0, MATCH_ANY, limits);
    Ok(mx.try_match(false, true)?.is_some())
}

// Translated from MSVC's <regex>: _Regex_search2 (with match results).
/// The first match in `text` from `first` on, with `flags` (`regex_constants::match_flag_type`).
pub fn search(
    re: &Regex,
    text: &[u8],
    first: usize,
    flags: u32,
) -> Result<Option<Captures>, Error> {
    search_with(re, text, first, flags, LIMITS)
}

fn search_with(
    re: &Regex,
    text: &[u8],
    first: usize,
    flags: u32,
    limits: (i64, i64),
) -> Result<Option<Captures>, Error> {
    let begin = first;
    let last = text.len();
    let mut mx = Matcher::new(re, text, first, flags, limits);
    let mut found = mx.try_match(true, false)?;
    if found.is_none() && first != last && flags & MATCH_CONTINUOUS == 0 {
        mx.mflags |= MATCH_PREV_AVAIL;
        mx.mflags &= !MATCH_NOT_NULL_AT_START;
        let mut at = first;
        loop {
            at = mx.skip(at + 1, last, None);
            if at == last {
                break;
            }
            mx.first = at;
            found = mx.try_match(true, false)?;
            if found.is_some() {
                break;
            }
        }
        if found.is_none() {
            mx.first = last;
            found = mx.try_match(true, false)?;
        }
    }
    Ok(found.map(|mut m| {
        m.prefix.0 = begin;
        m
    }))
}

// Translated from MSVC's <regex>: _Format_default (ECMAScript's `$` rules).
/// `fmt` with the match's parts put in, appended to `out`.
pub fn format(m: &Captures, text: &[u8], fmt: &[u8], out: &mut Vec<u8>) {
    let part = |r: (usize, usize)| &text[r.0..r.1];
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'$' {
            out.push(fmt[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&c) = fmt.get(i) else {
            out.push(b'$');
            break;
        };
        match c {
            b'$' => {
                out.push(b'$');
                i += 1;
            }
            b'`' => {
                out.extend_from_slice(part(m.prefix));
                i += 1;
            }
            b'\'' => {
                out.extend_from_slice(part(m.suffix));
                i += 1;
            }
            b'&' => {
                i += 1;
                if let Some(g) = m.groups[0] {
                    out.extend_from_slice(part(g));
                }
            }
            b'0'..=b'9' => {
                let mut num = usize::from(c - b'0');
                i += 1;
                let two = fmt.get(i).is_some_and(u8::is_ascii_digit);
                if two {
                    num = num * 10 + usize::from(fmt[i] - b'0');
                    i += 1;
                }
                if num == 0 {
                    out.extend_from_slice(if two { b"$00" } else { b"$0" });
                } else if num < m.groups.len() {
                    if let Some(g) = m.groups[num] {
                        out.extend_from_slice(part(g));
                    }
                }
            }
            _ => {
                out.push(b'$');
                out.push(c);
                i += 1;
            }
        }
    }
}

// Translated from MSVC's <regex>: _Regex_replace1 (default flags).
/// Every match in `text` replaced by `fmt` ([`format`]), the text between kept.
pub fn replace(re: &Regex, text: &[u8], fmt: &[u8]) -> Result<Vec<u8>, Error> {
    replace_with(re, text, fmt, LIMITS)
}

fn replace_with(re: &Regex, text: &[u8], fmt: &[u8], limits: (i64, i64)) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut flags = 0;
    let mut not_null = 0;
    while let Some(m) = search_with(re, text, pos, flags | not_null, limits)? {
        out.extend_from_slice(&text[m.prefix.0..m.prefix.1]);
        format(&m, text, fmt, &mut out);
        let whole = m.whole();
        pos = whole.1;
        if pos == text.len() {
            break;
        }
        if whole.0 == whole.1 {
            not_null = MATCH_NOT_NULL_AT_START;
        } else {
            not_null = 0;
            flags |= MATCH_PREV_AVAIL;
        }
    }
    out.extend_from_slice(&text[pos..]);
    Ok(out)
}
