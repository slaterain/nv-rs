//! UIO's scan, each expected call derived from `ResolveXMLFileHook`'s branches.

use super::*;

/// A builder that writes down what the scan asks: a few game names and settings.
#[derive(Default)]
struct Record {
    calls: Vec<(u32, Value)>,
    added: Vec<Vec<u8>>,
    looked_up: Vec<Vec<u8>>,
    settings: Vec<Vec<u8>>,
    /// After this many items the builder refuses more.
    room: Option<usize>,
}

fn game_id(name: &[u8]) -> Option<u32> {
    match name {
        b"rect" => Some(0x385),
        b"x" => Some(0xfa1),
        b"copy" => Some(0x7d0),
        b"&true;" => Some(2),
        b"&neg;" => Some(0xffff_fffe),
        _ => None,
    }
}

impl Builder for Record {
    fn lookup_trait(&mut self, name: &[u8]) -> Option<u32> {
        self.looked_up.push(name.to_vec());
        game_id(name)
    }
    fn add_trait_name(&mut self, name: &[u8]) -> Option<u32> {
        self.added.push(name.to_vec());
        if name.first() == Some(&b'_') {
            Some(10000)
        } else {
            game_id(name)
        }
    }
    fn game_setting(&mut self, name: &[u8]) -> Option<Value> {
        self.settings.push(name.to_vec());
        match name {
            b"sHello" => Some(Value::Str(b"Hi there".to_vec())),
            b"fHalf" => Some(Value::Num(0.5)),
            b"iSeven" => Some(Value::Num(7.0)),
            b"sNone" => Some(Value::NoText),
            _ => None,
        }
    }
    fn resolve_trait(&mut self, value_type: u32, value: Value) -> bool {
        if self.room == Some(self.calls.len()) {
            return false;
        }
        self.calls.push((value_type, value));
        true
    }
}

fn run(text: &str) -> Record {
    let mut r = Record::default();
    scan(text.as_bytes(), &mut r);
    r
}

fn calls(text: &str) -> Vec<(u32, Value)> {
    run(text).calls
}

use Value::{Id, Num, Str};

fn s(t: &str) -> Value {
    Str(t.as_bytes().to_vec())
}

#[test]
fn a_tile_with_a_name_and_a_trait() {
    assert_eq!(
        calls(r#"<rect name="a"><x>5</x></rect>"#),
        vec![
            // The tag's id at its first attribute (looked up), then the name.
            (0, Id(0x385)),
            (TAG_NAME, s("a")),
            // `<x>`: no attribute, the name through AddTraitName.
            (0, Id(0xfa1)),
            (TAG_VALUE, Num(5.0)),
            (1, Id(0xfa1)),
            (1, Id(0x385)),
        ]
    );
    let r = run(r#"<rect name="a"><x>5</x></rect>"#);
    assert_eq!(r.looked_up, vec![b"rect".to_vec()]);
    assert_eq!(
        r.added,
        vec![b"x".to_vec(), b"x".to_vec(), b"rect".to_vec()]
    );
}

#[test]
fn src_needs_a_trait_and_a_self_closed_tag_closes() {
    assert_eq!(
        calls(r#"<copy src="me()" trait="x"/>"#),
        vec![
            (0, Id(0x7d0)),
            (TAG_SRC, s("me()")),
            (TAG_TRAIT, Id(0xfa1)),
            // The `/` clears the attribute flag: `>` closes the tag by its name.
            (1, Id(0x7d0)),
        ]
    );
    // `trait` is matched by its four bytes masked with `TRAI` (any case), its `=` seven bytes on.
    assert_eq!(
        calls(r#"<copy src="a" TRAIT="x"/>"#)[2],
        (TAG_TRAIT, Id(0xfa1))
    );
    // No ` trait=`: an error after the src value; the rest of the tag is skipped (the `/` still
    // counts, so `>` closes by the name).
    assert_eq!(
        calls(r#"<copy src="a">"#),
        vec![(0, Id(0x7d0)), (TAG_SRC, s("a"))]
    );
    assert_eq!(
        calls(r#"<copy src="a"/>"#),
        vec![(0, Id(0x7d0)), (TAG_SRC, s("a")), (1, Id(0x7d0))]
    );
    // An unknown trait name: an error.
    assert_eq!(
        calls(r#"<copy src="a" trait="nope">"#),
        vec![(0, Id(0x7d0)), (TAG_SRC, s("a"))]
    );
}

#[test]
fn attribute_errors_skip_the_rest_of_the_tag() {
    // Neither `name=` nor `src=`.
    assert_eq!(
        calls(r#"<rect foo="1"><x>2</x></rect>"#),
        vec![
            (0, Id(0x385)),
            (0, Id(0xfa1)),
            (TAG_VALUE, Num(2.0)),
            (1, Id(0xfa1)),
            (1, Id(0x385)),
        ]
    );
    // `name` with no `=`; an empty value.
    assert_eq!(calls(r#"<rect name"a">"#), vec![(0, Id(0x385))]);
    assert_eq!(calls(r#"<rect name="">"#), vec![(0, Id(0x385))]);
    // `NAME=` is a name (the four bytes ORed with 0x20).
    assert_eq!(calls(r#"<rect NAME="b">"#)[1], (TAG_NAME, s("b")));
    // A tag with an attribute whose name is unknown: nothing, and nothing after it in the tag.
    assert_eq!(calls(r#"<zzz name="a"></zzz>"#), vec![]);
    // An unknown tag name without attributes: nothing.
    assert_eq!(calls("<zzz>1</zzz>"), vec![(TAG_VALUE, Num(1.0))]);
}

#[test]
fn quotes_are_looked_for_past_the_tag_and_a_cut_text_ends_the_scan() {
    // UIO looks for the value's quotes past the `>` (the rest, `</x>`, is skipped: a `<` in a
    // tag).
    assert_eq!(
        calls(r#"<rect name=><x>"q"</x>"#),
        vec![(0, Id(0x385)), (TAG_NAME, s("q"))]
    );
    // The text ends inside a value: the scan ends.
    assert_eq!(calls(r#"<rect name="ab"#), vec![(0, Id(0x385))]);
    assert_eq!(calls(r#"<rect name="#), vec![(0, Id(0x385))]);
    // Cut after ` trai`: the `=` would be past the end (zeros here): an error, nothing read past.
    assert_eq!(
        calls(r#"<copy src="a" trai"#),
        vec![(0, Id(0x7d0)), (TAG_SRC, s("a"))]
    );
    assert_eq!(
        calls(r#"<copy src="a" t"#),
        vec![(0, Id(0x7d0)), (TAG_SRC, s("a"))]
    );
    // A zero in the text ends it.
    assert_eq!(calls("<x>1\0<x>2</x>"), vec![(0, Id(0xfa1))]);
}

#[test]
fn values_between_tags() {
    let value = |t: &str| {
        let r = calls(&format!("<x>{t}</x>"));
        r.iter()
            .find(|(k, _)| *k == TAG_VALUE)
            .map(|(_, v)| v.clone())
    };
    // Game settings: text, float, integer as a float; no text; unknown or other kinds: nothing.
    assert_eq!(value("&-sHello;"), Some(s("Hi there")));
    assert_eq!(value("&-fHalf;"), Some(Num(0.5)));
    assert_eq!(value("&-iSeven;"), Some(Num(7.0)));
    assert_eq!(value("&-sNone;"), Some(Value::NoText));
    assert_eq!(value("&-sMissing;"), None);
    // The name ends at the first `;`; no `;` reads to the end.
    let r = run("<x>&-fHalf;junk</x><x>&-iSeven</x>");
    assert_eq!(r.settings, vec![b"fHalf".to_vec(), b"iSeven".to_vec()]);
    // A trait constant: its id as a number (signed); an unknown name stays text.
    assert_eq!(value("&true;"), Some(Num(2.0)));
    assert_eq!(value("&neg;"), Some(Num(-2.0)));
    assert_eq!(value("&nope;"), Some(s("&nope;")));
    // Numbers: every byte one of `-.0123456789`.
    assert_eq!(value("1.5"), Some(Num(1.5)));
    assert_eq!(value("-1.25"), Some(Num(-1.25)));
    assert_eq!(value("1-2"), Some(Num(1.0)));
    assert_eq!(value("12a"), Some(s("12a")));
    assert_eq!(value(" 12"), Some(s(" 12")));
    // Text from a space up; below a space nothing.
    assert_eq!(value("Hello world"), Some(s("Hello world")));
    assert_eq!(value("\tx"), None);
    // Text before any tag is a value too; text after the last tag is not (no `<` ends it).
    assert_eq!(calls("7<x>"), vec![(TAG_VALUE, Num(7.0)), (0, Id(0xfa1))]);
    assert_eq!(calls("<x>7"), vec![(0, Id(0xfa1))]);
}

#[test]
fn user_traits_and_odd_tags() {
    // `_name` through AddTraitName at `>`; with an attribute it is only looked up.
    let r = run("<_mine>1</_mine>");
    assert_eq!(r.added, vec![b"_mine".to_vec(), b"_mine".to_vec()]);
    assert_eq!(r.calls[0], (0, Id(10000)));
    let r = run(r#"<_mine name="a">"#);
    assert_eq!(r.looked_up, vec![b"_mine".to_vec()]);
    assert_eq!(r.calls, vec![]);
    // A `<` inside a tag: the rest of the tag skipped, its token dropped.
    assert_eq!(
        calls("<rect <x>5</x>"),
        vec![(TAG_VALUE, Num(5.0)), (1, Id(0xfa1))]
    );
    // A `>` outside a tag drops the text before it.
    assert_eq!(calls("5><x>"), vec![(0, Id(0xfa1))]);
    // A space ends the tag's name; a tab does not.
    assert_eq!(calls("<rect\tname=\"a\">"), vec![]);
    // A second space before the attribute: the attribute starts at its first byte.
    assert_eq!(calls(r#"<rect  name="a">"#)[1], (TAG_NAME, s("a")));
}

#[test]
fn a_builder_that_can_make_no_more_stops_the_scan() {
    let mut r = Record {
        room: Some(2),
        ..Record::default()
    };
    scan(br#"<rect name="a"><x>5</x></rect>"#, &mut r);
    assert_eq!(r.calls.len(), 2);
    assert_eq!(r.added, vec![b"x".to_vec()]);
}

#[test]
fn uios_number_reading() {
    assert_eq!(str_to_dbl(b"5"), 5.0);
    assert_eq!(str_to_dbl(b"-5"), -5.0);
    assert_eq!(str_to_dbl(b".5"), 0.5);
    assert_eq!(str_to_dbl(b"-.5"), -0.5);
    assert_eq!(str_to_dbl(b"-5.5"), -5.5);
    // "-0": neither part counts, so no sign.
    assert!(str_to_dbl(b"-0").is_sign_positive());
    assert!(str_to_dbl(b"-0.0").is_sign_positive());
    // Nine fraction digits at most.
    assert_eq!(str_to_dbl(b"0.1234567891"), 123456789.0 * 1.0e-09);
    assert_eq!(str_to_dbl(b"1..5"), 1.0);
    // 32-bit unsigned: the top half by +2^32; a wrap that comes out smaller gives ±2^32, one
    // that comes out larger is kept (705032704 for 5000000000).
    assert_eq!(str_to_dbl(b"4294967295"), 4294967295.0);
    assert_eq!(str_to_dbl(b"4294967296"), 4294967296.0);
    assert_eq!(str_to_dbl(b"-4294967296"), -4294967296.0);
    assert_eq!(str_to_dbl(b"5000000000"), 705032704.0);
    assert_eq!(str_to_dbl(b""), 0.0);
    assert_eq!(str_to_dbl(b"-"), 0.0);
    assert_eq!(str_to_int(b"12"), 12);
    assert_eq!(str_to_int(b"-12"), -12);
    assert_eq!(str_to_int(b"x"), 0);
    assert_eq!(str_to_int(b"7x"), 7);
    assert_eq!(str_to_int(b"2147483647"), i32::MAX);
    assert_eq!(str_to_int(b"2147483648"), i32::MAX);
    assert_eq!(str_to_int(b"-2147483648"), i32::MIN);
}
