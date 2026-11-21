//! Tests of `sv_RegexMatch`, `sv_RegexSearch` and `sv_RegexReplace`, called by their opcodes.
//! Expectations from JIP's bodies (functions_jip/jip_fn_string.h) and MSVC's `std::regex_match`,
//! `std::regex_search` and `std::regex_replace` (the engine's own tests: `ecma_regex_tests.rs`).

use crate::commands::test_support::*;
use engine_core::prelude::*;
use nvse::commands::array::{set_fix, LIST_GUARD};
use nvse::vars::{self, Data};

const MATCH: u32 = 0x2795;
const SEARCH: u32 = 0x2796;
const REPLACE: u32 = 0x2797;

fn setup(guard: bool) -> Engine {
    let mut e = engine();
    game_globals(&mut e);
    set_fix(&mut e, LIST_GUARD, guard);
    e
}

fn var(e: &mut Engine, text: &[u8]) -> i32 {
    vars::add_string(e, 6, text, false) as i32
}

fn res_of(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

fn is_match(e: &mut Engine, text: &[u8], pattern: &[u8]) -> f64 {
    let id = var(e, text);
    res_of(e, MATCH, &[n(id), s(pattern)])
}

fn found(e: &mut Engine, text: &[u8], pattern: &[u8]) -> Vec<Vec<u8>> {
    let id = var(e, text);
    let arr = res_of(e, SEARCH, &[n(id), s(pattern)]);
    array_data(e, arr)
        .into_iter()
        .map(|d| match d {
            Data::Str(t) => t,
            other => panic!("not a string: {other:?}"),
        })
        .collect()
}

fn replaced(e: &mut Engine, text: &[u8], arg: &[u8]) -> Vec<u8> {
    let id = var(e, text);
    let r = res_of(e, REPLACE, &[n(id), s(arg)]);
    string_result(e, r)
}

fn strings(list: &[&[u8]]) -> Vec<Vec<u8>> {
    list.iter().map(|t| t.to_vec()).collect()
}

/// `sv_RegexMatch`: 1 for a match of the whole text, 0 for none, a missing variable or a pattern
/// `std::regex` refuses (with the row; the original stops the game there).
#[test]
fn regex_match_answers_for_the_whole_text() {
    let mut e = setup(true);
    assert_eq!(is_match(&mut e, b"Goodsprings", b"Good\\w+"), 1.0);
    assert_eq!(
        is_match(&mut e, b"Goodsprings", b"Good"),
        0.0,
        "the whole text"
    );
    assert_eq!(
        is_match(&mut e, b"", b"a*"),
        1.0,
        "an empty text is still tried"
    );
    assert_eq!(is_match(&mut e, b"abc", b"(a"), 0.0);
    assert_eq!(
        res_of(&mut e, MATCH, &[n(999_999), s(b".*")]),
        0.0,
        "no such variable"
    );
}

#[test]
#[should_panic(expected = "regex_error")]
fn regex_match_stops_on_a_bad_pattern_without_the_row() {
    let mut e = setup(false);
    is_match(&mut e, b"abc", b"a**");
}

/// `sv_RegexSearch`: each match in turn, every search a new text from the end of the last match
/// (so `^` matches again at each start: JIP's loop); an empty array for no match, an empty text,
/// a missing variable or a refused pattern.
#[test]
fn regex_search_lists_the_matches_in_turn() {
    let mut e = setup(true);
    assert_eq!(
        found(&mut e, b"a1b22c333", b"\\d+"),
        strings(&[b"1", b"22", b"333"])
    );
    assert_eq!(
        found(&mut e, b"aaa", b"^a"),
        strings(&[b"a", b"a", b"a"]),
        "JIP's new texts"
    );
    assert_eq!(found(&mut e, b"abc", b"x"), Vec::<Vec<u8>>::new());
    assert_eq!(
        found(&mut e, b"", b"x*"),
        Vec::<Vec<u8>>::new(),
        "an empty text: no search"
    );
    assert_eq!(found(&mut e, b"abc", b"(b"), Vec::<Vec<u8>>::new());
    let arr = res_of(&mut e, SEARCH, &[n(999_999), s(b"a")]);
    assert!(
        array_data(&e, arr).is_empty(),
        "an array even for no variable"
    );
}

/// With the row an empty match at the start of the text searched no longer loops for ever: the
/// next search starts one byte later, and at the end of the text the search ends.
#[test]
fn regex_search_moves_on_after_an_empty_match() {
    let mut e = setup(true);
    assert_eq!(found(&mut e, b"baa", b"a*"), strings(&[b"", b"aa", b""]));
    assert_eq!(found(&mut e, b"ab", b"x*"), strings(&[b"", b"", b""]));
    assert_eq!(found(&mut e, b"aa", b"a*?"), strings(&[b"", b"", b""]));
    assert_eq!(found(&mut e, b"ab", b"b*$"), strings(&[b"b", b""]));
}

#[test]
#[should_panic(expected = "for ever")]
fn regex_search_stops_on_an_empty_match_without_the_row() {
    let mut e = setup(false);
    found(&mut e, b"baa", b"a*");
}

/// An empty match further in than the start of the text searched moves the next text's start on
/// (JIP's `pEnd - suffix`): JIP's answer, the row on or off, with no stop.
#[test]
fn regex_search_keeps_jips_answer_for_an_empty_match_further_in() {
    for guard in [true, false] {
        let mut e = setup(guard);
        assert_eq!(found(&mut e, b"abc", b"\\B"), strings(&[b"", b""]));
        assert_eq!(found(&mut e, b"xc", b"(?!^)|c+"), strings(&[b"", b"c"]));
        assert_eq!(found(&mut e, b"FooBar", b"(?!^)(?=[A-Z])"), strings(&[b""]));
        assert_eq!(
            found(&mut e, b"FooBBar", b"(?!^)(?=[A-Z])"),
            strings(&[b"", b""])
        );
    }
}

/// With the row, an empty match further in that the next text finds again at its start (where
/// JIP loops for ever) is in the array once. (`$` in a group: a format string starting with `$`
/// names a string variable.)
#[test]
fn regex_search_takes_a_repeated_empty_place_once() {
    let mut e = setup(true);
    assert_eq!(found(&mut e, b"abc", b"(?:$)"), strings(&[b""]));
    assert_eq!(found(&mut e, b"abab", b"(?=b)"), strings(&[b"", b""]));
}

#[test]
#[should_panic(expected = "for ever")]
fn regex_search_stops_on_a_repeated_empty_place_without_the_row() {
    let mut e = setup(false);
    found(&mut e, b"abab", b"(?=b)");
}

/// A runaway pattern (past MSVC's 10,000,000 steps from one place) ends the list there.
#[test]
fn regex_search_keeps_what_it_found_before_a_runaway() {
    let mut e = setup(true);
    let mut text = b"x x ".to_vec();
    text.extend(vec![b'a'; 28]);
    assert_eq!(found(&mut e, &text, b"x|(a*)*b"), strings(&[b"x", b"x"]));
}

/// `sv_RegexReplace "format|pattern"`: the part after the first run of `|` is the pattern, the
/// part before it the format; no `|` or nothing after it, a missing variable or a refused pattern
/// give an empty string.
#[test]
fn regex_replace_uses_the_parts_around_the_bar() {
    let mut e = setup(true);
    assert_eq!(replaced(&mut e, b"banana", b"o|a"), b"bonono");
    assert_eq!(
        replaced(&mut e, b"banana", b"o||a"),
        b"bonono",
        "a run of bars is one"
    );
    assert_eq!(
        // (A format string that starts with `$` is a string variable's name to xNVSE.)
        replaced(&mut e, b"one two", b"<$2 $1>|(\\w+) (\\w+)"),
        b"<two one>"
    );
    assert_eq!(replaced(&mut e, b"abc", b"-|x*"), b"-a-b-c-");
    assert_eq!(replaced(&mut e, b"abc", b"|b"), b"ac", "an empty format");
    assert_eq!(replaced(&mut e, b"abc", b"b"), b"", "no bar");
    assert_eq!(replaced(&mut e, b"abc", b"b|"), b"", "no pattern");
    assert_eq!(replaced(&mut e, b"abc", b"x|(b"), b"", "a refused pattern");
    let r = res_of(&mut e, REPLACE, &[n(999_999), s(b"x|a")]);
    assert_eq!(string_result(&e, r), b"");
}

#[test]
#[should_panic(expected = "regex_error")]
fn regex_replace_stops_on_a_runaway_without_the_row() {
    let mut e = setup(false);
    let text = vec![b'a'; 28];
    replaced(&mut e, &text, b"-|(a*)*b");
}

/// The pattern of `sv_RegexMatch` and `sv_RegexSearch` lives in JIP's 0x80-byte buffer: a longer
/// one runs into JIP's frame (the row reads it whole, without it the port stops).
#[test]
fn a_long_pattern_follows_the_buffer_rule() {
    let mut e = setup(true);
    let long = vec![b'a'; 0x90];
    assert_eq!(is_match(&mut e, &long, &long), 1.0);
}

#[test]
#[should_panic(expected = "buffer")]
fn a_long_pattern_stops_without_the_row() {
    let mut e = setup(false);
    let long = vec![b'a'; 0x80];
    is_match(&mut e, &long, &long);
}
