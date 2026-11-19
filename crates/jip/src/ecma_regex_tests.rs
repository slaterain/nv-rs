//! Tests of the MSVC regular expressions: each expectation follows from MSVC's `<regex>` code
//! (the parser's error paths, the matcher's steps) and, where they agree, from ECMAScript; the
//! cases where MSVC differs from ECMAScript (JavaScript) say so.

use super::*;

fn re(p: &str) -> Regex {
    Regex::new(p.as_bytes()).unwrap_or_else(|e| panic!("{p:?}: {e:?}"))
}

fn err(p: &[u8]) -> Error {
    Regex::new(p)
        .err()
        .unwrap_or_else(|| panic!("{p:?} compiled"))
}

/// The first match's group 0 in `text`, or `None`.
fn find(p: &str, text: &[u8]) -> Option<(usize, usize)> {
    search(&re(p), text, 0, 0).unwrap().map(|m| m.whole())
}

fn groups(p: &str, text: &[u8]) -> Vec<Option<(usize, usize)>> {
    search(&re(p), text, 0, 0).unwrap().expect("a match").groups
}

fn matches(p: &str, text: &[u8]) -> bool {
    full_match(&re(p), text).unwrap()
}

fn replaced(p: &str, text: &[u8], fmt: &[u8]) -> Vec<u8> {
    replace(&re(p), text, fmt).unwrap()
}

/// Runs `f` on a thread with a large stack (the deep tests nest a thousand steps in debug code).
fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn the_parser_refuses_what_msvc_refuses() {
    let cases: &[(&[u8], Error)] = &[
        (b"(", Error::Paren),
        (b"(a", Error::Paren),
        (b")", Error::Paren),
        (b"a)", Error::Paren),
        (b"[a", Error::Brack),
        (b"]", Error::Brack),
        (b"a]", Error::Brack),
        (b"}", Error::Brace),
        (b"a{", Error::Badbrace),
        (b"a{,3}", Error::Badbrace),
        (b"a{3,2}", Error::Badbrace),
        (b"a{2", Error::Badbrace),
        (b"a{2,", Error::Badbrace),
        (b"a{99999999999}", Error::Badbrace),
        (b"*a", Error::Badrepeat),
        (b"a**", Error::Badrepeat),
        (b"a+*", Error::Badrepeat),
        (b"^*", Error::Badrepeat),
        (b"$+", Error::Badrepeat),
        (b"\\b?", Error::Badrepeat),
        (b"\\B{2}", Error::Badrepeat),
        (b"(?=a)*", Error::Badrepeat),
        (b"(?!a)+", Error::Badrepeat),
        (b"a|*", Error::Badrepeat),
        (b"(*)", Error::Badrepeat),
        (b"\\", Error::Escape),
        (b"a\\", Error::Escape),
        (b"\\c", Error::Escape),
        (b"\\c1", Error::Escape),
        (b"\\x4", Error::Escape),
        (b"\\x4g", Error::Escape),
        (b"\\u0100", Error::Escape),
        (b"\\u12", Error::Escape),
        (b"[\\1]", Error::Escape),
        (b"\\1", Error::Backref),
        (b"(a)\\2", Error::Backref),
        (b"(a\\1)", Error::Backref),
        (b"\\1(a)", Error::Backref),
        (b"(a)\\99999999999", Error::Backref),
        (b"[b-a]", Error::Range),
        (b"[\\d-z]", Error::Range),
        (b"[a-\\d]", Error::Range),
        (b"[[:alpha:]-z]", Error::Range),
        (b"[[:foo:]]", Error::Ctype),
        (b"[[:alpha]]", Error::Ctype),
        (b"[[:al=pha:]]", Error::Ctype),
        (b"[[:alpha:]", Error::Brack),
        (b"{", Error::Badrepeat),
        (b"a{x}", Error::Badbrace),
        (b"[[==]]", Error::Collate),
        (b"[[..]]", Error::Collate),
        (b"[[=a]]", Error::Collate),
        (b"(?<a)", Error::Syntax),
        (b"(?", Error::Syntax),
        (b"(?a)", Error::Syntax),
    ];
    for &(p, want) in cases {
        assert_eq!(err(p), want, "{:?}", String::from_utf8_lossy(p));
    }
}

#[test]
fn the_parser_accepts_what_msvc_accepts() {
    for p in [
        "",
        "a|",
        "|a",
        "|",
        "()",
        "(|)",
        "(?:)",
        "a{0}",
        "a{2,}",
        "a{1,1}",
        "a{0,1}",
        "[]",
        "[^]",
        "[a-]",
        "[-a]",
        "[\\d-]",
        "[\\b]",
        "[\\\\]",
        "[\\0]",
        "\\0",
        "\\00",
        "(a)\\1",
        "(a)\\01",
        "(a)(b)\\2\\1",
        "\\cA\\cz",
        "\\x41\\xfF",
        "\\u00e9",
        "[[:alpha:][:digit:]]",
        "[[:ALPHA:]]",
        "[[:d:][:w:][:s:]]",
        "[[=a=]]",
        "[[=ab=]]",
        "[[.a.]]",
        "[[.ab.]]",
        "\\-\\,\\:\\=\\!",
        "\\q",
        "a*?",
        "a+?",
        "a??",
        "a{2,3}?",
        "(?=a)",
        "(?!a)",
        "^$",
        "\\b\\B",
        "\n\r",
        ",",
        ":",
        "=",
        "!",
        "-",
    ] {
        assert!(Regex::new(p.as_bytes()).is_ok(), "{p:?}");
    }
}

/// JIP's DLL has no limit on the groups (later MSVC refuses the 1000th); the port refuses a
/// nesting deeper than 10,000 (its own limit).
#[test]
fn groups_are_counted_without_a_limit_and_nested_up_to_10000() {
    assert_eq!(re("(a)(?:b)((c)(?=d))").mark_count(), 3);
    assert_eq!(re("").mark_count(), 0);
    let p1500 = "()".repeat(1500);
    assert_eq!(Regex::new(p1500.as_bytes()).unwrap().mark_count(), 1500);
    deep(|| {
        // Each group's alternative nests the rest of the match one step deeper (MSVC's way).
        assert!(matches(&"(a)".repeat(997), &[b'a'; 997]));
        let r = re(&"(a)".repeat(998));
        assert_eq!(full_match(&r, &[b'a'; 998]), Err(Error::Stack));
        let nest = |n: usize, open: &str| format!("{}a{}", open.repeat(n), ")".repeat(n));
        assert!(Regex::new(nest(MAX_NESTING as usize, "(").as_bytes()).is_ok());
        assert!(Regex::new(nest(MAX_NESTING as usize, "(?:").as_bytes()).is_ok());
        assert!(Regex::new(nest(MAX_NESTING as usize, "(?=").as_bytes()).is_ok());
        assert_eq!(
            err(nest(MAX_NESTING as usize + 1, "(").as_bytes()),
            Error::TooDeep
        );
        assert_eq!(
            err(nest(MAX_NESTING as usize + 1, "(?:").as_bytes()),
            Error::TooDeep
        );
        // Deep but parsed: every way through it passes the 1000 nested steps...
        let r = Regex::new(nest(998, "(?:").as_bytes()).unwrap();
        assert_eq!(full_match(&r, b"a"), Err(Error::Stack));
        let r = Regex::new(nest(997, "(?:").as_bytes()).unwrap();
        assert_eq!(full_match(&r, b"a"), Ok(true));
        // ...unless a branch before it matches.
        let r = Regex::new(format!("x|{}", nest(3000, "(")).as_bytes()).unwrap();
        assert_eq!(full_match(&r, b"x"), Ok(true));
    });
}

#[test]
fn the_dot_refuses_line_feed_and_carriage_return_only() {
    assert!(matches("a.c", b"abc"));
    assert!(!matches("a.c", b"a\nc"));
    assert!(!matches("a.c", b"a\rc"));
    assert!(matches("a.c", b"a\x0bc"));
    assert!(matches("a.c", b"a\x80c"));
    assert!(matches("a.c", b"a\xffc"));
    assert!(!matches(".", b""));
}

#[test]
fn anchors_match_at_every_line_feed() {
    // MSVC has no multiline switch: `^` after and `$` before every `\n`, always.
    assert_eq!(find("^b", b"a\nb"), Some((2, 3)));
    assert_eq!(find("^b", b"a\r\nb"), Some((3, 4)));
    assert_eq!(
        find("^b", b"a\rb"),
        None,
        "a carriage return is no line break"
    );
    assert_eq!(find("^b", b"ab"), None);
    assert_eq!(find("a$", b"a\nb"), Some((0, 1)));
    assert_eq!(find("a$", b"a\r\nb"), None);
    assert_eq!(find("$", b"ab\n"), Some((2, 2)), "before a final line feed");
    assert_eq!(find("$", b"ab"), Some((2, 2)));
    assert!(matches("a$\\n^b", b"a\nb"));
    assert!(matches("^$", b""));
}

#[test]
fn classes_are_the_c_locales_ascii_ones() {
    let all: Vec<u8> = (1..=255).collect();
    let members = |p: &str| -> Vec<u8> {
        let r = re(p);
        all.iter()
            .copied()
            .filter(|&c| full_match(&r, &[c]).unwrap())
            .collect()
    };
    let range = |a: u8, b: u8| (a..=b).collect::<Vec<u8>>();
    let digits = range(b'0', b'9');
    let upper = range(b'A', b'Z');
    let lower = range(b'a', b'z');
    let mut word = [digits.clone(), upper.clone(), vec![b'_'], lower.clone()].concat();
    word.sort_unstable();
    assert_eq!(members("\\d"), digits);
    assert_eq!(members("[[:digit:]]"), digits);
    assert_eq!(members("\\w"), word);
    assert_eq!(members("[[:w:]]"), word);
    let space = [range(9, 13), vec![b' ']].concat();
    assert_eq!(members("\\s"), space);
    assert_eq!(members("[[:space:]]"), space);
    assert_eq!(
        members("[[:blank:]]"),
        space,
        "MSVC's blank mask is its space mask"
    );
    assert_eq!(
        members("[[:alpha:]]"),
        [upper.clone(), lower.clone()].concat()
    );
    assert_eq!(
        members("[[:alnum:]]"),
        [digits.clone(), upper.clone(), lower.clone()].concat()
    );
    assert_eq!(members("[[:upper:]]"), upper);
    assert_eq!(members("[[:lower:]]"), lower);
    assert_eq!(members("[[:print:]]"), range(0x20, 0x7e));
    assert_eq!(members("[[:graph:]]"), range(0x21, 0x7e));
    assert_eq!(
        members("[[:cntrl:]]"),
        [range(1, 0x1f), vec![0x7f]].concat()
    );
    let punct: Vec<u8> = range(0x21, 0x7e)
        .into_iter()
        .filter(|c| !c.is_ascii_alphanumeric())
        .collect();
    assert_eq!(members("[[:punct:]]"), punct);
    assert_eq!(
        members("[[:xdigit:]]"),
        [digits.clone(), range(b'A', b'F'), range(b'a', b'f')].concat()
    );
    // The upper-case escapes are the complements, 0x80-0xFF included.
    let not =
        |set: &[u8]| -> Vec<u8> { all.iter().copied().filter(|c| !set.contains(c)).collect() };
    assert_eq!(members("\\D"), not(&digits));
    assert_eq!(members("\\W"), not(&word));
    assert_eq!(members("\\S"), not(&space));
    assert_eq!(members("[\\D]"), not(&digits));
    assert_eq!(members("[^\\D]"), digits);
    assert_eq!(members("[^\\d]"), not(&digits));
    // Equivalence: the byte made lower case; a name of two bytes is no byte.
    assert_eq!(members("[[=a=]]"), vec![b'A', b'a']);
    assert_eq!(members("[[=ab=]]"), Vec::<u8>::new());
}

#[test]
fn high_bytes_are_not_word_bytes() {
    assert_eq!(find("\\w+", b"\xe9t\xe9"), Some((1, 2)));
    assert_eq!(find("\\bt", b"\xe9t"), Some((1, 2)));
    assert_eq!(find("\\Bt", b"\xe9t"), None);
    assert!(matches("[\\xe0-\\xff]+", b"\xe9\xff\xe0"));
    assert!(matches("\\u00e9", b"\xe9"));
}

#[test]
fn escapes_follow_msvcs_parser() {
    assert!(matches("\\x41\\u0042\\cC", b"AB\x03"));
    assert!(matches("\\cc", b"\x03"));
    assert!(matches("\\f\\n\\r\\t\\v", b"\x0c\n\r\t\x0b"));
    // Inside brackets `\b` is the letter b (an identity escape), not a backspace.
    assert!(matches("[\\b]", b"b"));
    assert!(!matches("[\\b]", b"\x08"));
    assert!(matches("[\\B]", b"B"));
    // Any other byte escapes to itself.
    assert!(matches(
        "\\q\\.\\*\\[\\]\\(\\)\\{\\}\\|\\^\\$\\\\",
        b"q.*[](){}|^$\\"
    ));
    // `\01` reads both digits: a back-reference to group 1.
    assert!(matches("(a)\\01", b"aa"));
    assert!(!matches("(a)\\01", b"a\x001"));
    // Ranges with escapes.
    assert!(matches("[\\x30-\\x39]+", b"0123456789"));
    assert!(matches("[a-]", b"-"));
    assert!(matches("[\\d-]", b"-"));
    assert!(!matches("[\\d-]", b"a"));
}

#[test]
fn alternatives_take_the_first_that_matches() {
    assert_eq!(find("a|ab", b"ab"), Some((0, 1)));
    assert_eq!(find("ab|a", b"ab"), Some((0, 2)));
    assert_eq!(find("a|", b"b"), Some((0, 0)), "an empty alternative");
    assert_eq!(find("|a", b"a"), Some((0, 0)));
    assert!(matches("|a", b""));
    assert!(
        matches("|a", b"a"),
        "the whole-text match backtracks into the second"
    );
    assert!(matches("()", b""));
    assert!(matches("x(|y)z", b"xz"));
    assert!(matches("x(|y)z", b"xyz"));
}

#[test]
fn quantifiers_count_as_msvc() {
    assert_eq!(find("a{2,}", b"aaaa"), Some((0, 4)));
    assert_eq!(find("a{2,3}", b"aaaa"), Some((0, 3)));
    assert_eq!(find("a{2}", b"aaaa"), Some((0, 2)));
    assert_eq!(find("a{0}", b"aaaa"), Some((0, 0)));
    assert_eq!(find("a{0,1}", b"aa"), Some((0, 1)));
    assert_eq!(find("a{2,}?", b"aaaa"), Some((0, 2)));
    assert_eq!(find("a*?", b"aaa"), Some((0, 0)));
    assert_eq!(find("a+?", b"aaa"), Some((0, 1)));
    assert_eq!(find("a??", b"a"), Some((0, 0)));
    assert_eq!(find("a?", b"a"), Some((0, 1)));
    assert!(matches("a*?b", b"aaab"));
    assert!(matches("(ab){2,3}", b"ababab"));
    assert!(!matches("(ab){2,3}", b"abababab"));
    assert!(matches("(?:a|b){3}", b"bab"));
    assert!(matches("(a*)*", b"aaa"));
    assert!(matches("(a*)+b", b"b"));
    assert!(matches("(?:a?)*b", b"aab"));
    // A string's last byte takes the quantifier: `ab*` is `a` then `b*`.
    assert!(matches("ab*", b"abbb"));
    assert!(!matches("ab*", b"abab"));
    // A lazy loop with a group takes the fewest repeats.
    assert_eq!(find("(?:a)*?", b"aa"), Some((0, 0)));
    assert_eq!(find("(?:a)+?", b"aa"), Some((0, 1)));
    // A repeat that takes nothing ends the loop (and the required repeats) at once: twenty
    // million required repeats of an empty back-reference take one step, not twenty million.
    assert!(matches("()\\1*", b""));
    assert!(matches("()\\1{20000000}", b""));
    // The largest count: the simple loop's count stops at it (it is not taken past `i32::MAX`,
    // which a debug build would refuse).
    assert!(matches("(?:){2147483647}", b""));
    assert_eq!(find("(?:){2147483647}", b"abc"), Some((0, 0)));
}

/// A greedy loop of one simple part goes on after a match and keeps the longest overall match
/// (MSVC's `_Better_match`); ECMAScript takes `aa` here.
#[test]
fn a_simple_greedy_loop_keeps_the_longest_match() {
    assert_eq!(find("a*(ab)?", b"aab"), Some((0, 3)));
    assert_eq!(groups("a*(ab)?", b"aab")[1], Some((1, 3)));
    // With the loop in a group (not simple) the first match is taken, as ECMAScript does.
    assert_eq!(find("(?:a)*(ab)?", b"aab"), Some((0, 2)));
}

/// Entering a group forgets the groups numbered after it, not the ones before (ECMAScript forgets
/// every group in the loop at each repeat).
#[test]
fn groups_in_loops_keep_msvcs_values() {
    assert_eq!(
        groups("(?:(a)|b)*", b"ab"),
        vec![Some((0, 2)), Some((0, 1))]
    );
    assert_eq!(
        groups("(?:(a)|(b))*", b"ab"),
        vec![Some((0, 2)), Some((0, 1)), Some((1, 2))]
    );
    assert_eq!(
        groups("(?:(a)|(b))*", b"ba"),
        vec![Some((0, 2)), Some((1, 2)), None]
    );
    // An empty repeat ends the loop but its group stays matched (ECMAScript: no part).
    assert_eq!(groups("(a*)*", b"b"), vec![Some((0, 0)), Some((0, 0))]);
}

#[test]
fn back_references_match_what_the_group_took() {
    assert!(matches("(a|b)\\1", b"aa"));
    assert!(!matches("(a|b)\\1", b"ab"));
    // A group that took no part, or took nothing, matches the empty text.
    assert!(matches("(a)?b\\1", b"b"));
    assert!(matches("(a)|b\\1", b"b"));
    assert!(matches("(a*)b\\1c", b"bc"));
    assert!(matches("(a*)b\\1c", b"aabaac"));
    assert!(!matches("(a*)b\\1c", b"aabac"));
    // A back-reference that does not match fails the way through it (no empty match instead).
    assert_eq!(find("(a)b\\1", b"abc"), None);
}

#[test]
fn look_aheads_test_without_taking() {
    assert_eq!(find("a(?=b)", b"ab"), Some((0, 1)));
    assert_eq!(find("a(?!b)", b"ab"), None);
    assert_eq!(find("a(?!b)", b"ac"), Some((0, 1)));
    assert_eq!(groups("(?=(a))a", b"a"), vec![Some((0, 1)), Some((0, 1))]);
    assert!(matches("(?=.*b)a.", b"ab"));
}

#[test]
fn word_boundaries() {
    assert_eq!(find("\\bfoo\\b", b"a foo b"), Some((2, 5)));
    assert_eq!(find("\\bfoo\\b", b"afoo"), None);
    assert_eq!(find("\\Boo", b"foo"), Some((1, 3)));
    // `_` is a word byte: no boundary between `a` and `_`, one between `_` and `-`.
    assert_eq!(find("\\b_", b"a_"), None);
    assert_eq!(find("_\\b", b"a_-"), Some((1, 2)));
    assert!(matches("\\b", b""), "an empty text is a word boundary");
    assert_eq!(find("\\b", b"a"), Some((0, 0)));
    assert_eq!(find("\\B", b""), None);
}

/// MSVC's collating lookup only counts an element that ends at the text's end (its compare steps
/// past a byte that differs), so `[[.a.]]` takes any last byte.
#[test]
fn collating_elements_follow_msvcs_lookup() {
    assert_eq!(find("[[.a.]]", b"xa"), Some((1, 2)));
    assert_eq!(find("[[.a.]]", b"ax"), Some((1, 2)));
    assert_eq!(find("[[.a.]]", b"ab"), Some((1, 2)));
    assert!(!matches("[[.a.]]b", b"ab"));
    assert!(!matches("[^[.a.]]", b"b"));
}

#[test]
fn whole_text_matches_backtrack_to_the_end() {
    assert!(matches("a*", b"aaa"));
    assert!(matches("a*?", b"aaa"));
    assert!(matches("(a|ab)(c|bcd)(d*)", b"abcd"));
    assert!(!matches("a", b"ab"));
    assert!(!matches("b", b"ab"));
    assert!(matches("", b""));
    assert!(!matches("", b"a"));
}

#[test]
fn search_reports_the_prefix_and_suffix() {
    let m = search(&re("b+"), b"aabbc", 0, 0).unwrap().unwrap();
    assert_eq!(m.groups, vec![Some((2, 4))]);
    assert_eq!(m.prefix, (0, 2));
    assert_eq!(m.suffix, (4, 5));
    let m = search(&re("b+"), b"aabbc", 1, 0).unwrap().unwrap();
    assert_eq!(m.prefix, (1, 2), "from where the search started");
    assert_eq!(search(&re("x"), b"aabbc", 0, 0).unwrap(), None);
    // Unmatched groups.
    let m = search(&re("(x)|b"), b"ab", 0, 0).unwrap().unwrap();
    assert_eq!(m.groups, vec![Some((1, 2)), None]);
}

#[test]
fn search_flags_change_the_start() {
    let r = re("^a");
    // Started inside the text, `^` still matches there (no earlier byte is read)...
    assert_eq!(
        search(&r, b"ba", 1, 0).unwrap().map(|m| m.whole()),
        Some((1, 2))
    );
    // ...unless the earlier byte is said to be there.
    assert_eq!(search(&r, b"ba", 1, MATCH_PREV_AVAIL).unwrap(), None);
    assert_eq!(
        search(&r, b"b\na", 2, MATCH_PREV_AVAIL)
            .unwrap()
            .map(|m| m.whole()),
        Some((2, 3))
    );
    assert_eq!(search(&r, b"a", 0, MATCH_NOT_BOL).unwrap(), None);
    assert_eq!(search(&re("a$"), b"a", 0, MATCH_NOT_EOL).unwrap(), None);
    assert_eq!(search(&re("x*"), b"ab", 0, MATCH_NOT_NULL).unwrap(), None);
    // The replace loop's flag refuses the empty text only at the start.
    assert_eq!(
        search(&re("x*"), b"ab", 0, MATCH_NOT_NULL_AT_START)
            .unwrap()
            .map(|m| m.whole()),
        Some((1, 1))
    );
    assert_eq!(search(&re("b"), b"ab", 0, MATCH_CONTINUOUS).unwrap(), None);
    assert_eq!(
        search(&re("\\bb"), b"ab", 1, 0).unwrap().map(|m| m.whole()),
        Some((1, 2))
    );
    assert_eq!(
        search(&re("\\bb"), b"ab", 1, MATCH_PREV_AVAIL).unwrap(),
        None
    );
    assert_eq!(search(&re("\\b"), b"a", 1, MATCH_NOT_EOW).unwrap(), None);
    assert_eq!(
        search(&re("\\b"), b"a", 0, MATCH_NOT_BOW | MATCH_CONTINUOUS).unwrap(),
        None
    );
}

/// Every start place is tried in order (the skip ahead only passes places that cannot start a
/// match).
#[test]
fn search_finds_the_leftmost_start() {
    assert_eq!(find("b|c", b"aaacb"), Some((3, 4)));
    assert_eq!(find("[xy]z", b"axayz"), Some((3, 5)));
    assert_eq!(find("^c|b", b"ab\nc"), Some((1, 2)));
    assert_eq!(find("^c", b"ab\nc"), Some((3, 4)));
    assert_eq!(find("$", b"a\nb"), Some((1, 1)));
    assert_eq!(find("[^a]", b"aab"), Some((2, 3)));
    assert_eq!(find("(?:x|y)", b"aay"), Some((2, 3)));
    assert_eq!(find("x*", b"ab"), Some((0, 0)));
    assert_eq!(find("b*$", b"ab"), Some((1, 2)));
}

#[test]
fn the_format_puts_in_msvcs_parts() {
    let r = re("(b)(c)?");
    let text = b"abd";
    let m = search(&r, text, 0, 0).unwrap().unwrap();
    let f = |fmt: &[u8]| {
        let mut out = Vec::new();
        format(&m, text, fmt, &mut out);
        out
    };
    assert_eq!(f(b"[$&]"), b"[b]");
    assert_eq!(
        f(b"[$1][$2]"),
        b"[b][]",
        "a group with no part gives nothing"
    );
    assert_eq!(f(b"[$01]"), b"[b]");
    assert_eq!(f(b"[$0][$00]"), b"[$0][$00]");
    assert_eq!(f(b"[$3][$12]"), b"[][]", "past the group count: nothing");
    assert_eq!(f(b"[$10]"), b"[]", "two digits are taken");
    assert_eq!(f(b"[$`][$']"), b"[a][d]");
    assert_eq!(f(b"$$ $x $"), b"$ $x $");
    assert_eq!(f(b""), b"");
}

#[test]
fn replace_follows_msvcs_loop() {
    assert_eq!(replaced("a", b"banana", b"o"), b"bonono");
    assert_eq!(replaced("x*", b"abc", b"-"), b"-a-b-c-");
    assert_eq!(replaced("b*", b"abc", b"-"), b"-a--c-");
    assert_eq!(replaced("z", b"abc", b"-"), b"abc");
    // After a match the next search knows the byte before it: `^` does not match again.
    assert_eq!(replaced("^a", b"aaa", b"b"), b"baa");
    // `$`` is the text since the last match, not since the start.
    assert_eq!(replaced("b", b"abcb", b"[$`]"), b"a[a]c[c]");
    assert_eq!(replaced("(\\w+) (\\w+)", b"one two", b"$2 $1"), b"two one");
    assert_eq!(replaced("", b"", b"x"), b"x");
}

#[test]
fn the_limits_are_msvcs() {
    assert_eq!((MAX_STACK, MAX_COMPLEXITY), (1000, 10_000_000));
    // One attempt at "a" takes two steps (the pattern and its one alternative): a limit of 2 refuses
    // it, 3 lets it through.
    let r = re("a");
    assert_eq!(full_match_with(&r, b"a", (1000, 2)), Err(Error::Complexity));
    assert_eq!(full_match_with(&r, b"a", (1000, 3)), Ok(true));
    assert_eq!(full_match_with(&r, b"a", (2, 100)), Err(Error::Stack));
    assert_eq!(full_match_with(&r, b"a", (3, 100)), Ok(true));
    // The count starts again at each start place.
    let r = re("b");
    assert_eq!(
        search_with(&r, b"aaab", 0, 0, (1000, 3))
            .unwrap()
            .map(|m| m.whole()),
        Some((3, 4))
    );
    let r = re("a|b");
    assert_eq!(
        search_with(&r, b"xxxb", 0, 0, (1000, 4))
            .unwrap()
            .map(|m| m.whole()),
        Some((3, 4))
    );
    // A runaway pattern gives up with the complexity error instead of running on.
    let r = re("(a*)*b");
    assert_eq!(
        full_match_with(&r, b"aaaaaaaaaaaaaaaaaaaa", (1000, 100_000)),
        Err(Error::Complexity)
    );
    assert_eq!(
        replace_with(&r, b"aaaaaaaaaaaaaaaaaaaa", b"", (1000, 100_000)),
        Err(Error::Complexity)
    );
}

/// `(?:a|b)*` nests two steps per repeat after two for the pattern itself: 497 repeats stay under
/// MSVC's 1000 nested steps (998 deep), 498 reach them.
#[test]
fn the_nesting_limit_is_reached_where_msvcs_is() {
    deep(|| {
        let r = re("(?:a|b)*");
        let ok = vec![b'a'; 497];
        assert_eq!(full_match(&r, &ok), Ok(true));
        let over = vec![b'a'; 498];
        assert_eq!(full_match(&r, &over), Err(Error::Stack));
        assert_eq!(search(&r, &over, 0, 0), Err(Error::Stack));
        // A simple loop does not nest: a long run is no trouble.
        let r = re("a*b");
        assert_eq!(
            full_match(&r, &[vec![b'a'; 20_000], vec![b'b']].concat()),
            Ok(true)
        );
    });
}
