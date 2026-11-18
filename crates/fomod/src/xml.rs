//! A small XML reader for installer files: elements, attributes and text, as a tree.
//!
//! It takes every file a strict reader takes and keeps going where mods' files break the rules,
//! since a player cannot fix a downloaded installer: a `&` that starts no known reference (a bare
//! `&` in "Textures & Meshes") is kept as written; an end tag that closes nothing is skipped; an
//! element left open is closed at the end; a `<` that starts no tag is text; attribute values may
//! be in either kind of quote or none. Comments, processing instructions (the `<?xml ...?>`
//! declaration among them) and `<!DOCTYPE ...>` are skipped; CDATA is text. Names keep their case;
//! the helpers compare them without case and without a namespace prefix (`xsi:...`).
//!
//! A downloaded file may be hostile, so the work stays in proportion to its size and the tree
//! stays shallow: a file nested deeper than [`MAX_DEPTH`] (a deeper tree would overflow the stack
//! when it is walked or dropped) or with more than [`MAX_ATTRIBUTES`] attributes on an element is
//! refused, a reference is looked for in
//! the 11 bytes after a `&` only, and text runs (stray `<`s among them) make one node.

/// One element: its name as written, its attributes in order (the first of a repeated name
/// kept), and its children.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
}

/// A child of an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

/// The name without a namespace prefix (`xsi:schemaLocation` gives `schemaLocation`).
pub fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

impl Element {
    /// Whether this element's name (without prefix and case) is one of `names`.
    pub fn is(&self, names: &[&str]) -> bool {
        let own = local(&self.name);
        names.iter().any(|n| own.eq_ignore_ascii_case(n))
    }

    /// The value of the attribute `name` (without prefix and case).
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| local(n).eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// The child elements, in order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|c| match c {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    /// The first child element named one of `names`.
    pub fn child(&self, names: &[&str]) -> Option<&Element> {
        self.elements().find(|e| e.is(names))
    }

    /// The child elements named one of `names`, in order.
    pub fn children_named<'a>(&'a self, names: &'a [&'a str]) -> impl Iterator<Item = &'a Element> {
        self.elements().filter(move |e| e.is(names))
    }

    /// All the text inside this element, its descendants' too, in order, as written.
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        for child in &self.children {
            match child {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => e.collect_text(out),
            }
        }
    }
}

/// The text with its references replaced: the five named ones and `&#N;` / `&#xH;`. Anything
/// else after a `&` (no `;` within 10 characters, an unknown name, a number that is no character)
/// is kept as written.
pub fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        // Only the next 11 bytes are looked at: a long run of `&` stays linear.
        let semi = after.as_bytes().iter().take(11).position(|&c| c == b';');
        let found = semi.and_then(|semi| {
            let name = &after[..semi];
            let ch = match name {
                "lt" => Some('<'),
                "gt" => Some('>'),
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => {
                    if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                    } else if let Some(dec) = name.strip_prefix('#') {
                        dec.parse::<u32>().ok().and_then(char::from_u32)
                    } else {
                        None
                    }
                }
            };
            ch.map(|c| (c, semi))
        });
        match found {
            Some((c, semi)) => {
                out.push(c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b':' || b >= 0x80
}

fn is_name_end(b: u8) -> bool {
    b.is_ascii_whitespace() || matches!(b, b'/' | b'>' | b'=' | b'<')
}

/// The deepest elements may nest: a deeper file is refused ([`XmlError::TooDeep`]), so a hostile
/// file cannot make a tree deep enough to overflow the stack when it is walked or dropped. Real
/// installers nest under 15 levels.
pub const MAX_DEPTH: usize = 256;

/// The most attributes one element may have (real installer tags have at most 6); more is refused
/// ([`XmlError::TooManyAttributes`]).
pub const MAX_ATTRIBUTES: usize = 64;

/// Where `pattern` starts in `text` at or after `from`.
fn find(text: &str, from: usize, pattern: &str) -> Option<usize> {
    text.get(from..)?.find(pattern).map(|i| from + i)
}

/// Why a text could not be read as XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlError {
    /// No element in it.
    NoElement,
    /// Elements nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// An element with more than [`MAX_ATTRIBUTES`] attributes.
    TooManyAttributes,
}

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            XmlError::NoElement => "it holds no XML element",
            XmlError::TooDeep => "its elements nest deeper than any installer's",
            XmlError::TooManyAttributes => "an element has more attributes than any installer's",
        })
    }
}

/// The first element of `text` with everything inside it.
pub fn parse(text: &str) -> Result<Element, XmlError> {
    let b = text.as_bytes();
    let len = b.len();
    // The document itself is the bottom of the stack; its first element is the answer.
    let mut stack = vec![Element::default()];
    let mut i = 0;
    // Text joins the text just before it, so stray `<`s and comments make no node each.
    let push_text = |stack: &mut Vec<Element>, t: String| {
        if t.is_empty() {
            return;
        }
        if let Some(top) = stack.last_mut() {
            match top.children.last_mut() {
                Some(Node::Text(before)) => before.push_str(&t),
                _ => top.children.push(Node::Text(t)),
            }
        }
    };
    while i < len {
        if b[i] != b'<' {
            let end = find(text, i, "<").unwrap_or(len);
            push_text(&mut stack, unescape(&text[i..end]));
            i = end;
            continue;
        }
        let rest = &text[i..];
        if rest.starts_with("<!--") {
            i = find(text, i + 4, "-->").map_or(len, |e| e + 3);
        } else if rest.starts_with("<![CDATA[") {
            let end = find(text, i + 9, "]]>");
            push_text(&mut stack, text[i + 9..end.unwrap_or(len)].to_owned());
            i = end.map_or(len, |e| e + 3);
        } else if rest.starts_with("<?") {
            i = find(text, i + 2, "?>").map_or(len, |e| e + 2);
        } else if rest.starts_with("<!") {
            // A DOCTYPE, with its internal subset in brackets.
            let mut depth = 0i32;
            let mut j = i + 2;
            while j < len {
                match b[j] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    b'>' if depth <= 0 => break,
                    _ => {}
                }
                j += 1;
            }
            i = j + 1;
        } else if rest.starts_with("</") {
            let start = i + 2;
            let mut j = start;
            while j < len && !is_name_end(b[j]) {
                j += 1;
            }
            let name = &text[start..j];
            i = find(text, j, ">").map_or(len, |e| e + 1);
            // Close up to the innermost open element of that name; one that closes nothing is
            // skipped.
            if let Some(pos) = stack
                .iter()
                .rposition(|e| e.name.eq_ignore_ascii_case(name))
                .filter(|&p| p > 0)
            {
                while stack.len() > pos {
                    close(&mut stack);
                }
            }
        } else if b.get(i + 1).is_some_and(|&c| is_name_start(c)) {
            let start = i + 1;
            let mut j = start;
            while j < len && !is_name_end(b[j]) {
                j += 1;
            }
            let mut element = Element {
                name: text[start..j].to_owned(),
                ..Element::default()
            };
            let mut empty = false;
            loop {
                while j < len && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= len {
                    break;
                }
                match b[j] {
                    b'>' => {
                        j += 1;
                        break;
                    }
                    b'/' if b.get(j + 1) == Some(&b'>') => {
                        empty = true;
                        j += 2;
                        break;
                    }
                    // A tag that never ends before the next one: end it here.
                    b'<' => break,
                    b'/' | b'=' => {
                        j += 1;
                        continue;
                    }
                    _ => {}
                }
                let name_start = j;
                while j < len && !is_name_end(b[j]) {
                    j += 1;
                }
                let name = text[name_start..j].to_owned();
                let mut k = j;
                while k < len && b[k].is_ascii_whitespace() {
                    k += 1;
                }
                let mut value = String::new();
                if k < len && b[k] == b'=' {
                    k += 1;
                    while k < len && b[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if k < len && (b[k] == b'"' || b[k] == b'\'') {
                        let quote = if b[k] == b'"' { "\"" } else { "'" };
                        let end = find(text, k + 1, quote).unwrap_or(len);
                        value = unescape(&text[k + 1..end]);
                        k = (end + 1).min(len);
                    } else {
                        let v_start = k;
                        while k < len && !b[k].is_ascii_whitespace() && b[k] != b'>' {
                            if b[k] == b'/' && b.get(k + 1) == Some(&b'>') {
                                break;
                            }
                            k += 1;
                        }
                        value = unescape(&text[v_start..k]);
                    }
                    j = k;
                }
                if element.attributes.len() >= MAX_ATTRIBUTES {
                    return Err(XmlError::TooManyAttributes);
                }
                if !element
                    .attributes
                    .iter()
                    .any(|(n, _)| n.eq_ignore_ascii_case(&name))
                {
                    element.attributes.push((name, value));
                }
            }
            i = j;
            // The document is the stack's first entry: `MAX_DEPTH - 1` elements may be open, and an
            // empty one inside the last, so none is deeper than `MAX_DEPTH`.
            if !empty && stack.len() >= MAX_DEPTH {
                return Err(XmlError::TooDeep);
            }
            if empty {
                if let Some(top) = stack.last_mut() {
                    top.children.push(Node::Element(element));
                }
            } else {
                stack.push(element);
            }
        } else {
            push_text(&mut stack, "<".to_owned());
            i += 1;
        }
    }
    while stack.len() > 1 {
        close(&mut stack);
    }
    let document = stack.pop().ok_or(XmlError::NoElement)?;
    document
        .children
        .into_iter()
        .find_map(|c| match c {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
        .ok_or(XmlError::NoElement)
}

/// Ends the innermost open element: it becomes its parent's last child.
fn close(stack: &mut Vec<Element>) {
    if let Some(done) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(Node::Element(done));
        }
    }
}
